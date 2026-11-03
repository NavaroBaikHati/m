# Laporan Rilis 0.2.2 — mesin pencari yang bisa dipilih

Tanggal: 3 November 2026

## Masalah yang diselesaikan

Setelah 0.2.1, laporan pengguna di Arch: situs `http`/`https` terbuka normal,
tetapi **begitu mencari** muncul halaman "Unacceptable TLS certificate".
Artinya bukan tumpukan TLS yang rusak (itu akan menggagalkan semua HTTPS),
melainkan satu host saja: DuckDuckGo, yang dipakai sebagai mesin pencari bawaan.

Di jaringan yang memblokir/mengintersepsi DuckDuckGo, intersepsi itu sampai ke
WebKit sebagai sertifikat tak tepercaya. Karena URL pencarian dibuat otomatis
oleh aplikasi, pengguna tidak punya jalan keluar dari dalam browser.

## Perubahan

- `src-tauri/src/url_input.rs`: `MesinPencarian` (Google, Bing, DuckDuckGo,
  Brave) dengan `slug`/`nama`/`templat`, nilai aktif disimpan di `RwLock`
  global supaya address bar dan halaman tab baru selalu sama. Bawaan **Google**.
- `src-tauri/src/beranda.rs`: halaman tab baru tidak lagi mematok URL mesin
  pencari; templat disuntikkan saat berkas ditulis (`halaman(templat)`), dan
  berkas ditulis ulang setiap kali pilihan berubah.
- `src-tauri/src/commands.rs`: `daftar_mesin_pencarian` dan
  `atur_mesin_pencarian` (menyimpan ke pengaturan `mesin_pencarian` lalu
  menulis ulang beranda).
- `src-tauri/src/lib.rs`: pengaturan dibaca sebelum halaman tab baru dibuat.
- `ui/`: baris "Mesin pencari" di panel Menu (segmen Google / Bing / DDG /
  Brave) lewat `api.daftarMesinPencarian` dan `api.aturMesinPencarian`.

## Verifikasi

- `cargo test -p perisai --lib` — **23 lolos** (8 test baru: tiap mesin
  menghasilkan URL dengan `q` yang benar, slug tak dikenal ditolak, halaman
  beranda memakai templat mesin aktif).
- `cargo fmt --all --check` lolos, `cargo clippy -p perisai --all-targets
  -- -D warnings` bersih.
- `npm --prefix ui run build` lolos (65,78 kB JS).
- Build rilis `deb,rpm,appimage` → `EXIT=0`; AppImage 80,94 MiB.
- AppImage 0.2.2 diekstrak lalu dijalankan di Xvfb, `HOME` bersih:
  `beranda.html` yang dihasilkan berisi
  `https://www.google.com/search?q=` — bukan lagi DuckDuckGo.
- Di sesi Xvfb yang sama, `Ctrl+L` → ketik `jokowi` → Enter: halaman hasil
  Google termuat penuh (screenshot `/tmp/cari022.png`), tidak ada galat TLS.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.2.2_amd64.AppImage` | 81 MB |
| `Perisai_0.2.2_amd64.deb` | 6,4 MB |
| `Perisai-0.2.2-1.x86_64.rpm` | 6,4 MB |
| `perisai-0.2.2-linux-x86_64` | 21 MB |
| `SHA256SUMS` | — |

## Catatan

Mesin pencari bisa diganti langsung dari menu tanpa membuka situs mesin
pencari, jadi jaringan yang memblokir satu mesin tidak lagi memblokir
pencarian. Tab yang sudah terbuka memakai mesin baru setelah dimuat ulang atau
dibuka sebagai tab baru.
