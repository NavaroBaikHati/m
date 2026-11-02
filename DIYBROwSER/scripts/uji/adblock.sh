#!/usr/bin/env bash
# Skenario uji: buktikan content blocker benar-benar memblokir request.
#
# Halaman /iklan.html memuat /iklan.js yang menggambar kotak merah di pojok
# kiri atas area halaman. Kalau request diblokir, piksel di tengah kotak tetap
# putih. Skenario ini menulis warna piksel tersebut ke piksel-adblock.txt;
# pembandingnya ada di scripts/uji-adblock.sh yang menjalankan dua kali
# (dengan dan tanpa aturan uji).
set -euo pipefail

: "${SHOT_DIR:?SHOT_DIR harus diisi}"
: "${BASE_HALAMAN:?BASE_HALAMAN harus diisi}"
: "${LOG_APLIKASI:?LOG_APLIKASI harus diisi}"

. "$AKAR/scripts/uji/helper-xvfb.sh"

echo "==> Menunggu filter selesai dikompilasi WebKit"
for _ in $(seq 1 120); do
  grep -q "adblock: siap" "$LOG_APLIKASI" && break
  sleep 0.5
done
if ! grep -q "adblock: siap" "$LOG_APLIKASI"; then
  echo "GAGAL: filter tidak pernah siap"
  exit 1
fi
grep "adblock: siap" "$LOG_APLIKASI" | tail -1

echo "==> Buka halaman uji iklan"
# Halaman uji berwarna putih; kalau masih biru gelap berarti navigasi belum
# terjadi (fokus/pengetikan meleset), jadi dicoba lagi.
for percobaan in 1 2 3; do
  ketik_alamat "$BASE_HALAMAN/iklan.html" 3
  ambil_shot "$SHOT_DIR/adblock-coba.png"
  LATAR="$(piksel_jendela "$SHOT_DIR/adblock-coba.png" 700 500)"
  [[ "$LATAR" == "srgb(255,255,255)" ]] && break
  echo "    percobaan $percobaan: halaman belum termuat ($LATAR)"
done
# Webview sudah terbuka sebelum filter siap; muat ulang agar filter berlaku.
fokus_aplikasi
xdotool key --clearmodifiers ctrl+r
sleep 3
ambil_shot "$SHOT_DIR/adblock.png"

# Kotak iklan: kiri 40 px, atas 40 px, 120x80 di dalam halaman yang mulai
# pada (232,52). Titik tengahnya ada di (332,132).
PIKSEL="$(piksel_jendela "$SHOT_DIR/adblock.png" 332 132)"
echo "    piksel tengah kotak iklan = $PIKSEL"
echo "$PIKSEL" >"$SHOT_DIR/piksel-adblock.txt"

echo "==> Skenario selesai"
