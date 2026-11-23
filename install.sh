#!/usr/bin/env bash
set -euo pipefail

APP_NAME="image_tiler"
APP_BUNDLE="$APP_NAME.app"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILT_APP="$REPO_ROOT/src-tauri/target/release/bundle/macos/$APP_BUNDLE"
INSTALL_APP="/Applications/$APP_BUNDLE"
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
BIN_DIR="$HOME/bin"
SYMLINK="$BIN_DIR/$APP_NAME"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: this installer targets macOS (found $(uname -s))." >&2
  exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "error: npm is required but was not found in PATH." >&2
  exit 1
fi

echo "==> Building release app bundle"
cd "$REPO_ROOT"
npm run tauri build -- --bundles app

if [ ! -d "$BUILT_APP" ]; then
  echo "error: expected bundle not found at: $BUILT_APP" >&2
  exit 1
fi

echo "==> Installing to $INSTALL_APP"
if ! rm -rf "$INSTALL_APP" 2>/dev/null || ! ditto "$BUILT_APP" "$INSTALL_APP" 2>/dev/null; then
  echo "==> /Applications is not writable; retrying with sudo"
  sudo rm -rf "$INSTALL_APP"
  sudo ditto "$BUILT_APP" "$INSTALL_APP"
fi

echo "==> Refreshing LaunchServices (file associations)"
"$LSREGISTER" -f "$INSTALL_APP"

echo "==> Linking command-line entry point"
mkdir -p "$BIN_DIR"
ln -sfn "$INSTALL_APP/Contents/MacOS/$APP_NAME" "$SYMLINK"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo
    echo "Note: $BIN_DIR is not on your PATH. Add this to ~/.zshrc:"
    echo "  export PATH=\"\$HOME/bin:\$PATH\""
    ;;
esac

echo
echo "Installed:"
echo "  app: $INSTALL_APP"
echo "  cli: $SYMLINK"
echo
echo "Try: $APP_NAME /path/to/photo.png"
