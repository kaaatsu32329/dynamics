#!/usr/bin/env bash
# Run just this one script to:
#   1. build WebAssembly if not built / sources are newer (build-web.sh)
#   2. start a local HTTP server
#   3. open the page in the default browser
#   (Ctrl-C to stop; the server is cleaned up automatically)
#
#   ./run.sh            # auto-pick a default port (from 8000)
#   ./run.sh 8080       # specify a port
set -euo pipefail
cd "$(dirname "$0")"

WASM=web/pkg/dyn_wasm_bg.wasm

# --- 1. Decide whether a build is needed (not built, or sources newer than the artifact) ---
need_build=0
if [[ ! -f "$WASM" ]]; then
  need_build=1
elif [[ -n "$(find crates/dyn-core crates/dyn-wasm Cargo.lock -newer "$WASM" 2>/dev/null)" ]]; then
  echo "ℹ ソースが更新されています。再ビルドします。"
  need_build=1
fi
if [[ "$need_build" -eq 1 ]]; then
  ./build-web.sh
else
  echo "✓ ビルド済み ($WASM) を使用します。"
fi

# --- 2. Find a free port ---
PORT="${1:-}"
if [[ -z "$PORT" ]]; then
  for p in 8000 8001 8002 8003 8080 8081; do
    if ! nc -z localhost "$p" 2>/dev/null; then PORT=$p; break; fi
  done
  : "${PORT:=8000}"
fi
URL="http://localhost:${PORT}/index.html"

# --- 3. Start the server (always stopped on exit) ---
echo "▶ serving web/ at ${URL}"
python3 -m http.server "$PORT" --directory web >/dev/null 2>&1 &
SERVER_PID=$!
cleanup() { kill "$SERVER_PID" 2>/dev/null || true; echo; echo "■ サーバを停止しました。"; }
trap cleanup EXIT INT TERM

# Wait until the server responds
for _ in $(seq 1 50); do
  if nc -z localhost "$PORT" 2>/dev/null; then break; fi
  sleep 0.1
done

# --- 4. Open the browser ---
if command -v open >/dev/null 2>&1; then open "$URL"          # macOS
elif command -v xdg-open >/dev/null 2>&1; then xdg-open "$URL" # Linux
elif command -v start >/dev/null 2>&1; then start "$URL"       # Windows
else echo "ブラウザで開いてください: $URL"; fi

echo "（Ctrl-C で停止）"
wait "$SERVER_PID"
