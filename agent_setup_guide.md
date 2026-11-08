# Agent setup guide

This page is for AI agents (Claude Code, Codex, Cursor and similar) asked to install WinKillSwitch for a user. It gets you from nothing to a verified, running copy of the latest release, and ends with a report template. Every step has a check; do not move on until it passes.

WinKillSwitch is a single portable Windows executable. Installing it means downloading one file. It needs no administrator rights to run, writes nothing outside `%APPDATA%\winkillswitch`, and adds nothing to startup.

## 0. Confirm the machine

Run in PowerShell:

```powershell
[System.Environment]::OSVersion.Version
[Environment]::Is64BitOperatingSystem
```

**Pass:** major version `10` (Windows 10 and 11 both report 10) and `True`. Otherwise stop and tell the user WinKillSwitch needs 64 bit Windows 10 or 11.

## 1. Find the latest version

```powershell
$rel = Invoke-RestMethod https://api.github.com/repos/Ege-BULUT/WinKillSwitch/releases/latest
$rel.tag_name
```

**Pass:** a tag such as `v0.2.0`. Remember it as the expected version.

If the GitHub CLI is available and signed in, `gh release view --repo Ege-BULUT/WinKillSwitch --json tagName` gives the same answer.

## 2. Download into a folder the user owns

Ask the user where they want the app if they have not said. A sensible default:

```powershell
$dir = Join-Path $env:LOCALAPPDATA "Programs\WinKillSwitch"
New-Item -ItemType Directory -Force $dir | Out-Null
$exe = Join-Path $dir "WinKillSwitch.exe"
Invoke-WebRequest https://github.com/Ege-BULUT/WinKillSwitch/releases/latest/download/WinKillSwitch.exe -OutFile $exe
```

**Pass:** `Test-Path $exe` is `True` and the file is larger than 5 MB.

## 3. Verify the download

Each release publishes `WinKillSwitch.exe.sha256` next to the executable.

```powershell
$expected = (Invoke-WebRequest https://github.com/Ege-BULUT/WinKillSwitch/releases/latest/download/WinKillSwitch.exe.sha256).Content.Split(" ")[0].Trim().ToLower()
$actual = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
$expected -eq $actual
```

**Pass:** `True`. If it is `False`, delete the file, download once more, and if it still fails, stop and report the mismatch. Never run a file that fails this check.

## 4. Check the version

The executable is a windowed app, so capture its output through a redirect:

```powershell
$out = Join-Path $env:TEMP "wks_version.txt"
Start-Process $exe -ArgumentList "--version" -Wait -NoNewWindow -RedirectStandardOutput $out
Get-Content $out
```

**Pass:** `WinKillSwitch X.Y.Z` where `vX.Y.Z` matches the tag from step 1.

## 5. Smoke test without opening a window

WinKillSwitch can render itself headlessly. This proves the GPU stack works on this machine without disturbing the user's desktop. The `demo` scene uses made up processes, so the image never contains the user's real data.

```powershell
$png = Join-Path $env:TEMP "wks_check.png"
Start-Process $exe -ArgumentList "--scene demo --snap `"$png`" --size 1280x800 --scale 1" -Wait -NoNewWindow
(Get-Item $png).Length
```

**Pass:** the PNG exists and is larger than 100 KB. If you can view images, look at it: you should see glass panels, a process table and a floating tab bar at the bottom.

If it fails with a GPU or adapter error, the machine has no usable DirectX 12 or Vulkan driver. Report that; updating the graphics driver usually fixes it.

## 6. Optional: a Start menu shortcut

Only if the user asks for one:

```powershell
$lnk = Join-Path ([Environment]::GetFolderPath("Programs")) "WinKillSwitch.lnk"
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = $exe
$s.Save()
```

## 7. Launch

```powershell
Start-Process $exe
```

Windows SmartScreen may warn the first time because the executable is not code signed yet. Tell the user to choose **More info**, then **Run anyway**. Do not try to bypass SmartScreen yourself.

Some features (clearing the standby memory cache, cleaning system temp and update files) need administrator rights. The app has a **Restart as administrator** button; let the user decide whether to use it.

## 8. Report to the user

Use this template and fill in what you observed:

```
WinKillSwitch vX.Y.Z is installed at <path>.
- Download verified: SHA-256 matches the release.
- Version check: <output of --version>.
- Headless render test: passed (<size> KB image).
- Launched: yes / not yet.
Notes: <SmartScreen prompt, GPU warnings, anything unexpected>.
Known limitations in this release: ending processes and PauseSwitch's system changes are not implemented yet; the app says so when you use them.
```

## Updating

Repeat steps 1 to 5. The new file replaces the old one; settings and groups in `%APPDATA%\winkillswitch` are kept.

## Uninstalling

```powershell
Remove-Item $exe
Remove-Item (Join-Path ([Environment]::GetFolderPath("Programs")) "WinKillSwitch.lnk") -ErrorAction SilentlyContinue
```

Ask the user before also removing their settings in `%APPDATA%\winkillswitch`.

## Building from source instead

Only when the user asks for it, or no release fits their machine:

```powershell
git clone https://github.com/Ege-BULUT/WinKillSwitch.git
cd WinKillSwitch
cargo build --release
```

Requirements: rustup (the toolchain pinned in `rust-toolchain.toml` installs itself), Visual Studio Build Tools with the C++ workload, and the Windows 10 or 11 SDK. The first build takes several minutes. The result is `target\release\winkillswitch.exe`; run steps 4 and 5 against it.
