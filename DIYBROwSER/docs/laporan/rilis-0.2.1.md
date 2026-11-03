# Laporan Rilis 0.2.1 — perbaikan sertifikat HTTPS di AppImage

Tanggal: 3 November 2026
Mesin build: Ubuntu 24.04 headless (codespace), Rust 1.99.1, Node 24.14,
WebKitGTK 2.52.

Rilis ini **hanya** berisi satu perbaikan: situs HTTPS di AppImage menampilkan
"Unacceptable TLS certificate" di distro yang bukan Ubuntu/Debian — terutama
Arch Linux, tempat pengguna menjalankannya.

## 1. Gejala

Membuka situs apa pun (termasuk mesin pencari) di AppImage 0.2.0 pada Arch:

```
Unacceptable TLS certificate
```

Semua HTTPS gagal; HTTP biasa tidak terpengaruh. Browser lain di mesin yang
sama (dan biner telanjang Perisai) normal.

## 2. Penyebab (diperiksa langsung dari dalam AppImage)

AppImage hasil `linuxdeploy` bersifat mandiri: ia membawa glib, GTK, WebKit,
**dan** tumpukan TLS dari mesin build. `AppRun` menaruh `usr/lib` bawaan itu di
`LD_LIBRARY_PATH`, jadi yang dipakai proses adalah pustaka bawaan, bukan milik
distro.

Bukti dari `Perisai_0.2.0_amd64.AppImage` yang diekstrak:

| Pemeriksaan | Hasil |
| --- | --- |
| Pustaka TLS yang dibundel | `libgnutls.so.30`, `libsoup-3.0.so.0`, `libp11-kit.so.0`, `libcrypto.so.3`, `libcurl-gnutls.so.4` |
| Modul trust yang dibundel | **tidak ada** (`find -name '*p11-kit-trust*'` kosong) |
| Folder PKCS#11 yang dipatok `libp11-kit` bawaan | `/usr/lib/x86_64-linux-gnu/pkcs11` (layout Debian) |
| Folder konfigurasi modul yang dibaca `libp11-kit` bawaan | `/etc/pkcs11/modules`, `~/.config/pkcs11/modules`, `$XDG_CONFIG_HOME/pkcs11/modules`, `/usr/share/p11-kit/modules` |

Di Arch, `/usr/lib/x86_64-linux-gnu` tidak ada sama sekali; modul trust Arch ada
di `/usr/lib/p11-kit-trust.so` (Fedora: `/usr/lib64/pkcs11/`). GnuTLS meminta
trust store lewat p11-kit, tidak menemukan modul apa pun, lalu mengembalikan
**nol CA tepercaya**. libsoup melaporkan kegagalan verifikasi dan WebKit
menampilkan halaman "Unacceptable TLS certificate" — untuk semua situs.

Biner telanjang tidak terkena karena sepenuhnya memakai pustaka TLS milik
distro.

## 3. Perbaikan

Modul baru `src-tauri/src/lingkungan.rs`, dipanggil di awal `run()` **sebelum**
GTK/WebKit diinisialisasi:

1. Aktif hanya bila proses berjalan dari AppImage (`APPDIR`/`APPIMAGE` ada).
2. Berhenti tanpa efek bila folder modul PKCS#11 layout Debian ternyata ada
   (artinya host memang Debian/Ubuntu dan semuanya sudah benar).
3. Mencari modul trust sistem pada beberapa lokasi
   (`/usr/lib/p11-kit-trust.so`, `/usr/lib64/...`, `/usr/lib/pkcs11/...`,
   `/usr/lib64/pkcs11/...`, plus layout multiarch).
4. Menulis `~/.config/pkcs11/modules/perisai-trust.module`:

   ```
   module: /usr/lib/p11-kit-trust.so
   trust-policy: yes
   ```

   Path **absolut** inilah kuncinya: soname relatif dicari lewat folder yang
   dipatok saat build, sedangkan path absolut selalu ditemukan. Berkas hanya
   ditulis kalau isinya berbeda, dan bisa dihapus pengguna kapan saja.
5. Selain itu `SSL_CERT_FILE`/`SSL_CERT_DIR` diarahkan ke bundel CA sistem bila
   variabel itu kosong atau menunjuk berkas yang tidak ada — menutup jalur
   OpenSSL (libsoup/WebKit yang ditautkan ke OpenSSL).

Perbaikan untuk 0.2.0 yang sudah terpasang (tanpa menunggu rilis baru) ada di
README, bagian "Sertifikat HTTPS di AppImage".

## 4. Verifikasi yang dijalankan

| Pemeriksaan | Hasil |
| --- | --- |
| `cargo test -p perisai --lib` | **19 lolos** (4 test baru untuk modul `lingkungan`) |
| `cargo build` rilis + bundling AppImage | lihat bagian artefak di bawah |
| Ekstraksi AppImage lalu cek pustaka & path p11-kit | sesuai tabel di §2 |

## 5. Batasan yang harus diketahui

- Mesin build ini Ubuntu, jadi **gejala Arch tidak bisa direproduksi di sini**:
  pada host Debian/Ubuntu folder modul bawaan memang ada dan semuanya normal.
  Yang diverifikasi adalah mekanismenya — bahwa `libp11-kit` bawaan membaca
  `~/.config/pkcs11/modules`, bahwa folder bawaannya path Debian, dan bahwa
  AppImage tidak membawa modul trust.
- Bila pengguna Arch memasang `p11-kit` pada lokasi yang tidak lazim, modul
  trust-nya tidak ditemukan; aplikasi mencetak peringatan ke stderr dan
  menyarankan pemakaian biner telanjang.
- Jalur Windows/macOS tidak tersentuh.
