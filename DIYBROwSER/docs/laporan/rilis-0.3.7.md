# Rilis 0.3.7 — jendela membeku: main thread, bukan halaman

Laporan pengguna: "browser-nya sering freeze", seluruh jendela berhenti
menjawab klik, terjadi acak tapi paling sering saat membuka halaman, sesi
Wayland native (Hyprland, Intel HD Ivy Bridge, RAM 1,7 GB). Log pengguna
berakhir dengan `WebProcess didn't exit as expected after the UI process
connection was closed`, yaitu proses WebKit yang tidak mau berhenti ketika
aplikasi ditutup paksa.

## Yang diperbaiki

Tiga pekerjaan lambat berjalan di **main thread** GTK/WebKit. Selama salah
satunya berjalan, seluruh jendela berhenti menjawab — persis gejala yang
dilaporkan.

1. **Riwayat ditulis sinkron setiap judul berubah.** `PageLoadEvent::Finished`
   dan `on_document_title_changed` memanggil `catat_kunjungan` langsung, dan itu
   dua pernyataan SQLite per panggilan tanpa transaksi. Situs SPA (YouTube)
   mengubah judul puluhan kali per halaman; di disk lambat, tiap perubahan
   menahan main thread.

   Sekarang riwayat dan sesi dikirim ke satu thread penulis
   (`src-tauri/src/penulis.rs`) yang menggabungkan tugas mengantre (maks. 64 per
   putaran). Judul yang sama persis untuk URL yang sama disaring, jadi satu
   halaman tidak menulis berkali-kali untuk nilai yang sama.

2. **Menyimpan sesi juga menulis sinkron**, dan itu terjadi setiap tab dibuka,
   dipilih, atau ditutup. Ikut pindah ke thread penulis.

3. **Peniduran tab melepas webview tepat saat halaman baru dibuka.**
   `tidurkan_tab_lain()` dipanggil di akhir `buka()` dan `pilih()`; di dalamnya
   `webview.close()` menghancurkan widget GTK dan menghentikan proses WebKit.
   Pada mesin 1–2 GB ambang 300 MB sering tersentuh justru saat tab baru dibuka,
   jadi pembukaan halaman dan penghancuran proses terjadi bersamaan.

   Sekarang peniduran **dijadwalkan** 1,5 detik setelah permintaan terakhir
   (`jadwalkan_tidur`), sekali per gelombang, dan baru mengerjakan `close()` di
   main thread ketika halaman sudah tenang.

Selain itu, basis data dibuka dengan `journal_mode=WAL`, `synchronous=NORMAL`,
dan `busy_timeout` 5 detik. Tanpa WAL, penulis latar dan perintah UI saling
memblokir; tanpa `NORMAL`, setiap commit menunggu fsync.

## Pengukur macet

Tidak ada cara menebak dari luar apakah main thread berhenti, jadi sejak 0.3.7
aplikasi menanyakan main thread tiap detik (`src-tauri/src/macet.rs`). Kalau
jawabannya datang ≥ 2 detik kemudian, kejadian itu dicatat ke stderr dan ke
`macet.log`:

```
macet: main thread tidak menjawab selama 3204 ms
```

Lokasi berkas: `~/.local/share/dev.perisai.app/logs/macet.log` (mengikuti
`XDG_DATA_HOME`).

- Ada baris `macet:` saat jendela membeku → main thread memang tertahan, dan
  angkanya menunjukkan berapa lama. Itu jalur yang diperbaiki rilis ini; kalau
  masih terjadi, angka itu yang menunjukkan pekerjaan mana yang perlu dipindah
  berikutnya.
- `macet.log` kosong padahal jendela membeku → main thread sehat, jadi macetnya
  di compositor/WebKit (jalur render Wayland + GPU Ivy Bridge), bukan di kode
  aplikasi. Langkah berikutnya beda: `WEBKIT_DISABLE_DMABUF_RENDERER=1` atau
  `GDK_BACKEND=x11`.

## Verifikasi

- Pengukur diuji dengan sengaja menidurkan main thread 3,5 detik saat start:
  tercatat `macet: main thread tidak menjawab > 2000 ms` di stderr **dan** di
  `macet.log`. Kait uji itu dihapus sebelum build rilis.
- Sesi 5 tab dipulihkan di Xvfb dengan ambang tidur dipaksa (`PERISAI_TIDUR_MB=999999`):
  proses WebKit turun dari 6 ke 2 (chrome + tab aktif), riwayat dan sesi tetap
  tersimpan, tidak ada laporan macet palsu.
- `cargo test -p perisai --lib -p storage`: **44 + 6 lolos**, termasuk tes baru
  untuk penyaringan judul dan penyimpanan latar.
- `cargo clippy -p perisai --all-targets -- -D warnings` bersih, `cargo fmt` bersih.

## Batas pengukuran

Freeze yang dilaporkan **belum berhasil direproduksi** di mesin uji: kontainer
ini punya disk cepat dan memori longgar, jadi jalur yang diperbaiki tidak
pernah sampai menahan main thread di sini. Yang bisa dipastikan adalah
pekerjaan lambat itu memang berjalan di main thread sebelumnya (terlihat dari
kode dan dari `close()` yang mematikan proses), dan sekarang tidak lagi. Apakah
freeze di mesin pengguna hilang, ditentukan oleh `macet.log` pada pemakaian
berikutnya — bukan oleh klaim di catatan ini.

Belum diubah dan masih bisa menjadi penyebab di mesin 1,7 GB: WebKitGTK tetap
memakai satu proses per halaman, dan compositor Wayland tetap bisa berhenti
mengirim frame saat memori menipis.

## Berkas yang berubah

- `src-tauri/src/penulis.rs` (baru) — thread penulis riwayat/sesi + penyaringan.
- `src-tauri/src/macet.rs` (baru) — pengukur main thread + catatan berkas.
- `src-tauri/src/tabs.rs` — `jadwalkan_tidur`, riwayat/sesi lewat penulis.
- `src-tauri/src/lib.rs` — memasang pengukur macet.
- `crates/storage/src/lib.rs` — WAL, `synchronous=NORMAL`, `busy_timeout`.
- `README.md` — bagian "Kalau jendela membeku".
