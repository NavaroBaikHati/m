# Laporan Rilis 0.2.3 — video YouTube tidak lagi tersendat/macet

## Gejala yang dilaporkan

Membuka YouTube membuat Perisai terasa lag lalu freeze. Log dari mesin pengguna
(dijalankan dengan biner telanjang `perisai-0.2.1-linux-x86_64`) berisi:

```
Gdk-CRITICAL **: gdk_wayland_window_set_dbus_properties_libgtk_only:
  assertion 'GDK_IS_WAYLAND_WINDOW (window)' failed
```

Pesan itu tanda sesi Wayland dijalankan lewat jembatan X11 (XWayland) — GTK
mengira punya jendela Wayland, sementara salah satu jendela aslinya X11.

## Apa yang diukur, bukan ditebak

Sebelum mengubah kode, beban filter diukur dengan `crates/adblock-core`
(contoh sementara, tidak ikut dirilis):

| Tahap | Waktu |
|---|---|
| Bangun mesin dari daftar tersimpan | ± 348 ms (sekali, di latar belakang) |
| Query cosmetic + scriptlet per halaman | 10–120 µs |
| Query untuk host tanpa aturan | < 10 µs |

Artinya filter 130.343 aturan **bukan** penyebab lag saat menggulir atau memutar
video: biayanya konstan dan kecil per navigasi.

Penyebab yang tersisa ada di jalur render:

1. **Renderer DMA-BUF di XWayland.** Buffer GPU harus menyeberang antara Wayland
   dan X11. Pada WebKitGTK jalur ini dikenal membuat video tersendat, lalu
   jendela berhenti menggambar.
2. **`HardwareAccelerationPolicy::Always`.** Kebijakan itu dipaksa sejak 0.1.0
   dengan alasan "biar kencang". Di driver yang tidak stabil ia justru menghapus
   jalan mundur: WebKit tidak boleh jatuh ke compositing perangkat lunak, jadi
   halaman berat berhenti menggambar dan tampak freeze.

Bukti pendukung dari uji di mesin build: dengan `WAYLAND_DISPLAY` diarahkan ke
socket Wayland yang tidak ada **dan** `GDK_BACKEND=x11` (kondisi yang meniru
XWayland tanpa compositor), halaman `watch` YouTube membuat proses **segfault**
setelah beberapa detik, sementara `example.com` normal. Itu artefak lingkungan
uji, bukan kondisi pengguna, tetapi menunjukkan jalur render itulah yang rapuh
pada halaman video — bukan jaringan, dan bukan filter.

## Perubahan

- `src-tauri/src/lingkungan.rs`: fungsi `rapikan_renderer()` mendeteksi sesi
  Wayland yang berjalan lewat X11 (`WAYLAND_DISPLAY` ada **dan** `GDK_BACKEND=x11`)
  lalu memasang `WEBKIT_DISABLE_DMABUF_RENDERER=1` sebelum GTK dimulai. Ini
  menutup dua jalan sekaligus: AppImage (yang `AppRun`-nya memaksa `GDK_BACKEND=x11`)
  dan biner telanjang yang dijalankan dengan backend X11. Nilai yang sudah diatur
  pengguna tidak ditimpa. Wayland asli tidak disentuh karena di sana DMA-BUF
  justru jalur tercepat.
- `src-tauri/src/platform/linux/mod.rs`: kebijakan akselerasi sekarang `OnDemand`
  (bawaan WebKitGTK), bukan `Always`. Pilihan manual lewat `PERISAI_AKSELERASI`
  (`selalu` / `perangkat-lunak`).
- Tidak lagi memaksa render perangkat lunak hanya karena `LIBGL_ALWAYS_SOFTWARE`
  atau `WEBKIT_DISABLE_COMPOSITING_MODE` ada. Kedua variabel itu mengatur GL dan
  compositing, bukan kebijakan WebKit, dan memaksa `Never` di sana justru
  menyalin jalur yang di mesin build membuat proses crash.
- `README.md`: bagian akselerasi diperbarui, ditambah bagian penelusuran
  "Video YouTube tersendat atau macet" berisi langkah pembeda penyebab.

## Verifikasi di mesin build

- `cargo test --workspace` — **41 lolos** (adblock-core 10, perisai 25, storage 6),
  termasuk dua tes baru: keputusan mematikan DMA-BUF hanya untuk Wayland-lewat-X11,
  dan pemetaan `PERISAI_AKSELERASI` ke kebijakan WebKit.
- `cargo fmt --all --check` — lolos.
- `cargo clippy --workspace --all-targets -- -D warnings` — lolos, 0 warning.
- `npm --prefix ui run build` — lolos.
- Biner rilis 0.2.3 dijalankan di Xvfb dengan `WAYLAND_DISPLAY` palsu dan
  `GDK_BACKEND=x11`: stderr memuat baris deteksi (`lingkungan: sesi Wayland lewat
  X11; renderer DMA-BUF dimatikan...`), jadi jalur deteksi benar-benar aktif di
  biner yang dirilis.
- Tiga varian (`biasa`, `PERISAI_TANPA_ADBLOCK=1`, `PERISAI_AKSELERASI=perangkat-lunak`)
  memuat `youtube.com/watch?v=dQw4w9WgXcQ` dan **tetap hidup** setelah 14 detik
  tanpa panic di log. UI tetap responsif: `Ctrl+T` setelah halaman video terbuka
  langsung menampilkan tab baru dengan halaman beranda (screenshot diperiksa).

## Batasan

- Mesin build tidak punya GPU maupun sesi Wayland, jadi peningkatan kelancaran
  pemutaran **belum** diukur di sini; yang terverifikasi adalah mekanisme
  deteksi, kebijakan render, dan tidak ada regresi pada uji yang ada.
- Kalau setelah ini YouTube masih tersendat, langkah pembeda ada di README:
  `PERISAI_AKSELERASI=perangkat-lunak` (uji driver) dan `PERISAI_TANPA_ADBLOCK=1`
  (uji filter). Hasil keduanya menentukan langkah berikutnya.
- Pemutar video di mesin build menulis *"Your browser can't play this video"*
  karena container ini tidak punya plugin GStreamer H.264. Itu bukan bug
  Perisai: di Arch perlu `gst-plugins-good gst-plugins-bad gst-plugins-ugly
  gst-libav`. Video tanpa codec diputar lewat jalur perangkat lunak atau gagal,
  dan itu terasa sebagai lambat; bagian ini belum bisa diuji di sini.
