# Helper untuk skenario uji GUI. Hanya berlaku di dalam sesi Xvfb (butuh DISPLAY).
# Harus di-source:  . "$AKAR/scripts/uji/helper-xvfb.sh"

# Mengambil screenshot layar penuh.
ambil_shot() {
  import -window root "$1"
  echo "    screenshot: $1"
}

# Memberi fokus ke jendela aplikasi supaya input keyboard sampai.
# Menunggu sampai jendelanya ada, karena webview dibuat setelah window tampil.
fokus_aplikasi() {
  local jendela=""
  for _ in $(seq 1 40); do
    jendela="$(xdotool search --name 'DIY Browser' 2>/dev/null | head -1 || true)"
    [[ -n "$jendela" ]] && break
    sleep 0.5
  done
  if [[ -z "$jendela" ]]; then
    echo "GAGAL: jendela aplikasi tidak ditemukan"
    return 1
  fi
  xdotool windowfocus "$jendela"
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
