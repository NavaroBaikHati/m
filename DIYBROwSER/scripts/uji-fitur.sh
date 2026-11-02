#!/usr/bin/env bash
# Menjalankan skenario fitur lalu memeriksa hasilnya di SQLite.
set -euo pipefail

AKAR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
URL="${1:-http://127.0.0.1:8123/}"
DB="${PERISAI_DB:-$HOME/.local/share/dev.perisai.app/perisai.sqlite}"
TMP="$(mktemp -d -t perisai-fitur-XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

rm -f "$DB"
"$AKAR/scripts/smoke-gui.sh" "$URL" "$TMP/fitur.png" "$AKAR/scripts/uji/fitur.sh" >"$TMP/log.txt" 2>&1 ||
  { tail -40 "$TMP/log.txt"; exit 1; }

[[ -f "$DB" ]] || { echo "GAGAL: basis data $DB tidak dibuat"; exit 1; }

python3 - "$DB" <<'PY'
import sqlite3, sys

db = sqlite3.connect(sys.argv[1])
bookmark = db.execute("SELECT url, judul FROM bookmark").fetchall()
riwayat = db.execute("SELECT url FROM riwayat").fetchall()
zoom = db.execute("SELECT nilai FROM pengaturan WHERE kunci = 'zoom'").fetchone()
sesi = db.execute("SELECT nilai FROM pengaturan WHERE kunci = 'sesi'").fetchone()

if not bookmark:
    sys.exit("GAGAL: Ctrl+D tidak menyimpan bookmark")
if not riwayat:
    sys.exit("GAGAL: kunjungan tidak tercatat di riwayat")
if not zoom:
    sys.exit("GAGAL: zoom tidak disimpan")
import json
nilai_zoom = json.loads(zoom[0])["per_host"]
if not any(0.9 < v <= 1.3 and v != 1.0 for v in nilai_zoom.values()):
    sys.exit(f"GAGAL: zoom per situs tidak tercatat: {nilai_zoom}")
if not sesi:
    sys.exit("GAGAL: sesi tidak disimpan")

print(f"bookmark   : {bookmark[0][1]} ({bookmark[0][0]})")
print(f"riwayat    : {len(riwayat)} entri")
print(f"zoom       : {zoom[0]}")
print(f"sesi       : {sesi[0]}")
PY

echo "LULUS: bookmark, riwayat, zoom, dan sesi tersimpan di SQLite"
