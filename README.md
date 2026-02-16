# PadSwitch

Controller manager for Windows and Linux. Reorder, remap, and forward game controllers so the right player always gets the right slot.

## Features

- **Drag-and-drop controller ordering** — assign physical controllers to player slots (P1–P4)
- **Two routing modes**
  - **Minimal** (Windows only) — reorders XInput slots via disable/re-enable, no extra drivers needed
  - **Force** (Windows + Linux) — hides physical devices and creates virtual controllers with full input forwarding at 1000Hz
- **Profiles** — save and switch between named controller configurations
- **Game rules** — automatically activate a profile when a game launches
- **Process watcher** — monitors running processes and auto-switches profiles
- **System tray** integration with quick profile switching

## Download

Go to [**Releases**](../../releases) and download the latest version for your platform:

| Platform | File | Notes |
|----------|------|-------|
| Windows | `.msi` or `.exe` | Installer, requires admin for Minimal mode |
| Linux (Debian/Ubuntu) | `.deb` | Install with `sudo dpkg -i padswitch_*.deb` |
| Linux (universal) | `.AppImage` | Run directly: `chmod +x PadSwitch_*.AppImage && ./PadSwitch_*.AppImage` |

## Platform Notes

### Windows

**Minimal mode** requires administrator privileges (disable/re-enable devices via SetupDi).

**Force mode** requires two drivers:
- [HidHide](https://github.com/nefarius/HidHide/releases) — hides physical controllers from games
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) — creates virtual Xbox 360 controllers

### Linux

Only **Force mode** is supported. It uses the kernel's built-in `evdev` and `uinput` subsystems — no external drivers needed.

How it works:
1. Physical controllers are grabbed via `EVIOCGRAB` (exclusive access)
2. Virtual controllers are created via `uinput` in your chosen player order
3. Input events are forwarded from physical to virtual devices

Games see the virtual controllers in creation order, giving you consistent P1/P2/P3/P4 ordering.

**Permissions**: Force mode needs write access to `/dev/uinput`. Either run as root or add a udev rule:

```bash
echo 'KERNEL=="uinput", MODE="0660", GROUP="input"' | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules
sudo usermod -aG input $USER
# Log out and back in for the group change to take effect
```

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) stable toolchain
- **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`
- **Windows**: Visual Studio Build Tools with C++ workload

### Setup

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Outputs are in `src-tauri/target/release/bundle/`.

## Creating a Release

Push a version tag to trigger the build workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds for Windows and Linux, then creates a draft release on GitHub with all installers attached. Review and publish the draft from the [Releases](../../releases) page.

## Tech Stack

- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust + Tauri 2
- **Windows**: rusty-xinput, vigem-client, windows-rs (HidHide/SetupDi)
- **Linux**: evdev (physical devices + uinput virtual controllers)
