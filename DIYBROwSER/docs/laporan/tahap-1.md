# Laporan Tahap 1 — shell, tab tunggal, address bar, navigasi

Tanggal: 27 Oktober 2026
Mesin pengembangan: Ubuntu 24.04 headless (VPS), Rust 1.99.1, Node 24, WebKitGTK 2.52.6, Xvfb

## 1. Apa yang selesai

- **Workspace Cargo** dengan tiga anggota: `crates/adblock-core`, `crates/storage`,
  `src-tauri` (crate `diy-browser`), plus frontend `ui/` (Svelte 5 + TypeScript + Vite).
- **Shell Tauri v2** (`tauri 2.12.5`, `wry 0.57.1`, fitur `unstable`): window `main` memuat
  UI chrome dari Svelte, dan tiap tab adalah **child webview** terpisah (bukan iframe, karena
  banyak situs melarang iframe).
- **Tab manager** (`src-tauri/src/tabs.rs`): membuat tab, menyimpan keadaan tab, menghitung
  dan memperbarui batas tiap webview saat window berubah ukuran.
- **Address bar** (`src-tauri/src/url_input.rs`): URL lengkap, domain tanpa skema
  (`example.com` → `https://example.com/`), host lokal pakai HTTP (`localhost:1420`),
  sisanya jadi pencarian DuckDuckGo. Skema berbahaya seperti `javascript:` tidak pernah
  dibuka langsung.
- **Tombol mundur, maju, muat ulang** di chrome UI.
- **Sinkronisasi URL dan judul halaman** dari webview tab ke chrome UI lewat event
  `tab:navigasi`; event ditargetkan hanya ke webview UI.
- **Izin IPC dibatasi**: capability `default` hanya berlaku untuk webview label `main`,
  jadi situs yang dimuat di tab tidak punya akses ke perintah Tauri.
- **Fondasi tahap berikutnya**: `crates/storage` (tabel `pengaturan` di SQLite, untuk tahap 4)
  dan `crates/adblock-core` (katalog EasyList + EasyPrivacy, untuk tahap 3).
- **Uji GUI tanpa layar**: `scripts/smoke-gui.sh` (Xvfb + screenshot otomatis) dan skenario
  interaksi `scripts/uji/navigasi.sh` (xdotool).
- **Dockerfile `archlinux`** bertingkat + daftar dependensi sistem Ubuntu dan Arch di README.

## 2. Apa yang dites dan hasilnya

Semua perintah dijalankan dari akar repo.

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all --check` | Lolos, tidak ada diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lolos, 0 warning |
| `cargo test --workspace` | 13 lolos, 0 gagal (`adblock-core` 3, `storage` 3, `url_input` 7) |
| `bash scripts/smoke-gui.sh` | Aplikasi jalan di Xvfb, tidak ada panic, screenshot diambil |
| `bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap1.png scripts/uji/navigasi.sh` | 5 langkah interaksi berhasil |

Bukti screenshot (di `docs/screenshots/`):

| File | Isi yang dibuktikan |
| --- | --- |
| `tahap1.png` | Aplikasi terbuka: chrome di atas, isi tab mengisi area di bawahnya |
| `uji-1-klik-tautan.png` | Klik tautan di dalam halaman → pindah ke `halaman-kedua.html` |
| `uji-2-address-bar.png` | Ketik `example.com` di address bar → `https://example.com/` (HTTPS) |
| `uji-3-mundur.png` | Tombol mundur kembali ke halaman sebelumnya |
| `uji-4-maju.png` | Tombol maju kembali ke `https://example.com/` |
| `uji-5-muat-ulang.png` | Tombol muat ulang memuat ulang halaman |

Empat penanda kuning di halaman uji menempel tepat di setiap sudut area isi tab, jadi
penempatan webview tab memang pas di bawah chrome (bukan 0x0 maupun bertumpuk).

## 3. Temuan penting: multi-webview di Linux belum beres di Tauri

**Masalah.** Sampai `tauri 2.12.5` / `tauri-runtime-wry 2.12.3`, child webview di Linux
ditambahkan ke `GtkBox` bawaan window (`webview_builder.build_gtk(window.default_vbox())`).
`GtkBox` menyusun anaknya **vertikal**, jadi dua webview berbagi tinggi window (masing-masing
setengah) dan `Webview::set_bounds` tidak memindahkan apa pun — persis bug
[tauri-apps/tauri#10420](https://github.com/tauri-apps/tauri/issues/10420) yang masih terbuka.

**Bukti.** Pada percobaan pertama, `webview.bounds()` selalu melaporkan `0x0` / setengah
tinggi window, dan screenshot menampilkan chrome serta isi tab bertumpuk, bukan bersebelahan.

**Yang sudah dicoba.** Versi terbaru crates.io (`tauri` 2.12.5, `wry` 0.57.1, `tao` 0.37.2,
`tauri-runtime-wry` 2.12.3) semuanya belum memuat perbaikan; branch `dev` tao juga masih hanya
punya `default_vbox` (GtkBox).

**Solusi yang dipakai.** wry sendiri mendukung penempatan absolut asalkan wadah webview berupa
`gtk::Fixed`. Jadi `src-tauri/src/platform/linux.rs` membuat `GtkFixed` sendiri, menyisipkannya
ke `GtkBox` window, lalu memindahkan webview chrome dan setiap webview tab ke dalamnya.
Windows dan macOS tidak punya masalah ini; `src-tauri/src/platform/lainnya.rs` langsung memakai
`Webview::set_bounds`. Ukuran dan posisi dihitung di `tabs.rs` supaya hanya satu tempat yang
tahu tata letaknya.

**Risiko yang harus kamu tahu.** Modul Linux itu memakai `with_webview`, yang menurut dokumentasi
Tauri bergantung pada tipe `webkit2gtk`/`gtk` dan bisa berubah di rilis minor. Karena itu
`tauri` dipatok ke `2.12` dan `gtk` ke `0.18` di `src-tauri/Cargo.toml`. Kalau Tauri menaikkan
`gtk`/webkit2gtk, modul itu perlu disesuaikan. Modul ini bisa dihapus begitu tao/wry memakai
`GtkFixed` secara bawaan — dan kalau batas multi-webview ini nanti makin sering menghambat
(mis. butuh tab yang benar-benar independen, zoom per tab, atau devtools per tab sesuai P1),
opsi CEF di PRD jadi masuk akal. **Ini butuh keputusanmu** (lihat bagian 6).

## 4. Asumsi karena PRD ambigu

| Hal | Asumsi yang dipakai |
| --- | --- |
| Mesin pencari address bar | DuckDuckGo (`https://duckduckgo.com/`), karena tidak menyimpan profil pencarian; mudah diganti nanti |
| Alamat awal aplikasi | `https://example.com/` sebagai placeholder; halaman tab baru (pencarian + favorit) menyusul di tahap berikutnya |
| Pemulihan sesi | Belum ada — aplikasi selalu mulai dengan satu tab baru |
| Nama browser | Belum ditentukan (pertanyaan terbuka PRD), sementara dipakai "DIY Browser" |
| Tema | Mengikuti tema sistem lewat `prefers-color-scheme`; kontrol tema manual masuk tahap 4 |
| Tampilan sidebar | Belum ada; `rect_konten()` sudah dipisah supaya tahap 2 tinggal mengurangi `x` dan `lebar` |

## 5. Belum terverifikasi

- **Dockerfile Arch** dan **`cargo tauri dev`** (perintah persis yang kamu pakai): lihat bagian 7.
- **Windows dan macOS**: jalur platform non-Linux sudah dikompilasi (dengan `cfg` dibalik
  sementara supaya ikut di-`check`), tetapi belum pernah dijalankan.
- **Wayland dan layar HiDPI**: hanya X11 dengan skala 1.0 yang diuji.
- **Pemulihan setelah crash**, profil pengguna terpisah, dan pengaturan persistensi belum ada.

## 6. Keputusan yang perlu persetujuanmu

1. **Workaround `GtkFixed` di Linux** — lanjut pakai (rekomendasiku), atau tunggu perbaikan
   upstream, atau pindah ke CEF sekarang?
2. Lanjut ke **Tahap 2 (banyak tab + sidebar vertikal)** setelah kamu setuju soal nomor 1.

## 7. Perintah persis untukmu di Arch

```bash
# sekali saja
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libsoup3 gtk3 \
  librsvg openssl libayatana-appindicator xdotool xorg-server-xvfb imagemagick python
cargo install tauri-cli --locked

# tiap tahap
git pull
npm --prefix ui install
cargo tauri dev
```

Uji cepat tanpa membuka jendela:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/smoke-gui.sh
```
