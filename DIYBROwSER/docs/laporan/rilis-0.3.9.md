# Rilis 0.3.9 — dua `macet:` tersisa, dan cara membacanya

Laporan pengguna setelah mencoba 0.3.8: "udah better dari sebelumnya". Log
terminalnya kini bersih saat dibuka — tidak ada lagi `macet:` di awal, yang
sebelumnya selalu muncul tepat setelah jendela tampil. Dua baris masih ada, dan
keduanya **tanpa label**:

```
adblock: siap 131412 aturan
adblock: siap 131412 aturan
… GStreamer-WARNING … (WebKitWebProcess, berulang selama memutar media)
macet: main thread tidak menjawab > 2000 ms
macet: main thread tidak menjawab > 2000 ms
```

## Apa yang bisa dan tidak bisa disimpulkan

Yang bisa: waktu kejadiannya bergeser. Di 0.3.7 macet pertama datang saat
aplikasi dibuka; di 0.3.8 tidak ada sama sekali, jadi biaya kompilasi filter
(950–1.069 ms, lihat [rilis 0.3.8](rilis-0.3.8.md)) memang hilang dari jalur
startup. Dua kejadian yang tersisa datang setelah media diputar beberapa menit.

Yang tidak bisa: label kosong hanya berarti penandanya tidak menyala, dan itu
tidak mempersempit apa pun — bisa pekerjaan main thread yang belum diberi nama,
bisa juga seluruh sistem sedang menukar halaman ke disk sehingga main thread
menunggu page fault. Menebak di antara keduanya dari log lama tidak jujur.

## Yang ditambahkan di 0.3.9

Hanya alat ukur, tanpa perubahan perilaku:

1. **Label untuk dua jalur main thread yang belum bernama.**
   - `tutup-tab` — `webview.close()` menghancurkan widget GTK lalu menghentikan
     proses WebKit; di halaman yang sedang memutar media ini yang paling mahal.
   - `pilih-tab` dan label bersarang `bangun-webview` — tab yang pernah
     ditidurkan dibangun ulang lengkap dengan proses WebKit baru saat dipilih.

   Dengan ini semua jalur yang kita tahu berat sudah tercakup: `buka-tab`,
   `bangun-webview`, `pilih-tab`, `tutup-tab`, `tidurkan-tab`, `pasang-filter`.

2. **Konteks memori dan waktu di setiap baris**, sehingga satu baris cukup untuk
   memisahkan "ada pekerjaan berat" dari "mesin sedang menukar":

   ```
   > 2000 ms (bangun-webview) [tersedia 210 MB, swap 640 MB, t=182 s]
   ```

   `tersedia` memakai `MemAvailable` (bukan `MemFree`), `swap` adalah
   `SwapTotal - SwapFree`, dan `t` adalah detik sejak aplikasi dibuka sehingga
   baris bisa dicocokkan dengan urutan kejadian di terminal.

3. **Tes baru**: format keterangan (dengan dan tanpa label, dengan dan tanpa
   `/proc`), serta pembacaan swap dari `/proc/meminfo` termasuk kasus swap 0 dan
   meminfo yang tidak lengkap.

## Verifikasi

- Formatnya diuji dengan pancingan sementara: jeda 3 detik di main thread pada
  build rilis menghasilkan

  ```
  macet: main thread tidak menjawab > 2000 ms (bangun-webview) [tersedia 4805 MB, swap 0 MB, t=3 s]
  > 2000 ms (bangun-webview) [tersedia 4805 MB, swap 0 MB, t=3 s]     ← isi macet.log
  ```

  Pancingannya sudah dihapus; `git diff` untuk `lib.rs` kosong.
- `cargo test -p perisai --lib -p storage` → **50 + 6 lolos**.
- `cargo clippy --all-targets -p perisai -- -D warnings` bersih;
  `cargo fmt --all --check` bersih.
- Uji GUI Xvfb tanpa pancingan: `macet.log` kosong (tidak ada laporan palsu),
  `adblock: siap` tetap muncul, dan uji blokir banner tetap 0 permintaan dengan
  filter versus 1 tanpa perlindungan.

## Langkah berikutnya

Kirim lagi `~/.local/share/dev.perisai.app/logs/macet.log` (atau baris `macet:`
dari terminal) setelah pemakaian sehari. Isinya sekarang langsung menentukan
arah:

- berlabel `tutup-tab` atau `bangun-webview` → pekerjaan itu yang perlu
  dipecah/ditunda berikutnya;
- tanpa label dengan `swap` ratusan MB → penyebabnya jumlah tab dan memori,
  bukan satu fungsi, dan perbaikannya di arah lain (batas tab, agresivitas
  peniduran, atau mematikan `PERISAI_TIDUR_MB` untuk melihat bedanya);
- tanpa label dengan `swap 0 MB` → main thread tertahan di dalam GTK/WebKit
  sendiri, dan jalur render (`WEBKIT_DISABLE_DMABUF_RENDERER=1`) jadi tersangka
  utama di Ivy Bridge.
