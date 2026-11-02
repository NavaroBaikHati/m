# Laporan Rilis 0.2.0 — "Perisai"

Tanggal: 2 November 2026
Mesin build: Ubuntu 24.04 headless (VPS/codespace), Rust 1.99.1, Node 24.14,
WebKitGTK 2.52, `target` dipindah ke `/tmp/perisai-target` lewat symlink
(partisi `/workspaces` tinggal ±147 MB saat build).

Rilis ini adalah **Tahap 3 + 4 + 5**: adblock nyata (Brave adblock-rust),
bookmark/riwayat/unduhan/zoom/sesi, cosmetic filter + scriptlet, dan rombak UI
(address bar pill, tab manager, side panel). Detail teknis di
[laporan tahap-3](tahap-3.md).

## 1. Perintah build

```bash
export CARGO_TARGET_DIR=/tmp/perisai-target TMPDIR=/tmp CARGO_PROFILE_RELEASE_DEBUG=0
cargo tauri build --bundles deb,rpm,appimage
```

Waktu build rilis penuh dari kosong: ±12 menit (289 crate), `EXIT=0`,
`Finished 3 bundles`. `rpmbuild` sudah terpasang di mesin ini.

## 2. Artefak

Disalin ke `rilis/` di akar repo (tidak di-commit, ada di `.gitignore`).
Semua untuk x86_64, glibc (bukan musl).

| Berkas | Ukuran | Dipakai untuk |
| --- | --- | --- |
| `Perisai_0.2.0_amd64.AppImage` | 81 MB | semua distro Linux, termasuk **Arch** |
| `Perisai_0.2.0_amd64.deb` | 6,3 MB | Debian/Ubuntu |
| `Perisai-0.2.0-1.x86_64.rpm` | 6,3 MB | Fedora/openSUSE |
| `perisai-0.2.0-linux-x86_64` | 21 MB | biner telanjang tanpa paket |
| `SHA256SUMS` | — | checksum keempat berkas di atas |

Ukuran naik dari 0.1.0 (2,9 MB → 6,3 MB) karena biner kini memuat mesin
adblock-rust dan SQLite bundel; AppImage ikut naik 78 → 81 MB.

Isi paket `.deb` (sama seperti 0.1.0):

```
/usr/bin/perisai
/usr/share/applications/Perisai.desktop
/usr/share/icons/hicolor/512x512/apps/perisai.png
```

Ketergantungan: `libwebkit2gtk-4.1-0`, `libgtk-3-0`.

## 3. Verifikasi

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all --check` | Lolos |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lolos, 0 warning |
| `cargo test --workspace` | 31 lolos, 0 gagal |
| `npm --prefix ui run build` | Lolos, tanpa peringatan a11y |
| `scripts/uji/tab.sh`, `pintasan.sh`, `uji-adblock.sh`, `uji-fitur.sh` (debug, Xvfb) | Lolos semua |
| Biner rilis `target/release/perisai` di Xvfb + `tab.sh` | **LULUS** |
| AppImage (`--appimage-extract` + `AppRun`) di Xvfb + `tab.sh` | **LULUS** |
| `dpkg -i Perisai_0.2.0_amd64.deb` lalu `/usr/bin/perisai` + `tab.sh` | **LULUS**, `install ok installed` |

Skenario tab pada ketiga biner rilis: tab baru, pindah tab, tutup tab,
ciutkan sidebar, buka sidebar, tutup tab terakhir — plus pemeriksaan piksel.
Tidak ada panic di log aplikasi mana pun. Screenshot:
`/tmp/uji-rilis2/`.

Bukti adblock (pembanding): `scripts/uji/adblock.sh` — dengan aturan uji kotak
iklan hilang, tanpa aturan kotak merah muncul. Panel Perlindungan dan seluruh
pintasan keyboard teruji di `uji-fitur.sh`.

## 4. Yang belum terverifikasi

- Iklan video **YouTube sungguhan** belum diputar di mesin ini; scriptlet
  `SCRIPTLET_YOUTUBE` masih best-effort bergantung DOM YouTube.
- FUSE AppImage belum diuji (mesin tanpa `/dev/fuse`); di Arch pasang `fuse2`
  kalau diminta. Mode extract-and-run sudah terbukti.
- Akselerasi GPU belum diukur (semua uji Xvfb render perangkat lunak).
- Windows/macOS: jalur proteksi & pintasan belum dibungkus.
- Unduhan belum diuji dengan berkas nyata di mesin ini.
