# Rilis 0.3.8 — freeze yang terukur: kompilasi filter di main thread

Laporan pengguna setelah 0.3.7, dari terminal (Wayland native, Manjaro, kernel
6.18, Intel HD Ivy Bridge, RAM 1,69 GB):

```
(perisai:1527): Gdk-CRITICAL **: … gdk_wayland_window_set_dbus_properties_libgtk_only …
macet: main thread tidak menjawab > 2000 ms      ← tepat setelah jendela dibuka
adblock: siap 131412 aturan
MESA-INTEL: warning: Ivy Bridge Vulkan support is incomplete
macet: main thread tidak menjawab > 2000 ms      ← beberapa menit kemudian
```

Dua baris `macet:` itu menjawab pertanyaan yang tersisa dari 0.3.7: freeze-nya
**bukan** compositor atau proses WebKit, melainkan main thread aplikasi sendiri.
Karena barisnya memakai ambang `> 2000 ms` (bukan angka pasti), main thread
tertahan lebih dari dua detik dan belum sempat menjawab — persis "seluruh
jendela mati".

## Yang diukur, bukan ditebak

Di mesin uji, closure pemasangan filter di main thread diukur dengan pengukur
sementara pada build rilis:

| Tahap | Sebelum | Sesudah |
| --- | --- | --- |
| Closure di main thread | **950–1.069 ms** | **0 ms** |
| `webkit_user_content_filter_store_save_from_file` | 921–1.067 ms (di main thread) | 1.441 ms (di thread pekerja) |
| Folder cache filter | 2,5 GB | 148 MB |

Angka itu menjelaskan kenapa di laptop 1,7 GB gejalanya melewati 2 detik: mesin
uji pun sudah 1 detik, dan mesin pengguna punya disk lebih lambat, CPU dua inti,
serta page cache yang terus ditekan.

Penyebabnya bukan kode kita sendiri. `save_from_file` memang **bernama**
asinkron, tetapi sumber WebKit menunjukkan ia memetakan berkas 14 MB lalu
memanggil `compileContentRuleList` — dan kompilasi 131 ribu aturan itu berjalan
di utas pemanggil sebelum callback-nya dijadwalkan. Waktu kami pindahkan I/O
berkas saja (menulis `filter.json`, membaca salinan JSON), angkanya tetap
1.069 ms; yang mahal ternyata kompilasinya.

## Yang diperbaiki

1. **Kompilasi content blocker pindah ke thread pekerja**
   (`platform/linux/filter.rs`). Pemanggilan WebKit dikirim ke thread
   `perisai-filter`; callback-nya tetap kembali ke main context global, jadi
   `webkit_user_content_manager_add_filter` masih berjalan di thread utama GTK
   seperti yang diwajibkan. Dua kompilasi diserialkan dengan satu mutex supaya
   tidak menulis store bersamaan.

2. **I/O filter dipisah dari pemasangan**: `siapkan()` (tulis/baca 14 MB,
   buat folder) berjalan di thread pekerja; `pasang()` hanya menyentuh objek
   WebKit.

3. **Cache filter tidak lagi menumpuk.** Setiap isi filter baru meninggalkan
   salinan JSON 14 MB, dan WebKit meninggalkan hasil kompilasi ~30 MB dengan
   nama acak yang tidak pernah ia hapus sendiri. Terukur 2,5 GB di profil uji
   (1,9 GB di antaranya milik WebKit) hanya dari beberapa kali jalan. Sekarang
   salinan JSON lama dibuang, dan `webkit-store` disusutkan ke dua berkas
   terbaru — cukup untuk filter aktif dan satu sebelumnya.

4. **Peniduran tab satu per putaran** (`tabs.rs`). Dulu semua tab yang perlu
   tidur dilepas dalam satu closure; `webview.close()` menghancurkan widget dan
   menghentikan proses WebKit di main thread. Sekarang satu tab per putaran,
   lalu putaran berikutnya dijadwalkan 1,5 detik kemudian selama memori masih
   sempit; kalau tidak ada tab yang bisa ditidurkan, penjadwalan berhenti.

5. **`macet.log` menyebut pekerjaannya.** Penanda RAII (`macet::kerja`) dipasang
   di `buka-tab`, `pasang-filter`, dan `tidurkan-tab`, sehingga baris berikutnya
   berbunyi `> 2000 ms (pasang-filter)`.

## Verifikasi

- **Pemblokiran masih nyata**, bukan hanya "siap": halaman uji meminta
  `banner.png` 10 detik setelah dibuka (filter sudah terpasang), server lokal
  mencatat setiap permintaan.

  | Putaran | `halaman.html` | `banner.png` |
  | --- | --- | --- |
  | dengan filter | 1 permintaan | **0 permintaan** (diblokir WebKit) |
  | `PERISAI_TANPA_ADBLOCK=1` | 1 permintaan | 1 permintaan |

- **Main thread tidak lagi menunggu**: closure pemasangan tercatat 0 ms,
  sedangkan kompilasi 1.441 ms selesai di thread pekerja dan `adblock: siap
  131091 aturan` tetap tercetak.
- `cargo test -p perisai --lib -p storage` → **48 + 6 lolos** (tes baru:
  penanda kerja dipulihkan, salinan filter lama dibuang, store disusutkan ke dua
  terbaru, store dengan dua berkas tidak disentuh).
- `cargo clippy --all-targets … -- -D warnings` bersih; `cargo fmt --all
  --check` bersih.
- `macet.log` kosong pada seluruh uji GUI Xvfb (tidak ada laporan palsu).

## Batas pengukuran

- Uji ini berjalan di kontainer dengan disk cepat dan RAM longgar. Angka 950 ms
  adalah **batas bawah** untuk mesin pengguna; kami tidak bisa mengukur langsung
  di laptop Ivy Bridge-nya.
- Pemindahan kompilasi ke thread pekerja bergantung pada perilaku WebKit yang
  terdokumentasi sebagai asinkron tetapi tidak menyatakan syarat thread. Yang
  bisa dibuktikan: filter terpasang, halaman terlindungi, callback sampai di
  main thread, dan tidak ada crash selama uji. Kalau versi WebKit lain
  menolaknya, gejalanya akan terlihat sebagai `adblock: filter WebKit ditolak`
  di stderr — bukan macet.
- Freeze yang tersisa (kalau ada) seharusnya kini menyebut pekerjaannya di
  `macet.log`; tanpa baris itu, arahnya jalur render/compositor seperti yang
  sudah dicatat di laporan 0.3.7.

## Alternatif WebKit

Tidak ada mesin lain yang realistis untuk mesin 1,7 GB ini: WPE WebKit dan
WebKitGTK 6.0 memakai mesin yang sama (masalahnya di API store, bukan di mesin),
Servo belum siap untuk pemakaian harian, dan Chromium/CEF menambah ratusan MB
RAM sebelum satu tab pun dibuka. Perbaikannya karena itu dilakukan di jalur yang
salah ini, dan sekarang terukur.
