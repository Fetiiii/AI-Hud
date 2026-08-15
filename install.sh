#!/usr/bin/env bash
# Installs the release build into the user's own directories - no root, no
# package manager. Arch-based systems have none of the bundlers Tauri would
# otherwise target (dpkg/rpm/appimagetool), so the binary is placed by hand.
#
#   ./install.sh            install (or update) and register the launcher
#   ./install.sh --autostart  also start it on login
#   ./install.sh --uninstall  remove everything this script installed
set -euo pipefail

APP_ID="ai-hud"
APP_NAME="AI HUD"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SRC_DIR/src-tauri/target/release/ai-hud"

BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
DESKTOP_DIR="$HOME/.local/share/applications"
AUTOSTART_DIR="$HOME/.config/autostart"
DESKTOP_FILE="$DESKTOP_DIR/$APP_ID.desktop"
AUTOSTART_FILE="$AUTOSTART_DIR/$APP_ID.desktop"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "$BIN_DIR/$APP_ID" "$DESKTOP_FILE" "$AUTOSTART_FILE" \
        "$ICON_DIR/$APP_ID.png"
  update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
  echo "Kaldırıldı. (Ayarlar ~/.config/ai-hud ve önbellek ~/.cache/ai-hud duruyor.)"
  exit 0
fi

if [[ ! -x "$BINARY" ]]; then
  echo "Release binary yok: $BINARY" >&2
  echo "Önce derle:  pnpm tauri build --no-bundle" >&2
  exit 1
fi

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

# Replacing a running binary in place would fail with "Text file busy".
install -m755 "$BINARY" "$BIN_DIR/$APP_ID.new"
mv -f "$BIN_DIR/$APP_ID.new" "$BIN_DIR/$APP_ID"
install -m644 "$SRC_DIR/src-tauri/icons/128x128.png" "$ICON_DIR/$APP_ID.png"

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=Claude Code ve Codex kullanım limitleri
Exec=$BIN_DIR/$APP_ID
Icon=$APP_ID
Terminal=false
Categories=Utility;Monitor;
StartupNotify=false
EOF

update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

if [[ "${1:-}" == "--autostart" ]]; then
  mkdir -p "$AUTOSTART_DIR"
  cp "$DESKTOP_FILE" "$AUTOSTART_FILE"
  echo "Oturum açılışında otomatik başlayacak."
fi

echo "Kuruldu: $BIN_DIR/$APP_ID"
case ":$PATH:" in
  *":$BIN_DIR:"*) echo "Çalıştırmak için:  $APP_ID" ;;
  *) echo "NOT: $BIN_DIR PATH'te değil; uygulama menüsünden açabilirsin." ;;
esac
