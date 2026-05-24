#!/usr/bin/env bash
# Downloads the pdfium dynamic library from bblanchon/pdfium-binaries
# and places it in src-tauri/pdfium/ for bundling.
set -euo pipefail

REPO="bblanchon/pdfium-binaries"
RELEASE_TAG="${1:-latest}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/pdfium"
mkdir -p "$OUT_DIR"

# Map Rust target triple → pdfium archive name
# https://github.com/bblanchon/pdfium-binaries#download
detect_archive() {
    local arch
    arch="$(uname -m)"
    local os
    os="$(uname -s)"

    case "$os-$arch" in
        Darwin-arm64)  echo "pdfium-mac-arm64.tgz" ;;
        Darwin-x86_64) echo "pdfium-mac-x64.tgz" ;;
        Linux-x86_64)  echo "pdfium-linux-x64.tgz" ;;
        Linux-aarch64) echo "pdfium-linux-arm64.tgz" ;;
        Linux-armv7l)  echo "pdfium-linux-arm.tgz" ;;
        MINGW*|MSYS*|*-x86_64) echo "pdfium-win-x64.zip" ;;
        *)
            echo "Unsupported platform: $os-$arch" >&2
            exit 1
            ;;
    esac
}

ARCHIVE="$(detect_archive)"
echo "Target archive: $ARCHIVE"

# Resolve release URL
if [[ "$RELEASE_TAG" == "latest" ]]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ARCHIVE"
else
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$RELEASE_TAG/$ARCHIVE"
fi

echo "Downloading from: $DOWNLOAD_URL"
TMP_FILE="$(mktemp)"

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o "$TMP_FILE"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL" -O "$TMP_FILE"
else
    echo "Neither curl nor wget found" >&2
    exit 1
fi

# Extract
case "$ARCHIVE" in
    *.zip)
        unzip -o "$TMP_FILE" -d "$OUT_DIR"
        ;;
    *.tgz|*.tar.gz)
        tar -xzf "$TMP_FILE" -C "$OUT_DIR"
        ;;
esac

rm -f "$TMP_FILE"

# Remove extracted cruft, keep only the lib
# pdfium archives contain: lib/libpdfium.dylib (macOS), lib/libpdfium.so (Linux), bin/pdfium.dll (Windows)
if [[ -d "$OUT_DIR/lib" ]]; then
    mv "$OUT_DIR/lib/"* "$OUT_DIR/"
    rmdir "$OUT_DIR/lib"
fi
if [[ -d "$OUT_DIR/bin" ]]; then
    mv "$OUT_DIR/bin/"* "$OUT_DIR/"
    rmdir "$OUT_DIR/bin"
fi

# Strip everything except the dylib/so/dll itself
rm -f "$OUT_DIR/PLACEHOLDER" "$OUT_DIR/README.txt" "$OUT_DIR/args.gn" \
      "$OUT_DIR/LICENSE" "$OUT_DIR/PDFiumConfig.cmake" "$OUT_DIR/VERSION" 2>/dev/null || true
rm -rf "$OUT_DIR/include" "$OUT_DIR/licenses" 2>/dev/null || true

echo "=== Files in $OUT_DIR ==="
ls -la "$OUT_DIR/"
echo "Done. Pdfium library placed in $OUT_DIR"
