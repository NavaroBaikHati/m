# DIY Browser (nama kerja)

Browser desktop lintas OS: UI ringkas ala Arc + adblock bawaan ala Brave, dibangun
dengan Rust (Tauri v2) dan Svelte. Tanpa telemetri; semua data tinggal di lokal.

PRD lengkap ada di [PRD.md](PRD.md).

## Status

| Tahap | Isi                                                              | Status    |
| ----- | ---------------------------------------------------------------- | --------- |
| 1     | Workspace, shell Tauri, satu tab, address bar, back/forward/reload | Selesai   |
| 2     | Banyak tab + sidebar vertikal                                     | Selesai   |
| 3     | Adblock request blocking, EasyList + EasyPrivacy, shield per situs | Belum     |
| 4     | Bookmark, history, download manager, mode gelap/terang            | Belum     |
| 5     | Cosmetic filter, scriptlet, pembaruan daftar filter               | Belum     |

Catatan verifikasi tiap tahap ada di `docs/laporan/`.

## Struktur repo

```
crates/adblock-core/   Logika adblock murni, tanpa dependensi Tauri/UI
crates/storage/        SQLite: pengaturan (bookmark & history menyusul)
src-tauri/             Aplikasi Tauri: tab manager, perintah IPC, penyesuaian platform
ui/                    Frontend Svelte + TypeScript (chrome browser)
scripts/               Uji GUI tanpa layar (Xvfb) + skenario navigasi
docs/screenshots/      Bukti screenshot hasil uji
Dockerfile             Build/uji di lingkungan Arch Linux
```

## Dependensi sistem

Rust stable (rustup) dan Node.js 20+ diperlukan di semua platform.

**Ubuntu/Debian** (dipakai saat pengembangan):

```bash
sudo apt install build-essential curl wget file libwebkit2gtk-4.1-dev libgtk-3-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
# tambahan untuk uji GUI tanpa layar:
sudo apt install xvfb imagemagick xdotool
```

**Arch Linux** (platform uji):

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libsoup3 gtk3 \
  librsvg openssl libayatana-appindicator xdotool xorg-server-xvfb imagemagick python
```

## Menjalankan

```bash
npm --prefix ui install
cargo tauri dev
```

`cargo tauri dev` butuh Tauri CLI (`cargo install tauri-cli --locked`). Alternatif
tanpa memasang CLI Rust: `npm --prefix ui run tauri dev`.

Argumen pertama yang bukan flag diperlakukan sebagai alamat awal, misalnya
`cargo tauri dev -- https://example.com/`. Tanpa argumen, aplikasi membuka
`https://example.com/` (placeholder sampai ada halaman tab baru).

Build rilis: `cargo tauri build`.

## Uji

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Smoke test GUI di server tanpa layar + screenshot ke docs/screenshots/
bash scripts/smoke-gui.sh

# Uji interaksi: ketik alamat, mundur, maju, muat ulang, situs luar
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap1.png scripts/uji/navigasi.sh

# Uji tab: tab baru, pindah tab, tutup tab, ciutkan/buka sidebar
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap2.png scripts/uji/tab.sh
```

Skrip smoke menyalakan server dev UI (Vite) dan server halaman uji sendiri, lalu
menjalankan aplikasi di dalam Xvfb. Build debug memang memuat UI dari `devUrl`,
jadi Vite harus hidup saat aplikasi debug dijalankan.

Docker (lingkungan Arch):

```bash
docker build -t diybrowser-arch .
docker run --rm diybrowser-arch cargo test --workspace
```

## Catatan teknis

### Multi-webview di Linux

Setiap tab adalah child webview terpisah (fitur `unstable` Tauri), bukan iframe,
karena banyak situs melarang dirinya di-iframe.

Sampai Tauri 2.12.5, `tauri-runtime-wry` menambahkan child webview ke `GtkBox`
bawaan window (`build_gtk(window.default_vbox())`). `GtkBox` menyusun anaknya
secara vertikal, jadi semua webview berbagi tinggi window dan `set_bounds` tidak
memindahkan apa pun (tauri-apps/tauri#10420, masih terbuka). Karena itu
`src-tauri/src/platform/linux.rs` menyisipkan `GtkFixed` sendiri ke dalam
`GtkBox` tersebut dan memindahkan seluruh webview ke dalamnya — wadah yang memang
didukung wry untuk penempatan absolut. Di Windows dan macOS tidak ada masalah ini,
jadi `src-tauri/src/platform/lainnya.rs` langsung memakai `Webview::set_bounds`.

Modul Linux itu bisa dihapus begitu tao/wry menambahkan dukungan `GtkFixed` bawaan.
Karena `with_webview` bergantung pada tipe webkit2gtk/gtk, versi Tauri dipatok ke
minor (mis. `2.12`).

### Izin IPC

Capability `default` hanya berlaku untuk webview chrome (label `main`). Webview
tab yang memuat situs web tidak punya akses IPC sama sekali, dan event dari Rust
ditargetkan khusus ke label `main`.

### Tinggi chrome dan sidebar

`TINGGI_CHROME` dan `LEBAR_SIDEBAR` di `src-tauri/src/tabs.rs` harus sama dengan
`--tinggi-chrome` dan `--lebar-sidebar` di `ui/src/app.css`. Saat sidebar
diciutkan, `x` area isi tab menjadi 0 dan lebarnya ditambah `LEBAR_SIDEBAR`.

Hanya tab aktif yang diposisikan dan ditampilkan; tab lain disembunyikan dengan
`Webview::hide`. Karena `atur_batas` di Linux memanggil `widget.show()`, jangan
memanggilnya untuk tab yang tidak aktif.
