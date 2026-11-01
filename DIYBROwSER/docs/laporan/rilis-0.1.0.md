# Laporan Rilis 0.1.0 — "Perisai"

Tanggal: 1 November 2026
Mesin build: Ubuntu 24.04 headless (VPS), Rust 1.99.1, Node 24.14, WebKitGTK 2.52.6,
`target` dipindah ke `/tmp/diy-target` (partisi `/` hanya 3,7 GB bebas).

## 1. Ganti nama jadi Perisai

Nama produk resmi: **Perisai**. Yang berubah:

| Tempat | Sebelum | Sesudah |
| --- | --- | --- |
| `src-tauri/tauri.conf.json` `productName` | DIY Browser | Perisai |
| `identifier` | `dev.diybrowser.app` | `dev.perisai.app` |
| Judul jendela + `<title>` UI | DIY Browser | Perisai |
| Crate Tauri (`src-tauri/Cargo.toml`) | `diy-browser` / `diy_browser` | `perisai` |
| Nama biner | `target/*/diy-browser` | `target/*/perisai` |
| Merek di sidebar | `DIY` | `Perisai` |
| Paket UI | `diy-browser-ui` | `perisai-ui` |
| Skrip uji + Dockerfile | `diy-browser`, `diybrowser-arch` | `perisai`, `perisai-arch` |

Crate `adblock-core` dan `storage` tidak berubah namanya. `identifier` berubah
berarti direktori data aplikasi ikut pindah (`dev.perisai.app`); belum ada data
pengguna yang perlu dimigrasi karena tahap 3–5 belum menyimpan apa pun.

## 2. Bug yang ketemu saat uji dan diperbaiki

`scripts/uji/helper-xvfb.sh` mencari jendela dengan
`xdotool search --name 'Perisai'`. Pencarian itu cocok sebagian dan tidak peka
huruf besar-kecil, jadi selain jendela utama 1200×800 ia juga mengenai jendela
pembantu GTK 10×10 bernama `perisai`. `xdotool windowfocus` ke jendela pembantu
itu gagal (`BadMatch` pada `X_SetInputFocus`) dan seluruh skenario berhenti.
Perbaikannya: `cari_jendela_utama()` menyaring hasil berdasarkan ukuran
(≥ 400×300) sebelum memberi fokus.

## 3. Perintah build

```bash
export CARGO_TARGET_DIR=/tmp/diy-target TMPDIR=/tmp CARGO_PROFILE_RELEASE_DEBUG=0
cargo tauri build --bundles deb,rpm,appimage
```

`rpmbuild` dipasang lebih dulu (`sudo apt install rpm`) karena bundler RPM
membutuhkannya. Waktu build rilis penuh: ±8 menit (`Finished 3 bundles`).

## 4. Artefak

Disalin ke `rilis/` di akar repo (tidak di-commit, ada di `.gitignore`).
Semua untuk x86_64, glibc (bukan musl).

| Berkas | Ukuran | Dipakai untuk |
| --- | --- | --- |
| `Perisai_0.1.0_amd64.AppImage` | 78 MB | semua distro Linux, termasuk **Arch** |
| `Perisai_0.1.0_amd64.deb` | 2,9 MB | Debian/Ubuntu |
| `Perisai-0.1.0-1.x86_64.rpm` | 2,9 MB | Fedora/openSUSE |
| `perisai-0.1.0-linux-x86_64` | 12 MB | biner telanjang tanpa paket |
| `SHA256SUMS` | — | checksum keempat berkas di atas |

Isi paket `.deb` (diperiksa dengan `dpkg -c`):

```
/usr/bin/perisai
/usr/share/applications/Perisai.desktop
/usr/share/icons/hicolor/512x512/apps/perisai.png
```

`.desktop`: `Exec=perisai`, `Icon=perisai`, `StartupWMClass=perisai`,
`Name=Perisai`. Ketergantungan: `libwebkit2gtk-4.1-0`, `libgtk-3-0`.

## 5. Verifikasi

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all --check` | Lolos, tidak ada diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lolos, 0 warning |
| `cargo test --workspace` | 17 lolos, 0 gagal (`adblock-core` 3, `perisai` 11, `storage` 3) |
| `npm --prefix ui run build` | Lolos |
| `bash scripts/smoke-gui.sh ... scripts/uji/tab.sh` (debug) | Lolos, 7 screenshot + 3 pemeriksaan piksel |
| Biner rilis `target/release/perisai` di Xvfb + skenario tab | Lolos |
| `dpkg -i Perisai_0.1.0_amd64.deb` lalu `/usr/bin/perisai` di Xvfb | Lolos, paket terpasang `install ok installed` |
| AppImage (`APPIMAGE_EXTRACT_AND_RUN=1`) di Xvfb + skenario tab | Lolos |

Skenario tab yang dijalankan pada tiap biner rilis: tab baru, pindah tab, tutup
tab, ciutkan sidebar, buka sidebar, tutup tab terakhir. Pemeriksaan piksel
memastikan sidebar benar-benar memakan ruang dan hilang saat diciutkan
(`srgb(236,238,242)` → `srgb(15,41,80)`), dan menutup tab terakhir membuka tab
baru (`srgb(238,238,238)`). Tidak ada panic di log aplikasi mana pun.

Screenshot hasil uji rilis (sementara, tidak di-commit): `/tmp/uji-rilis/`,
`/tmp/uji-deb/`, `/tmp/uji-appimage/`.

## 6. Yang belum terverifikasi

- Menjalankan AppImage lewat FUSE (`chmod +x` lalu klik dua kali) — mesin ini
  tidak punya `/dev/fuse`, jadi yang dites hanya mode `--appimage-extract-and-run`.
  Di Arch, pasang `fuse2` kalau muncul pesan soal FUSE.
- Layar nyata: Wayland/X11 dengan GPU, HiDPI, dan resize interaktif. Semua uji
  di sini memakai Xvfb + rendering perangkat lunak.
- Build Windows/macOS (butuh mesin masing-masing).
- Tahap 3–5 (adblock, bookmark/history, cosmetic filter) memang belum ada di
  0.1.0; rilis ini setara Tahap 1 + 2.
