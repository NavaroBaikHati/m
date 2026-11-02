# Rencana Fitur Perisai

Tanggal: 2 November 2026 · Status: sebagian besar P0 sudah dikerjakan di 0.2.0 (lihat `docs/laporan/tahap-3.md`); bagian Windows/macOS dan P1 masih terbuka · Basis: rilis 0.1.0 (Tahap 1–2 selesai)

Dokumen ini menjawab satu pertanyaan: **fitur apa yang wajib ada supaya Perisai terasa
seperti browser sungguhan, dan mana yang benar-benar bisa dikerjakan di stack sekarang
(Tauri + WebView sistem).** Isinya gabungan dua hal: daftar fitur yang dianggap standar
pada browser populer, dan pengecekan API nyata di Tauri/WebKitGTK — karena beberapa fitur
"wajib" ternyata tidak punya jalur resmi di stack ini.

## 0. Keputusan yang sudah diambil (2 Nov 2026)

- Jalur adblock Linux memakai content blocker WebKit yang dikompilasi
  `adblock-rust`; Windows/macOS ditunda.
- Tetap Tauri untuk 0.2.x. Penghitung per request dan scriptlet presisi lintas OS
  belum jadi syarat, jadi belum ada alasan pindah ke CEF.
- Batas fitur: P0 selesai; P1 menyusul satu per satu.

## 1. Tiga keputusan yang menunggu

1. **Jalur adblock per OS.** Tidak ada satu jalur kode untuk tiga OS (lihat §6.1). Ini
   menentukan apakah Tahap 3 dikerjakan di Linux saja dulu atau langsung lintas OS.
2. **Tetap Tauri atau pindah ke CEF.** Selama fitur yang dibutuhkan masih bisa dicapai
   lewat API native masing-masing webview, Tauri cukup. Titik pindahnya: kalau
   penghitung iklan per situs dan scriptlet harus presisi lintas OS.
3. **Batas "ringan" vs "lengkap".** Dokumen ini menyarankan berhenti di P1; semua di §5
   sengaja tidak dikerjakan.

## 2. Yang sudah ada di 0.1.0

Tab banyak dengan sidebar vertikal yang bisa diciutkan, address bar dengan pencarian,
mundur/maju/muat ulang, judul tab dan indikator memuat, tutup tab terakhir yang tidak
meninggalkan jendela kosong, tema mengikuti sistem (`prefers-color-scheme`), dan paket
AppImage/deb/rpm. Belum ada: pintasan keyboard, pemulihan sesi, halaman tab baru khusus.

## 3. Wajib berikutnya (P0)

Semua di bawah ini dianggap "tidak ada = bukan browser" oleh pengguna biasa. Kolom
"jalur" merujuk ke temuan di §6.

| Fitur | Kenapa wajib | Jalur di stack sekarang | Usaha |
| --- | --- | --- | --- |
| Blokir iklan & tracker (EasyList + EasyPrivacy) | Inti produk, ada di PRD | `adblock-rust` + content blocker WebKit (§6.1) | L |
| Shield per situs + penghitung | Pembeda vs browser lain | UI + storage; penghitung butuh keputusan §6.1 | M |
| Bookmark | Standar semua browser | crate `storage` (SQLite) + UI sidebar | S |
| History + hapus riwayat | Standar; dibutuhkan agar tidak terasa "setengah jadi" | `storage` + hook navigasi | S |
| Download manager | Tanpa ini unduhan hilang tanpa jejak | `on_download` + `DownloadEvent` (§6.2) | M |
| Pintasan keyboard (Ctrl+T/W/L/Tab, Ctrl+Shift+T) | Pengguna keyboard langsung menilainya rusak | aksen menu/aplikasi, bukan webview | S |
| Pemulihan sesi | Tab hilang saat crash = kepercayaan hilang | simpan daftar URL di SQLite (§6.7) | S |
| Halaman tab baru (pencarian + situs favorit) | Kesan pertama; sekarang `example.com` | UI murni | S |
| Zoom per situs + Ctrl+=/-/0 | Aksesibilitas dasar | `set_zoom` / `set_zoom_level` (§6.4) | S |
| Toggle tema terang/gelap manual | Sistem saja tidak cukup | UI + storage | S |

## 4. Penting (P1)

| Fitur | Catatan | Usaha |
| --- | --- | --- |
| Spaces (kelompok tab berwarna) | Murni UI + kolom `ruang` di tabel tab | M |
| Pin tab | Murni UI + urutan | S |
| Command palette (Ctrl+K) | Cari tab, bookmark, history, aksi | M |
| Find in page | Tidak ada API di Tauri; native per OS (§6.5) | M |
| Mode privat | Konteks webview terpisah, data tidak ditulis (§6.6) | M |
| HTTPS upgrade otomatis | `on_navigation` hanya boleh menolak, tidak menulis ulang URL (§6.3) | M |
| Blokir cookie pihak ketiga | Bergantung jalur adblock yang dipilih | M |
| Reader mode | Ekstraksi teks via `initialization_script`, lalu render lokal | M |
| Scriptlet injection (anti-adblock) | `adblock-rust` + resource assembler | L |
| Pembaruan daftar filter otomatis | Unduh berkala, verifikasi, muat ulang engine | M |
| Print / simpan PDF | Jalur native per OS, belum ada di API Tauri | M |
| Devtools | Tauri: `devtools` saat debug; rilis perlu keputusan | S |

## 5. Tidak dikerjakan (dan alasannya)

| Fitur | Alasan |
| --- | --- |
| DRM/Widevine (Netflix, Spotify) | Butuh lisensi berbayar; tidak layak untuk proyek perorangan |
| Ekstensi Chrome | Butuh Chromium/CEF; bertentangan dengan target ringan |
| Sinkronisasi awan | Butuh server + akun; di luar cakupan "semua data lokal" |
| PWA install | Dukungan WebView sistem tidak seragam, hasilnya mengecewakan |
| Password manager | Risiko keamanan tinggi; cukup serahkan ke keychain OS nanti |
| AI sidebar | Bisa jadi pembeda (tren 2025–2026), tapi menambah ketergantungan API berbayar |

## 6. Temuan teknis yang mengubah rencana

### 6.1 Adblock: satu engine, tiga jalur OS

Ini temuan terpenting. Tauri **tidak** menyediakan API intersepsi request lintas OS, dan
wry (lapisan webview di bawah Tauri) hanya memakai `WebResourceRequested` di backend
WebView2 — artinya jalur itu hanya ada di Windows.

| OS | Engine | Cara memblokir | Status di wry/Tauri |
| --- | --- | --- | --- |
| Linux | WebKitGTK | `WebKitUserContentManager.add_filter` (content blocker JSON) | Belum dibungkus; bisa lewat `with_webview` yang sudah dipakai di `platform/linux.rs` |
| macOS | WKWebView | `WKContentRuleList` (format sama) | Belum ada di wry; butuh `objc2` |
| Windows | WebView2 | `WebResourceRequested` (blokir per request) | Sudah dipakai wry |

Kabar baiknya: `adblock-rust` punya fitur `content-blocking` yang mengubah aturan gaya
ABP menjadi format content blocker Apple. Jadi satu daftar filter bisa dipakai untuk
WebKit (Linux/macOS) dan tetap memakai pencocokan native untuk Windows.

**Konsekuensi yang harus disadari:** content blocker WebKit tidak memberi callback per
request yang diblokir, sehingga "penghitung iklan per situs" ala Brave tidak bisa
diambil langsung dari engine. Pilihan: (a) hitung perkiraan dari aturan yang match saat
kompilasi filter, (b) proxy lokal yang memfilter lalu menghitung, (c) pindah ke CEF.
Keputusan ini menentukan besar-kecilnya Tahap 3.

### 6.2 Download manager tersedia tanpa kode native

`WebviewBuilder::on_download` menerima callback berisi `DownloadEvent` (mulai, progres,
selesai). Artinya manajer unduhan bisa ditulis penuh di Rust, dengan daftar unduhan di
crate `storage`.

### 6.3 HTTPS upgrade tidak bisa lewat `on_navigation`

Callback itu hanya memutuskan boleh/tidak, tidak bisa mengganti URL. Untuk menulis ulang
`http://` → `https://` perlu intervensi lebih awal (script inisialisasi) atau proxy.
Karena itu fitur ini turun dari P0 ke P1.

### 6.4 Zoom sudah lengkap

`Webview::set_zoom(scale)` / `set_zoom_level`, dan `WebviewBuilder::zoom_hotkeys_enabled`.
Tinggal menyimpan zoom per host di SQLite dan menyediakan Ctrl+=/-/0 di UI.

### 6.5 Find in page: tidak ada API

Tidak ada method pencarian di API `Webview`/`WebviewBuilder` Tauri. Perlu
`WebKitFindController` di Linux, dan jalur lain di Windows/macOS. Ini satu-satunya fitur
P1 yang mau tidak mau menulis kode native per OS.

### 6.6 Mode privat

API cookie sudah ada (`cookies`, `cookies_for_url`). Mode privat dijalankan sebagai webview
dengan konteks/data directory terpisah yang dibuang saat ditutup, bukan sebagai flag.

### 6.7 Pemulihan sesi murah

Cukup simpan `(ruang, urutan, url, judul)` setiap kali tab berubah dan pulihkan saat
start. Tidak perlu menyentuh engine.

## 7. Urutan pengerjaan yang diusulkan

| Tahap | Isi | Definisi selesai |
| --- | --- | --- |
| 3 | `adblock-core` sungguhan: unduh EasyList + EasyPrivacy, kompilasi ke content blocker JSON, pasang ke webview Linux, hitung blokir, panel shield per situs | Halaman uji berisi 3 script tracker: 2 halaman tidak memuatnya, hitungan tampil benar, ada unit test di `adblock-core` |
| 4 | Bookmark, history, download manager, zoom per situs, toggle tema, halaman tab baru | Semua punya tabel SQLite + skenario GUI di `scripts/uji/` dengan screenshot |
| 5 | Cosmetic filter, scriptlet, pembaruan daftar filter otomatis | Uji halaman dengan placeholder iklan: elemen kosong hilang, tidak ada panic |
| 6 | Pintasan keyboard, pemulihan sesi, mode privat, find in page, spaces, pin, command palette | Uji keyboard di Xvfb (`xdotool key`) lolos; sesi pulih setelah kill -9 |
| 7 | Keputusan Windows/macOS: bungkus `WebResourceRequested` atau pindah CEF | Satu halaman uji yang sama lolos di dua OS |

Pola verifikasi tetap seperti sekarang: `cargo fmt`, clippy `-D warnings`, `cargo test`,
lalu skenario `scripts/uji/*.sh` di Xvfb dengan pemeriksaan piksel, bukan hanya screenshot.

## 8. Sumber yang dipakai

Diambil 2 November 2026 memakai Lightpanda (`lightpanda fetch --dump markdown`), bukan
dari ingatan:

| Sumber | Yang diambil |
| --- | --- |
| `en.wikipedia.org/wiki/Web_browser` | Daftar fitur UI standar: tab, back/forward, reload+stop, home, address bar gabungan pencarian, riwayat, bookmark, ekstensi, unduhan, password, sinkronisasi, DRM |
| `en.wikipedia.org/wiki/Comparison_of_web_browsers` | Matriks fitur yang dianggap pembeda: bookmark, unduhan, password, history, pulihkan sesi, spellcheck, mode privasi, auto-update, blokir pop-up, blokir autoplay, ad filtering, zoom, proteksi fingerprint |
| `raw.githubusercontent.com/brave/adblock-rust/master/README.md` | Kemampuan engine: network blocking, cosmetic, resource replacement, sintaks uBO, fitur `content-blocking` untuk format Apple |
| `docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html` | `on_download` + `DownloadEvent`, `on_navigation`, `on_new_window`, `initialization_script`, `zoom_hotkeys_enabled` |
| `docs.rs/tauri/latest/tauri/webview/struct.Webview.html` | `cookies`, `cookies_for_url`, `set_zoom`, `set_zoom_level`; tidak ada method pencarian teks |
| `webkitgtk.org/reference/webkit2gtk/stable/class.UserContentManager.html` | `add_filter`, `add_script`, `add_style_sheet` — jalur content blocker WebKit |
| `github.com/tauri-apps/wry` (kode) | `WebResourceRequested` hanya ada di `src/webview2/mod.rs`; tidak ada dukungan `WKContentRuleList`/user content filter |

Yang **belum** diverifikasi dan sebaiknya diuji sebelum Tahap 3 dimulai: apakah
`with_webview` di Tauri 2.12 benar-benar memberi akses ke `WebKitUserContentManager` pada
webview anak (bukan hanya jendela utama), dan berapa lama kompilasi EasyList+EasyPrivacy
ke format content blocker JSON (target di bawah 200 ms agar tidak terasa saat start).
