#!/usr/bin/env bash
# Build dyn-wasm to WebAssembly and generate the JS glue into web/pkg.
# The required toolchain (wasm target / wasm-bindgen-cli) is set up automatically.
#
# Usage:
#   ./build-web.sh          # build only
#   ./build-web.sh --serve  # build and serve at http://localhost:8000
set -euo pipefail
cd "$(dirname "$0")"

# Get the required wasm-bindgen version from Cargo.lock (to match the CLI).
WB_VER=$(awk '/name = "wasm-bindgen"/{f=1} f&&/version = /{gsub(/[" ]/,"",$3); print $3; exit}' Cargo.lock)
: "${WB_VER:=0.2.125}"

ensure_toolchain() {
  if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "▶ rustup target add wasm32-unknown-unknown"
    rustup target add wasm32-unknown-unknown
  fi
  local have
  have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
  if [[ "$have" != "$WB_VER" ]]; then
    echo "▶ cargo install wasm-bindgen-cli --version $WB_VER  (現在: ${have:-なし})"
    cargo install wasm-bindgen-cli --version "$WB_VER" --force
  fi
}

ensure_toolchain

echo "▶ cargo build (wasm32-unknown-unknown, release)"
cargo build --release --target wasm32-unknown-unknown -p dyn-wasm

echo "▶ wasm-bindgen → web/pkg"
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/dyn_wasm.wasm

echo "✓ build complete → web/index.html"

if [[ "${1:-}" == "--serve" ]]; then
  echo "▶ serving web/ at http://localhost:8000  (Ctrl-C で停止)"
  python3 -m http.server 8000 --directory web
fi
