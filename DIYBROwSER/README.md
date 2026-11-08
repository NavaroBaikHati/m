# Perisai

Browser desktop lintas OS: UI ringkas ala Arc + adblock bawaan ala Brave, dibangun
dengan Rust (Tauri v2) dan Svelte. Tanpa telemetri; semua data tinggal di lokal.

Nama produk **Perisai** (pelindung) dipakai sejak 1 Nov 2026; crate Rust-nya
bernama `perisai`, biner hasil build `target/*/perisai`, dan paket installer
`.deb`/`.rpm`/AppImage memakai nama `Perisai`/`perisai`.

PRD lengkap ada di [PRD.md](PRD.md).

## Status

| Tahap | Isi                                                              | Status    |
| ----- | ---------------------------------------------------------------- | --------- |
| 1     | Workspace, shell Tauri, satu tab, address bar, back/forward/reload | Selesai   |
| 2     | Banyak tab + sidebar vertikal                                     | Selesai   |
| 3     | Adblock (EasyList + EasyPrivacy), Perlindungan per situs, UI baru, bookmark, riwayat, unduhan, zoom, sesi, pintasan | Selesai (Linux) |
| 4     | Mode privat, find in page, spaces, pin tab, command palette       | Belum     |
| 5     | Penghitung iklan per request, jalur Windows/macOS                 | Belum     |

Versi 0.3.9. Catatan verifikasi tiap tahap ada di `docs/laporan/`, termasuk
[laporan rilis 0.1.0](docs/laporan/rilis-0.1.0.md),
[laporan Tahap 3](docs/laporan/tahap-3.md),
[laporan rilis 0.2.0](docs/laporan/rilis-0.2.0.md), dan
[laporan rilis 0.2.1](docs/laporan/rilis-0.2.1.md),
[laporan rilis 0.2.2](docs/laporan/rilis-0.2.2.md),
[laporan rilis 0.3.6](docs/laporan/rilis-0.3.6.md),
[laporan rilis 0.3.7](docs/laporan/rilis-0.3.7.md),
[laporan rilis 0.3.8](docs/laporan/rilis-0.3.8.md), dan
[laporan rilis 0.3.9](docs/laporan/rilis-0.3.9.md).

## Fitur

- **Perlindungan Perisai** — EasyList + EasyPrivacy dikompilasi menjadi content
  blocker WebKit (130.066 aturan), dipasang sebelum halaman dimuat, plus
  scriptlet khusus YouTube dan aturan tambahan Perisai. Bisa dimatikan per situs
  lewat panel di address bar.
- **Tab manager vertikal** dengan monogram situs, indikator memuat, dan sidebar
  yang bisa diciutkan.
- **Address bar tipis ala Arc** yang menampilkan host tebal + jalur samar saat
  tidak difokuskan, melebar saat diklik, dengan tombol Perlindungan dan
  bookmark.
- **Garis progres muat** di bawah toolbar, memakai
  `estimated-load-progress` WebKitGTK (bukan animasi karangan), plus tombol
  berhenti saat halaman masih dimuat.
- **Layar penuh sungguhan** untuk video HTML5: toolbar dan sidebar disembunyikan,
  isi tab memenuhi jendela, dan jendela ikut penuh layar.
- **Panel melayang sungguhan** (Perlindungan, bookmark/riwayat, unduhan, menu):
  kartu dirender di webview sendiri di atas halaman, jadi halaman tetap selebar
  jendela dan tidak menyempit. Panel menutup saat halaman diklik, `Esc`,
  pindah halaman, atau pindah tab. Lihat “Panel melayang” di catatan teknis.
- **User agent Safari** agar YouTube tidak memblokir permintaan medianya (lihat
  “Video YouTube” di catatan teknis).
- **Palet monokrom hitam** — tanpa aksen ungu; tema bawaan gelap.
- **Halaman tab baru** dengan pencarian dan pintasan situs favorit.
- **Bookmark, riwayat, unduhan, zoom per situs, pemulihan sesi, tema
  sistem/terang/gelap.**
- **Pintasan keyboard** tanpa menu bar (lihat tabel di bawah).

### Pintasan (Linux)

`Ctrl+T` tab baru · `Ctrl+Shift+T` buka tab tertutup · `Ctrl+W` tutup tab ·
`Ctrl+Tab` pindah tab · `Ctrl+L` fokus address bar · `Ctrl+R` muat ulang ·
`Ctrl+B` sidebar · `Ctrl+D` bookmark · `Ctrl+J` unduhan · `Ctrl+H` riwayat ·
`Ctrl+K` menu · `Ctrl+=` / `Ctrl+-` / `Ctrl+0` zoom · `Alt+←` / `Alt+→`
mundur/maju · `Ctrl+Q` keluar.

## Paket terpasang (rilis 0.3.0)

Hasil `cargo tauri build` ada di `rilis/` (tidak di-commit): AppImage, `.deb`,
biner telanjang, dan `SHA256SUMS`.

**Arch Linux** — pakai AppImage, karena `.deb`/`.rpm` bukan format pacman:

```bash
chmod +x Perisai_0.3.0_amd64.AppImage
./Perisai_0.3.0_amd64.AppImage          # tambahkan fuse2 kalau diminta
```

Biner telanjang juga bisa langsung dijalankan (`./perisai-0.3.0-linux-x86_64`)
selama `webkit2gtk-4.1` dan `gtk3` terpasang.

**Debian/Ubuntu:** `sudo apt install ./Perisai_0.3.0_amd64.deb`

Setelah terpasang, Perisai muncul di menu aplikasi (`Name=Perisai`) atau bisa
dijalankan dengan perintah `perisai`.

## Struktur repo

```
crates/adblock-core/   Mesin adblock (adblock-rust) + content blocker + scriptlet
crates/storage/        SQLite: pengaturan, riwayat, bookmark, unduhan
src-tauri/             Aplikasi Tauri (crate `perisai`): tab manager, IPC, penyesuaian platform
ui/                    Frontend Svelte + TypeScript (chrome browser)
scripts/               Uji GUI tanpa layar (Xvfb) + skenario navigasi
docs/screenshots/      Bukti screenshot hasil uji
Dockerfile             Build/uji di lingkungan Arch Linux
```

## Dependensi sistem

Rust stable (rustup) dan Node.js 20+ diperlukan di semua platform.

**Ubuntu/Debian** (dipakai saat pengembangan):

```bash
sudo apt install build-essential curl wget file libwebkit2gtk-4.1-dev libgtk-3-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
# tambahan untuk uji GUI tanpa layar:
sudo apt install xvfb imagemagick xdotool
```

**Arch Linux** (platform uji):

```bash
sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 libsoup3 gtk3 \
  librsvg openssl libayatana-appindicator xdotool xorg-server-xvfb imagemagick python
```

## Menjalankan

```bash
npm --prefix ui install
cargo tauri dev
```

`cargo tauri dev` butuh Tauri CLI (`cargo install tauri-cli --locked`). Alternatif
tanpa memasang CLI Rust: `npm --prefix ui run tauri dev`.

Argumen pertama yang bukan flag diperlakukan sebagai alamat awal, misalnya
`cargo tauri dev -- https://example.com/`. Tanpa argumen, aplikasi membuka
halaman tab baru Perisai dan memulihkan sesi terakhir kalau ada.

Build rilis beserta paketnya:

```bash
cargo tauri build --bundles deb,rpm,appimage   # butuh rpmbuild untuk target rpm
```

Hasilnya di `target/release/bundle/`. `target/release/perisai` adalah binernya
sendiri (UI sudah tertanam, jadi tidak butuh Vite seperti build debug).

## Uji

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Smoke test GUI di server tanpa layar + screenshot ke docs/screenshots/
bash scripts/smoke-gui.sh

# Uji interaksi: ketik alamat, mundur, maju, muat ulang, situs luar
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap1.png scripts/uji/navigasi.sh

# Uji tab: tab baru, pindah tab, tutup tab, ciutkan/buka sidebar
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap2.png scripts/uji/tab.sh

# Uji pintasan keyboard (Ctrl+T/W/L/B)
bash scripts/smoke-gui.sh http://127.0.0.1:8123/ docs/screenshots/tahap3-pintasan.png scripts/uji/pintasan.sh

# Adblock: dua putaran, dengan pembanding "tanpa aturan uji"
bash scripts/uji-adblock.sh

# Bookmark, riwayat, zoom, dan sesi: diperiksa langsung di SQLite
bash scripts/uji-fitur.sh
```

Skrip smoke menyalakan server dev UI (Vite) dan server halaman uji sendiri, lalu
menjalankan aplikasi di dalam Xvfb. Build debug memang memuat UI dari `devUrl`,
jadi Vite harus hidup saat aplikasi debug dijalankan.

Docker (lingkungan Arch):

```bash
docker build -t perisai-arch .
docker run --rm perisai-arch cargo test --workspace
```

## Catatan teknis

### Multi-webview di Linux

Setiap tab adalah child webview terpisah (fitur `unstable` Tauri), bukan iframe,
karena banyak situs melarang dirinya di-iframe.

Sampai Tauri 2.12.5, `tauri-runtime-wry` menambahkan child webview ke `GtkBox`
bawaan window (`build_gtk(window.default_vbox())`). `GtkBox` menyusun anaknya
secara vertikal, jadi semua webview berbagi tinggi window dan `set_bounds` tidak
memindahkan apa pun (tauri-apps/tauri#10420, masih terbuka). Karena itu
`src-tauri/src/platform/linux.rs` menyisipkan `GtkFixed` sendiri ke dalam
`GtkBox` tersebut dan memindahkan seluruh webview ke dalamnya — wadah yang memang
didukung wry untuk penempatan absolut. Di Windows dan macOS tidak ada masalah ini,
jadi `src-tauri/src/platform/lainnya.rs` langsung memakai `Webview::set_bounds`.

Modul Linux itu bisa dihapus begitu tao/wry menambahkan dukungan `GtkFixed` bawaan.
Karena `with_webview` bergantung pada tipe webkit2gtk/gtk, versi Tauri dipatok ke
minor (mis. `2.12`).

### Izin IPC

Capability `default` hanya berlaku untuk webview chrome (label `main`). Webview
tab yang memuat situs web tidak punya akses IPC sama sekali, dan event dari Rust
ditargetkan khusus ke label `main`.

### Cara Perlindungan bekerja

Mesin adblock (`crates/adblock-core`) mengompilasi EasyList, EasyPrivacy, dan
aturan tambahan Perisai menjadi **content blocker JSON** (format Safari, dipakai
WebKitGTK). `src-tauri/src/platform/linux/filter.rs` menyimpannya lewat
`WebKitUserContentFilterStore` — binding Rust-nya tidak membungkus tipe filter,
jadi modul itu memakai FFI — lalu memasangnya ke `WebKitUserContentManager`
setiap webview. WebKit yang mencocokkan request, jadi tidak ada biaya
per-request di sisi Rust.

Konsekuensi yang perlu diketahui: WebKit tidak melaporkan request yang diblokir,
sehingga panel Perlindungan menampilkan jumlah aturan aktif dan selector khusus
situs, bukan angka iklan yang diblokir. Jalur Windows (`WebResourceRequested`)
dan macOS (`WKContentRuleList`) belum dikerjakan.

Filter hasil kompilasi di-cache di `~/.cache/dev.perisai.app/adblock/`.
Daftar diunduh ulang bila lebih tua dari 24 jam; kalau unduhan gagal, cache lama
tetap dipakai. Cache hanya dipakai ulang kalau sidiknya cocok dengan daftar yang
ada (`filter.sig`), supaya filter basi tidak sempat dipasang.

### Sertifikat HTTPS di AppImage (rilis ≤ 0.2.0)

AppImage membawa tumpukan TLS dari mesin build (Ubuntu): `libgnutls`,
`libsoup-3.0`, dan `libp11-kit`. `libp11-kit` itu mematok folder modul PKCS#11
ke `/usr/lib/x86_64-linux-gnu/pkcs11`, dan AppImage **tidak** membawa
`p11-kit-trust.so`. Di distro yang tidak memakai layout Debian — Arch memakai
`/usr/lib/p11-kit-trust.so`, Fedora `/usr/lib64/pkcs11/` — GnuTLS tidak
menemukan satu pun CA, sehingga **semua** situs HTTPS gagal dengan halaman
"Unacceptable TLS certificate". Biner telanjang tidak terkena karena memakai
pustaka TLS milik distro.

Sejak 0.2.1, `src-tauri/src/lingkungan.rs` mendeteksi keadaan itu saat start
(hanya bila berjalan dari AppImage dan folder modul Debian tidak ada) lalu
menulis `~/.config/pkcs11/modules/perisai-trust.module` yang menunjuk modul
trust sistem lewat path absolut. Berkas itu bisa dihapus kapan saja.

### Mesin pencari (rilis ≥ 0.2.2)

Mesin pencari bisa diganti di menu (☰ → Mesin pencari): **Google** (bawaan),
Bing, DDG, dan Brave. Pilihan disimpan di basis data dan dipakai oleh address
bar maupun kotak pencarian di halaman tab baru.

Bawaan diganti dari DuckDuckGo ke Google karena di sebagian jaringan
DuckDuckGo diblokir atau diintersepsi penyedia internet. Intersepsi itu tampil
sebagai halaman "Unacceptable TLS certificate" **hanya** saat mencari, padahal
situs HTTPS lain terbuka normal. Kalau satu mesin terkena, pilih mesin lain —
browser tidak perlu diubah.

Untuk 0.2.0 yang sudah terpasang, perbaikannya bisa dilakukan manual tanpa
menunggu rilis baru:

```bash
mkdir -p ~/.config/pkcs11/modules
modul=$(ls /usr/lib/p11-kit-trust.so /usr/lib64/p11-kit-trust.so \
             /usr/lib/pkcs11/p11-kit-trust.so 2>/dev/null | head -1)
printf 'module: %s\ntrust-policy: yes\n' "$modul" \
  > ~/.config/pkcs11/modules/perisai-trust.module
```

### AppImage membawa WebKit dan GStreamer sendiri

AppImage bersifat mandiri: `linuxdeploy` ikut menyalin `libwebkit2gtk-4.1`
(±95 MB), GTK, dan GStreamer **1.24.2** dari mesin build ke dalam paket, lalu
`AppRun` menaruh folder itu paling depan di `LD_LIBRARY_PATH`. Pustaka itulah
yang dipakai, bukan milik distro.

Sampai 0.3.2 ada lubang di situ: pustaka GStreamer ikut dibundel, **plugin**-nya
tidak. WebKit lalu memindai `/usr/lib/gstreamer-1.0` milik host, menemukan plugin
yang dibangun untuk GStreamer 1.26 (Arch), gagal memuatnya ke inti 1.24, dan
melaporkan `GStreamer element appsink not found`. Akibatnya
`MediaSource.isTypeSupported()` menjawab `false` untuk h264, vp9, av1, opus, dan
aac sekaligus — persis pesan *"Your browser can't play this video"* dari
YouTube, berapa pun user agent yang dikirim. Ini juga sebabnya 0.3.2 tidak
mempan.

Sejak 0.3.3 `bundle.linux.appimage.bundleMediaFramework` dinyalakan, jadi
`linuxdeploy-plugin-gstreamer` menyalin plugin dan helper (`gst-plugin-scanner`)
ke dalam AppImage dan `AppRun` mengarahkan `GST_PLUGIN_SYSTEM_PATH_1_0` ke sana.
Konsekuensinya paket membesar dari ±81 MB menjadi ±160 MB — itu harga media yang
benar-benar bisa diputar tanpa bergantung pada pustaka distro.

Verifikasi di mesin sendiri, tanpa menebak:

```bash
perisai --diagnostik          # atau: Perisai_0.3.5_amd64.AppImage --diagnostik
```

Halaman itu menampilkan user agent yang dikirim dan tabel
`MediaSource.isTypeSupported` + `canPlayType` dari WebKit yang sedang berjalan.
Kalau h264/vp9/av1 `true`, masalah pemutaran bukan di codec. EME/Widevine tetap
`false` — video ber-DRM tidak bisa diputar, dan itu batas WebKitGTK, bukan
setelan.

Paket `.deb` dan biner telanjang memakai WebKit/GStreamer distro; di Arch dengan
`gst-plugins-good` dan `gst-libav` terpasang, keduanya sudah mendukung codec di
atas. Cek decoder host dengan `gst-inspect-1.0 | grep -E "avdec_h264|vp9dec"`.

### Kalau halaman terasa lambat

Lambat bisa berarti tiga hal yang berbeda, dan obatnya berbeda pula: proses
mulai lambat, halaman butuh lama untuk dimuat, atau menggulir/merender berat.
Daripada menebak, ukur:

```bash
perisai --diagnostik
```

Halaman itu menggulir teks satu layar penuh selama 5 detik lalu menampilkan
berapa fps yang benar-benar sampai ke layar, isi `/dev/dri`, WebKit mana yang
dipakai (sistem atau bundel AppImage), dan setelan render yang aktif.

Patokan: menggulir teks ≥ 45 fps berarti jalur render sehat; dua digit bawah
berarti beban raster, bukan jaringan.

Hasil pengukuran di mesin uji (Xvfb 1366×768, kontainer tanpa GPU, beban gulir
teks satu layar penuh). Versi 0.3.4 dulu menguji perbedaan paket:

| Yang dijalankan | fps |
| --- | --- |
| biner `perisai-0.3.4-linux-x86_64` (WebKit distro) | 16–17 |
| `Perisai_0.3.4_amd64.AppImage` (WebKit bundel) | 0 |
| AppImage + `WEBKIT_DISABLE_DMABUF_RENDERER=1` | 0 |
| AppImage + `WEBKIT_DISABLE_COMPOSITING_MODE=1` | 0 |

Content blocker bukan penyebabnya: 250 request CSS selesai 9 ms dengan
perlindungan dan 10 ms tanpanya. Yang membedakan hanya WebKit mana yang dipakai —
build Ubuntu yang dibundel AppImage jauh lebih lambat meraster halaman daripada
WebKit milik distro.

**Karena itu, kalau kecepatan penting: pakai biner telanjang atau `.deb`, bukan
AppImage.** Di Arch, biner butuh `webkit2gtk-4.1`, `gst-plugins-good`, dan
`gst-libav`; setelah itu jalankan langsung tanpa pembungkus.

Setelan akselerasi juga bisa lebih lambat daripada tidak dipakai. Di mesin uji
yang tidak punya GPU (3–4 kali jalan per setelan, versi 0.3.5). Tabel ini dari
halaman uji gulir yang polos, jadi angkanya lebih tinggi daripada yang tampil di
`--diagnostik` — di sana beban yang sama ditambah panel dan header lengket:

| Setelan | fps |
| --- | --- |
| bawaan (`OnDemand`) | 26–46 |
| `PERISAI_AKSELERASI=perangkat-lunak` | 44–57 |
| `PERISAI_AKSELERASI=selalu` | 42–47 |

Bedanya masuk akal: tanpa GPU, `OnDemand` tetap mencoba jalur GL lewat llvmpipe
(OpenGL di atas CPU) lalu menambah compositing di atasnya, sedangkan
`perangkat-lunak` melukis langsung. Di mesin yang GPU-nya benar-benar dipakai,
urutan ini bisa terbalik — karena itu ukur sendiri, jangan ikut tabel ini.

Kalau ingin membandingkan di mesin sendiri:

```bash
./perisai-0.3.5-linux-x86_64 --diagnostik                # WebKit distro
Perisai_0.3.5_amd64.AppImage --diagnostik               # WebKit bundel
WEBKIT_DISABLE_DMABUF_RENDERER=1 perisai --diagnostik   # renderer SHM
PERISAI_AKSELERASI=perangkat-lunak perisai --diagnostik # paksa CPU
PERISAI_AKSELERASI=selalu perisai --diagnostik          # paksa GPU
```

Kalau `perangkat GPU` kosong, tidak ada akselerasi sama sekali dan semua cat di
CPU. Kalau angka biner dan AppImage sama-sama ≥ 45 fps, kemacetan bukan di render
melainkan di jaringan atau di halaman itu sendiri.

### Memori: tab lama ditidurkan

WebKitGTK memakai satu proses untuk setiap halaman, dan setiap proses itu bisa
memakan 80–250 MB. Di mesin 1–2 GB, beberapa tab saja sudah cukup membuat kernel
menukar halaman ke disk; setelah itu **semua** terasa lambat, bukan hanya tab
baru. Karena itu, saat memori tersedia menipis, Perisai melepas webview tab yang
tidak aktif (proses WebKit-nya ikut berhenti) dan menyimpan URL-nya. Tab itu
tetap ada di sidebar; memilihnya memuat ulang halaman.

Pengukuran di mesin uji, tiga tab terbuka (RSS seluruh pohon proses):

| Keadaan | tanpa tidur | dengan tidur |
| --- | --- | --- |
| 3 tab terbuka | 1487–1530 MB | 1019 MB |
| setelah berpindah tab | 1393–1413 MB | 842 MB |

Bawaannya tab ditidurkan kalau `MemAvailable` di bawah 300 MB, dibaca dari
`/proc/meminfo` — `MemFree` tidak dipakai karena tidak menghitung cache yang bisa
dibuang kernel. Tab yang sedang memutar suara (musik dari tab lain) tidak pernah
ditidurkan, dan yang paling lama tidak disentuh ditidurkan lebih dulu.

```bash
PERISAI_TIDUR_MB=500 perisai   # lebih agresif
PERISAI_TIDUR_MB=0 perisai     # matikan peniduran
```

Angka tersedia dan ambangnya juga tampil di `perisai --diagnostik`, baris
`memori`.

### Kalau jendela membeku

"Freeze" yang membuat **seluruh** jendela berhenti menjawab klik berbeda dari
halaman yang berhenti: yang pertama berarti main thread GTK/WebKit tertahan.
Sejak 0.3.7 ada pengukur yang menanyakan main thread tiap detik
(`src-tauri/src/macet.rs`); kalau jawabannya datang ≥ 2 detik kemudian, kejadian
itu dicatat ke stderr dan ke berkas:

```bash
cat ~/.local/share/dev.perisai.app/logs/macet.log   # "selama 3204 ms"
```

Sejak 0.3.8 baris itu menyebut pekerjaan yang sedang berjalan, dan sejak 0.3.9
keadaan memori serta waktu sejak aplikasi dibuka, misalnya:

```
> 2000 ms (bangun-webview) [tersedia 210 MB, swap 640 MB, t=182 s]
```

Label yang mungkin muncul: `pasang-filter`, `buka-tab`, `bangun-webview`,
`pilih-tab`, `tutup-tab`, `tidurkan-tab`. Kolom `swap` yang besar (ratusan MB)
berarti macetnya karena mesin menukar ke disk, bukan karena satu pekerjaan
di kode — di mesin 1,7 GB letaknya di jumlah tab, bukan di fungsi tertentu.

Arti angkanya:

- **Ada baris `macet:`** — main thread memang tertahan, dan lamanya menunjukkan
  beratnya. Sejak 0.3.7 penulisan riwayat/sesi dan peniduran tab sudah pindah
  keluar dari main thread, dan sejak 0.3.8 kompilasi content blocker juga;
  kalau masih muncul, keterangan dalam kurung menunjukkan pekerjaan berikutnya
  yang perlu dipindah.
- **Berkasnya kosong padahal jendela membeku** — main thread sehat, jadi
  macetnya di compositor atau di proses WebKit, bukan di kode aplikasi. Coba
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 perisai` (jalur SHM) atau
  `GDK_BACKEND=x11 perisai`, lalu bandingkan.

Kompilasi filter iklan (~14 MB, 131 ribu aturan) adalah penyebab freeze yang
terukur: WebKit mengompilasinya di utas pemanggil meski API-nya bernama
asinkron, dan dulu itu terjadi di main thread. Sejak 0.3.8 seluruh pekerjaan itu
berjalan di thread pekerja; pemasangan ke webview tetap di main thread lewat
callback-nya.

Pelepasan webview saat menidurkan tab dijadwalkan 1,5 detik setelah halaman
tenang, bukan di tengah pembukaan tab, dan hanya satu tab per putaran: melepas
webview mematikan proses WebKit dan itu dikerjakan di main thread. Untuk
mematikan peniduran sama sekali:

```bash
PERISAI_TIDUR_MB=0 perisai
```

### Panel melayang

Panel tidak bisa dirender di webview chrome: webview tab selalu berada di atas
chrome, jadi kartu apa pun di sana akan tertutup halaman. Menyisakan ruang di
kanan halaman (kompromi lama) membuat panel terasa seperti sidebar dan halaman
menyempit.

Sekarang panel punya webview sendiri (`panel.html`, label `panel`) yang dibuat
sekali, diposisikan menempel tepi kanan di bawah toolbar, dan dinaikkan ke atas
semua tab. `GtkFixed` menumpuk anak sesuai urutan pemasangan dan tidak punya API
z-order, jadi `platform::naikkan` melepas lalu memasang ulang widget panel tiap
kali tab baru dibuat. Latar webview transparan agar sudut kartu tidak menjadi
kotak gelap.

Panel menerima jenisnya lewat event `panel:jenis`; karena event pertama bisa
tiba sebelum halaman panel selesai dimuat, ia juga menanyakan keadaan terakhir
lewat perintah `jenis_panel`.

### Video YouTube

YouTube memblokir permintaan media dari browser yang dikenali sebagai
WebKitGTK: pemutar menampilkan *"Your browser can't play this video."* walau
decoder ada. Tim WebKit mencatat penyaringan itu bergantung pada string user
agent (WebKit bug 289194). Perisai mengirim UA Safari macOS — mesin yang sama,
klaim yang paling jujur, dan yang diterima YouTube. `PERISAI_USER_AGENT=bawaan`
mengembalikan UA WebKitGTK asli untuk diagnosis; nilai lain dipakai apa adanya.

UA saja tidak cukup kalau codec-nya tidak ada: pada 0.3.2 AppImage mengirim UA
Safari dengan benar tetapi semua codec `false` karena plugin GStreamer tidak ikut
dibundel (lihat “AppImage membawa WebKit dan GStreamer sendiri”). Jalankan
`perisai --diagnostik` untuk melihat keduanya sekaligus — UA dan dukungan codec.

Kalau video tetap gagal, periksa decoder host: `gst-inspect-1.0 | grep -E
"avdec_h264|vp9dec"`. Di Arch, `gst-plugins-good` (VP9) dan `gst-libav`
(H.264) menyediakan keduanya.

### Akselerasi perangkat keras

Sejak 0.2.3 kebijakannya `OnDemand` (bawaan WebKitGTK): GPU dipakai saat halaman
memang membutuhkannya, dan WebKit tetap bisa mundur ke compositing perangkat
lunak kalau driver bermasalah. Sebelumnya aplikasi memaksa `Always`, dan pada
driver yang tidak stabil itu membuat halaman berat berhenti menggambar lalu
macet. WebGL, smooth scrolling, dan media stream tetap aktif.

Pilihan manual lewat satu variabel lingkungan:

```bash
PERISAI_AKSELERASI=selalu ./perisai            # paksa GPU
PERISAI_AKSELERASI=perangkat-lunak ./perisai  # paksa render CPU
```

`PERISAI_RENDER_SOFTWARE=1` tetap bekerja sebagai alias render perangkat lunak.
Sesi uji repo ini berjalan dengan render perangkat lunak, jadi performa GPU
nyata masih belum diukur.

### Video YouTube tersendat atau macet (rilis ≥ 0.2.3)

Gejala: halaman YouTube berat, gulir tersendat, lalu jendela berhenti merespons
atau prosesnya hilang. Penyebab yang sudah ditemukan:

1. **Sesi Wayland lewat XWayland.** Kalau `WAYLAND_DISPLAY` ada dan
   `GDK_BACKEND=x11` (AppImage selalu begitu; kursi X11 sungguhan tidak),
   renderer DMA-BUF WebKitGTK membuat buffer GPU menyeberang antara Wayland dan
   X11; video tersendat lalu macet. Sejak 0.2.3 keadaan itu dideteksi di awal
   start dan `WEBKIT_DISABLE_DMABUF_RENDERER=1` dipasang otomatis. Kalau kamu
   memasang variabel itu sendiri, nilai kamu yang dipakai.
2. **Kebijakan GPU dipaksa `Always`.** Sudah diganti `OnDemand` (lihat bagian
   sebelumnya).
3. **GPU/driver tidak sanggup jalur GPU.** Coba `PERISAI_AKSELERASI=perangkat-lunak`
   dan `GDK_BACKEND=x11`; kalau jadi lancar, sumbernya driver, bukan halaman.
4. **Bukan render.** Untuk memastikan filter tidak terlibat, jalankan
   `PERISAI_TANPA_ADBLOCK=1 ./perisai`. Beban filter di sisi Rust sudah diukur:
   query cosmetic 10–120 µs per halaman, jadi filter bukan penyebab lag.

Melihat backend yang benar-benar dipakai bisa dengan `GDK_DEBUG=all` (banyak
output) atau cukup membaca baris `lingkungan:` di stderr saat start.

Kalau halaman YouTube memuat normal tetapi pemutarnya menulis *"Your browser
can't play this video"*, itu masalah codec GStreamer di distro, bukan Perisai:
WebKitGTK memakai GStreamer untuk H.264/VP9. Di Arch pasang
`gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav`.
Video tanpa codec akan dipaksa ke jalur perangkat lunak atau gagal sama sekali,
dan itu terasa sebagai lambat.

### Resize, ukuran minimum, dan tiling compositor

Anak `GtkFixed` dialokasikan berdasarkan `size_request`-nya, dan `GtkFixed`
melaporkan ukuran anak terbesar sebagai ukuran minimum jendela. Dulu setiap
webview memakai `size_request` seukuran jendela, jadi **ukuran minimum jendela
selalu sama dengan ukuran jendelanya saat itu**: compositor tiling (Hyprland,
sway, i3) memaksa ukuran baru, GTK menolak, dan isi tab terlihat terpotong.
Sekarang request-nya 1×1 dan ukuran sebenarnya dipasang lewat
`size_allocate` — langsung saat batas berubah, dan sekali lagi dari handler
`size-allocate` (mode *after*) setiap kali GTK selesai menata wadah.

Konsekuensinya jendela bisa dikecilkan sampai batas `minWidth`/`minHeight` di
`tauri.conf.json` (640×400). Kalau suatu saat tata letak terlihat 1×1 atau
menyusut, periksa `PENEMPATAN` dan handler `susun_ulang` di
`src-tauri/src/platform/linux/mod.rs`.

### Setelan WebKit hanya sekali per webview

`terapkan_setelan` dipanggil saat webview dibuat, bukan di setiap perubahan
batas. Menulis ulang kebijakan akselerasi perangkat keras dan warna latar pada
webview yang sedang berjalan memaksa WebKit menyusun ulang compositing — pada
resize cepat, pindah tab, atau saat panel dibuka itu terasa sebagai tersendat.

### Layar penuh dan garis progres

WebKitGTK menyerahkan layar penuh HTML5 ke aplikasi lewat sinyal
`enter-fullscreen`/`leave-fullscreen`; tanpa penanganan, video hanya memenuhi
kotak tab dan toolbar tetap terlihat. `platform/linux/mod.rs` (`pantau`)
menyambungkan kedua sinyal itu ke `TabManager::atur_layar_penuh`: isi tab
memakai seluruh jendela, webview chrome disembunyikan, dan jendela ikut
`set_fullscreen(true)`. Kalau compositor mengabaikan permintaan fullscreen,
bagian pertama tetap bekerja karena batas webview dihitung sendiri.

Progres muat diambil dari properti `estimated-load-progress` dan dikirim lewat
event `tab:progres`. Perubahan di bawah 1% tidak dikirim supaya IPC tidak
dibanjiri ratusan event per halaman. Platform lain belum punya sumber progres,
jadi UI memakai garis bergerak sendiri sebagai cadangan.

Status "sedang memuat" (tombol berhenti dan garis progres) diambil dari properti
`is-loading` milik WebKit, bukan dari event navigasi Tauri. Event navigasi ikut
menyala untuk iframe, sehingga satu iframe lambat (pelacak, pemutar, kolom
komentar) membuat tombol dan garis itu menyala terus seolah halaman belum
selesai — padahal dokumen utama sudah selesai. `is-loading` hanya mengikuti
dokumen utama dan padam juga saat pemuatan dihentikan.

### Tab aktif tetap dianggap terlihat

WebKitGTK 2.46+ meneruskan keadaan jendela ke halaman: saat jendela
di-minimize, berpindah workspace, atau tertutup jendela lain, halaman menerima
`visibilitychange` dan `document.hidden` menjadi true. Situs memakai itu untuk
menghentikan pemutaran, jadi musik berhenti begitu browser tidak difokuskan.

Chrome memilih lain: jendela yang di-minimize tidak menyembunyikan tab yang
sedang aktif. Perisai mengikuti pilihan itu lewat user script yang dipasang
sebelum dokumen dimuat (`src-tauri/src/latar.rs`): `hidden` dan
`visibilityState` ditimpa, dan event `visibilitychange` **yang menyembunyikan**
ditahan. Event "terlihat lagi" tetap diteruskan supaya halaman yang sempat
berhenti bisa melanjutkan.

Yang dianggap terlihat hanya tab aktif dan tab yang sedang bersuara; tab lain
tetap menerima `visibilitychange` dan animasinya berhenti, jadi CPU tidak
terpakai untuk halaman yang tidak dilihat. Kalau lebih suka perilaku lama:

```bash
PERISAI_TETAP_LATAR=0 perisai
```

Batasnya: WebKit tetap menurunkan frekuensi timer halaman yang jendelanya
tidak terlihat, dan compositor tetap menghentikan pengiriman frame. Bunyi tetap
keluar karena pemutaran berjalan di proses WebKit, tetapi animasi CSS yang
berjalan saat jendela disembunyikan akan tersendat sampai jendela terlihat lagi.

### Halaman tab baru

Halaman tab baru adalah berkas HTML statis yang ditulis ulang tiap kali mesin
pencari atau tema berubah (`src-tauri/src/beranda.rs`). Rancangannya "lembar
indeks": kertas bergaris, judul serif besar, label monospace huruf kecil,
tautan bernomor dengan titik pemandu, dan jam di kanan atas. Tanpa fon dari
internet dan tanpa skrip luar, jadi tetap tampil sama saat offline dan tidak
menambah permintaan jaringan saat tab dibuka.

### Tinggi chrome dan sidebar

`TINGGI_CHROME` dan `LEBAR_SIDEBAR` di `src-tauri/src/tabs.rs` harus sama dengan
`--tinggi-chrome` dan `--lebar-sidebar` di `ui/src/app.css`. Saat sidebar
diciutkan, `x` area isi tab menjadi 0 dan lebarnya ditambah `LEBAR_SIDEBAR`.

Hanya tab aktif yang diposisikan dan ditampilkan; tab lain disembunyikan dengan
`Webview::hide`. Karena `atur_batas` di Linux memanggil `widget.show()`, jangan
memanggilnya untuk tab yang tidak aktif.
