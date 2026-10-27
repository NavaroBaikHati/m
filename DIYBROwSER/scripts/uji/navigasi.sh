#!/usr/bin/env bash
# Skenario uji: klik tautan di halaman, address bar, mundur/maju, muat ulang,
# dan membuka situs luar. Dijalankan oleh scripts/smoke-gui.sh di dalam Xvfb.
set -euo pipefail

: "${SHOT_DIR:?SHOT_DIR harus diisi}"
: "${BASE_HALAMAN:?BASE_HALAMAN harus diisi}"

. "$AKAR/scripts/uji/helper-xvfb.sh"

echo "==> Skenario: klik tautan di dalam halaman"
# Tautan "Buka halaman kedua" pada scripts/smoke/index.html (koordinat layar).
klik_layar 513 464 5
ambil_shot "$SHOT_DIR/uji-1-klik-tautan.png"

echo "==> Skenario: ketik alamat situs luar lewat address bar"
ketik_alamat "example.com" 6
ambil_shot "$SHOT_DIR/uji-2-address-bar.png"

echo "==> Skenario: tombol mundur (kembali ke halaman lokal)"
tombol_chrome mundur 4
ambil_shot "$SHOT_DIR/uji-3-mundur.png"

echo "==> Skenario: tombol maju (ke example.com lagi)"
tombol_chrome maju 4
ambil_shot "$SHOT_DIR/uji-4-maju.png"

echo "==> Skenario: muat ulang"
tombol_chrome muat-ulang 3
ambil_shot "$SHOT_DIR/uji-5-muat-ulang.png"

echo "==> Skenario selesai"
