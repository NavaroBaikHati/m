# Laporan Tahap 3 — adblock sungguhan, Perlindungan per situs, UI baru

Tanggal: 2 November 2026 · Versi: 0.2.0 · Mesin: Ubuntu 24.04 headless, Rust 1.99.1, WebKitGTK 2.52.6, Xvfb

Tahap ini mengerjakan P0 dari `docs/rencana-fitur.md`: mesin adblock nyata,
panel Perlindungan per situs, bookmark, riwayat, unduhan, zoom per situs,
pemulihan sesi, halaman tab baru, pintasan keyboard, tema manual, dan polesan
UI (address bar + tab manager).

## 1. Cara adblock bekerja

Tauri tidak punya API intersepsi request lintas OS, jadi perlindungan dipasang di
lapisan yang memang milik WebKit:

```
EasyList + EasyPrivacy + aturan Perisai
        │
        ├─ adblock-rust (crate `adblock` 0.13, fitur content-blocking)
        │
        ├─► content blocker JSON (130.066 aturan) ──► WebKitUserContentFilterStore
        │                                             └─► WebKitUserContentManager tiap webview
        └─► mesin cosmetic/scriptlet ──► eval scriptlet per halaman
```

- **Blokir request dan penyembunyian elemen dikerjakan WebKit**, bukan kode kita.
  JSON diubah WebKit menjadi *content rule list* 42 MB sekali per kombinasi
  daftar filter, lalu dipakai ulang saat aplikasi dibuka lagi.
- **Scriptlet** dijalankan dari Rust saat halaman mulai dimuat. Isinya berasal
  dari `url_cosmetic_resources()` milik mesin, ditambah `SCRIPTLET_YOUTUBE` di
  `crates/adblock-core/src/lib.rs`.
- **Panel Perlindungan tidak mengarang angka.** WebKit tidak melaporkan request
  yang diblokir, jadi panel menampilkan jumlah aturan aktif dan jumlah selector
  khusus situs itu — bukan "iklan diblokir: 12".
- **Toggle per situs** memakai `webkit_user_content_manager_remove_all_filters()`
  pada webview tab itu; karena tiap tab punya `WebContentManager` sendiri,
  mematikan Perlindungan di satu situs tidak mengganggu tab lain.

### Catatan implementasi yang perlu diketahui

- Binding `webkit2gtk` 2.0 **tidak membungkus** `WebKitUserContentFilter` (tipe
  `Ignored` di file hasil generate). Karena itu
  `src-tauri/src/platform/linux/filter.rs` memakai FFI mentah ke
  `webkit_user_content_filter_store_save_from_file()` dan
  `webkit_user_content_manager_add_filter()`. Satu-satunya alternatif adalah
  WebExtension `send-request`, yang jauh lebih berat.
- JSON 14 MB memerlukan waktu kompilasi di WebKit. Filter yang sedang aktif
  disimpan sebagai `filter.json` + `filter.sig`; `sig` adalah sidik dari daftar
  yang tersedia (ukuran, waktu ubah, dan isi aturan uji). Kalau daftar berubah,
  cache dianggap basi dan **tidak** dipakai — tanpa pemeriksaan itu, filter lama
  sempat terpasang dan situs terbuka dengan perlindungan yang salah (bug ini
  tertangkap oleh uji adblock, lihat §4).
- Daftar filter diunduh ulang bila cache-nya lebih tua dari 24 jam
  (`ureq`, batas 30 detik, isi dibatasi 32 MB). Kalau unduhan gagal, cache lama
  tetap dipakai; kalau belum ada sama sekali, aturan tambahan Perisai (YouTube +
  beberapa jaringan iklan) tetap jalan.
- `#![forbid(unsafe_code)]` tetap berlaku di `adblock-core` dan `storage`; kode
  `unsafe` hanya ada di modul FFI Linux.

## 2. Hardware acceleration

`src-tauri/src/platform/linux/mod.rs` menyetel WebKit per webview:

| Setelan | Nilai | Alasan |
| --- | --- | --- |
| `hardware-acceleration-policy` | `Always` | Bawaan WebKitGTK `OnDemand` membuat halaman ringan tidak memakai GPU sehingga animasi tersendat |
| `enable-webgl` | aktif | Diperlukan banyak aplikasi web modern |
| `enable-smooth-scrolling` | aktif | Perilaku gulir yang diharapkan di Linux |
| `enable-media-stream`, `enable-webrtc` | aktif | Panggilan video dan kamera |
| `enable-developer-extras` | hanya build debug | Devtools berguna saat uji, tidak perlu di rilis |
| `background-color` | gelap netral | Mengurangi kedipan putih saat halaman baru dimuat |

Render perangkat lunak tetap bisa dipaksa lewat `PERISAI_RENDER_SOFTWARE=1`
(dihormati bersama `LIBGL_ALWAYS_SOFTWARE` dan `WEBKIT_DISABLE_COMPOSITING_MODE`
yang sudah dipakai sesi Xvfb). Sesi uji selalu memakai jalur perangkat lunak,
jadi **akselerasi GPU yang sebenarnya belum diukur di mesin ini** — tidak ada
klaim kecepatan apa pun.

## 3. Fitur lain yang masuk

| Fitur | Ringkasan |
| --- | --- |
| Halaman tab baru | `beranda.html` ditulis ke folder cache saat start: pencarian, empat pintasan, ikut tema sistem. Berkas statis, tanpa IPC |
| Bookmark | Tabel `bookmark` (SQLite), tombol bintang di address bar, panel Pustaka, Ctrl+D |
| Riwayat | Tabel `riwayat`, tercatat saat halaman selesai dimuat, bisa dicari dan dihapus semua |
| Unduhan | `WebviewBuilder::on_download`, tujuan `~/Downloads` dengan nama unik, status tercatat di tabel `unduhan`, panel Unduhan |
| Zoom per situs | `set_zoom`, peta host → skala di `pengaturan`, badge persen di chrome, Ctrl+= / Ctrl+- / Ctrl+0 |
| Pemulihan sesi | Daftar tab disimpan tiap perubahan; saat start tanpa argumen, tab terakhir dibuka kembali (maksimal 20) |
| Pintasan keyboard | Di Linux ditangkap di level `GtkWindow` (lihat `platform/linux/pintasan.rs`) supaya tidak perlu menu bar |
| Tema | Sistem / Terang / Gelap, dipilih dari panel Menu |
| Panel samping | Setiap panel memesan ruang di kanan (`atur_panel`), karena child webview selalu berada di atas webview chrome |

### Pintasan yang berlaku (Linux)

`Ctrl+T` tab baru · `Ctrl+Shift+T` buka tab tertutup · `Ctrl+W` tutup tab ·
`Ctrl+Tab` / `Ctrl+Shift+Tab` pindah tab · `Ctrl+L` fokus address bar ·
`Ctrl+R` muat ulang · `Ctrl+B` sidebar · `Ctrl+D` bookmark · `Ctrl+J` unduhan ·
`Ctrl+H` riwayat · `Ctrl+K` menu · `Ctrl+=` / `Ctrl+-` / `Ctrl+0` zoom ·
`Alt+←` / `Alt+→` mundur/maju · `Ctrl+Q` keluar.

## 4. Uji dan hasil

Perintah dijalankan dari akar repo.

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all --check` | Lolos |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lolos, 0 warning |
| `cargo test --workspace` | **31 lolos** (`adblock-core` 10, `perisai` 15, `storage` 6) |
| `npm --prefix ui run build` | Lolos, tanpa peringatan a11y |
| `scripts/smoke-gui.sh … scripts/uji/tab.sh` | Lolos (3 pemeriksaan piksel) |
| `scripts/smoke-gui.sh … scripts/uji/pintasan.sh` | Lolos (Ctrl+T, Ctrl+W, Ctrl+L, Ctrl+B) |
| `scripts/uji-adblock.sh` | Lolos — lihat di bawah |
| `scripts/uji-fitur.sh` | Lolos — bookmark, riwayat, zoom, sesi terbaca di SQLite |

### Bukti adblock (dengan pembanding)

`scripts/uji-adblock.sh` menjalankan aplikasi dua kali pada halaman yang sama:

- dengan aturan uji `iklan.js` → piksel tengah kotak iklan **putih** (skrip tidak pernah dimuat);
- tanpa aturan uji → piksel **`srgb(255,0,0)`** (kotak merah muncul).

Diulang tujuh kali setelah perbaikan thread: enam lolos, satu sempat gagal karena
navigasi uji tidak termuat. Skrip sekarang mengulang navigasi dan memeriksa latar
halaman (putih) sebelum menilai, jadi kegagalan navigasi tidak lagi terbaca sebagai
"iklan diblokir". Pembanding ini penting: tanpa putaran kedua, uji hanya
membuktikan "kotak tidak muncul", yang bisa juga terjadi kalau halamannya gagal
dimuat sama sekali.

### Angka nyata dari mesin ini

| Ukuran | Nilai |
| --- | --- |
| Aturan content blocker (EasyList + EasyPrivacy + aturan Perisai) | 130.066 |
| Ukuran JSON filter | 14 MB |
| Hasil kompilasi WebKit (`webkit-store/`) | 42 MB |
| Waktu kompilasi di build debug | ~4 detik (di thread latar, jendela tidak menunggu) |

### Bukti visual

Screenshot di `docs/screenshots/`: `tahap3-1-aplikasi.png` (UI baru: sidebar,
address bar pill, badge zoom), `tahap3-2-perlindungan.png` (panel Perlindungan
dengan 130.066 aturan dan toggle per situs), `tahap3-3-pustaka.png` (bookmark),
`tahap3-4-menu.png` (tema, zoom, daftar pintasan), `tahap3-5-tab-baru.png`
(halaman tab baru yang terbuka lewat Ctrl+T, termasuk daftar tab yang ikut
diperbarui).

## 5. Belum terverifikasi / batas

- **Iklan video YouTube belum diuji dengan pemutaran sungguhan.** Yang ada:
  aturan cosmetic untuk permukaan iklan YouTube, blokir endpoint statistik iklan,
  dan scriptlet pelompat iklan video (`SCRIPTLET_YOUTUBE`). Hasilnya
  *best-effort* dan tergantung DOM YouTube; tidak bisa diklaim setara Brave
  sebelum ada uji manual di Arch.
- **Penghitung "iklan diblokir" ala Brave tidak ada** dan tidak dikarang. Itu
  butuh intersepsi per request (WebExtension `send-request` atau pindah CEF).
- **Windows dan macOS belum punya Perlindungan.** `platform/lainnya.rs` masih
  no-op; jalur WebView2 (`WebResourceRequested`) dan `WKContentRuleList` belum
  dibungkus. Pintasan keyboard juga masih Linux saja.
- **Akselerasi GPU belum diukur** (sesi uji selalu perangkat lunak).
- **Unduhan belum diuji end-to-end** dengan berkas sungguhan; yang ada: tujuan,
  pencatatan, dan panel.
- **Drag-reorder tab, spaces, pin tab, command palette, find in page, mode privat**
  belum dikerjakan.
