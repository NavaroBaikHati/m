# Rilis 0.3.3 — AppImage tanpa codec

0.3.2 memperbaiki user agent, tetapi keluhan *"your browser can't play this
video"* tetap muncul. Ternyata UA bukan satu-satunya pintu: video YouTube butuh
MediaSource yang didukung codec. Di AppImage, tidak ada satu pun codec yang
didukung.

## Akar masalah: plugin GStreamer tidak ikut dibundel

AppImage 0.1.0 sampai 0.3.2 membawa `libwebkit2gtk-4.1`, `libjavascriptcoregtk`,
dan pustaka GStreamer **1.24.2** dari mesin build, tetapi **tidak** membawa
folder plugin `gstreamer-1.0`. WebKit lalu memindai plugin milik host; di Arch
plugin itu dibangun untuk GStreamer 1.26, sehingga gagal dimuat ke inti 1.24.
Gejalanya di log:

```
GStreamer element appsink not found. Please install it.
```

`appsink` adalah elemen paling dasar dari `gst-plugins-base`. Tanpa satu plugin
pun terdaftar, jawaban WebKit untuk semua format media menjadi `false`.

Bukti terukur, halaman yang sama dimuat di kontainer yang sama:

| Pemeriksaan | Biner 0.3.2 (pustaka sistem) | AppImage 0.3.2 |
| --- | --- | --- |
| MSE ada | true | true |
| MSE h264 (avc1.42E01E) | true | **false** |
| MSE h264 high | true | **false** |
| MSE vp9 | true | **false** |
| MSE av1 | true | **false** |
| MSE opus / aac | true / true | **false / false** |
| `canPlayType` h264/vp9/av1 | probably | kosong |

YouTube membaca tabel itu sebelum memuat segmen media; kalau semuanya `false`,
pemutar menampilkan *"Your browser can't play this video"* tanpa pernah menyentuh
jaringan. Itu sebabnya mengganti UA tidak mengubah apa pun di AppImage, dan itu
juga sebabnya biner/deb terasa normal — keduanya memakai WebKit dan plugin
GStreamer milik distro.

## Perbaikan

`src-tauri/tauri.conf.json`:

```json
"bundle": { "linux": { "appimage": { "bundleMediaFramework": true } } }
```

Dengan itu `linuxdeploy-plugin-gstreamer` menyalin seluruh plugin 1.24.2 (26 MB)
dan helper `gst-plugin-scanner` ke AppImage, lalu memasang hook `AppRun`:

```
GST_PLUGIN_SYSTEM_PATH_1_0=$APPDIR/usr/lib/gstreamer-1.0
GST_PLUGIN_PATH_1_0=$APPDIR/usr/lib/gstreamer-1.0
GST_PLUGIN_SCANNER_1_0=$APPDIR/usr/lib/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner
```

Inti GStreamer, plugin, dan WebKit kini berasal dari satu mesin build yang sama,
jadi tidak ada lagi percampuran versi.

AppImage membesar dari 81 MB ke 160 MB. Itu memang mahal, tetapi ini satu-satunya
cara AppImage bisa memutar video tanpa bergantung pada pustaka distro.

## `perisai --diagnostik`

Menebak dari jarak jauh tidak lagi perlu. Flag baru menulis halaman diagnostik
(dari `scripts/smoke/kodek.html`, ikut dikompilasi ke biner) ke folder temp dan
membukanya di tab:

- user agent yang benar-benar dikirim,
- `typeof MediaSource`, `MediaSource.isTypeSupported` untuk h264 (baseline dan
  high), vp9 (mp4 dan webm), av1, opus, aac,
- `HTMLVideoElement.canPlayType` untuk h264/vp9/av1,
- ketersediaan EME dan WebGL.

## Verifikasi

- AppImage 0.3.3 di Xvfb: `--diagnostik` → h264, vp9, av1, opus, aac semuanya
  `true`, `canPlayType` `probably`, tanpa `appsink not found` di log.
- YouTube (`Big Buck Bunny 4K`) di AppImage 0.3.3: halaman watch termuat penuh
  (judul, thumbnail, deskripsi), pemutar tidak lagi menulis *"can't play"*;
  yang muncul hanya *"Sign in to confirm that you're not a bot"* karena IP
  datacenter. Pemutaran penuh harus dipastikan di mesin pengguna.
- `cargo test -p perisai --lib` 26 lolos, `cargo clippy -p perisai --all-targets
  -- -D warnings` bersih, `cargo fmt --all` bersih, `npm --prefix ui run build`
  bersih.
- Minimum jendela tetap 640×400 dan panel melayang tetap menempel tepi kanan
  setelah resize.

## Batasan

- EME/Widevine tetap tidak tersedia: video ber-DRM (film YouTube) tidak bisa
  diputar. Butuh CDM dari Google, tidak boleh didistribusikan ulang.
- Kontainer uji tanpa GPU dan tanpa audio; akselerasi dan sinkronisasi audio
  tidak bisa dinilai di sini.
- Pemutaran nyata pada driver Intel HD dan Hyprland hanya bisa dipastikan di
  mesin pengguna.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.3.3_amd64.AppImage` | ±160 MB |
| `Perisai_0.3.3_amd64.deb` | ±6,3 MB |
| `perisai-0.3.3-linux-x86_64` | ±21 MB |
| `SHA256SUMS` | — |
