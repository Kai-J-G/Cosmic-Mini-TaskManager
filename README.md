# Cosmic Mini Task Manager

Native task manager and system resource applet for the **COSMIC Desktop Environment** running on **Kashi OS** and Arch-based Linux distributions.

Designed to sit discreetly in your COSMIC panel or dock, the applet opens a theme-adapted tray popup featuring frosted glass subsurface blur effects. It provides at-a-glance insight into running applications and high-resource processes, with proactive detection and immediate one-click controls to **Stop, Resume, or Kill** unresponsive or stopped tasks.

---

## 📸 Showcase

![COSMIC Mini Task Manager Overview](data/screenshots/task-manager-overview.png)

*The 640px frosted glass popup displaying live CPU & RAM gauges, search bar, category tabs, and tabular process rows with Stop/Kill controls.*

---

## ✨ Features

- **🎨 Native COSMIC Look & Feel**:
  - Automatically adapts to desktop dark/light mode preference or explicit override.
  - Native frosted glass blur (`LiveSettings { blur: Some(true) }`) rendered via `cosmic-comp`.
  - Consistent typography, spacing, and styling using COSMIC theme tokens.
  - Dynamic panel activity pulse icon that shifts hue (cyan → amber → red) based on system load and alert conditions.

- **🚨 Proactive Detection of Stopped & Hung Processes**:
  - Immediate identification of stopped (`SIGSTOP` / `ProcessStatus::Stop`), zombie (`Z`), or disk-sleep (`D` state / hung I/O) processes.
  - Debounced polling to eliminate false-positive flicker on transient mutex waits.
  - Optional warning badge in the panel button (`!N`) when any process is unresponsive.
  - High-visibility alert banner inside the popup with a 1-click **Kill All** button.

- **⚡ "Stop or Kill" Controls**:
  - **Stop**: Pauses the process via `SIGSTOP`.
  - **Resume**: Resumes a stopped process via `SIGCONT`.
  - **Kill**: Force terminates runaway processes via `SIGKILL`.
  - **Kill All**: Bulk terminates all unresponsive or stopped processes at once.

- **📊 Resource Gauges & Categorized Navigation**:
  - Live linear CPU & RAM utilization progress meters.
  - Quick category tabs: **All**, **Apps** (desktop GUI applications scanned from `.desktop` files), **Top CPU**, **Top RAM**, and **Stopped / Hung**.
  - Real-time instant search by application name, executable path, or PID.
  - Total order sorting algorithm with PID tiebreakers to guarantee stable 60fps rendering without jitter.

- **⚙️ Configurable Settings**:
  - Appearance preference: System, Dark, or Light.
  - Configurable refresh interval: 1s, 2s, or 5s.
  - Toggle panel alert indicator.
  - Settings persisted across reboots via `cosmic-config`.

---

## 📦 Installation Guide

### Option 1: Arch Linux / CachyOS (`PKGBUILD`)

If you are running Arch Linux, CachyOS, or any Arch-based distribution, a native `PKGBUILD` is included:

```bash
# Clone the repository
git clone https://github.com/Kai-J-G/Cosmic-Mini-TaskManager.git
cd Cosmic-Mini-TaskManager

# Build and install the package
makepkg -si
```

This compiles an optimized release binary and installs all desktop entries, AppStream metadata, scalable icons, and licenses into `/usr`.

---

### Option 2: Using the `just` Command Runner

For quick local user installation without root privileges:

```bash
# Clone the repository
git clone https://github.com/Kai-J-G/Cosmic-Mini-TaskManager.git
cd Cosmic-Mini-TaskManager

# Build and install to ~/.local
just install
```

To install system-wide using `just`:
```bash
sudo just prefix=/usr install
```

To uninstall at any time:
```bash
just uninstall
```

---

### Option 3: Manual Build with Cargo

```bash
cargo build --release

# Install binary
install -Dm0755 target/release/cosmic-mini-taskmanager ~/.local/bin/cosmic-mini-taskmanager

# Install desktop entry and icons
install -Dm0644 data/io.github.kai_j_g.CosmicMiniTaskManager.desktop ~/.local/share/applications/io.github.kai_j_g.CosmicMiniTaskManager.desktop
install -Dm0644 data/io.github.kai_j_g.CosmicMiniTaskManager.metainfo.xml ~/.local/share/metainfo/io.github.kai_j_g.CosmicMiniTaskManager.metainfo.xml
install -Dm0644 data/icons/io.github.kai_j_g.CosmicMiniTaskManager-symbolic.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.kai_j_g.CosmicMiniTaskManager-symbolic.svg
install -Dm0644 data/icons/io.github.kai_j_g.CosmicMiniTaskManager.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.kai_j_g.CosmicMiniTaskManager.svg
```

---

## 🖥️ Adding to the COSMIC Panel

Once installed, add the applet to your desktop panel:

1. Open **COSMIC Settings** (or press `Super` and search for *Settings*).
2. Go to **Desktop** -> **Panel** -> **Applets**.
3. Click **Add Applet** (+).
4. Locate **Mini Task Manager** in the list and click **Add**.
5. Drag it to your desired position (Left, Center, or Right wing).

Alternatively, launch it standalone from your terminal or app launcher:
```bash
cosmic-mini-taskmanager
```

---

## 🏗️ Project Architecture

```
src/
├── main.rs              # Application entry point & runtime runner
├── app.rs               # Elm-style Model-View-Update (MVU) state machine & subscriptions
├── config.rs            # Persistent user configuration (cosmic-config)
├── process/
│   ├── mod.rs           # Process subsystem re-exports
│   ├── types.rs         # ProcessItem, ProcessState, and SystemOverview models
│   ├── collector.rs     # sysinfo poller, desktop app mapper & debounced status tracking
│   └── actions.rs       # POSIX signal handlers (SIGSTOP, SIGCONT, SIGKILL)
└── views/
    ├── mod.rs           # Main popup view composition
    ├── panel.rs         # Panel icon button, Wayland subsurface & autosize popup limits
    ├── header.rs        # CPU/RAM utilization progress bars & header controls
    ├── filter_bar.rs    # Search bar & category filter pills
    ├── alert_banner.rs  # High-visibility warning alert with "Kill All" action
    ├── process_row.rs   # Tabular process row with formatted metrics & Stop/Kill buttons
    ├── settings.rs      # User settings panel
    └── style.rs         # Theme-aware container & pill badge styling helpers
```

- **State Management**: Built on the Elm Model-View-Update (MVU) pattern provided by `libcosmic` / `iced`.
- **Debounced Process Monitor**: Processes in uninterruptible sleep (`D`) are debounced across multiple polling ticks to distinguish momentary I/O flushes from actual hung states.
- **Strict Total Order**: Sorting avoids unstable float comparisons by using integer bucketing and PID tiebreakers, guaranteeing zero crashes and predictable sorting.

---

## 🗺️ Roadmap

- [x] **Native COSMIC Applet UI**: Frosted glass tray popup with dark/light theme switching.
- [x] **Proactive Hung/Stopped Detection**: Immediate alerts for paused or non-responsive tasks.
- [x] **One-Click Actions**: Individual Stop/Resume/Kill and bulk "Kill All".
- [x] **Resource Gauges**: Real-time CPU and RAM utilization meters.
- [x] **Application Categorization**: Separation of desktop GUI apps from background daemons.
- [x] **Arch Packaging**: `PKGBUILD` and `justfile` packaging support.
- [ ] **Network & Disk I/O Gauges**: Per-process read/write throughput and network transmission metrics.
- [ ] **Process Tree Hierarchy**: Collapsible tree view showing parent/child process relationships.
- [ ] **Cgroups Resource Limits**: Ability to throttle CPU shares or set memory limits on specific process cgroups.
- [ ] **Global Shortcut**: Configurable keybinding to summon the mini task manager popup from anywhere.
- [ ] **Internationalization (i18n)**: Localization support via `fluent` and `i18n-embed`.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
