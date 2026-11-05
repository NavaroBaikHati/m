//! Perbaikan lingkungan proses sebelum GTK/WebKit diinisialisasi.
//!
//! Kenapa perlu: AppImage hasil `linuxdeploy` bersifat "mandiri" — ia membawa
//! glib, GTK, WebKit, **dan** tumpukan TLS dari mesin build (Ubuntu): `libgnutls`,
//! `libsoup-3.0`, `libp11-kit`. `AppRun` menaruh folder itu di `LD_LIBRARY_PATH`,
//! jadi pustaka bawaan itulah yang dipakai proses, bukan milik distro pengguna.
//!
//! Masalahnya ada di `libp11-kit` bawaan. Lokasi modul PKCS#11-nya dipatok saat
//! build ke `/usr/lib/x86_64-linux-gnu/pkcs11` (layout Debian), sedangkan AppImage
//! **tidak** ikut membawa `p11-kit-trust.so`. Di distro lain — Arch memakai
//! `/usr/lib/p11-kit-trust.so`, Fedora `/usr/lib64/pkcs11/` — folder Debian itu
//! tidak ada. GnuTLS lalu kehilangan seluruh *trust anchor*, libsoup melaporkan
//! kegagalan verifikasi, dan WebKit menampilkan "Unacceptable TLS certificate"
//! untuk **semua** situs HTTPS. Gejalanya hanya muncul di Perisai karena browser
//! lain memakai pustaka TLS milik distro.
//!
//! Perbaikannya: saat berjalan dari AppImage dan layout bawaan tidak ada, tulis
//! konfigurasi modul p11-kit milik pengguna yang menunjuk modul trust sistem
//! lewat **path absolut** (p11-kit membaca `$XDG_CONFIG_HOME/pkcs11/modules`).
//! Modul milik sistem itu akan dimuat oleh `libp11-kit` bawaan AppImage, jadi
//! CA kembali ditemukan tanpa mengubah pustaka apa pun.
//!
//! Untuk jalur OpenSSL (`SSL_CERT_FILE`/`SSL_CERT_DIR`) lokasi bundel CA juga
//! diarahkan ke milik sistem, karena OpenSSL bawaan AppImage memakai path
//! build-nya sendiri.

use std::path::{Path, PathBuf};

/// User agent bawaan Perisai.
///
/// YouTube memblokir permintaan media dari browser yang dikenali sebagai
/// WebKitGTK: permintaan ke `/videoplayback` ditolak dan pemutar hanya
/// menampilkan "Your browser can't play this video." — bukan karena codec
/// hilang. Tim WebKit mencatat penyaringan itu bergantung pada string user
/// agent, dan UA Safari macOS lolos (WebKit bug 289194). Mesin Perisai memang
/// WebKit, jadi Safari adalah klaim yang paling jujur sekaligus yang diterima.
///
/// `PERISAI_USER_AGENT=bawaan` mengembalikan UA WebKitGTK asli untuk diagnosis,
/// dan nilai lain dipakai apa adanya.
pub const UA_SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";

/// User agent yang dipakai setiap webview; `None` berarti UA WebKitGTK asli.
pub fn user_agent() -> Option<String> {
    match std::env::var("PERISAI_USER_AGENT") {
        Ok(nilai) if nilai == "bawaan" => None,
        Ok(nilai) if !nilai.trim().is_empty() => Some(nilai),
        _ => Some(UA_SAFARI.to_string()),
    }
}

/// Folder modul PKCS#11 yang dipatok `libp11-kit` bawaan AppImage (build Debian).
const FOLDER_PKCS11_BAWAAN: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/pkcs11",
    "/usr/lib/aarch64-linux-gnu/pkcs11",
];

/// Modul trust milik distro pengguna, urut prioritas.
const MODUL_TRUST_SISTEM: &[&str] = &[
    "/usr/lib/p11-kit-trust.so",
    "/usr/lib64/p11-kit-trust.so",
    "/usr/lib/pkcs11/p11-kit-trust.so",
    "/usr/lib64/pkcs11/p11-kit-trust.so",
    "/usr/lib/x86_64-linux-gnu/pkcs11/p11-kit-trust.so",
    "/usr/lib/aarch64-linux-gnu/pkcs11/p11-kit-trust.so",
];

/// Bundel CA milik sistem untuk jalur OpenSSL.
const BUNDEL_CA_SISTEM: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    "/etc/ssl/cert.pem",
];

/// Folder CA sistem (dipakai OpenSSL lewat `SSL_CERT_DIR`).
const FOLDER_CA_SISTEM: &str = "/etc/ssl/certs";

/// Nama berkas konfigurasi modul p11-kit buatan kita.
const BERKAS_MODUL: &str = "perisai-trust.module";

/// Merapikan variabel lingkungan dan trust store. Dipanggil sekali di awal
/// `run()`, sebelum GTK dimulai. Tanpa efek kalau tidak berjalan dari AppImage.
pub fn rapikan() {
    rapikan_trust_appimage();
    rapikan_ca();
    rapikan_renderer();
}

/// Memilih renderer WebKit yang aman saat sesi Wayland dijalankan lewat X11.
///
/// Dua jalan menuju situasi itu: `AppRun` bawaan AppImage (hook
/// `linuxdeploy-plugin-gtk`) memaksa `GDK_BACKEND=x11`, dan pengguna sendiri yang
/// memaksa GTK-X11 di sesi Wayland. Di jalur XWayland, renderer DMA-BUF
/// WebKitGTK sering membuat video tersendat lalu memacetkan jendela — halaman
/// YouTube paling terasa — karena buffer GPU harus menyeberang antara Wayland
/// dan X11, dan pesan `gdk_wayland_window_set_dbus_properties_libgtk_only`
/// muncul di log.
///
/// Karena itu renderer DMA-BUF dimatikan **hanya** untuk kombinasi tersebut.
/// Kalau pengguna sudah mengatur sendiri (atau memakai Wayland asli), nilai
/// mereka dihormati — di Wayland asli DMA-BUF justru jalur tercepat.
fn rapikan_renderer() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() {
        return;
    }
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let backend_x11 =
        std::env::var_os("GDK_BACKEND").as_deref() == Some(std::ffi::OsStr::new("x11"));
    if perlu_mati_dmabuf(wayland, backend_x11) {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        eprintln!(
            "lingkungan: sesi Wayland lewat X11; renderer DMA-BUF dimatikan agar video tidak macet"
        );
        eprintln!("lingkungan: untuk render native, jalankan dengan GDK_BACKEND=wayland");
    }
}

/// Keputusan murni (tanpa efek samping) agar bisa dites.
fn perlu_mati_dmabuf(wayland: bool, backend_x11: bool) -> bool {
    wayland && backend_x11
}

/// Apakah proses ini berjalan dari AppImage? `APPDIR` dipasang `AppRun`;
/// `APPIMAGE` berisi path berkas `.AppImage` aslinya.
fn appimage_aktif() -> bool {
    ["APPDIR", "APPIMAGE"]
        .iter()
        .any(|nama| std::env::var_os(nama).is_some())
}

/// Menyediakan trust anchor untuk tumpukan TLS bawaan AppImage.
fn rapikan_trust_appimage() {
    if !appimage_aktif() {
        return;
    }
    if folder_pkcs11_bawaan_ada() {
        // Build Debian dan host-nya sama; tidak ada yang perlu diperbaiki.
        return;
    }
    let Some(modul) = modul_trust_sistem(MODUL_TRUST_SISTEM) else {
        eprintln!("lingkungan: modul trust p11-kit tidak ditemukan; sertifikat HTTPS bisa ditolak");
        return;
    };
    let Some(folder) = folder_konfigurasi_p11kit() else {
        eprintln!("lingkungan: folder konfigurasi p11-kit tidak bisa ditentukan");
        return;
    };

    let isi = isi_berkas_modul(&modul);
    if tulis_modul(&folder, &isi).unwrap_or(false) {
        eprintln!(
            "lingkungan: CA sistem dipasang untuk tumpukan TLS AppImage lewat {}",
            folder.join(BERKAS_MODUL).display()
        );
    }
}

/// Menulis konfigurasi modul p11-kit; `Ok(true)` kalau berkas benar-benar
/// ditulis, `Ok(false)` kalau isinya sudah sama.
fn tulis_modul(folder: &Path, isi: &str) -> Result<bool, ()> {
    let tujuan = folder.join(BERKAS_MODUL);
    if std::fs::read_to_string(&tujuan).ok().as_deref() == Some(isi) {
        return Ok(false);
    }
    std::fs::create_dir_all(folder).map_err(|_| ())?;
    std::fs::write(&tujuan, isi).map_err(|_| ())?;
    Ok(true)
}

/// Isi berkas modul p11-kit: modul trust sistem dengan path absolut dan tanda
/// bahwa modul ini boleh dipakai sebagai sumber kebijakan kepercayaan.
fn isi_berkas_modul(modul: &str) -> String {
    format!("module: {modul}\ntrust-policy: yes\n")
}

/// Folder `<config>/pkcs11/modules` milik pengguna, mengikuti `XDG_CONFIG_HOME`.
fn folder_konfigurasi_p11kit() -> Option<PathBuf> {
    let dasar = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|jalur| jalur.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(dasar.join("pkcs11").join("modules"))
}

fn folder_pkcs11_bawaan_ada() -> bool {
    FOLDER_PKCS11_BAWAAN
        .iter()
        .any(|jalur| Path::new(jalur).is_dir())
}

/// Modul trust pertama yang benar-benar ada.
fn modul_trust_sistem(kandidat: &[&str]) -> Option<String> {
    kandidat
        .iter()
        .find(|jalur| Path::new(jalur).is_file())
        .map(|jalur| (*jalur).to_string())
}

/// Menunjuk `SSL_CERT_FILE`/`SSL_CERT_DIR` ke bundel sistem kalau nilainya kosong
/// atau menunjuk berkas yang tidak ada (mis. path build AppImage).
fn rapikan_ca() {
    let perlu_berkas = std::env::var_os("SSL_CERT_FILE")
        .map(|nilai| !Path::new(&nilai).is_file())
        .unwrap_or(true);
    if perlu_berkas {
        if let Some(bundel) = bundel_ca_sistem(BUNDEL_CA_SISTEM) {
            std::env::set_var("SSL_CERT_FILE", &bundel);
        }
    }

    let perlu_dir = std::env::var_os("SSL_CERT_DIR")
        .map(|nilai| !Path::new(&nilai).is_dir())
        .unwrap_or(true);
    if perlu_dir && Path::new(FOLDER_CA_SISTEM).is_dir() {
        std::env::set_var("SSL_CERT_DIR", FOLDER_CA_SISTEM);
    }
}

/// Bundel CA sistem pertama yang ada dan tidak kosong.
fn bundel_ca_sistem(kandidat: &[&str]) -> Option<String> {
    kandidat
        .iter()
        .find(|jalur| {
            std::fs::metadata(jalur)
                .map(|meta| meta.is_file() && meta.len() > 0)
                .unwrap_or(false)
        })
        .map(|jalur| (*jalur).to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        bundel_ca_sistem, isi_berkas_modul, modul_trust_sistem, perlu_mati_dmabuf, tulis_modul,
    };

    #[test]
    fn user_agent_bawaan_menyamar_sebagai_safari() {
        let ua = super::UA_SAFARI;
        assert!(ua.contains("Safari/605.1.15"));
        // Jangan sampai UA WebKitGTK/Perisai bocor dan diblokir lagi.
        assert!(!ua.contains("WebKitGTK"));
        assert!(!ua.contains("Tauri"));
        assert!(!ua.contains("Linux"));
    }

    #[test]
    fn dmabuf_hanya_dimatikan_untuk_wayland_lewat_x11() {
        assert!(perlu_mati_dmabuf(true, true));
        // Wayland asli: DMA-BUF justru jalur tercepat, jangan disentuh.
        assert!(!perlu_mati_dmabuf(true, false));
        // Sesi X11 sungguhan, bukan XWayland.
        assert!(!perlu_mati_dmabuf(false, true));
        assert!(!perlu_mati_dmabuf(false, false));
    }

    #[test]
    fn modul_trust_dicari_berdasarkan_keberadaan() {
        let tmp = std::env::temp_dir().join("perisai-trust-uji.so");
        std::fs::write(&tmp, b"").unwrap();
        let jalur = tmp.display().to_string();
        let tidak_ada = format!("{jalur}.tidak-ada");

        assert_eq!(modul_trust_sistem(&[&tidak_ada, &jalur]), Some(jalur));
        assert_eq!(modul_trust_sistem(&[&tidak_ada]), None);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn berkas_modul_memakai_path_absolut_dan_trust_policy() {
        let isi = isi_berkas_modul("/usr/lib/p11-kit-trust.so");
        assert_eq!(
            isi,
            "module: /usr/lib/p11-kit-trust.so\ntrust-policy: yes\n"
        );
        // Harus absolut: justru soname relatif inilah yang gagal di Arch.
        assert!(isi.lines().next().unwrap().contains(": /usr/"));
    }

    #[test]
    fn bundel_ca_dipilih_yang_berisi() {
        let isi = std::env::temp_dir().join("perisai-ca-uji.pem");
        std::fs::write(&isi, b"-----BEGIN CERTIFICATE-----\n").unwrap();
        let kosong = std::env::temp_dir().join("perisai-ca-kosong.pem");
        std::fs::write(&kosong, b"").unwrap();

        let jalur = isi.display().to_string();
        let jalur_kosong = kosong.display().to_string();
        assert_eq!(bundel_ca_sistem(&[&jalur_kosong, &jalur]), Some(jalur));
        assert_eq!(bundel_ca_sistem(&[&jalur_kosong]), None);

        let _ = std::fs::remove_file(&isi);
        let _ = std::fs::remove_file(&kosong);
    }

    #[test]
    fn modul_ditulis_sekali_dan_tidak_ditimpa() {
        let folder = std::env::temp_dir().join(format!("perisai-p11-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let isi = isi_berkas_modul("/usr/lib/p11-kit-trust.so");

        // Folder belum ada: harus dibuat, lalu berkas ditulis.
        assert_eq!(tulis_modul(&folder, &isi), Ok(true));
        let berkas = folder.join(super::BERKAS_MODUL);
        assert_eq!(std::fs::read_to_string(&berkas).unwrap(), isi);

        // Isi sama: tidak menulis ulang.
        assert_eq!(tulis_modul(&folder, &isi), Ok(false));

        let _ = std::fs::remove_dir_all(&folder);
    }
}
