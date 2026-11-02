#!/usr/bin/env bash
# Membuktikan content blocker bekerja, dengan pembanding.
#
# Dijalankan dua kali pada halaman yang sama:
#   1. dengan aturan uji "blokir iklan.js"  -> kotak merah tidak boleh ada
#   2. tanpa aturan uji                     -> kotak merah harus ada
# Kalau keduanya sama, uji ini tidak membuktikan apa pun dan dianggap gagal.
set -euo pipefail

AKAR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
URL="${1:-http://127.0.0.1:8123/}"
TMP="$(mktemp -d -t perisai-adblock-XXXXXX)"
# Artefak disimpan kalau PRESERVE=1 supaya kegagalan bisa diperiksa.
if [[ "${PRESERVE:-0}" == "1" ]]; then
  echo "artefak: $TMP"
else
  trap 'rm -rf "$TMP"' EXIT
fi

echo "=== Putaran 1: content blocker aktif ==="
PERISAI_FILTER_UJI="$AKAR/scripts/uji/aturan-uji.txt" \
  "$AKAR/scripts/smoke-gui.sh" "$URL" "$TMP/dengan.png" "$AKAR/scripts/uji/adblock.sh" >"$TMP/log1.txt" 2>&1 ||
  { tail -40 "$TMP/log1.txt"; exit 1; }
DENGAN="$(cat "$TMP/piksel-adblock.txt")"

echo "=== Putaran 2: tanpa aturan uji (pembanding) ==="
env -u PERISAI_FILTER_UJI \
  "$AKAR/scripts/smoke-gui.sh" "$URL" "$TMP/tanpa.png" "$AKAR/scripts/uji/adblock.sh" >"$TMP/log2.txt" 2>&1 ||
  { tail -40 "$TMP/log2.txt"; exit 1; }
TANPA="$(cat "$TMP/piksel-adblock.txt")"

echo "piksel dengan filter = $DENGAN"
echo "piksel tanpa aturan  = $TANPA"

[[ "$TANPA" == "srgb(255,0,0)" ]] || {
  echo "GAGAL: pembanding tidak menampilkan kotak merah; uji tidak sah"
  exit 1
}
[[ "$DENGAN" != "srgb(255,0,0)" ]] || {
  echo "GAGAL: kotak iklan tetap muncul meski aturan blokir aktif"
  exit 1
}
echo "LULUS: request iklan diblokir (kotak merah hilang) dan pembanding membuktikan ujinya sah"
