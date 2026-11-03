# PROGRESS

## Status tahap

Nama produk: **Perisai** (crate `perisai`, biner `perisai`, paket `perisai`).
Versi sekarang: **0.2.0**.

| Tahap | Isi | Status |
| --- | --- | --- |
| 1 | Workspace, shell Tauri, satu tab, address bar, back/forward/reload | Selesai |
| 2 | Banyak tab + sidebar vertikal yang bisa diciutkan | Selesai |
| 3 | Adblock (EasyList + EasyPrivacy) + Perlindungan per situs, UI baru, bookmark, riwayat, unduhan, zoom, sesi, pintasan | Selesai (Linux) |
| 4 | Mode privat, find in page, spaces, pin tab, command palette | Belum |
| 5 | Penghitung iklan per request, jalur Windows/macOS | Belum |

## Selesai

- **Tahap 1** — lihat `docs/laporan/tahap-1.md`.
- **Tahap 2** — lihat `docs/laporan/tahap-2.md`. Multi tab, sidebar vertikal,
  hanya tab aktif yang ditempatkan.
- **Tahap 3 (2 Nov 2026)** — lihat `docs/laporan/tahap-3.md`.
  - `adblock-core` kini mesin sungguhan (adblock-rust 0.13): EasyList +
    EasyPrivacy + aturan Perisai → 130.066 aturan content blocker JSON (14 MB),
    dikompilasi WebKit menjadi rule list 42 MB.
  - Content blocker dipasang di Linux lewat FFI (`WebKitUserContentFilterStore`
    tidak dibungkus binding Rust) ke setiap `WebKitUserContentManager`.
  - Scriptlet YouTube (`SCRIPTLET_YOUTUBE`) dan cosmetic filter untuk permukaan
    iklan YouTube; endpoint statistik iklan diblokir.
  - Panel Perlindungan per situs (aturan aktif + selector, toggle yang langsung
    berlaku), tanpa mengarang angka "iklan diblokir".
  - Halaman tab baru (`beranda.html`), bookmark, riwayat, unduhan, zoom per
    situs, pemulihan sesi, tema sistem/terang/gelap, pintasan keyboard lewat
    handler GTK, dan UI baru (address bar pill, tab manager, panel samping).
  - Setelan render: akselerasi GPU `Always`, WebGL, smooth scrolling, devtools
    hanya di build debug, dan jalur perangkat lunak lewat
    `PERISAI_RENDER_SOFTWARE=1`.
- **Rilis 0.1.0** — lihat `docs/laporan/rilis-0.1.0.md`.
- **Rilis 0.2.0 (2 Nov 2026)** — lihat `docs/laporan/rilis-0.2.0.md`. AppImage
  81 MB, `.deb`/`.rpm` 6,3 MB, biner 21 MB + `SHA256SUMS` di `rilis/`. Biner
  rilis, AppImage (extract-and-run), dan `.deb` terpasang semuanya lolos uji
  skenario tab di Xvfb tanpa panic.
- **Rilis 0.2.1 (3 Nov 2026)** — lihat `docs/laporan/rilis-0.2.1.md`. Perbaikan
  "Unacceptable TLS certificate" pada AppImage di distro non-Debian: tumpukan
  TLS bawaan AppImage kehilangan seluruh CA karena `libp11-kit` bawaan mematok
  folder PKCS#11 Debian. `src-tauri/src/lingkungan.rs` menulis modul p11-kit
  pengguna yang menunjuk modul trust sistem lewat path absolut.

## Verifikasi terakhir (2 November 2026)

- `cargo fmt --all --check` — lolos.
- `cargo clippy --workspace --all-targets -- -D warnings` — lolos, 0 warning.
- `cargo test --workspace` — **31 lolos** (adblock-core 10, perisai 15, storage 6).
- `npm --prefix ui run build` — lolos tanpa peringatan.
- `scripts/uji/tab.sh`, `scripts/uji/pintasan.sh` — lolos dengan pemeriksaan piksel.
- `scripts/uji-adblock.sh` — lolos dua putaran berturut-turut: dengan aturan uji
  kotak iklan hilang, tanpa aturan uji kotak muncul (pembanding sah).
- `scripts/uji-fitur.sh` — bookmark, riwayat, zoom per host (1.2), dan sesi
  terbaca benar di SQLite setelah aplikasi keluar.
- Panel Perlindungan, Pustaka, dan Menu diperiksa lewat screenshot Xvfb.

## Langkah berikut

- Paket 0.2.0 (`cargo tauri build`) lalu uji manual di Arch: pemutaran YouTube,
  akselerasi GPU nyata, Wayland, dan HiDPI.
- Uji unduhan sungguhan dan pemulihan sesi setelah `kill -9`.
- Jalur Windows (`WebResourceRequested`) dan macOS (`WKContentRuleList`), serta
  pintasan keyboard non-Linux.
- Fitur P1: find in page, mode privat, spaces, pin tab, command palette.
- Kalau penghitung iklan per request jadi syarat: pertimbangkan WebExtension
  `send-request` atau pindah ke CEF.

## Masalah / catatan yang diketahui

- **Iklan video YouTube belum diuji dengan pemutaran sungguhan.** Yang tersedia
  baru aturan cosmetic + scriptlet pelompat iklan; hasilnya best-effort.
- **Panel samping memesan ruang** (halaman menyempit saat panel dibuka), karena
  child webview selalu di atas webview chrome sehingga popover melayang akan
  tertutup. Ini kompromi yang disengaja.
- **Akselerasi GPU belum diukur**; semua uji berjalan dengan render perangkat lunak.
- **Windows/macOS**: Perlindungan belum aktif (no-op), pintasan keyboard belum ada.
- **Disk `/workspaces` penuh** saat mengerjakan tahap ini; `target/` dipindah ke
  `/tmp/perisai-target` (symlink) agar linker tidak gagal. Bukan masalah kode.
- **Workaround `GtkFixed`** di `src-tauri/src/platform/linux/mod.rs` bergantung
  pada Tauri 2.12/gtk 0.18 (`with_webview`); perlu dicek kalau Tauri dinaikkan.
