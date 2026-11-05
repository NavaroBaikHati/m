# Rilis 0.3.2

## YouTube: user agent, bukan codec

Laporan *"Your browser can't play this video."* ternyata cocok dengan bug WebKit
resmi: YouTube memblokir permintaan media (`/videoplayback`) dari browser yang
dikenali sebagai WebKitGTK, dan penyaringannya bergantung pada string user agent
(WebKit bug 289194 — komentar di sana menyebut UA Safari macOS lolos).

Perisai sekarang mengirim UA Safari macOS untuk semua webview:

```
Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15
```

Klaim itu jujur — mesinnya memang WebKit — dan menghapus penanda WebKitGTK/Linux
yang dipakai YouTube untuk memblokir. `PERISAI_USER_AGENT=bawaan` mengembalikan
UA asli untuk diagnosis; nilai lain dipakai apa adanya. Media Source Extensions
juga dinyalakan eksplisit (`set_enable_mediasource`) karena YouTube memutarnya
lewat MSE.

**Bukti di kontainer:** halaman `youtube.com/watch` yang sama dimuat dengan UA
asli menampilkan *"Your browser can't play this video."*; dengan UA Safari pesan
itu hilang dan pemutar memuat halaman video. Verifikasi pemutaran penuh tidak
bisa dilakukan di sini: IP datacenter memicu *"Sign in to confirm that you're not
a bot"* dan kontainer tidak punya sesi desktop/audio. Decoder host tetap perlu
ada: `gst-inspect-1.0 | grep -E "avdec_h264|vp9dec"` (Arch: `gst-plugins-good`,
`gst-libav`).

## Panel melayang akhirnya benar-benar melayang

Kompromi 0.3.x sebelumnya: panel dirender di webview chrome, dan karena webview
tab selalu berada di atas chrome, panel hanya bisa terlihat kalau ruang di kanan
halaman **dipesan** — hasilnya panel terasa seperti sidebar dan halaman
menyempit.

Sekarang panel punya webview sendiri (`panel.html` + `src/panel.ts` +
`Panel.svelte`) di atas halaman:

- Dibuat sekali saat pertama dibuka, diletakkan menempel tepi kanan di bawah
  toolbar (344 px; 376 px untuk pustaka), lalu ditampilkan/disembunyikan.
- `GtkFixed` menumpuk anak sesuai urutan pemasangan dan tidak punya API z-order;
  `platform::naikkan` melepas lalu memasang ulang widget panel setiap kali tab
  baru dibuat, jadi panel tidak pernah tertimpa.
- `rect_konten` tidak lagi mengurangi lebar panel: halaman tetap selebar
  jendela, panel menimpanya.
- Latar webview transparan supaya sudut membulat kartu tidak jadi kotak gelap.
- Panel menerima jenisnya lewat event `panel:jenis`. Event pertama bisa tiba
  sebelum halaman panel selesai dimuat, jadi panel juga menanyakan keadaan
  terakhir lewat perintah `jenis_panel` — tanpa itu panel tampil kosong.
- Menutup: tombol ×, `Esc`, klik pada halaman (handler `button-press-event` di
  webview tab), pindah halaman, pindah tab, atau masuk layar penuh. Setelah
  ditutup fokus kembali ke halaman agar pintasan keyboard tetap jalan.
- Capability `default` sekarang mencakup webview `panel`; sebelumnya IPC ditolak
  (`Command plugin:event|listen not allowed by ACL`).

Chrome hanya menyimpan sorotan tombol panel; daftar bookmark, riwayat, unduhan,
dan setelan dimuat oleh webview panel sendiri.

## Verifikasi

- `cargo test -p perisai --lib` — 26 lolos.
- `cargo clippy -p perisai --all-targets -- -D warnings` — bersih.
- `cargo fmt --all` — bersih.
- `npm --prefix ui run build` — dua entry (`index.html`, `panel.html`).
- Biner rilis di Xvfb: panel Menu dan Unduhan melayang di atas halaman yang tetap
  selebar jendela; klik halaman menutup panel; `Ctrl+J`/`Ctrl+K` berganti panel;
  jendela diperkecil 1300×860 → 900×620 dan panel ikut menempel tepi kanan;
  minimum jendela tetap 640×400; log tanpa panic/CRITICAL.

## Batasan yang diketahui

- Tidak ada compositor di kontainer uji, jadi sudut transparan panel tampil
  sebagai kotak gelap di tangkapan layar. Di Hyprland (Wayland) latar transparan
  bekerja.
- Bayangan kartu tidak bisa digambar di luar batas webview; panel memakai garis
  tepi 1 px dan sudut membulat sebagai gantinya.
- Perilaku pemutaran YouTube yang sebenarnya hanya bisa dipastikan di mesin
  pengguna; kontainer memicu verifikasi bot.

## Artefak

| Berkas | Ukuran |
| --- | --- |
| `Perisai_0.3.2_amd64.AppImage` | ±81 MB |
| `Perisai_0.3.2_amd64.deb` | ±6,3 MB |
| `perisai-0.3.2-linux-x86_64` | ±21 MB |
| `SHA256SUMS` | — |

`.rpm` tidak dibangun pada rilis ini.
