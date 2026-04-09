# Nyxen

A powerful Linux AI assistant with an elegant overlay UI, powered by Groq's fast LLMs.

## Features

- **AI-Powered Assistant**: Powered by Groq (Llama 3.3 70B), with support for OpenAI, Cohere, and Anthropic
- **Voice Input**: Vosk-based wake word detection with Piper TTS
- **System Integration**: Execute shell commands, manage files, open apps, install packages
- **Beautiful UI**: Custom orb animation with multiple color themes (Monochrome, Dracula, Nord, Catppuccin, Solarized Dark)
- **System Tray**: Runs in background with tray menu
- **Global Hotkey**: Quick access overlay (default: Ctrl+Space)

## Requirements

- Linux (X11 or Wayland)
- Rust toolchain
- System packages:
  - GTK3
  - libsqlite3-dev
  - libxdo-dev
  - libappindicator3-dev
  - espeak-ng
  - ffmpeg
  - alsa-utils or pulseaudio-utils

## Installation

### Quick Install

```bash
git clone https://github.com/RzEazy/Nyxen.git
cd Nyxen
chmod +x install.sh build.sh
./install.sh
```

This will:
1. Install required system packages
2. Download Vosk and Piper voice models
3. Build the release binary
4. Install to `/usr/local/bin/nyx`
5. Set up systemd user service for auto-start
6. Create desktop entry

### Manual Build

```bash
# Install dependencies (Debian/Ubuntu)
sudo apt install build-essential pkg-config libssl-dev libgtk-3-dev libxdo-dev \
  libsqlite3-dev libayatana-appindicator3-dev espeak-ng ffmpeg

# Build
cargo build --release

# Run
./target/release/nyx
```

## Configuration

On first run, you'll be prompted to enter your Groq API key. Get one free at https://console.groq.com/

Settings are stored in `~/.local/share/nyx/nyx.db` and can be configured via the Settings UI:
- **General**: Wake word, hotkey, startup options
- **Appearance**: Color themes, orb size, window opacity
- **Voice**: TTS settings, wake sensitivity
- **Agent**: API keys for different providers, model selection

## Usage

- **Ctrl+Space**: Open/close overlay (X11 global hotkey)
- **Tray icon**: Left-click to open, right-click for menu
- **Voice wake word**: Say "hey nyx" (if enabled)

## System Tray

The app runs in the system tray. Use the tray menu to:
- Show/Hide the main window
- Access Settings
- Quit the application

## Logs

- Application logs: `~/.local/share/nyx/nyx.log`
- Database: `~/.local/share/nyx/nyx.db`

## License

MIT