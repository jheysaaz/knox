#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG="$ROOT/src-tauri/tauri.conf.json"
BACKUP="$ROOT/src-tauri/tauri.conf.json.test-update-bak"
TEMP_DIR="$ROOT/tmp/test-update"
PORT=9876
BIN_SIZE_MB=10

cleanup() {
  echo ""
  echo "Restoring tauri.conf.json..."
  if [ -f "$BACKUP" ]; then
    cp "$BACKUP" "$CONFIG"
    rm -f "$BACKUP"
  fi
  echo "Done."
}
trap cleanup EXIT INT TERM

# ── Backup config ────────────────────────────────────────────────
cp "$CONFIG" "$BACKUP"

# ── Create test update artifacts ──────────────────────────────────
mkdir -p "$TEMP_DIR"

# Generate a dummy binary
dd if=/dev/urandom of="$TEMP_DIR/test.dmg" bs=1M count="$BIN_SIZE_MB" 2>/dev/null
echo "Created $TEMP_DIR/test.dmg ($BIN_SIZE_MB MB)"

# Read current version from config
CURRENT_VERSION="$(jq -r '.version' "$CONFIG")"
echo "Current app version: $CURRENT_VERSION"

# Bump the version patch, keep same pre-release channel
# e.g., 1.0.0-beta.1 → 1.0.0-beta.2
INCREMENTED="$(echo "$CURRENT_VERSION" | awk -F. '{
  if ($NF ~ /^[0-9]+$/) {
    $NF = $NF + 1
  } else {
    split($NF, parts, "-")
    if (parts[2] ~ /^[0-9]+$/) {
      parts[2] = parts[2] + 1
      $NF = parts[1] "-" parts[2]
    } else {
      $NF = $NF ".1"
    }
  }
  print
}' OFS='.')"

# If the above awk didn't handle it, try simpler approach
if [ "$INCREMENTED" = "$CURRENT_VERSION" ]; then
  if [[ "$CURRENT_VERSION" =~ ^(.+\.)([0-9]+)$ ]]; then
    INCREMENTED="${BASH_REMATCH[1]}$((BASH_REMATCH[2] + 1))"
  elif [[ "$CURRENT_VERSION" =~ ^(.+)(beta|alpha|rc|dev)\.([0-9]+)$ ]]; then
    INCREMENTED="${BASH_REMATCH[1]}${BASH_REMATCH[2]}.$((BASH_REMATCH[3] + 1))"
  else
    INCREMENTED="$CURRENT_VERSION-dev.1"
  fi
fi

echo "Test update version: $INCREMENTED"

# Extract channel for display
CHANNEL=""
if [[ "$INCREMENTED" =~ beta ]]; then
  CHANNEL="beta"
elif [[ "$INCREMENTED" =~ alpha ]]; then
  CHANNEL="alpha"
elif [[ "$INCREMENTED" =~ rc ]]; then
  CHANNEL="rc"
elif [[ "$INCREMENTED" =~ dev ]]; then
  CHANNEL="dev"
else
  CHANNEL="stable"
fi

cat > "$TEMP_DIR/latest.json" <<JSON
{
  "version": "$INCREMENTED",
  "notes": "Local test update ($CHANNEL channel)",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {
    "darwin-aarch64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRlc3QgdXBkYXRlIHNlcnZlcgprZXk6IHRlc3QK",
      "url": "http://localhost:$PORT/test.dmg"
    },
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRlc3QgdXBkYXRlIHNlcnZlcgprZXk6IHRlc3QK",
      "url": "http://localhost:$PORT/test.msi"
    }
  }
}
JSON

echo "Created $TEMP_DIR/latest.json"

# ── Patch tauri.conf.json to point to local server ──────────────
if command -v jq &>/dev/null; then
  TMP_CONFIG="$TEMP_DIR/tauri.conf.patched.json"
  jq --arg url "http://localhost:$PORT/latest.json" \
    '.plugins.updater.endpoints = [$url]' \
    "$CONFIG" > "$TMP_CONFIG"
  cp "$TMP_CONFIG" "$CONFIG"
  echo "Patched updater endpoint to http://localhost:$PORT/latest.json"
else
  # Fallback: sed-based replacement
  sed -i.bak 's|https://github.com/jheysaaz/knox/releases/latest/download/latest.json|http://localhost:'"$PORT"'/latest.json|' "$CONFIG"
  rm -f "$CONFIG.bak"
  echo "Patched updater endpoint (sed fallback)"
fi

# ── Start HTTP server ────────────────────────────────────────────
echo ""
echo "============================================"
echo "  Test update server running on port $PORT"
echo "  Version: $CURRENT_VERSION → $INCREMENTED"
echo "  Channel: $CHANNEL"
echo "============================================"
echo ""
echo "Start your Tauri app in another terminal:"
echo "  pnpm tauri dev"
echo ""
echo "Press Ctrl+C to stop server and restore config."
echo ""

cd "$TEMP_DIR"
python3 -m http.server "$PORT" --bind 127.0.0.1
