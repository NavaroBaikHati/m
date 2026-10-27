#!/usr/bin/env bash
# Smoke test GUI tanpa layar: jalankan aplikasi di Xvfb, pastikan tidak crash,
# lalu simpan screenshot sebagai bukti.
#
# Build debug memuat UI dari devUrl (http://localhost:1420), jadi skrip ini
# menyalakan server dev Vite dan server halaman uji lebih dulu.
#
# Pakai: scripts/smoke-gui.sh [URL] [FILE_SCREENSHOT] [SKENARIO]
#   SKENARIO (opsional) = skrip yang dijalankan di dalam sesi Xvfb, lihat
#   scripts/uji/navigasi.sh.
#
# Contoh:
#   scripts/smoke-gui.sh
#   scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap1.png scripts/uji/navigasi.sh
set -euo pipefail

AKAR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
URL="${1:-http://127.0.0.1:8123/}"
SHOT="${2:-$AKAR/docs/screenshots/tahap1.png}"
SKENARIO="${3:-}"
PORT_UI=1420
PORT_HALAMAN=8123
BASE_HALAMAN="http://127.0.0.1:$PORT_HALAMAN"
DETIK_TUNGGU="${DETIK_TUNGGU:-16}"
SHOT_DIR="$(dirname "$SHOT")"

export PATH="$HOME/.cargo/bin:$PATH"
TARGET_DIR="${CARGO_TARGET_DIR:-$AKAR/target}"
BIN="$TARGET_DIR/debug/diy-browser"

# Rendering perangkat lunak: VPS tanpa GPU sering bikin WebKitGTK gagal render.
export LIBGL_ALWAYS_SOFTWARE=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GDK_BACKEND=x11
export NO_AT_BRIDGE=1

LOG_APLIKASI="$(mktemp -t diybrowser-app-XXXXXX.log)"
LOG_UI="$(mktemp -t diybrowser-ui-XXXXXX.log)"
LOG_HALAMAN="$(mktemp -t diybrowser-halaman-XXXXXX.log)"
GAGAL=0

bersihkan() {
  for pid in "${PID_UI:-}" "${PID_HALAMAN:-}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  if [[ "$GAGAL" == "1" ]]; then
    echo "--- log aplikasi ---"
    tail -40 "$LOG_APLIKASI" || true
  fi
}
trap bersihkan EXIT

tunggu_url() {
  for _ in $(seq 1 40); do
    curl -sf "$1" -o /dev/null && return 0
    sleep 0.5
  done
  return 1
}

echo "==> Build debug (cargo build -p diy-browser)"
(cd "$AKAR" && cargo build -p diy-browser)
[[ -x "$BIN" ]] || {
  echo "GAGAL: biner $BIN tidak ada"
  exit 1
}

echo "==> Nyalakan server dev UI di http://localhost:$PORT_UI/"
# Vite mengikat ke "localhost", yang bisa berarti IPv6 saja.
(cd "$AKAR/ui" && npm run dev >"$LOG_UI" 2>&1) &
PID_UI=$!
tunggu_url "http://localhost:$PORT_UI/" || {
  GAGAL=1
  echo "GAGAL: server dev UI tidak siap"
  tail -20 "$LOG_UI"
  exit 1
}

echo "==> Nyalakan server halaman uji di $BASE_HALAMAN/"
(cd "$AKAR/scripts/smoke" && python3 -m http.server "$PORT_HALAMAN" --bind 127.0.0.1 \
  >"$LOG_HALAMAN" 2>&1) &
PID_HALAMAN=$!
tunggu_url "$BASE_HALAMAN/" || {
  GAGAL=1
  echo "GAGAL: server halaman uji tidak siap"
  exit 1
}

mkdir -p "$SHOT_DIR"

SKRIP_DALAM="$(mktemp -t diybrowser-xvfb-XXXXXX.sh)"
cat >"$SKRIP_DALAM" <<'SKRIP'
set -u
export APP_PID=""
"${BIN}" "${URL}" >"${LOG_APLIKASI}" 2>&1 &
APP_PID=$!

if [[ -n "${SKENARIO}" ]]; then
  # Beri waktu UI terpasang sebelum skenario mulai berinteraksi.
  sleep "${DETIK_AWAL:-8}"
  # shellcheck source=/dev/null
  . "${SKENARIO}"
else
  sleep "${DETIK_TUNGGU}"
fi

if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "GAGAL: aplikasi berhenti sendiri sebelum screenshot"
  exit 1
fi

import -window root "${SHOT}" || { echo "GAGAL: screenshot tidak bisa diambil"; exit 1; }
kill "$APP_PID" 2>/dev/null
# Skenario bisa menyalakan `set -e`, dan aplikasi yang di-terminate membuat
# `wait` mengembalikan 143 — jangan sampai itu dianggap kegagalan.
wait "$APP_PID" 2>/dev/null || true
exit 0
SKRIP

echo "==> Jalankan aplikasi di Xvfb, buka $URL"
set +e
BIN="$BIN" URL="$URL" LOG_APLIKASI="$LOG_APLIKASI" SKENARIO="$SKENARIO" SHOT="$SHOT" \
  SHOT_DIR="$SHOT_DIR" AKAR="$AKAR" BASE_HALAMAN="$BASE_HALAMAN" DETIK_TUNGGU="$DETIK_TUNGGU" \
  xvfb-run -a --server-args="-screen 0 1280x800x24" bash "$SKRIP_DALAM"
KODE_XVFB=$?
set -e
rm -f "$SKRIP_DALAM"

if [[ $KODE_XVFB -ne 0 ]]; then
  GAGAL=1
  echo "GAGAL: xvfb-run keluar dengan kode $KODE_XVFB"
  exit 1
fi

if grep -qi "panicked" "$LOG_APLIKASI"; then
  GAGAL=1
  echo "GAGAL: ada panic di log aplikasi"
  exit 1
fi

[[ -s "$SHOT" ]] || {
  GAGAL=1
  echo "GAGAL: screenshot kosong"
  exit 1
}

# Screenshot seragam (standar deviasi rendah) berarti layar kosong -> bukti tidak sah.
SIMPANGAN="$(identify -format '%[standard-deviation]' "$SHOT" 2>/dev/null || echo 0)"
if awk "BEGIN{exit !($SIMPANGAN < 100)}"; then
  GAGAL=1
  echo "GAGAL: screenshot tampak kosong (standar deviasi $SIMPANGAN)"
  exit 1
fi

echo "LULUS: aplikasi jalan di Xvfb tanpa crash"
echo "Screenshot akhir: $SHOT"
echo "Ukuran: $(identify -format '%wx%h' "$SHOT")"
