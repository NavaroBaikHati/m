#!/usr/bin/env bash
# Skenario uji pintasan keyboard: Ctrl+T (tab baru), Ctrl+W (tutup tab),
# Ctrl+B (ciutkan sidebar), dan Ctrl+L (fokus address bar).
#
# Bukti diambil dari piksel: halaman uji berwarna biru gelap, halaman tab baru
# berwarna terang. Kalau Ctrl+T tidak sampai ke aplikasi, warna tetap biru.
set -euo pipefail

: "${SHOT_DIR:?SHOT_DIR harus diisi}"
: "${BASE_HALAMAN:?BASE_HALAMAN harus diisi}"

. "$AKAR/scripts/uji/helper-xvfb.sh"

gagal() {
  echo "GAGAL: $*"
  exit 1
}

fokus_aplikasi
sleep 1
ambil_shot "$SHOT_DIR/pintasan-0-awal.png"
AWAL="$(piksel_jendela "$SHOT_DIR/pintasan-0-awal.png" 700 500)"
BAR_PAWAI="$(piksel_jendela "$SHOT_DIR/pintasan-0-awal.png" 800 26)"

echo "==> Ctrl+T harus membuka halaman tab baru"
xdotool key --clearmodifiers ctrl+t
sleep 3
ambil_shot "$SHOT_DIR/pintasan-1-ctrl-t.png"
BARU="$(piksel_jendela "$SHOT_DIR/pintasan-1-ctrl-t.png" 700 500)"
[[ "$BARU" != "$AWAL" ]] || gagal "Ctrl+T tidak mengubah halaman ($BARU)"
echo "    piksel: $AWAL -> $BARU"

echo "==> Ctrl+W harus kembali ke tab pertama"
xdotool key --clearmodifiers ctrl+w
sleep 3
ambil_shot "$SHOT_DIR/pintasan-2-ctrl-w.png"
KEMBALI="$(piksel_jendela "$SHOT_DIR/pintasan-2-ctrl-w.png" 700 500)"
[[ "$KEMBALI" == "$AWAL" ]] || gagal "Ctrl+W tidak kembali ke tab semula ($KEMBALI != $AWAL)"
echo "    piksel kembali = $KEMBALI"

echo "==> Ctrl+L harus memfokuskan address bar"
xdotool key --clearmodifiers ctrl+l
sleep 1
xdotool type --delay 30 "contoh alamat"
sleep 1
ambil_shot "$SHOT_DIR/pintasan-3-ctrl-l.png"

echo "==> Ctrl+L harus memfokuskan address bar"
FOKUS="$(piksel_jendela "$SHOT_DIR/pintasan-3-ctrl-l.png" 800 26)"
[[ "$FOKUS" != "$BAR_PAWAI" ]] || gagal "address bar tidak berubah saat difokuskan ($FOKUS)"
echo "    latar address bar: $BAR_PAWAI -> $FOKUS"

echo "==> Ctrl+B harus menciutkan sidebar"
xdotool key --clearmodifiers Escape
xdotool key --clearmodifiers ctrl+b
sleep 2
ambil_shot "$SHOT_DIR/pintasan-4-ctrl-b.png"
KIRI="$(piksel_jendela "$SHOT_DIR/pintasan-4-ctrl-b.png" 100 300)"
[[ "$KIRI" != "$(piksel_jendela "$SHOT_DIR/pintasan-1-ctrl-t.png" 100 300)" ]] ||
  gagal "Ctrl+B tidak menciutkan sidebar"
echo "    piksel kiri setelah Ctrl+B = $KIRI"

echo "==> Skenario selesai"
