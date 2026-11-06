# Rilis 0.3.6 — indikator muat, tab aktif di latar, lembar indeks

Tiga keluhan pengguna dijawab di rilis ini:

1. garis progres dan tombol muat ulang tidak pernah selesai, seolah halaman
   terus memuat;
2. browser terasa berhenti saat jendela tidak difokuskan (di-minimize atau
   ditinggal ke workspace lain), termasuk musik yang ikut mati;
3. halaman tab baru diminta punya tampilan dan tipografi sendiri.

## 1. Indikator muat yang tidak pernah selesai

**Penyebab.** Status "sedang memuat" diambil dari event navigasi Tauri
(`on_navigation`). Event itu ikut menyala untuk **iframe**, sedangkan
"selesai" (`PageLoadEvent::Finished`) hanya untuk dokumen utama. Satu iframe
yang lambat — pelacak, pemutar, kolom komentar — sudah cukup membuat tombol
berhenti dan garis progres menyala terus walaupun halaman sudah tergambar
penuh.

**Bukti.** Halaman uji dengan iframe yang baru menjawab setelah 12 detik,
difoto 9 detik setelah dibuka:

- sebelum: toolbar menampilkan tombol **✕ (hentikan)** dan garis progres masih
  menyala, padahal teks halaman sudah tampil;
- sesudah: toolbar menampilkan **⟳ (muat ulang)**, tanpa garis progres.

**Perbaikan.** Status muat sekarang diambil dari properti `is-loading` milik
WebKit (`platform/linux/mod.rs`), yang hanya mengikuti dokumen utama dan padam
juga saat pemuatan dihentikan. `on_navigation` kembali hanya melaporkan URL.
Laporan keadaan awal dikirim sekali saat webview dipasang, karena properti bisa
sudah menyala sebelum pendengar terpasang.

## 2. Tab aktif tetap berjalan saat jendela tidak fokus

**Penyebab.** WebKitGTK 2.46+ meneruskan keadaan jendela ke halaman. Saat
jendela disembunyikan, halaman menerima `visibilitychange` dan
`document.hidden` menjadi true. Situs memakai itu untuk menghentikan pemutaran
— persis mekanisme yang mematikan musik.

Ini terbukti di mesin uji dengan memindahkan tab (jalur yang bisa direproduksi
tanpa window manager): halaman menerima `vischange=hidden` lalu berhenti
mengirim detak.

**Perbaikan.** `src-tauri/src/latar.rs` menambahkan user script yang dipasang
sebelum dokumen dimuat. Skrip itu menimpa `hidden`/`visibilityState` (termasuk
awalan `webkit`) dan menahan event `visibilitychange` **hanya saat halaman
disembunyikan**. Event "terlihat lagi" sengaja dibiarkan lewat: tanpa itu
halaman yang sempat berhenti tidak pernah tahu ia boleh melanjutkan.

Yang dianggap terlihat: tab aktif dan tab yang sedang bersuara (statusnya sudah
dipakai peniduran tab sejak 0.3.5). Tab lain dibiarkan menerima
`visibilitychange` supaya animasinya berhenti dan CPU tidak terpakai.

**Bukti yang bisa diukur.**

| Uji | Hasil |
| --- | --- |
| Skrip terpasang di halaman | `shim=ada` |
| Tab ditinggal | `vischange=hidden`, halaman berhenti (sesuai rancangan) |
| Tab dipilih lagi | `vischange=visible` diterima, timer hidup lagi (detak 10 → 18) |
| Halaman detak, `PERISAI_TETAP_LATAR=0` vs bawaan | keduanya `vis=visible`, tidak ada `vischange` saat jendela di-minimize |

Baris terakhir adalah **batas pengukuran yang harus disebut**: di kontainer uji
(Xvfb + openbox + WebKitGTK 2.52 lewat X11), minimize tidak pernah mengirim
`visibilitychange` sama sekali — baik dengan paksaan menyala maupun mati. Jadi
jalur minimize/workspace **belum terbukti terukur** di sini; yang terbukti
adalah mekanisme penimpaan bekerja (skrip terpasang, event disembunyikan bisa
ditahan, event terlihat tetap lewat) dan jalur sembunyikan-tab berperilaku
benar. Pengguna bisa membandingkan sendiri di mesinnya:

```bash
perisai                        # bawaan: tab aktif tetap dianggap terlihat
PERISAI_TETAP_LATAR=0 perisai  # perilaku lama: halaman tahu ia disembunyikan
```

Batas lain: WebKit tetap menurunkan frekuensi timer halaman yang jendelanya
tidak terlihat, dan compositor tetap menghentikan pengiriman frame. Bunyi tetap
keluar karena pemutaran berjalan di proses WebKit, tetapi animasi CSS akan
tersendat sampai jendela terlihat lagi.

## 3. Halaman tab baru: "lembar indeks"

Rancangan baru di `src-tauri/src/beranda.rs`, dibuat dengan konsep lembar indeks
kertas buku besar:

- latar bergaris rambut tiap 34 px, bukan gradien;
- judul serif besar ("Mau ke mana / hari ini?") dengan baris kedua miring;
- label monospace huruf kecil dengan jarak lebar, seperti keterangan arsip;
- kolom pencarian berupa satu garis bawah, bukan pil;
- tautan bernomor `01`–`04` dengan titik pemandu seperti daftar isi;
- jam dan tanggal Indonesia di kanan atas, diperbarui tiap 15 detik.

Semua fon berasal dari sistem (serif Iowan/Palatino/Georgia, monospace sistem),
tanpa `@import` dan tanpa permintaan jaringan, jadi halaman tetap sama saat
offline dan tidak menambah beban saat tab dibuka.

Halaman kini juga mengikuti **tema aplikasi**, bukan tema desktop: sebelumnya
halaman putih muncul di tengah UI gelap. Temanya ditempel saat berkas ditulis;
memilih tema baru atau mesin pencari baru langsung menulis ulang halaman.

## Verifikasi

- `cargo test -p perisai --lib`: **37 lolos** (tes baru untuk skrip visibilitas,
  tema halaman tab baru, dan sakelar `PERISAI_TETAP_LATAR`).
- `cargo clippy -p perisai --all-targets -- -D warnings`: bersih.
- `cargo fmt`: bersih.
- Uji GUI di Xvfb 1366×768: iframe lambat, pindah tab bolak-balik, minimize
  dengan openbox, tangkapan layar halaman tab baru (tema gelap).
- Kait uji sementara untuk menyembunyikan jendela sudah dihapus sebelum build
  rilis.

## Berkas yang berubah

- `src-tauri/src/latar.rs` (baru) — user script visibilitas + sakelar.
- `src-tauri/src/tabs.rs` — status muat dari `is-loading`, penyelarasan
  visibilitas per tab, pemutusan sebelum `hide`/`show`.
- `src-tauri/src/platform/linux/mod.rs` — pemasangan user script, pendengar
  `is-loading`.
- `src-tauri/src/platform/{mod.rs,lainnya.rs}` — ekspor `pasang_skrip_latar`.
- `src-tauri/src/beranda.rs` — halaman tab baru + tema.
- `src-tauri/src/{lib.rs,commands.rs}` — tema untuk halaman tab baru.
- `README.md` — catatan teknis indikator muat, latar tab aktif, halaman tab baru.
