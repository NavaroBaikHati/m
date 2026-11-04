#!/usr/bin/env bash
# Reproduksi: apakah isi tab terpotong setelah jendela diperbesar/mengecil cepat?
# Pakai: scripts/uji/resize.sh <biner> <halaman> <tangkap-dir>
set -u
BINER=${1:?biner}
HALAMAN=${2:?url}
OUT=${3:-/tmp/resize}
mkdir -p "$OUT"
rm -f "$OUT"/*.png

Xvfb :99 -screen 0 1400x1000x24 >/tmp/xvfb-resize.log 2>&1 &
XVFB=$!
sleep 1
export DISPLAY=:99
export HOME=/tmp/home-resize
rm -rf "$HOME"; mkdir -p "$HOME"

"$BINER" "$HALAMAN" >"$OUT/app.log" 2>&1 &
APP=$!
sleep 7

WIN=$(xdotool search --onlyvisible --class Perisai | head -1)
if [ -z "$WIN" ]; then
  WIN=$(xdotool search --onlyvisible --name "Perisai" | head -1)
fi
echo "window=$WIN"
xdotool windowsize "$WIN" 1000 700
sleep 1
import -window root "$OUT/1-awal.png"

# Resize cepat: banyak ukuran beruntun tanpa jeda, lalu berhenti di ukuran akhir.
for ukuran in 600x420 1250x900 520x380 1180x620 700x460 1100x760 640x400 1240x880 900x560 1150x720; do
  xdotool windowsize "$WIN" "${ukuran%x*}" "${ukuran#*x}"
done
import -window root "$OUT/2-tepat-setelah.png"
sleep 0.4
import -window root "$OUT/3-400ms.png"
sleep 1.5
import -window root "$OUT/4-2s.png"

echo "--- geometri X window: $(xdotool getwindowgeometry "$WIN" | tr '\n' ' ')"
xwininfo -id "$WIN" | grep -E "Width|Height|Absolute upper-left"

kill "$APP" 2>/dev/null
sleep 1
kill "$XVFB" 2>/dev/null
echo "selesai: $OUT"
