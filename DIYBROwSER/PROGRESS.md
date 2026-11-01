# PROGRESS

## Status tahap

Nama produk: **Perisai** (crate `perisai`, biner `perisai`, paket `perisai`).

| Tahap | Isi | Status |
| --- | --- | --- |
| 1 | Workspace, shell Tauri, satu tab, address bar, back/forward/reload | Selesai |
| 2 | Banyak tab + sidebar vertikal yang bisa diciutkan | Selesai |
| 3 | Adblock request blocking (EasyList + EasyPrivacy) + shield per situs | Belum |
| 4 | Bookmark, history, download manager, mode gelap/terang | Belum |
| 5 | Cosmetic filter, scriptlet, pembaruan daftar filter | Belum |

## Selesai

- **Tahap 1** — lihat `docs/laporan/tahap-1.md`. Verifikasi diulang 28 Okt: fmt,
  clippy, 13 test, dan smoke GUI (Xvfb) lolos.
- **Tahap 2 (28 Okt)** — `TabManager` kini mengelola banyak tab (`buka`, `pilih`,
  `tutup`, `atur_sidebar`); hanya tab aktif yang ditempatkan dan ditampilkan.
  Sidebar vertikal di UI Svelte berisi judul tab, indikator memuat, tombol tutup,
  tombol tab baru, dan tombol ciut/buka. Menutup tab terakhir membuka tab baru.
  IPC baru: `daftar_tab`, `tab_baru`, `pilih_tab`, `tutup_tab`, `atur_sidebar`.
  Bukti: `docs/laporan/tahap-2.md` + 7 screenshot `docs/screenshots/tahap2-*.png`
  dan 3 pemeriksaan piksel di `scripts/uji/tab.sh`.

## Rilis 0.1.0 (1 November 2026)

- Ganti nama produk jadi **Perisai**: `productName`, `identifier`
  (`dev.perisai.app`), crate `perisai`, judul jendela, merek sidebar, skrip uji,
  dan Dockerfile. Lihat `docs/laporan/rilis-0.1.0.md`.
- `cargo tauri build --bundles deb,rpm,appimage` menghasilkan AppImage 78 MB,
  `.deb` dan `.rpm` 2,9 MB, plus biner telanjang 12 MB di `rilis/`.
- Biner rilis, paket `.deb` yang sudah dipasang, dan AppImage masing-masing
  dijalankan di Xvfb dengan skenario tab lengkap: lolos tanpa panic.
- Bug uji diperbaiki: `cari_jendela_utama()` di `scripts/uji/helper-xvfb.sh`
  menyaring jendela pembantu GTK 10x10 bernama `perisai` yang membuat
  `xdotool windowfocus` gagal dengan `BadMatch`.

## Verifikasi terakhir (1 November 2026)

- `cargo fmt --all --check` — lolos.
- `cargo clippy --workspace --all-targets -- -D warnings` — lolos, 0 warning.
- `cargo test --workspace` — 17 lolos (adblock-core 3, perisai 11, storage 3).
- `npm --prefix ui run build` — lolos.
- `bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap2.png scripts/uji/tab.sh` — lolos.
- Biner rilis + `.deb` terpasang + AppImage di Xvfb — lolos (skenario tab, 0 panic).

## Langkah berikut

- **Tahap 3:** intersepsi request memakai crate `adblock` (adblock-rust), unduh
  EasyList + EasyPrivacy, hitung request yang diblokir per tab, dan panel shield
  per situs dengan toggle on/off. `adblock-core` wajib punya unit test.

## Masalah / catatan yang diketahui

- **Belum terverifikasi dari server:** `cargo tauri dev` di Arch dengan layar
  nyata, Windows/macOS, Wayland, dan HiDPI. Perlu dites manual di Arch:
  `git pull && cargo tauri dev`.
- **Disk `/workspaces` penuh** (0 byte) membuat linker gagal; `cargo clean`
  membebaskan 3,9 GB dan build uji memakai `CARGO_TARGET_DIR=/tmp/diy-target`.
  Bukan masalah kode, tapi mesin ini perlu ruang kosong sebelum build besar.
- **Workaround `GtkFixed`** di `src-tauri/src/platform/linux.rs` bergantung pada
  Tauri 2.12/gtk 0.18 (`with_webview`). Perlu dicek kalau Tauri dinaikkan.
- **Belum ada:** pintasan keyboard (Ctrl+T/W), drag-reorder tab, pemulihan sesi,
  dan halaman tab baru khusus. Tab baru membuka `https://example.com/`.
- Riwayat pemulihan sesi sebelumnya (28 Okt): tidak ada perubahan yang belum
  di-commit dan build sehat, jadi tidak ada branch `recovery/` yang dibuat.
