<div align="center">

<img src="docs/assets/title.svg" alt="WinKillSwitch" width="620">

<p><strong>A process manager for Windows that is fast to use and pleasant to look at.<br>Native, GPU rendered, built with Rust and the Rune UI framework.</strong></p>

<p>
  <a href="https://github.com/Ege-BULUT/WinKillSwitch/releases"><img alt="Download" src="https://img.shields.io/badge/download-releases-7c5cff?style=flat-square&logo=github"></a>
  <a href="https://winkillswitch.vercel.app"><img alt="Website" src="https://img.shields.io/badge/website-winkillswitch.vercel.app-ff5ca8?style=flat-square&logo=vercel"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-22d3ee?style=flat-square&logo=windows">
  <a href="https://www.rust-lang.org"><img alt="Rust" src="https://img.shields.io/badge/rust-1.99-orange?style=flat-square&logo=rust"></a>
  <a href="https://github.com/anomalyco/rune"><img alt="Rune" src="https://img.shields.io/badge/UI-Rune-4f8dff?style=flat-square"></a>
  <img alt="Status" src="https://img.shields.io/badge/status-alpha-ffb547?style=flat-square">
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-34d399?style=flat-square"></a>
</p>

<p>
  <a href="#features">Features</a> ·
  <a href="#gallery">Gallery</a> ·
  <a href="#getting-started">Getting started</a> ·
  <a href="#known-limitations">Known limitations</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#contributing">Contributing</a>
</p>

<p><a href="agent_setup_guide.md"><strong>If you are an agent</strong></a>, follow the step by step guide to install, verify and report on the latest release without reading this whole page.</p>

<img src="docs/assets/tour.gif" alt="WinKillSwitch tour: switching tabs, groups and palettes" width="820">

</div>

## About

WinKillSwitch is a desktop process manager for Windows. It shows what is running, lets you collect the apps you care about into groups, and is built toward one goal: shutting an app down completely, related processes and auto starts included, and keeping it down until you open it again yourself.

The interface is drawn entirely on the GPU by [Rune](https://github.com/anomalyco/rune). Panels are real glass over a live gradient, every change animates on springs, and the whole look follows the palette you pick.

> [!NOTE]
> WinKillSwitch is in **alpha**. The interface, process monitoring, groups, theming and the memory and disk cleaners work today. Ending processes and the PauseSwitch system changes are designed and wired into the UI but not implemented yet. See [Known limitations](#known-limitations).

## Features

**Monitoring**
- Live process table refreshed every 1.5 seconds: name, PID, CPU and memory, plus machine wide CPU and memory use.
- Search by name, PID or executable path.
- Sortable, resizable, virtualised table with multi selection and keyboard navigation.
- Right click the header to choose columns: type, status, publisher, description, PID, command line, CPU, memory, disk, **GPU** and **GPU engine**. Your choice is saved.
- Per process GPU use from the same GPU Engine counters Task Manager reads, with the busiest engine (3D, video decode, copy and so on).
- **Process tree**: switch the table between **List** and **Tree**. The tree nests each process under the one that started it, folds and unfolds branches, keeps siblings in your sort order, and during a search shows matches together with their parents. Reused process IDs are not mistaken for parents.
- **Quick palette**: press **Ctrl+K** in the app, or **Ctrl+Shift+K** from anywhere, and type. Processes, groups and commands (clean memory, analyze disk, switch theme or mode, jump to a tab) come up as you type; arrow keys and Enter do the rest. The global shortcut can be turned off in Settings.
- **Find by window**: drag the target next to the search box onto any window. While you drag it names the process underneath; let go and that process is selected and scrolled into view.

**Memory cleaner**
- A ring gauge of memory in use, cached and free, with a two minute history.
- One **Clean memory** button: trims every app's working set and, as administrator, clears the standby cache. Nothing is closed.
- Shows exactly what it freed, as a number that counts up and as the cached slice of the ring shrinking.

**Disk cleaner**
- One **Analyze** button finds temporary files, browser caches, crash dumps, thumbnail and shader caches, Windows Update leftovers and the Recycle Bin, with a live count while it looks.
- Review the list: safe categories start ticked; the Recycle Bin and anything that makes the next launch slower start unticked; a browser's cache is skipped while it runs.
- **Clean** shows what it freed, file counts and anything skipped because it was in use.
- Never follows links or junctions, never deletes the folders themselves, and keeps temporary files changed in the last week.

**About this process**
- Double click any process, or press **Details**, for a card that says what it is in plain language, with a note on whether ending it is safe for about 40 common Windows processes.
- Shows whether WinKillSwitch can manage it, and when it cannot, why: a critical system process, a process Windows protects, or one that runs as SYSTEM or as administrator, with a one click restart as administrator when that helps.
- Verifies the executable's digital signature, and shows who it runs as, when it started, its parent (click to jump to it), its children, its location and its command line, each copyable.

**Why is my PC slow?**
- One button (on the Clean tab, or in the quick palette) watches the machine for about five seconds, then answers in plain words: processor, memory, disk space, disk activity, graphics and uptime, worst first.
- It names the process behind a problem, averaged over the samples so a one-off spike is not blamed, and puts the fix one click away: **Show it**, **Clean memory** or **Analyze disk**.

**Limits**
- Cap a program's processor and memory use from its **Details** card: 50%, 25% or 10% of the processor, 4 GB down to 512 MB of RAM per process.
- The processor cap covers all of the program's processes together through a Windows Job Object, so it only runs slower. The memory cap is a working set ceiling: Windows pages the rest out instead of failing allocations, so the program keeps running.
- Limits are saved by program name and come back whenever it starts again. Settings lists them; remove one and the program is free at once. Windows' own processes cannot be limited.

**New process alerts**
- In Settings, list the programs you want to hear about (wildcards work: `steam*`) and get a small note in the corner when one starts, with **Show** one click away.
- Optionally get the same note for anything that starts from Downloads or a temporary folder, where installers and most malware run from. Off by default, so nothing nags you.

**Who is using this file?**
- Drop a file or folder anywhere on the window (or use the quick palette) to see every app and service holding it open, through the same Restart Manager API PowerToys File Locksmith uses.
- Each holder shows its app name, process, PID and type, with **Details** one click away. Folders are checked file by file, up to 2,000 files.

**Groups**
- Create, rename in place and delete groups of processes, keyed by executable path.
- Add the selected rows or the active process to a group; remove members one by one.
- Each member shows whether it is running right now. Groups are saved to `%APPDATA%\winkillswitch` with atomic writes.

**Safety first**
- Every destructive action opens a confirmation sheet listing each process it would touch.
- Detects whether it runs elevated and offers a one click relaunch as administrator.

**Look and feel**
- Liquid, fluted and frosted glass panels over a palette tinted mesh gradient, and a floating glass tab bar whose selection springs between tabs.
- Six palettes with official light and dark variants: Rune Aurora, Catppuccin, Rosé Pine, Tokyo Night, Kanagawa and Everforest.
- Accent picker, glass style and strength, light, dark or system mode, and an animated background you can switch off to keep the GPU idle.
- Responsive layout down to small window sizes.

## Gallery

<table>
  <tr>
    <td width="50%"><img src="docs/assets/hero-dark.png" alt="Processes, Rune Aurora dark"></td>
    <td width="50%"><img src="docs/assets/hero-light.png" alt="Processes, Rune Aurora light"></td>
  </tr>
  <tr>
    <td align="center"><sub>Processes and group inspector, Rune Aurora dark</sub></td>
    <td align="center"><sub>Same view in light mode</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/clean-dark.png" alt="Memory and disk cleaner"></td>
    <td><img src="docs/assets/clean-light.png" alt="Disk cleaner after a clean, Catppuccin Latte"></td>
  </tr>
  <tr>
    <td align="center"><sub>Memory freed, disk analyzed and ready to clean</sub></td>
    <td align="center"><sub>After a disk clean, Catppuccin Latte</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/info-dark.png" alt="About this process"></td>
    <td><img src="docs/assets/pause-dark.png" alt="PauseSwitch"></td>
  </tr>
  <tr>
    <td align="center"><sub>About this process: what it is and whether it can be managed</sub></td>
    <td align="center"><sub>PauseSwitch</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/locks-dark.png" alt="Who is using this file"></td>
    <td><img src="docs/assets/diagnose-dark.png" alt="Why is my PC slow"></td>
  </tr>
  <tr>
    <td align="center"><sub>Who is using this file?</sub></td>
    <td align="center"><sub>Why is my PC slow?</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/settings-dark.png" alt="Appearance settings"></td>
    <td><img src="docs/assets/settings-light.png" alt="Appearance settings, Catppuccin Latte"></td>
  </tr>
  <tr>
    <td align="center"><sub>Appearance settings</sub></td>
    <td align="center"><sub>Catppuccin Latte</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/pal-rose-dark.png" alt="Rosé Pine Moon with fluted glass"></td>
    <td><img src="docs/assets/pal-kanagawa-dark.png" alt="Kanagawa Wave with frosted glass"></td>
  </tr>
  <tr>
    <td align="center"><sub>Rosé Pine Moon, fluted glass</sub></td>
    <td align="center"><sub>Kanagawa Wave, frosted glass</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/palette-dark.png" alt="Quick palette"></td>
    <td><img src="docs/assets/pause-light.png" alt="PauseSwitch in Tokyo Night Day"></td>
  </tr>
  <tr>
    <td align="center"><sub>Quick palette: processes, groups and commands</sub></td>
    <td align="center"><sub>PauseSwitch, Tokyo Night Day</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/columns-dark.png" alt="Extra columns including GPU"></td>
    <td><img src="docs/assets/tree-dark.png" alt="Process tree"></td>
  </tr>
  <tr>
    <td align="center"><sub>Extra columns, including GPU and GPU engine</sub></td>
    <td align="center"><sub>Process tree</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/pal-tokyo-dark.png" alt="Tokyo Night"></td>
    <td><img src="docs/assets/pal-everforest-light.png" alt="Everforest Light"></td>
  </tr>
  <tr>
    <td align="center"><sub>Tokyo Night</sub></td>
    <td align="center"><sub>Everforest Light</sub></td>
  </tr>
</table>

<sub>All screenshots use the built in demo machine, not a real process list.</sub>

## Getting started

### Download

Get the latest **[WinKillSwitch.exe](https://github.com/Ege-BULUT/WinKillSwitch/releases/latest/download/WinKillSwitch.exe)** or browse every version on the [Releases](https://github.com/Ege-BULUT/WinKillSwitch/releases) page. It is a single portable `.exe`: no installer and no runtime. Each release lists the file's SHA-256 so you can check the download.

> [!TIP]
> Binaries are not code signed yet, so Windows SmartScreen may warn on first launch. Choose **More info**, then **Run anyway**.

### Build from source

Requirements:
- Windows 10 or 11
- [Rust](https://rustup.rs) (the pinned toolchain in `rust-toolchain.toml` installs automatically)
- Visual Studio Build Tools with the C++ workload, and the Windows 10/11 SDK

```powershell
git clone https://github.com/Ege-BULUT/WinKillSwitch.git
cd WinKillSwitch
cargo run --release
```

The first build compiles Rune and its GPU stack from source and takes a few minutes. Later builds take seconds.

### Using it

| Where | What you can do |
| --- | --- |
| **Processes** tab | Search, sort and select processes. Pick a group on the left, then **Add to group**. |
| Group list | **+** creates a group. Double click a group to rename it. Click it to open the inspector. |
| Group inspector | See members and whether they run, remove members, toggle PauseSwitch, end the group. |
| **Settings** tab | Palette, mode, accent, glass style and strength, animated background. |

## Known limitations

- **Process actions are stubs.** Ending a process, ending everything related to an app, the respawn guard and PauseSwitch's autostart changes are defined behind interfaces in [`src/sys`](src/sys) and wired into the UI, but in this alpha they do nothing and say so. Nothing on your system is modified.
- **Windows only.** Rune itself runs on macOS and Linux, but the system layer is written for Windows.
- **Pre-release UI framework.** Rune has no tagged releases; this project pins an exact commit.
- **Not signed.** No code signing and no installer yet.
- **Collected but not shown.** Type, status, publisher, description, command line and disk I/O are sampled per process but not yet shown as columns.

## Roadmap

Planned releases: **v0.2** adds the memory cleaner, **v0.3** the disk cleaner, **v0.3.x** ships the rest of this list one feature at a time, and **v0.4** is the complete, tested set.

- [x] Responsive under load: sampling and cleaning run off the UI thread, and the UI runs at above normal priority
- [x] Memory cleaner: a live memory gauge and one Clean button that shows exactly what it freed (v0.2)
- [x] Disk cleaner: analyze temporary files, caches and leftovers, review them, then clean in one step (v0.3)
- [x] "What is this process?" card that also explains why access is denied (v0.3.1)
- [x] Column picker on header right click, including per process GPU usage (v0.3.2)
- [x] Find a process by pointing at its window (v0.3.3)
- [x] Global hotkey and quick palette (v0.3.4)
- [x] Process tree view (v0.3.5)
- [x] Find out which process is locking a file (v0.3.6)
- [x] "Why is my PC slow?" one click diagnosis (v0.3.7)
- [x] Notifications when new processes start, by rule (v0.3.8)
- [x] CPU and memory ceilings per program (v0.3.9)
- [ ] Wider autostart coverage
- [ ] Per process network traffic
- [ ] Rule packs you can export and import, with ready made profiles such as Gaming mode and Bloat blocker
- [ ] Ending processes: one, a tree, or everything related, asking apps to close before forcing them
- [ ] PauseSwitch backend: Startup apps, services and scheduled tasks, restored exactly when switched off
- [ ] Signed releases and a winget package

The full research behind this list, 50 ideas scored by impact, effort and urgency, lives in [`research/`](research).

## Project structure

```
src/
  main.rs        app entry, poll loop, headless scenes
  app.rs         application state
  actions.rs     what buttons do, confirmation sheets
  model.rs       shared types and saved settings
  store.rs       JSON persistence with atomic writes
  demo.rs        the fixed demo machine used for screenshots
  sys/           Windows access: sampling, elevation, version info, process actions
  ui/            the Rune view: glass, palettes, compositor, tabs and settings
docs/            README assets and the title generator
research/        feature research report
```

## Contributing

Contributions are welcome, from bug reports to whole features on the roadmap.

1. Open an issue first for anything larger than a small fix, so we can agree on the approach.
2. Fork the repository and create a branch from `main`.
3. Keep changes focused, and run `cargo fmt` and `cargo clippy` before pushing.
4. For UI changes, attach before and after screenshots. You can render them without opening a window:
   ```powershell
   $env:WKS_TAB = "settings"; $env:WKS_PALETTE = "catppuccin"
   cargo run --release -- --scene demo --snap shot.png --theme dark --size 1440x900 --scale 2
   ```
5. Open a pull request that explains what changed and why.

Please be kind and constructive in issues and reviews.

## Acknowledgements

- [Rune](https://github.com/anomalyco/rune) by Anomaly for the UI framework, rune-kit components and rune-fx glass.
- Palettes from [Catppuccin](https://catppuccin.com), [Rosé Pine](https://rosepinetheme.com), [Tokyo Night](https://github.com/folke/tokyonight.nvim), [Kanagawa](https://github.com/rebelot/kanagawa.nvim) and [Everforest](https://github.com/sainnhe/everforest).
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) for process sampling.
- Title set in Cinzel and Noto Sans Runic, both under the SIL Open Font License.

## License

Released under the [MIT License](LICENSE).
