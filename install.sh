#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-/usr/local}"

echo "==> Nyxen installer"

if command -v apt-get >/dev/null 2>&1; then
  echo "Installing packages with apt-get (needs sudo)..."
  sudo apt-get update
  sudo apt-get install -y \
    build-essential pkg-config libssl-dev \
    libgtk-3-dev libxdo-dev libasound2-dev \
    libsqlite3-dev sqlite3 \
    libayatana-appindicator3-dev \
    espeak-ng ffmpeg wget curl unzip \
    pipewire-audio-client-libraries libspa-0.2-bluetooth || true
  # Prefer pipewire tools; aplay still often present
  sudo apt-get install -y alsa-utils pulseaudio-utils || true
elif command -v pacman >/dev/null 2>&1; then
  sudo pacman -S --needed base-devel pkgconf gtk3 libxdo alsa-lib openssl sqlite \
    libayatana-appindicator espeak-ng wget curl unzip || true
elif command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y gcc gtk3-devel libXdo-devel alsa-lib-devel openssl-devel sqlite-devel \
    libappindicator-gtk3 espeak-ng wget curl unzip || true
else
  echo "Unknown package manager — install GTK3, OpenSSL dev, SQLite, espeak-ng, libxdo, appindicator manually."
fi

echo "==> Rust toolchain"
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

DATA="$HOME/.local/share/nyxen"
MODELS="$DATA/models"
mkdir -p "$MODELS/vosk" "$MODELS/piper" "$ROOT/assets/sounds" "$DATA/assets/sounds"

if [[ ! -f "$MODELS/vosk/vosk-model-small-en-us-0.15/README" ]]; then
  echo "==> Downloading Vosk small English model..."
  wget -qO /tmp/vosk-small.zip "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip"
  unzip -q /tmp/vosk-small.zip -d "$MODELS/vosk"
  rm -f /tmp/vosk-small.zip
fi

if [[ ! -f "$MODELS/piper/en_US-lessac-medium.onnx" ]]; then
  echo "==> Downloading Piper voice (en_US lessac medium)…"
  mkdir -p "$MODELS/piper/en"
  wget -qO /tmp/piper.tgz "https://github.com/rhasspy/piper/releases/download/v0.0.2/voice-en-us-lessac-medium.tar.gz"
  tar -xzf /tmp/piper.tgz -C "$MODELS/piper" || true
  rm -f /tmp/piper.tgz
fi

if [[ ! -f "$DATA/assets/sounds/chime.wav" ]]; then
  echo "==> Generating soft chime with ffmpeg..."
  ffmpeg -y -f lavfi -i "sine=frequency=880:duration=0.15" -af "afade=t=in:st=0:d=0.05,afade=t=out:st=0.1:d=0.05" "$DATA/assets/sounds/chime.wav" 2>/dev/null || \
    touch "$DATA/assets/sounds/chime.wav"
fi
cp -f "$DATA/assets/sounds/chime.wav" "$ROOT/assets/sounds/chime.wav" 2>/dev/null || true

echo "==> Building Nyxen"
(cd "$ROOT" && cargo build --release)

sudo install -Dm755 "$ROOT/target/release/nyxen" "$PREFIX/bin/nyxen"

mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/nyxen.service" <<EOF
[Unit]
Description=Nyxen AI Assistant
After=graphical-session.target

[Service]
Type=simple
# User services often start without DISPLAY; tray + GTK need it. Change :0 if you use another screen.
Environment=DISPLAY=:0
Environment=XAUTHORITY=%h/.Xauthority
ExecStart=$PREFIX/bin/nyxen --daemon
Restart=on-failure
RestartSec=5
StartLimitIntervalSec=60
StartLimitBurst=5

[Install]
WantedBy=default.target
EOF

mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/nyxen.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Nyxen
Exec=$PREFIX/bin/nyxen --daemon
Icon=$ROOT/assets/icon.svg
Categories=Utility;
EOF

systemctl --user daemon-reload || true
systemctl --user enable --now nyxen.service || true

echo ""
echo "Nyxen installed to $PREFIX/bin/nyxen"
echo "- Logs: ~/.local/share/nyxen/nyxen.log"
echo "- DB:   ~/.local/share/nyxen/nyxen.db"
echo "- Set GROQ API key in the welcome/settings UI."
echo ""
echo "Note: Global hotkeys use X11; Wayland support depends on your session."
echo "Voice wake word: this build ships a vosk stub — enable full vosk in source when libvosk is linked."
