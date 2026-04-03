# Yeezy

Linux AI assistant overlay powered by **Groq** (streaming `llama-3.3-70b-versatile` and friends), with optional local **Piper** / **espeak-ng** TTS, **SQLite** history, and system tools (shell, files, browser, packages, sysinfo).

## Stack

Rust **egui/eframe**: single native binary, GPU-accelerated UI, custom **epaint** orb (see the comment block in `src/main.rs`).

## Build

**System packages** (Debian/Ubuntu example):

```bash
sudo apt install build-essential pkg-config libssl-dev libgtk-3-dev libxdo-dev \
  libsqlite3-dev libayatana-appindicator3-dev espeak-ng ffmpeg
```

Then:

```bash
./build.sh        # release binary → ./yeezy-bin
# or
cargo build --release && ./target/release/yeezy
```

`build.rs` adds a `libxdo.so` symlink when only `libxdo.so.*` is installed so linking works without `libxdo-dev`.

## Install (user systemd + models)

```bash
chmod +x install.sh build.sh
./install.sh
```

This downloads the **Vosk** small English model and a **Piper** voice under `~/.local/share/yeezy/models/`, installs the binary to `/usr/local/bin/yeezy`, and enables `systemd --user` service `yeezy.service`.

## Usage

- **Super+Y**: global shortcut (X11; Wayland varies).
- **Tray**: left-click opens overlay; menu for settings/quit.
- **First run**: set Groq API key in the welcome dialog.
- **Settings**: tabs for General, Appearance (live orb/chat preview), Voice, Agent, About — persisted to `~/.local/share/yeezy/yeezy.db`.

## Voice / wake word

The current `vosk` Rust dependency is **not** linked by default (avoids hard `libvosk` requirement). `voice/listener.rs` is a **stub**; after `install.sh` you still have models on disk — wire the `vosk` crate back in when `libvosk` is available, or patch `listener.rs` accordingly.

## License

MIT or your choice — project scaffold for local use.
