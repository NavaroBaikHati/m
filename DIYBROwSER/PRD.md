# PRD: Browser dengan Adblocker ala Brave dan Tampilan ala Arc

Versi 0.1 · Status: Draft · Nama kerja: *(isi nanti)*

## 1. Ringkasan

Browser desktop lintas OS yang cantik dan rapi seperti Arc (sidebar, ruang kerja), dengan perlindungan iklan dan tracker bawaan sekelas Brave Shields. Fitur lainnya sengaja dibuat dasar saja agar proyek realistis dikerjakan sendiri.

## 2. Masalah dan Tujuan

**Masalah:** Browser dengan UI bagus (Arc) tidak punya adblock sekuat Brave, sedangkan Brave tampilannya biasa dan tidak seringan yang diharapkan.

**Tujuan:**

- Browsing bebas iklan dan tracker secara default tanpa ekstensi.
- UI minimalis, vertical tabs, terasa halus.
- Jalan di Windows, macOS, dan Linux dengan satu basis kode.
- Ringan: cepat dibuka, hemat RAM dibanding browser biasa.

**Bukan tujuan (v1):** engine rendering sendiri, ekstensi Chrome, sinkronisasi cloud, versi mobile.

## 3. Target Pengguna

Pengguna harian yang ingin browser bersih dan cepat, terutama pengguna yang terganggu iklan dan suka tampilan rapi. Untuk tahap awal, pengguna utamanya adalah pembuatnya sendiri.

## 4. Fitur

### P0: Wajib (MVP)

- Tab, address bar, back/forward/reload, pencarian dari address bar.
- **Vertical tabs di sidebar**, bisa diciutkan.
- **Adblocker:** blokir request berdasarkan daftar filter (EasyList, EasyPrivacy), cosmetic filtering, tombol shield per situs dengan penghitung yang diblokir, toggle on/off per situs.
- Bookmark dan history.
- Download manager sederhana.
- Mode gelap dan terang mengikuti sistem.

### P1: Penting

- **Ruang kerja (spaces):** kelompok tab dengan warna berbeda, ala Arc.
- Pin tab, command palette (Ctrl/Cmd+K) untuk cari tab dan aksi.
- Scriptlet injection untuk iklan YouTube dan situs yang melawan adblock.
- Upgrade otomatis ke HTTPS, blokir fingerprinting dasar, pemblokiran cookie pihak ketiga.
- Pembaruan otomatis daftar filter.
- Find in page, zoom, print/save PDF, devtools.
- Mode incognito.

### P2: Nanti

- Split view, mini-player picture-in-picture.
- Daftar filter kustom dan aturan buatan pengguna.
- Impor bookmark dari browser lain.
- Tema kustom.
- Sinkronisasi (butuh server).

## 5. Desain dan UX

- Sidebar kiri berisi tab vertikal, ruang kerja di bagian bawah, address bar bisa tersembunyi sampai dipanggil.
- Sudut membulat, transparansi halus, animasi singkat (di bawah 200 ms).
- Satu warna aksen per ruang kerja.
- Halaman tab baru bersih: kolom pencarian dan situs favorit.
- Panel shield menampilkan jumlah iklan dan tracker yang diblokir pada halaman itu.

## 6. Arsitektur dan Teknologi

**Bahasa inti: Rust.** Alasannya: cepat dan hemat memori, aman dari bug memori, satu basis kode untuk semua OS, dan mesin adblock Brave (`adblock-rust`) sudah tersedia sebagai library Rust, jadi bisa dipakai langsung tanpa binding.

**Rendering:** memakai engine yang sudah ada, tidak membuat sendiri. Dua opsi:

|  | Tauri (WebView sistem) | CEF (Chromium Embedded) |
| --- | --- | --- |
| Ukuran dan RAM | Sangat ringan | Berat (puluhan hingga ratusan MB) |
| Konsistensi antar OS | Berbeda per OS (WebView2, WebKit) | Sama di semua OS |
| Kemampuan browser (multi-tab, intercept request) | Terbatas, sebagian fitur masih eksperimental | Lengkap |
| Kesulitan | Sedang | Lebih tinggi |

**Rekomendasi:** mulai dengan Tauri untuk prototipe cepat dan ringan. Jika terbentur batas fitur (intersepsi request, banyak webview, perilaku beda tiap OS), pindah ke CEF. Pisahkan inti adblock sebagai modul tersendiri agar mudah dipindah.

**UI:** web (HTML/CSS/JS dengan framework ringan seperti Svelte) untuk sidebar dan chrome browser, dengan Rust sebagai backend.

**Komponen:**

1. Shell UI (sidebar, tab, address bar)
2. Tab manager (webview per tab)
3. Adblock engine (adblock-rust, pencocokan request, cosmetic filter, scriptlet)
4. Storage lokal (SQLite: bookmark, history, pengaturan)
5. Updater daftar filter

## 7. Persyaratan Non-Fungsional

- Pembukaan aplikasi di bawah 2 detik.
- Pemeriksaan adblock tidak menambah latensi yang terasa (target di bawah 1 ms per request).
- Semua data tersimpan lokal, tanpa telemetri.
- Pembaruan engine rutin demi keamanan.

## 8. Metrik Keberhasilan

- Halaman berat berisi iklan memuat jauh lebih sedikit request dibanding browser tanpa adblock.
- Iklan YouTube tidak muncul pada pengujian rutin.
- RAM idle dengan 10 tab lebih rendah dari Chrome.
- Tidak ada crash dalam pemakaian harian satu minggu.

## 9. Risiko

| Risiko | Dampak | Mitigasi |
| --- | --- | --- |
| Iklan YouTube terus berubah | Adblock bocor | Ambil scriptlet dan filter dari komunitas, update sering |
| WebView sistem beda perilaku | Bug khusus OS | Uji di tiga OS, siap pindah ke CEF |
| Keamanan engine | Celah browser | Ikuti update engine, jangan menunda |
| Ruang lingkup membengkak | Proyek tidak selesai | Kunci P0 dulu, P1 setelah stabil |

## 10. Tahapan

1. **Minggu 1:** shell, tab, address bar, navigasi dasar.
2. **Minggu 2:** adblock request blocking dan shield per situs.
3. **Minggu 3:** sidebar vertikal, bookmark, history, download.
4. **Minggu 4:** cosmetic filter, scriptlet, polesan UI.
5. **Setelah itu:** ruang kerja, command palette, fitur P1 lain.

## 11. Pertanyaan Terbuka

- Nama dan identitas visual browser?
- Mulai dari Tauri atau langsung CEF?
- OS utama untuk pengujian pertama?
