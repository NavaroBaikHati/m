# PROGRESS

## Laporan pemulihan konteks — 28 Oktober 2026

- **Tahap 1 (selesai):** shell Tauri, satu tab (child webview), address bar, back/forward/reload, smoke test Xvfb — kode dan `docs/laporan/tahap-1.md` ada dan lolos uji ulang.
- **Tahap 2 (belum mulai):** belum ada daftar tab, sidebar, atau perintah buka/tutup/pindah tab; `TabManager` hanya melacak satu `aktif` tanpa operasi tutup/pilih, dan `rect_konten()` masih selebar window.
- **Tahap 3 (belum mulai):** `adblock-core` baru berisi katalog EasyList/EasyPrivacy; crate `adblock` belum jadi dependensi, belum ada intersepsi request maupun shield.
- **Tahap 4 (belum mulai):** `storage` baru punya tabel `pengaturan`; belum ada tabel bookmark/history, download manager, atau kontrol tema.
- **Tahap 5 (belum mulai):** belum ada cosmetic filter, scriptlet, atau updater daftar filter.
- **fmt:** `cargo fmt --all --check` lolos (tanpa diff).
- **clippy:** `cargo clippy --workspace --all-targets -- -D warnings` lolos, 0 warning.
- **test:** `cargo test --workspace` lolos, 13 tes (adblock-core 3, storage 3, url_input 7).
- **Smoke GUI:** `scripts/smoke-gui.sh` lolos di Xvfb (1280x800), screenshot `docs/screenshots/tahap1.png` diperbarui.
- **Belum terverifikasi dari server:** `cargo tauri dev` di Arch dengan layar nyata (webkit2gtk-4.1), Windows/macOS, Wayland/HiDPI. Perlu dites manual di Arch.
- **Catatan lingkungan:** disk `/workspaces` sempat penuh (0 byte); `cargo clean` membebaskan 3,9 GB dan build uji diarahkan ke `CARGO_TARGET_DIR=/tmp/diy-target`. Tidak mengubah kode.
- **Rencana berikutnya:** kerjakan Tahap 2 (banyak tab + sidebar vertikal yang bisa diciutkan), lalu laporan tahap dan commit.
