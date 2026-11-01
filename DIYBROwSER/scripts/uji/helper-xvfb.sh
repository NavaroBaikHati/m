# Helper untuk skenario uji GUI. Hanya berlaku di dalam sesi Xvfb (butuh DISPLAY).
# Harus di-source:  . "$AKAR/scripts/uji/helper-xvfb.sh"

# Mengambil screenshot layar penuh.
ambil_shot() {
  import -window root "$1"
  echo "    screenshot: $1"
}

# Posisi jendela di layar, diisi oleh fokus_aplikasi.
JENDELA_X=0
JENDELA_Y=0

# Mencari jendela utama berdasarkan judul dan ukuran.
#
# `xdotool search --name` cocok sebagian dan tidak peka huruf besar-kecil, jadi
# judul "Perisai" juga mengenai jendela pembantu GTK 10x10 bernama "perisai".
# Jendela pembantu itu tidak bisa difokuskan (X_SetInputFocus -> BadMatch),
# maka pilih jendela yang benar-benar besar.
cari_jendela_utama() {
  local w
  for w in $(xdotool search --name 'Perisai' 2>/dev/null || true); do
    local geometri lebar tinggi
    geometri="$(xdotool getwindowgeometry --shell "$w" 2>/dev/null || true)"
    lebar="$(sed -n 's/^WIDTH=\([0-9]*\)$/\1/p' <<<"$geometri")"
    tinggi="$(sed -n 's/^HEIGHT=\([0-9]*\)$/\1/p' <<<"$geometri")"
    if [[ -n "$lebar" && -n "$tinggi" && "$lebar" -ge 400 && "$tinggi" -ge 300 ]]; then
      echo "$w"
      return 0
    fi
  done
  return 0
}

# Memberi fokus ke jendela aplikasi supaya input keyboard sampai.
# Menunggu sampai jendelanya ada, karena webview dibuat setelah window tampil.
fokus_aplikasi() {
  local jendela=""
  for _ in $(seq 1 40); do
    jendela="$(cari_jendela_utama)"
    [[ -n "$jendela" ]] && break
    sleep 0.5
  done
  if [[ -z "$jendela" ]]; then
    echo "GAGAL: jendela aplikasi tidak ditemukan"
    return 1
  fi
  xdotool windowfocus "$jendela"
  eval "$(xdotool getwindowgeometry --shell "$jendela")"
  JENDELA_X=$X
  JENDELA_Y=$Y
}

# Mengetikkan alamat ke address bar lalu menekan Enter.
# $1 = teks alamat, $2 = detik tunggu sampai halaman dimuat
ketik_alamat() {
  fokus_aplikasi
  sleep 1
  # Address bar ada di baris chrome (tinggi 48 px, jadi titik tengahnya ~y=24).
  xdotool mousemove 400 24 click 1
  sleep 0.5
  xdotool key --clearmodifiers ctrl+a
  xdotool type --delay 30 "$1"
  xdotool key Return
  sleep "${2:-4}"
}

# Klik pada koordinat layar (mis. tautan di dalam halaman).
# $1 x, $2 y, $3 detik tunggu
klik_layar() {
  fokus_aplikasi
  xdotool mousemove "$1" "$2" click 1
  sleep "${3:-3}"
}

# Klik pada koordinat lokal jendela (0,0 = pojok kiri atas klien).
# $1 x, $2 y, $3 detik tunggu
klik_jendela() {
  fokus_aplikasi
  xdotool mousemove $((JENDELA_X + $1)) $((JENDELA_Y + $2)) click 1
  sleep "${3:-2}"
}

# Warna piksel pada screenshot $1 di koordinat lokal jendela ($2,$3).
piksel_jendela() {
  convert "$1" -format "%[pixel:p{$((JENDELA_X + $2)),$((JENDELA_Y + $3))}]" info:
}

# Menekan tombol di chrome berdasarkan nama aksinya.
# $1 = mundur | maju | muat-ulang, $2 = detik tunggu
tombol_chrome() {
  local x
  case "$1" in
  mundur) x=68 ;;
  maju) x=103 ;;
  muat-ulang) x=138 ;;
  *)
    echo "GAGAL: tombol chrome tidak dikenal: $1"
    return 1
    ;;
  esac
  fokus_aplikasi
  xdotool mousemove "$x" 24 click 1
  sleep "${2:-3}"
}
