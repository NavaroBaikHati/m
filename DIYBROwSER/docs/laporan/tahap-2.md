# Laporan Tahap 2 — banyak tab + sidebar vertikal

Tanggal: 28 Oktober 2026
Mesin pengembangan: Ubuntu 24.04 headless (VPS), Rust 1.99.1, Node 24, WebKitGTK 2.52.6, Xvfb

## 1. Apa yang selesai

- **Tab manager multi-tab** (`src-tauri/src/tabs.rs`): `buka`, `pilih`, `tutup`,
  dan `atur_sidebar`. Setiap tab tetap satu child webview; hanya tab aktif yang
  diposisikan dan ditampilkan, tab lain disembunyikan (`Webview::hide`) supaya
  tidak menutupi chrome dan tidak ikut dirender.
- **Menutup tab** memilih tab di kanan (atau kiri kalau tab terakhir), dan
  menutup tab terakhir langsung membuka tab baru berisi alamat awal sehingga
  jendela tidak pernah kosong. Pemilihan tab pengganti punya 3 unit test.
- **Sidebar vertikal** (`ui/src/App.svelte`): daftar tab dengan judul, penanda
  tab aktif, indikator memuat, tombol tutup per tab, tombol tab baru, dan tombol
  ciutkan. Saat diciutkan sidebar hilang seluruhnya dan tombol buka pindah ke
  kiri toolbar.
- **Tata letak sadar sidebar**: `rect_konten()` memakai `x = LEBAR_SIDEBAR` dan
  mengurangi lebar saat sidebar terbuka. `LEBAR_SIDEBAR` (216.0) harus sama
  dengan `--lebar-sidebar` di `ui/src/app.css`, sama seperti `TINGGI_CHROME`.
- **IPC baru**: `daftar_tab`, `tab_baru`, `pilih_tab`, `tutup_tab`,
  `atur_sidebar`; `tab_aktif` (tahap 1, hanya satu tab) diganti `daftar_tab`.
  URL di `daftar_tab` dibaca ulang dari tiap webview supaya tetap benar setelah
  redirect, dan judul disimpan di state Rust saat event judul datang.
- **Skenario uji GUI** `scripts/uji/tab.sh`: tab baru, pindah tab, tutup tab,
  ciutkan/buka sidebar, tutup tab terakhir. Selain screenshot, skenario ini
  memeriksa **warna piksel** untuk membuktikan sidebar benar-benar memakan ruang
  dan hilang saat diciutkan.

## 2. Apa yang dites dan hasilnya

Semua perintah dijalankan dari akar repo; build di mesin ini memakai
`CARGO_TARGET_DIR=/tmp/diy-target` karena disk `/workspaces` penuh (lihat §5).

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all --check` | Lolos, tidak ada diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lolos, 0 warning |
| `cargo test --workspace` | 17 lolos, 0 gagal (`adblock-core` 3, `diy-browser` 11, `storage` 3) |
| `npm --prefix ui run build` | Lolos, 115 modul, bundle 44,21 kB |
| `bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap2.png scripts/uji/tab.sh` | Lolos: 7 screenshot, 3 pemeriksaan piksel, tanpa panic |

Bukti screenshot (di `docs/screenshots/`):

| File | Isi yang dibuktikan |
| --- | --- |
| `tahap2-1-awal.png` | Satu tab, sidebar terbuka, isi tab mulai di kanan sidebar + bawah toolbar |
| `tahap2-2-tab-baru.png` | Klik `+` membuka tab kedua (`example.com`) dan menjadikannya aktif |
| `tahap2-3-pindah-tab.png` | Klik baris tab pertama mengembalikan halaman uji dan address bar-nya |
| `tahap2-4-tutup-tab.png` | Tombol silang menutup tab kedua, tersisa satu tab |
| `tahap2-5-ciut.png` | Sidebar ciut: toolbar melebar ke kiri, isi tab mengisi seluruh lebar |
| `tahap2-6-buka.png` | Tombol buka sidebar mengembalikan sidebar dan tata letak awal |
| `tahap2-7-tutup-terakhir.png` | Menutup tab terakhir membuka tab baru, bukan jendela kosong |

Pemeriksaan piksel pada screenshot (koordinat lokal jendela):

| Pemeriksaan | Hasil |
| --- | --- |
| `(100,300)` vs `(230,300)` saat sidebar terbuka | `srgb(236,238,242)` vs `srgb(15,40,79)` — sidebar memakan ruang, konten mulai setelahnya |
| `(100,300)` setelah sidebar diciutkan | `srgb(15,41,80)` — kolom kiri sudah menjadi konten |
| `(230,300)` setelah tab terakhir ditutup | `srgb(238,238,238)` — konten berganti ke tab baru |

## 3. Asumsi karena PRD ambigu

| Hal | Asumsi yang dipakai |
| --- | --- |
| Tab baru | Membuka alamat awal (`https://example.com/`). Halaman tab baru khusus belum ada |
| Sidebar diciutkan | Hilang sepenuhnya (bukan rail sempit); tombol buka pindah ke toolbar |
| Pintasan keyboard | Belum ada (Ctrl+T/Ctrl+W); PRD menaruh command palette di P1 |
| Urutan tab | Tetap sesuai urutan dibuat; drag-reorder belum ada |
| Persistensi | Daftar tab dan keadaan sidebar belum disimpan antar sesi |
| Judul tab kosong | Ditampilkan sebagai host URL, bukan teks "Tab baru" |

## 4. Belum terverifikasi

- **`cargo tauri dev` di Arch dengan layar nyata** (webkit2gtk-4.1): belum
  terverifikasi, perlu dites manual di Arch.
- **Windows dan macOS**: jalur `platform/lainnya.rs` belum pernah dijalankan.
- **Wayland, HiDPI (skala ≠ 1.0), dan window manager nyata**: uji GUI memakai
  X11 tanpa WM dengan skala 1.0.
- **Tab drag-reorder, pintasan keyboard, pemulihan sesi**: belum ada fiturnya.

## 5. Catatan lingkungan (bukan bug aplikasi)

Disk `/workspaces` sempat 100% penuh sehingga linker gagal (`ld terminated with
signal 7`). `cargo clean` membebaskan 3,9 GB, dan build uji diarahkan ke
`CARGO_TARGET_DIR=/tmp/diy-target`. Tidak ada perubahan kode yang dibuat untuk ini.

## 6. Langkah berikutnya

Tahap 3 sesuai PRD: adblock request blocking dengan crate `adblock`
(EasyList + EasyPrivacy), penghitung yang diblokir, dan shield per situs dengan
toggle on/off.
