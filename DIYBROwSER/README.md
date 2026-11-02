# Perisai

Browser desktop lintas OS: UI ringkas ala Arc + adblock bawaan ala Brave, dibangun
dengan Rust (Tauri v2) dan Svelte. Tanpa telemetri; semua data tinggal di lokal.

Nama produk **Perisai** (pelindung) dipakai sejak 1 Nov 2026; crate Rust-nya
bernama `perisai`, biner hasil build `target/*/perisai`, dan paket installer
`.deb`/`.rpm`/AppImage memakai nama `Perisai`/`perisai`.

PRD lengkap ada di [PRD.md](PRD.md).

## Status

| Tahap | Isi                                                              | Status    |
| ----- | ---------------------------------------------------------------- | --------- |
| 1     | Workspace, shell Tauri, satu tab, address bar, back/forward/reload | Selesai   |
| 2     | Banyak tab + sidebar vertikal                                     | Selesai   |
| 3     | Adblock (EasyList + EasyPrivacy), Perlindungan per situs, UI baru, bookmark, riwayat, unduhan, zoom, sesi, pintasan | Selesai (Linux) |
| 4     | Mode privat, find in page, spaces, pin tab, command palette       | Belum     |
| 5     | Penghitung iklan per request, jalur Windows/macOS                 | Belum     |

Versi 0.2.0. Catatan verifikasi tiap tahap ada di `docs/laporan/`, termasuk
[laporan rilis 0.1.0](docs/laporan/rilis-0.1.0.md),
[laporan Tahap 3](docs/laporan/tahap-3.md), dan
[laporan rilis 0.2.0](docs/laporan/rilis-0.2.0.md).

## Fitur

- **Perlindungan Perisai** — EasyList + EasyPrivacy dikompilasi menjadi content
  blocker WebKit (130.066 aturan), dipasang sebelum halaman dimuat, plus
  scriptlet khusus YouTube dan aturan tambahan Perisai. Bisa dimatikan per situs
  lewat panel di address bar.
- **Tab manager vertikal** dengan avatar situs, indikator memuat, nomor tab, dan
  sidebar yang bisa diciutkan.
- **Address bar pill** yang menampilkan host tebal + jalur samar saat tidak
  difokuskan, tombol Perlindungan, dan tombol bookmark.
- **Halaman tab baru** dengan pencarian dan pintasan situs favorit.
- **Bookmark, riwayat, unduhan, zoom per situs, pemulihan sesi, tema
  sistem/terang/gelap.**
- **Pintasan keyboard** tanpa menu bar (lihat tabel di bawah).

### Pintasan (Linux)

`Ctrl+T` tab baru · `Ctrl+Shift+T` buka tab tertutup · `Ctrl+W` tutup tab ·
`Ctrl+Tab` pindah tab · `Ctrl+L` fokus address bar · `Ctrl+R` muat ulang ·
`Ctrl+B` sidebar · `Ctrl+D` bookmark · `Ctrl+J` unduhan · `Ctrl+H` riwayat ·
`Ctrl+K` menu · `Ctrl+=` / `Ctrl+-` / `Ctrl+0` zoom · `Alt+←` / `Alt+→`
mundur/maju · `Ctrl+Q` keluar.

## Paket terpasang (rilis 0.2.0)

Hasil `cargo tauri build` ada di `rilis/` (tidak di-commit) dan di GitHub
Release `v0.2.0`: AppImage, `.deb`, `.rpm`, biner telanjang, dan `SHA256SUMS`.

**Arch Linux** — pakai AppImage, karena `.deb`/`.rpm` bukan format pacman:

```bash
chmod +x Perisai_0.2.0_amd64.AppImage
./Perisai_0.2.0_amd64.AppImage          # tambahkan fuse2 kalau diminta
```

Biner telanjang juga bisa langsung dijalankan (`./perisai-0.2.0-linux-x86_64`)
selama `webkit2gtk-4.1` dan `gtk3` terpasang.

**Debian/Ubuntu:** `sudo apt install ./Perisai_0.2.0_amd64.deb`
**Fedora:** `sudo dnf install ./Perisai-0.2.0-1.x86_64.rpm`

Setelah terpasang, Perisai muncul di menu aplikasi (`Name=Perisai`) atau bisa
dijalankan dengan perintah `perisai`.

## Struktur repo

```
crates/adblock-core/   Mesin adblock (adblock-rust) + content blocker + scriptlet
crates/storage/        SQLite: pengaturan, riwayat, bookmark, unduhan
src-tauri/             Aplikasi Tauri (crate `perisai`): tab manager, IPC, penyesuaian platform
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
halaman tab baru Perisai dan memulihkan sesi terakhir kalau ada.

Build rilis beserta paketnya:

```bash
cargo tauri build --bundles deb,rpm,appimage   # butuh rpmbuild untuk target rpm
```

Hasilnya di `target/release/bundle/`. `target/release/perisai` adalah binernya
sendiri (UI sudah tertanam, jadi tidak butuh Vite seperti build debug).

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

# Uji pintasan keyboard (Ctrl+T/W/L/B)
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap3-pintasan.png scripts/uji/pintasan.sh

# Adblock: dua putaran, dengan pembanding "tanpa aturan uji"
bash scripts/uji-adblock.sh

# Bookmark, riwayat, zoom, dan sesi: diperiksa langsung di SQLite
bash scripts/uji-fitur.sh
```

Skrip smoke menyalakan server dev UI (Vite) dan server halaman uji sendiri, lalu
menjalankan aplikasi di dalam Xvfb. Build debug memang memuat UI dari `devUrl`,
jadi Vite harus hidup saat aplikasi debug dijalankan.

Docker (lingkungan Arch):

```bash
docker build -t perisai-arch .
docker run --rm perisai-arch cargo test --workspace
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

### Cara Perlindungan bekerja

Mesin adblock (`crates/adblock-core`) mengompilasi EasyList, EasyPrivacy, dan
aturan tambahan Perisai menjadi **content blocker JSON** (format Safari, dipakai
WebKitGTK). `src-tauri/src/platform/linux/filter.rs` menyimpannya lewat
`WebKitUserContentFilterStore` — binding Rust-nya tidak membungkus tipe filter,
jadi modul itu memakai FFI — lalu memasangnya ke `WebKitUserContentManager`
setiap webview. WebKit yang mencocokkan request, jadi tidak ada biaya
per-request di sisi Rust.

Konsekuensi yang perlu diketahui: WebKit tidak melaporkan request yang diblokir,
sehingga panel Perlindungan menampilkan jumlah aturan aktif dan selector khusus
situs, bukan angka iklan yang diblokir. Jalur Windows (`WebResourceRequested`)
dan macOS (`WKContentRuleList`) belum dikerjakan.

Filter hasil kompilasi di-cache di `~/.cache/dev.perisai.app/adblock/`.
Daftar diunduh ulang bila lebih tua dari 24 jam; kalau unduhan gagal, cache lama
tetap dipakai. Cache hanya dipakai ulang kalau sidiknya cocok dengan daftar yang
ada (`filter.sig`), supaya filter basi tidak sempat dipasang.

### Akselerasi perangkat keras

WebKit disetel `HardwareAccelerationPolicy::Always`, WebGL, smooth scrolling, dan
media stream aktif. Untuk memaksa render perangkat lunak (mesin tanpa GPU,
driver rusak, atau sesi Xvfb):

```bash
PERISAI_RENDER_SOFTWARE=1 ./target/debug/perisai
```

Variabel `LIBGL_ALWAYS_SOFTWARE` dan `WEBKIT_DISABLE_COMPOSITING_MODE` juga
dihormati. Sesi uji repo ini selalu memakai render perangkat lunak, jadi
performa GPU nyata belum diukur.

### Tinggi chrome dan sidebar

`TINGGI_CHROME` dan `LEBAR_SIDEBAR` di `src-tauri/src/tabs.rs` harus sama dengan
`--tinggi-chrome` dan `--lebar-sidebar` di `ui/src/app.css`. Saat sidebar
diciutkan, `x` area isi tab menjadi 0 dan lebarnya ditambah `LEBAR_SIDEBAR`.

Hanya tab aktif yang diposisikan dan ditampilkan; tab lain disembunyikan dengan
`Webview::hide`. Karena `atur_batas` di Linux memanggil `widget.show()`, jangan
memanggilnya untuk tab yang tidak aktif.
