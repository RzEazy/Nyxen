#!/usr/bin/env bash
set -euo pipefail

RED='\033[1;31m'
GREEN='\033[1;32m'
NC='\033[0m'

echo -e "${RED}==> Nyxen Uninstaller${NC}"
echo ""
echo "This will remove Nyxen and ALL associated data:"
echo "  - /usr/local/bin/nyxen"
echo "  - ~/.local/share/nyxen/"
echo "  - ~/.local/share/applications/nyxen.desktop"
echo "  - ~/.config/systemd/user/nyxen.service"
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo -e "\033[1m[1/5]\033[0m Stopping and disabling systemd service..."
systemctl --user disable --now nyxen.service 2>/dev/null || true
systemctl --user daemon-reload 2>/dev/null || true

echo -e "\033[1m[2/5]\033[0m Removing binary from /usr/local/bin/..."
sudo rm -f /usr/local/bin/nyxen

echo -e "\033[1m[3/5]\033[0m Removing data directory (~/.local/share/nyxen/)..."
rm -rf "$HOME/.local/share/nyxen"

echo -e "\033[1m[4/5]\033[0m Removing desktop entry..."
rm -f "$HOME/.local/share/applications/nyxen.desktop"

echo -e "\033[1m[5/5]\033[0m Removing systemd service file..."
rm -f "$HOME/.config/systemd/user/nyxen.service"
systemctl --user daemon-reload 2>/dev/null || true

echo ""
echo -e "${GREEN}==> Nyxen has been removed from your system.${NC}"
echo "Your API keys and settings in the database have been deleted."
echo "Consider revoking them at your provider dashboards for security."
