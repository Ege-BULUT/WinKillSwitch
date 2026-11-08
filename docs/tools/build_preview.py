"""Writes preview.html at the repository root: the README rendered as on GitHub, and the landing
page in a device frame. Run from the repository root after editing README.md or site/:

    py -3 docs/tools/build_preview.py
"""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
readme = (root / "README.md").read_text(encoding="utf-8").replace("</script", "<\\/script")
template = (root / "docs" / "tools" / "preview_template.html").read_text(encoding="utf-8")
(root / "preview.html").write_text(template.replace("/*__README__*/", readme), encoding="utf-8")
print("wrote", root / "preview.html")
