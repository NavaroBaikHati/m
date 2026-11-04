# Rilis 0.3.0

## Yang berubah

**Layar penuh.** WebKitGTK mengirim `enter-fullscreen`/`leave-fullscreen` tetapi
tidak menyembunyikan chrome aplikasi. Tab sekarang dipindah ke seluruh jendela,
webview chrome disembunyikan, dan jendela ikut `set_fullscreen(true)`; keluar
dari video mengembalikan toolbar, sidebar, dan ukuran tab. Kalau tab yang sedang
layar penuh ditutup, tampilan normal dipulihkan tanpa menunggu sinyal.

**Garis progres muat.** Progres asli dari `estimated-load-progress` dikirim
sebagai event `tab:progres` (perubahan < 1% dilewati). Toolbar menampilkan garis
2 px di tepi bawah; platform tanpa sumber progres memakai garis bergerak.

**Tombol berhenti.** Tombol muat ulang berubah menjadi tombol hentikan selama
halaman dimuat (`window.stop()`).

**Chrome lebih tipis.** `TINGGI_CHROME` 52 → 40 px, address bar 38 → 26 px dan
melebar saat difokuskan, tombol 30 → 26 px, sidebar 232 → 224 px dengan tab
34 px.

**Panel melayang.** Kartu Perlindungan/unduhan/menu 320 px dan pustaka 348 px,
sudut membulat, bayangan, tidak menempel tepi. Panel menutup sendiri saat
pindah halaman atau menekan `Esc`.

**Palet monokrom.** Semua aksen ungu/oranye dihapus dari UI, halaman tab baru,
avatar tab, dan logo; tema bawaan sekarang gelap. Tombol dan sakelar memakai
hitam/putih.

**Optimasi kecil.** Status halaman tidak lagi diminta untuk setiap event
navigasi (debounce 120 ms), sehingga query bookmark dan pencarian selector
cosmetic tidak berulang dalam satu gelombang muat.

## Verifikasi

- `cargo test -p perisai --lib` — **25 lolos**.
- `cargo clippy -p perisai --all-targets -- -D warnings` — bersih.
- `cargo fmt --all --check` — lolos.
- `npm --prefix ui run build` — lolos (66,69 kB JS, 15,67 kB CSS), tanpa
  peringatan selector CSS tak terpakai.
- Build rilis `cargo tauri build --bundles deb,appimage` — `EXIT=0`,
  AppImage 80,96 MiB, `.deb` 6,30 MiB. Dibangun ulang setelah perbaikan
  "pindah tab saat layar penuh".
- Biner rilis dijalankan di Xvfb 1280×800 dengan `HOME` bersih selama 30 detik:
  proses tetap hidup sampai timeout, tidak ada panic, basis data dan cache
  WebKit terbentuk.

## Yang belum terverifikasi

- Layar penuh HTML5 belum diuji dengan klik nyata di desktop; jalur sinyal,
  penyembunyian chrome, dan `set_fullscreen` baru diverifikasi lewat kompilasi
  dan start bersih. Di sesi Xvfb tanpa window manager, fullscreen jendela tidak
  bisa diamati.
- Tampilan baru belum ditinjau lewat tangkapan layar.
- `.rpm` tidak dibangun pada rilis ini (ruang disk workspace terbatas); AppImage
  adalah jalur pemasangan yang didokumentasikan untuk Arch.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.3.0_amd64.AppImage` | 81 MB |
| `Perisai_0.3.0_amd64.deb` | 6,3 MB |
| `perisai-0.3.0-linux-x86_64` | 21 MB |
| `SHA256SUMS` | — |
