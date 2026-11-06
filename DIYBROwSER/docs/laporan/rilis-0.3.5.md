# Rilis 0.3.5 — mesin 1,7 GB: tab lama ditidurkan

0.3.4 menunjukkan bahwa biner dengan WebKit distro jauh lebih cepat daripada
AppImage. Laporan berikutnya datang dari laptop yang lebih kecil dari mesin uji:
Celeron 1000M dua inti, RAM 1,7 GB, Intel HD Ivy Bridge, Hyprland, 1366×768.
Di mesin sekecil itu, "lancar" bukan hanya soal kecepatan meraster — tapi juga
soal berapa banyak proses yang harus hidup bersamaan.

## Yang diukur

### Memori per tab

WebKitGTK memakai satu proses untuk setiap halaman. Di mesin uji (Xvfb
1366×768, tanpa GPU sehingga raster dilakukan CPU), RSS seluruh pohon proses
dengan tiga tab terbuka:

| Keadaan | tanpa peniduran | dengan peniduran |
| --- | --- | --- |
| 3 tab terbuka | 1487 MB / 1530 MB | 1019 MB |
| setelah berpindah tab | 1413 MB / 1393 MB | 842 MB |

Selisihnya 470–570 MB untuk tiga tab. Pada mesin 1,7 GB, angka sebesar ini
menentukan apakah kernel mulai menukar ke disk — dan begitu swap dipakai, semua
halaman terasa lambat, bukan hanya tab terakhir.

### Setelan akselerasi

Uji gulir teks satu layar penuh, 3–4 kali per setelan, mesin uji yang tidak
punya GPU:

| Setelan | fps |
| --- | --- |
| bawaan (`OnDemand`) | 26–46 |
| `PERISAI_AKSELERASI=perangkat-lunak` | 44–57 |
| `PERISAI_AKSELERASI=selalu` | 42–47 |

Tanpa GPU, `OnDemand` tetap mencoba jalur GL (llvmpipe) lalu menambah
compositing, sedangkan `perangkat-lunak` melukis langsung. Di mesin yang GPU-nya
benar-benar terpakai urutannya bisa terbalik, jadi angka ini dipakai sebagai
alasan menyediakan pembanding, bukan sebagai setelan baru yang dipaksa.

### Yang dicoba lalu dibuang

Mematikan gulir halus di mesin dua inti terdengar masuk akal, tetapi tidak
terbukti: dengan event roda tetikus asli (XTEST), satu putaran roda menghabiskan
5–8 frame baik dengan `set_enable_smooth_scrolling(true)` maupun `false`.
Perubahan itu dibuang daripada diklaim sebagai perbaikan.

## Yang baru

- **Tab tidur otomatis.** Saat `MemAvailable` di bawah 300 MB, webview tab yang
  tidak aktif dilepas (proses WebKit-nya berhenti) dan URL-nya disimpan. Tab
  tetap ada di sidebar; memilihnya memuat ulang halaman. Urutan peniduran
  mengikuti kapan tab terakhir dipakai, jadi tab yang barusan ditinggal masih
  hangat.
- **Tab yang memutar suara tidak ditidurkan.** Status diambil dari sinyal WebKit
  `is-playing-audio`, bukan dari judul atau domain.
- **`PERISAI_TIDUR_MB`** untuk mengatur ambang (bawaan 300, `0` mematikan).
- **`--diagnostik` menampilkan baris `memori`**: MB tersedia dan ambang tidur.
- `MemFree` sengaja tidak dipakai; hanya `MemAvailable`, karena `MemFree` tidak
  menghitung cache yang bisa dibuang kernel dan akan melebih-lebihkan sisa
  memori.

## Batas pengukuran

- Angka fps berasal dari kontainer tanpa GPU. Pada laptop Celeron + Intel HD,
  jalur GPU-nya berbeda, jadi yang bisa dipercaya dari tabel akselerasi adalah
  arahnya (setelan bisa lebih lambat daripada tidak dipakai), bukan angka
  absolutnya.
- Peniduran tab diuji dengan ambang dipaksa tinggi supaya selalu aktif. Ambang
  300 MB yang sebenarnya belum bisa diuji di mesin uji 7,9 GB tanpa membatasi
  memori proses.
- Pemutaran YouTube nyata di mesin tersebut tetap tidak bisa diuji di kontainer
  (tanpa audio/GPU, dan YouTube menolak sesi otomatis).
