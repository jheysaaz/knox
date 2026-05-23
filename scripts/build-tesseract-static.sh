#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/third_party/native"

if ! command -v cmake >/dev/null 2>&1; then
  echo "cmake not found" >&2
  exit 1
fi

build_leptonica() {
  local prefix="$1"
  local build_dir="$2"
  local src_dir="$3"

  cmake -S "$src_dir" -B "$build_dir" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_INSTALL_PREFIX="$prefix"
  cmake --build "$build_dir" --config Release
  cmake --install "$build_dir"
}

build_tesseract() {
  local prefix="$1"
  local build_dir="$2"
  local src_dir="$3"
  local leptonica_prefix="$4"

  cmake -S "$src_dir" -B "$build_dir" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DLeptonica_DIR="$leptonica_prefix/lib/cmake/Leptonica"
  cmake --build "$build_dir" --config Release
  cmake --install "$build_dir"
}

copy_libs() {
  local prefix="$1"
  local target_triple="$2"
  local dest="$OUT_DIR/$target_triple"
  mkdir -p "$dest"

  find "$prefix/lib" -maxdepth 1 -type f \( -name "libtesseract.a" -o -name "liblept.a" -o -name "*.a" -o -name "*.lib" \) -print0 \
    | xargs -0 -I{} cp "{}" "$dest/"

  cat > "$dest/link-libs.txt" <<'EOF'
tesseract
lept
png
jpeg
tiff
z
webp
openjp2
gif
EOF
}

if [[ "$#" -lt 3 ]]; then
  echo "Usage: $0 <target-triple> <leptonica-src> <tesseract-src>" >&2
  exit 1
fi

TARGET_TRIPLE="$1"
LEPTONICA_SRC="$2"
TESSERACT_SRC="$3"

BUILD_ROOT="$ROOT_DIR/target/ocr-libs/$TARGET_TRIPLE"
LEPTONICA_BUILD="$BUILD_ROOT/leptonica-build"
TESSERACT_BUILD="$BUILD_ROOT/tesseract-build"
PREFIX="$BUILD_ROOT/prefix"

mkdir -p "$BUILD_ROOT"

build_leptonica "$PREFIX" "$LEPTONICA_BUILD" "$LEPTONICA_SRC"
build_tesseract "$PREFIX" "$TESSERACT_BUILD" "$TESSERACT_SRC" "$PREFIX"
copy_libs "$PREFIX" "$TARGET_TRIPLE"

echo "Static libs copied to $OUT_DIR/$TARGET_TRIPLE"
