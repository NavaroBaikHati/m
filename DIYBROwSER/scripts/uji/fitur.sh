#!/usr/bin/env bash
# Skenario uji fitur penyimpanan: bookmark (Ctrl+D), zoom (Ctrl+=), dan
# riwayat. Pembuktiannya dilakukan setelah aplikasi keluar, langsung di
# basis data SQLite oleh scripts/uji-fitur.sh.
set -euo pipefail

: "${SHOT_DIR:?SHOT_DIR harus diisi}"

. "$AKAR/scripts/uji/helper-xvfb.sh"

fokus_aplikasi
sleep 2

echo "==> Ctrl+D menyimpan bookmark"
xdotool key --clearmodifiers ctrl+d
sleep 1

echo "==> Ctrl+= memperbesar dua kali"
xdotool key --clearmodifiers ctrl+equal
xdotool key --clearmodifiers ctrl+equal
sleep 1
ambil_shot "$SHOT_DIR/fitur-zoom.png"

echo "==> Simpan keadaan akhir"
sleep 1
ambil_shot "$SHOT_DIR/fitur-akhir.png"

echo "==> Skenario selesai"
