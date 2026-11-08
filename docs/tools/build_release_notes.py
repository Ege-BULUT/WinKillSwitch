"""Rebuild site/release-notes/index.html from the GitHub release notes.

Usage: py -3 docs/tools/build_release_notes.py
Needs the GitHub CLI (gh). Everything between the page's RELEASES markers is replaced; the rest
of the page (styles, nav, footer) is kept as written.
"""

import html
import json
import re
import subprocess
from datetime import datetime
from pathlib import Path

REPO = "Ege-BULUT/WinKillSwitch"
PAGE = Path(__file__).resolve().parents[2] / "site" / "release-notes" / "index.html"
START, END = "<!-- RELEASES -->", "<!-- /RELEASES -->"


def gh(*args):
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True, encoding="utf-8").stdout


def inline(s):
    s = html.escape(s, quote=False)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"\[([^\]]+)\]\((https?://[^)\s]+)\)", r'<a href="\2">\1</a>', s)
    return s


def body_html(md):
    """The release body as HTML, without its Download section (the page has its own)."""
    out, lists = [], 0
    def close_to(depth):
        nonlocal lists
        while lists > depth:
            out.append("</ul>")
            lists -= 1
    for raw in md.replace("\r\n", "\n").split("\n"):
        line = raw.rstrip()
        if re.match(r"^#{2,3} Download\b", line):
            break
        m = re.match(r"^(\s*)- (.*)$", line)
        if m:
            depth = len(m.group(1)) // 2 + 1
            while lists < depth:
                out.append("<ul>")
                lists += 1
            close_to(depth)
            out.append(f"<li>{inline(m.group(2))}</li>")
            continue
        close_to(0)
        if not line.strip():
            continue
        h = re.match(r"^(#{2,3}) (.*)$", line)
        if h:
            out.append(f"<h3>{inline(h.group(2))}</h3>")
        else:
            out.append(f"<p>{inline(line)}</p>")
    close_to(0)
    return "\n    ".join(out)


def sha_of(md):
    m = re.search(r"\b([0-9a-f]{64})\s+WinKillSwitch\.exe", md)
    return m.group(1) if m else ""


def main():
    rels = json.loads(gh("release", "list", "--repo", REPO, "--limit", "100", "--json", "tagName,publishedAt,name,isLatest"))
    parts = []
    for r in rels:
        tag = r["tagName"]
        body = json.loads(gh("release", "view", tag, "--repo", REPO, "--json", "body"))["body"]
        d = datetime.fromisoformat(r["publishedAt"].replace("Z", "+00:00"))
        date = f"{d:%B} {d.day}, {d.year}"
        title = r["name"].split(": ", 1)[1] if ": " in r["name"] else ""
        first = re.search(r"^#{2,3} (.*)$", body, re.M)
        if first and title and first.group(1).strip().lower().rstrip("?") == title.lower().rstrip("?"):
            title = ""  # the notes open with the same heading
        badge = '<span class="tag latest">Latest</span>' if r.get("isLatest") else ""
        sha = sha_of(body)
        parts.append(f"""  <article class="rel glass" id="{tag}">
    <header><h2>{tag}</h2>{badge}<span class="date">{date}</span></header>
    {f'<p class="lede-sm">{html.escape(title.capitalize() if title.islower() else title)}</p>' if title else ''}
    {body_html(body)}
    <div class="dl">
      <a class="btn" href="https://github.com/{REPO}/releases/download/{tag}/WinKillSwitch.exe">Download {tag}</a>
      {f'<span class="sha">SHA-256 {sha}</span>' if sha else ''}
    </div>
  </article>""")
    page = PAGE.read_text(encoding="utf-8")
    a, b = page.index(START) + len(START), page.index(END)
    page = page[:a] + "\n" + "\n\n".join(parts) + "\n  " + page[b:]
    PAGE.write_text(page, encoding="utf-8", newline="\n")
    print(f"wrote {len(rels)} releases to {PAGE}")


if __name__ == "__main__":
    main()
