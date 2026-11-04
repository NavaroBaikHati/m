# Rilis 0.3.1

## Bug resize dan tiling compositor (Hyprland/sway/i3)

**Gejala:** jendela tidak bisa dikecilkan dengan wajar; setelah tiling
compositor memaksa ukuran baru, isi tab terpotong dan chrome tampak tidak pas.

**Penyebab:** setiap webview diletakkan di `GtkFixed` dengan
`set_size_request` seukuran jendela. `GtkFixed` melaporkan ukuran anak terbesar
sebagai ukuran minimum, dan GTK meneruskannya ke `WM_NORMAL_HINTS`. Jadi ukuran
minimum jendela **selalu sama dengan ukuran jendela saat itu** — jendela hanya
bisa membesar, tidak pernah mengecil; compositor tiling yang memaksa ukuran
lebih kecil membuat isi terpotong.

**Perbaikan** (`src-tauri/src/platform/linux/mod.rs`):

- `size_request` anak menjadi 1×1; ukuran sebenarnya dipasang lewat
  `size_allocate`, cara yang juga dipakai wry untuk wadah `GtkFixed`.
- Ukuran dipasang ulang dari handler `size-allocate` mode *after*
  (`g_signal_connect_data` + `G_CONNECT_AFTER`) setiap kali GTK selesai menata
  wadah, supaya tata letak tidak pernah tertinggal satu langkah.
- `GtkFixed` kini dimasukkan ke vbox window dengan `pack_start(expand)`, jadi
  ia tetap mengisi jendela meski ukuran minimumnya kecil.

**Bukti:** `WM_NORMAL_HINTS` dibaca langsung dengan `scripts/uji/hint-minimum.py`:

| Build | Ukuran minimum |
| --- | --- |
| 0.3.0 | 1000×700 — sama dengan ukuran jendela saat diukur |
| 0.3.1 | 640×400 — batas `minWidth`/`minHeight` di `tauri.conf.json` |

Sesudah 10 resize beruntun (600×420 … 1150×720) tanpa jeda, tangkapan layar
menunjukkan toolbar, sidebar, dan isi tab tetap menempel ke tepi jendela yang
benar (`scripts/uji/resize.sh`).

## Setelan WebKit tidak lagi ditulis ulang tiap perubahan batas

`terapkan_setelan` dulu dipanggil di dalam `pindahkan`, jadi kebijakan
akselerasi perangkat keras, WebGL, dan warna latar ditulis ulang pada **setiap**
resize, pindah tab, buka panel, dan keluar layar penuh. Menulis ulang kebijakan
akselerasi pada webview yang sedang berjalan memaksa WebKit menyusun ulang
compositing — itu sumber tersendat saat resize dan pindah tab. Sekarang setelan
dipasang sekali per webview (chrome di `siapkan_wadah`, tab di `pantau`).

## Verifikasi

- `cargo test -p perisai --lib` — 25 lolos.
- `cargo clippy -p perisai --all-targets -- -D warnings` — bersih.
- `cargo fmt --all --check` — lolos.
- `cargo tauri build --bundles deb,appimage` — AppImage 80,96 MiB, `.deb` 6,31 MiB.
- Biner rilis dijalankan di Xvfb: minimum 640×400, tangkapan layar sebelum dan
  sesudah resize cepat benar, tanpa panic/CRITICAL di log.
- `npm --prefix ui run build` — lolos (66,69 kB JS).

## Temuan untuk keluhan video YouTube dan render lambat

AppImage membawa `libwebkit2gtk-4.1` (±95 MB) dan **GStreamer 1.24.2** dari
mesin build, lalu `AppRun` menaruh folder itu di depan `LD_LIBRARY_PATH`. Di
Arch (GStreamer 1.26) inti 1.24 memuat plugin host yang dibangun untuk 1.26;
decoder bisa gagal dimuat sehingga muncul *"Your browser can't play this
video"*, dan render memakai WebKit versi build, bukan WebKitGTK distro. Isi
AppImage 0.2.3 dan 0.3.0 identik (405 berkas, hash `libwebkit2gtk` sama), jadi
ini bukan regresi kode 0.3.x.

Biner telanjang memakai pustaka distro. Rilis ini menyertakannya supaya bisa
dipakai sebagai pembanding; pembenahan pembungkusan AppImage belum dikerjakan.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.3.1_amd64.AppImage` | 81 MB |
| `Perisai_0.3.1_amd64.deb` | 6,3 MB |
| `perisai-0.3.1-linux-x86_64` | 21 MB |
| `SHA256SUMS` | — |

`.rpm` tidak dibangun pada rilis ini.
