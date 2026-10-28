#!/usr/bin/env bash
# Skenario uji tahap 2: tab baru, pindah tab, tutup tab, ciutkan/buka sidebar.
# Dijalankan oleh scripts/smoke-gui.sh di dalam Xvfb.
#
# Selain screenshot, skenario ini memeriksa warna piksel: dengan sidebar terbuka,
# piksel di kolom kiri harus berwarna sidebar sedangkan piksel di kanan tetap
# konten; setelah diciutkan, kolom kiri harus berubah menjadi konten.
set -euo pipefail

: "${SHOT_DIR:?SHOT_DIR harus diisi}"

. "$AKAR/scripts/uji/helper-xvfb.sh"

gagal() {
  echo "GAGAL: $*"
  exit 1
}

echo "==> Skenario: keadaan awal (satu tab, sidebar terbuka)"
ambil_shot "$SHOT_DIR/tahap2-1-awal.png"

echo "==> Skenario: tombol tab baru di kepala sidebar"
klik_jendela 159 24 4
ambil_shot "$SHOT_DIR/tahap2-2-tab-baru.png"

echo "==> Skenario: pilih tab pertama"
klik_jendela 100 71 3
ambil_shot "$SHOT_DIR/tahap2-3-pindah-tab.png"

echo "==> Skenario: tutup tab kedua lewat tombol silang"
klik_jendela 193 109 3
ambil_shot "$SHOT_DIR/tahap2-4-tutup-tab.png"

SIDEBAR="$(piksel_jendela "$SHOT_DIR/tahap2-1-awal.png" 100 300)"
KONTEN="$(piksel_jendela "$SHOT_DIR/tahap2-1-awal.png" 230 300)"
[[ "$SIDEBAR" != "$KONTEN" ]] ||
  gagal "sidebar dan konten berwarna sama ($SIDEBAR); sidebar mungkin tidak memakan ruang"
echo "    piksel kiri (sidebar) = $SIDEBAR, kanan (konten) = $KONTEN"

echo "==> Skenario: ciutkan sidebar"
klik_jendela 193 24 3
ambil_shot "$SHOT_DIR/tahap2-5-ciut.png"

SETELAH_CIUT="$(piksel_jendela "$SHOT_DIR/tahap2-5-ciut.png" 100 300)"
[[ "$SETELAH_CIUT" != "$SIDEBAR" ]] ||
  gagal "kolom kiri masih berwarna sidebar setelah diciutkan ($SETELAH_CIUT)"
echo "    piksel kiri setelah ciut = $SETELAH_CIUT (konten, bukan sidebar)"

echo "==> Skenario: buka sidebar lagi"
klik_jendela 25 24 3
ambil_shot "$SHOT_DIR/tahap2-6-buka.png"

# Menutup tab terakhir tidak boleh meninggalkan jendela tanpa tab: tab baru
# harus langsung terbuka.
echo "==> Skenario: tutup tab terakhir"
klik_jendela 193 71 5
ambil_shot "$SHOT_DIR/tahap2-7-tutup-terakhir.png"

KONTEN_AKHIR="$(piksel_jendela "$SHOT_DIR/tahap2-7-tutup-terakhir.png" 230 300)"
[[ "$KONTEN_AKHIR" != "$KONTEN" ]] ||
  gagal "konten tidak berganti setelah tab terakhir ditutup ($KONTEN_AKHIR)"
echo "    piksel konten setelah tutup tab terakhir = $KONTEN_AKHIR"

echo "==> Skenario selesai"
