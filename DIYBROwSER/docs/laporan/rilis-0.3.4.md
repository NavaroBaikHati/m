# Rilis 0.3.4 — mengukur "lemot" sebelum memperbaikinya

YouTube sudah normal sejak 0.3.3. Keluhan berikutnya: halaman terasa lambat saat
dimuat dan dirender. Laporan seperti ini paling gampang salah tebak, jadi rilis
ini mengukur dulu — dan mengukur dari mesin yang mengalami masalahnya, bukan
dari kontainer pengembangan.

## Yang diukur di kontainer (dan hasilnya)

Supaya tidak menebak, empat dugaan diuji langsung dengan biner 0.3.3 di Xvfb.
Semua angka di bawah adalah hasil pengukuran, bukan taksiran.

| Dugaan | Cara uji | Hasil |
| --- | --- | --- |
| Content blocker (130 ribu aturan) memperlambat muat halaman | 250 request CSS dari server lokal, perlindungan hidup vs `PERISAI_TANPA_ADBLOCK=1` | 9 ms vs 10 ms — tidak ada biaya terukur |
| AppImage lambat karena ekstraksi 160 MB | waktu sampai jendela muncul | 427 ms vs 199 ms (biner) — selisih kecil |
| WebKit bundel AppImage lebih lambat dari WebKit distro | gulir teks satu layar penuh, 5 detik, mesin yang sama | biner **16–17 fps**, AppImage **0 fps** dalam 4 percobaan |
| Renderer DMA-BUF / compositing penyebabnya | AppImage dengan `WEBKIT_DISABLE_DMABUF_RENDERER=1` dan `WEBKIT_DISABLE_COMPOSITING_MODE=1` | tetap 0 fps — bukan setelan renderer |

Satu pengukuran awal sempat tampak dramatis (1 fps vs 10 fps saat renderer
DMA-BUF dimatikan), tetapi setelah diperiksa ternyata halaman ujinya membuat
lapisan 5200 px yang harus diraster utuh tiap frame — beban yang tidak dimiliki
halaman biasa. Pengukuran itu dibuang dan diganti uji gulir teks layar penuh
yang hasilnya di atas.

Artinya: muat halaman dan content blocker tidak bersalah, dan mematikan
DMA-BUF tidak menolong. Yang membedakan hanya WebKit mana yang dipakai —
bundel AppImage (build Ubuntu) jauh lebih lambat meraster halaman daripada
WebKit milik distro pada mesin yang sama.

## Yang baru di 0.3.4

`perisai --diagnostik` sekarang menjawab pertanyaan yang selama ini hanya bisa
ditebak:

- **fps saat menggulir** — teks satu layar penuh digulir 5 detik, dihitung frame
  yang benar-benar sampai ke layar. Hijau kalau ≥ 45 fps.
- **perangkat GPU** — isi `/dev/dri`. Kalau kosong, semua cat dilakukan CPU dan
  halaman akan berat berapa pun setelannya.
- **WebKit yang dipakai** — pustaka sistem atau bundel AppImage, beserta path-nya.
- **setelan render** — kebijakan akselerasi, status renderer DMA-BUF,
  `GDK_BACKEND`, tipe sesi (Wayland/X11), dan apakah berjalan dari AppImage.
- CPU, jumlah inti, devicePixelRatio, ukuran layar, user agent.
- Tabel dukungan codec (seperti 0.3.2).

Catatan: nama GPU di WebGL selalu "Apple GPU" karena WebKit menyamarkannya;
karena itu status `/dev/dri` yang ditampilkan, bukan string WebGL.

## Cara memakainya untuk mencari penyebab

Jalankan tiga varian dan bandingkan angka "fps saat menggulir":

```bash
Perisai_0.3.4_amd64.AppImage --diagnostik              # WebKit bundel
./perisai-0.3.4-linux-x86_64 --diagnostik              # WebKit distro
WEBKIT_DISABLE_DMABUF_RENDERER=1 perisai --diagnostik  # renderer SHM
```

Patokan: menggulir teks ≥ 45 fps berarti jalur render sehat. Kalau AppImage jauh
di bawah biner, penyebabnya WebKit bundel — pakai biner atau `.deb`. Kalau kedua
biner sama-sama rendah dan `perangkat GPU` kosong, tidak ada akselerasi sama
sekali. Kalau semuanya ≥ 45 fps, kemacetan bukan di render.

Di mesin uji, biner 0.3.4 mencapai 16–17 fps dan AppImage 0 fps untuk beban yang
sama; itu pengukuran yang membuat rekomendasi ini ditulis, bukan dugaan.

## Verifikasi rilis ini

- `--diagnostik` di biner 0.3.4 (Xvfb, tanpa GPU): uji gulir melaporkan
  `15 fps (5037 ms)`, `/dev/dri/{card0}` terdeteksi, WebKit sistem terdeteksi,
  tabel codec tetap `true` untuk h264/vp9/av1/opus/aac.
- `cargo test -p perisai --lib` 26 lolos, clippy `-D warnings` bersih, fmt bersih.
- Tidak ada perubahan perilaku render: rilis ini menambah pengukuran, bukan
  menebak setelan baru.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.3.4_amd64.AppImage` | ±160 MB |
| `Perisai_0.3.4_amd64.deb` | ±6,3 MB |
| `perisai-0.3.4-linux-x86_64` | ±21 MB |
| `SHA256SUMS` | — |
