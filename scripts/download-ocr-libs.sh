#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found" >&2
  exit 1
fi

if [[ "$#" -ne 2 ]]; then
  echo "Usage: $0 <release-tag> <target-triple>" >&2
  exit 1
fi

RELEASE_TAG="$1"
TARGET_TRIPLE="$2"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/third_party/native"

mkdir -p "$OUT_DIR/$TARGET_TRIPLE"

ARCHIVE_TGZ="ocr-libs-$TARGET_TRIPLE.tar.gz"
ARCHIVE_ZIP="ocr-libs-$TARGET_TRIPLE.zip"

set +e
gh release download "$RELEASE_TAG" -p "$ARCHIVE_TGZ" -D "$OUT_DIR"
RESULT=$?
set -e

if [[ $RESULT -ne 0 ]]; then
  gh release download "$RELEASE_TAG" -p "$ARCHIVE_ZIP" -D "$OUT_DIR"
  unzip -o "$OUT_DIR/$ARCHIVE_ZIP" -d "$OUT_DIR"
  rm -f "$OUT_DIR/$ARCHIVE_ZIP"
else
  tar -xzf "$OUT_DIR/$ARCHIVE_TGZ" -C "$OUT_DIR"
  rm -f "$OUT_DIR/$ARCHIVE_TGZ"
fi

echo "OCR static libs installed in $OUT_DIR/$TARGET_TRIPLE"
