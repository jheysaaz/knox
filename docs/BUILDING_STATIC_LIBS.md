# Building Static Tesseract/Leptonica

This project expects prebuilt static archives in `src-tauri/third_party/native/<target-triple>`.

## Local build (CMake)

1) Install build tools:

- macOS:
```
brew install cmake ninja pkg-config autoconf automake libtool
```

- Linux:
```
sudo apt-get install -y cmake ninja-build pkg-config autoconf automake libtool build-essential yasm
```

- Windows:
```
choco install -y cmake ninja
```

2) Clone sources:
```
git clone https://github.com/DanBloomberg/leptonica.git
git clone https://github.com/tesseract-ocr/tesseract.git
```

3) Build and install static libs:
```
bash scripts/build-tesseract-static.sh <target-triple> <leptonica-src> <tesseract-src>
```

Example (macOS ARM):
```
bash scripts/build-tesseract-static.sh aarch64-apple-darwin leptonica tesseract
```

The script will copy static archives and emit `link-libs.txt` in the target directory.

## GitHub Actions (CI)

The release workflow builds static libs on each runner using the same script and caches them.
For reusable artifacts, run the `ocr-libs` workflow which publishes per-target archives.

### Publish libraries from CI

1) Run the `ocr-libs` workflow with a tag like `ocr-libs-v1`.
2) It publishes:
   - `ocr-libs-aarch64-apple-darwin.tar.gz`
   - `ocr-libs-x86_64-pc-windows-msvc.zip`
   - `ocr-libs-x86_64-unknown-linux-gnu.tar.gz`

## Output layout

```
src-tauri/third_party/native/<target-triple>/
  libtesseract.a
  liblept.a
  link-libs.txt
```

## link-libs.txt

Contains one static library name per line for `build.rs` to link. Example:

```
tesseract
lept
png
jpeg
tiff
z
webp
openjp2
gif
```

## Local download (from release)

```
bash scripts/download-ocr-libs.sh ocr-libs-v1 aarch64-apple-darwin
```

## Supported targets

- macOS Apple Silicon: `aarch64-apple-darwin`
- Windows: `x86_64-pc-windows-msvc`
- Linux: `x86_64-unknown-linux-gnu`
