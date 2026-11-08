//! Content blocker WebKitGTK (`WebKitUserContentFilter`).
//!
//! WebKitGTK 2.24+ bisa mencocokkan request sendiri memakai JSON content
//! blocker (format yang sama dengan Safari). Sayangnya binding Rust
//! `webkit2gtk` sengaja tidak membungkus `WebKitUserContentFilter` (tipe
//! `Ignored` di file hasil generate), jadi bagian ini memakai FFI mentah.
//!
//! Alur dua tahap, dan pemisahannya penting:
//!
//! 1. [`siapkan`] — menulis/membandingkan berkas JSON (~14 MB) dan merapikan
//!    cache lama. I/O murni, dijalankan di thread pekerja.
//! 2. [`pasang`] — mengompilasi lewat `WebKitUserContentFilterStore`.
//!
//! Meski API WebKit menamainya asinkron, `save_from_file` mengompilasi seluruh
//! aturan **di utas pemanggil** (terukur ~1.070 ms untuk 131 ribu aturan).
//! Karena itu tahap 2 berjalan di thread pekerja juga; callback-nya tetap
//! dikirim ke main context global, sehingga pemasangan ke
//! `WebKitUserContentManager` (yang memang hanya boleh di thread utama GTK)
//! tetap terjadi di sana. Objek filter dan store disimpan di `thread_local`
//! karena hanya boleh disentuh dari thread utama.

use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};
use std::ptr;

use webkit2gtk::glib::translate::ToGlibPtr;
use webkit2gtk::{ffi, gio, glib, WebView, WebViewExt};

thread_local! {
    /// Filter yang sudah selesai dikompilasi WebKit. `None` berarti belum siap.
    static FILTER: RefCell<Option<*mut ffi::WebKitUserContentFilter>> = const { RefCell::new(None) };
    /// Store harus hidup selama pemuatan; disimpan sekali per proses.
    static STORE: RefCell<Option<*mut ffi::WebKitUserContentFilterStore>> = const { RefCell::new(None) };
}

/// Menyerialkan kompilasi filter yang berjalan di thread pekerja.
static KUNCI: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Permintaan {
    store: *mut ffi::WebKitUserContentFilterStore,
    saat_siap: Option<Box<dyn FnOnce() + Send>>,
}

/// Apakah filter sudah terpasang dan bisa dipakai webview baru.
pub fn siap() -> bool {
    FILTER.with(|slot| slot.borrow().is_some())
}

/// Menyiapkan berkas filter di disk dan mengembalikan path salinannya.
///
/// **Pekerjaan ini sengaja dipisah dari [`pasang`]** karena isinya I/O berat:
/// JSON content blocker berukuran ~14 MB, jadi menulis lalu membandingkannya
/// di thread utama GTK menahan seluruh jendela. Di mesin uji closure lama
/// terukur ~950 ms; di laptop bermemori 1,7 GB dengan disk lebih lambat angka
/// itu melewati dua detik dan terasa sebagai "browser freeze" tepat saat
/// dibuka. Panggil fungsi ini dari thread pekerja, lalu serahkan hasilnya ke
/// [`pasang`] lewat main loop.
///
/// Nama berkas diturunkan dari isi JSON, jadi filter yang sama tidak ditulis
/// ulang dan tidak dikompilasi ulang saat aplikasi dibuka lagi. Salinan lama
/// (masing-masing 14 MB) dibuang di sini.
pub fn siapkan(folder: &Path, json: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("folder filter gagal dibuat: {e}"))?;
    let store_dir = folder.join("webkit-store");
    std::fs::create_dir_all(&store_dir)
        .map_err(|e| format!("folder store filter gagal dibuat: {e}"))?;

    let sidik = sidik_isi(json.as_bytes());
    let sumber = folder.join(format!("perisai-{sidik:016x}.json"));
    if std::fs::read(&sumber).ok().as_deref() != Some(json.as_bytes()) {
        std::fs::write(&sumber, json.as_bytes())
            .map_err(|e| format!("menulis filter sementara gagal: {e}"))?;
    }

    buang_salinan_lama(folder, &sumber);
    rapikan_store(&store_dir);
    Ok(sumber)
}

/// Membuang salinan JSON dari isi filter sebelumnya.
///
/// Setiap perubahan daftar filter meninggalkan satu berkas 14 MB, dan tanpa
/// pembersihan folder cache tumbuh ratusan MB (terukur 633 MB setelah beberapa
/// minggu pemakaian) — di mesin 1,7 GB itu ikut menekan page cache.
fn buang_salinan_lama(folder: &Path, simpan: &Path) {
    let Ok(isi) = std::fs::read_dir(folder) else {
        return;
    };
    for entri in isi.flatten() {
        let path = entri.path();
        if path == simpan {
            continue;
        }
        let nama = entri.file_name();
        let Some(nama) = nama.to_str() else {
            continue;
        };
        if nama.starts_with("perisai-") && nama.ends_with(".json") {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// WebKit menyimpan hasil kompilasi content blocker sebagai satu berkas ~30 MB
/// per isi filter di `webkit-store`, dengan nama acak yang tidak bisa dipetakan
/// ke id filter. Karena itu WebKit sendiri tidak pernah menghapus yang lama:
/// setiap pembaruan daftar filter menambah ~30 MB permanen (terukur 1,9 GB di
/// mesin uji). Dua yang terbaru dipertahankan — cukup untuk filter aktif dan
/// satu sebelumnya — sisanya dihapus sebelum store dibuka. Berkas yang hilang
/// hanya membuat WebKit mengompilasi ulang di thread latarnya sendiri.
fn rapikan_store(store_dir: &Path) {
    const SIMPAN: usize = 2;
    let Ok(isi) = std::fs::read_dir(store_dir) else {
        return;
    };
    let mut berkas: Vec<(std::time::SystemTime, PathBuf)> = isi
        .flatten()
        .filter(|entri| {
            entri
                .file_name()
                .to_string_lossy()
                .starts_with("ContentRuleList-")
        })
        .filter_map(|entri| {
            let waktu = entri.metadata().ok()?.modified().ok()?;
            Some((waktu, entri.path()))
        })
        .collect();
    if berkas.len() <= SIMPAN {
        return;
    }
    berkas.sort_by_key(|entri| std::cmp::Reverse(entri.0));
    for (_, path) in berkas.into_iter().skip(SIMPAN) {
        let _ = std::fs::remove_file(path);
    }
}

/// Memasang berkas filter yang sudah disiapkan [`siapkan`].
///
/// Meski namanya "asinkron", `webkit_user_content_filter_store_save_from_file`
/// **mengompilasi seluruh aturan di utas pemanggil** sebelum callback-nya
/// dijadwalkan: untuk filter 14 MB (131 ribu aturan) bagian itu terukur
/// ~1.070 ms di sini. Dijalankan di main thread, angka itu langsung terlihat
/// sebagai jendela yang membeku saat browser dibuka — dan di laptop 1,7 GB
/// melewati ambang dua detik.
///
/// Karena itu pemanggilannya dipindah ke thread pekerja. Aman karena: objek
/// `WebKitUserContentFilterStore` hanya memegang `API::ContentRuleListStore`
/// (kompilasi berjalan di work queue WebKit), `GTask` boleh dibuat dari utas
/// mana pun, dan callback-nya dikirim ke main context global — jadi
/// `saat_siap` tetap berjalan di main thread seperti yang dibutuhkan
/// `pasang_ke` (`WebKitUserContentManager` memang hanya boleh disentuh di
/// sana). Ini diuji dengan aplikasi berjalan: filter tetap terpasang dan
/// halaman tetap terlindungi.
pub fn pasang(sumber: &Path, saat_siap: Box<dyn FnOnce() + Send>) -> Result<(), String> {
    let store_dir = sumber
        .parent()
        .map(|folder| folder.join("webkit-store"))
        .ok_or("path filter tidak punya folder")?;
    let sumber = sumber.to_path_buf();
    std::thread::Builder::new()
        .name("perisai-filter".into())
        .spawn(move || {
            // Satu kompilasi pada satu waktu: dua penyimpanan yang berjalan
            // bersamaan akan membuat dua store untuk folder yang sama dan
            // menulis berkas hasil kompilasi secara bersamaan.
            let _kunci = KUNCI.lock().unwrap_or_else(|rusak| rusak.into_inner());
            let store = match store(&store_dir) {
                Ok(store) => store,
                Err(pesan) => {
                    eprintln!("adblock: store filter gagal dibuat: {pesan}");
                    return;
                }
            };
            let id = match sumber.file_stem().and_then(|nama| nama.to_str()) {
                Some(id) => id.to_string(),
                None => {
                    eprintln!("adblock: nama berkas filter tidak valid");
                    return;
                }
            };
            let (Ok(id), Ok(berkas)) = (
                CString::new(id),
                CString::new(sumber.to_string_lossy().as_bytes()),
            ) else {
                eprintln!("adblock: path filter mengandung NUL");
                return;
            };

            // GFile dan store hidup selama operasi asinkron; keduanya sengaja
            // tidak di-unref karena filter dipakai sampai proses berakhir.
            let file = unsafe { gio::ffi::g_file_new_for_path(berkas.as_ptr()) };
            let permintaan = Box::new(Permintaan {
                store,
                saat_siap: Some(saat_siap),
            });
            unsafe {
                ffi::webkit_user_content_filter_store_save_from_file(
                    store,
                    id.as_ptr(),
                    file,
                    ptr::null_mut(),
                    Some(selesai_simpan),
                    Box::into_raw(permintaan) as *mut c_void,
                );
            }
        })
        .map_err(|e| format!("thread filter gagal dibuat: {e}"))?;
    Ok(())
}

fn store(folder: &Path) -> Result<*mut ffi::WebKitUserContentFilterStore, String> {
    STORE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(store) = *slot {
            return Ok(store);
        }
        let path = CString::new(folder.to_string_lossy().as_bytes())
            .map_err(|_| "path store filter mengandung NUL".to_string())?;
        // `webkit_user_content_filter_store_new` membuat folder bila belum ada.
        let store = unsafe { ffi::webkit_user_content_filter_store_new(path.as_ptr()) };
        *slot = Some(store);
        Ok(store)
    })
}

/// Callback GLib ketika kompilasi filter selesai.
///
/// # Safety
/// Dipanggil oleh GLib dengan `data` hasil `Box::into_raw(Permintaan)`.
unsafe extern "C" fn selesai_simpan(
    _sumber: *mut glib::gobject_ffi::GObject,
    hasil: *mut gio::ffi::GAsyncResult,
    data: *mut c_void,
) {
    let mut permintaan = Box::from_raw(data as *mut Permintaan);
    let mut galat: *mut glib::ffi::GError = ptr::null_mut();
    let filter = ffi::webkit_user_content_filter_store_save_from_file_finish(
        permintaan.store,
        hasil,
        &mut galat,
    );

    if filter.is_null() {
        let pesan = if galat.is_null() {
            "tanpa keterangan".to_string()
        } else {
            let teks = std::ffi::CStr::from_ptr((*galat).message)
                .to_string_lossy()
                .into_owned();
            glib::ffi::g_error_free(galat);
            teks
        };
        eprintln!("adblock: filter WebKit ditolak: {pesan}");
        return;
    }

    FILTER.with(|slot| *slot.borrow_mut() = Some(filter));
    if let Some(saat_siap) = permintaan.saat_siap.take() {
        saat_siap();
    }
}

/// Memasang filter ke satu webview. Mengembalikan `false` kalau filter belum
/// siap (pemanggil bisa mencoba lagi setelah event "siap").
pub fn pasang_ke(webview: &WebView) -> bool {
    let filter = FILTER.with(|slot| *slot.borrow());
    let Some(filter) = filter else {
        return false;
    };
    let Some(manager) = webview.user_content_manager() else {
        return false;
    };
    unsafe {
        ffi::webkit_user_content_manager_add_filter(manager.to_glib_none().0, filter);
    }
    true
}

/// Melepas semua filter dari satu webview (dipakai saat perlindungan situs
/// dimatikan). Filter global tetap ada untuk webview lain.
pub fn lepas_dari(webview: &WebView) {
    if let Some(manager) = webview.user_content_manager() {
        unsafe {
            ffi::webkit_user_content_manager_remove_all_filters(manager.to_glib_none().0);
        }
    }
}

/// Perubahan kecil pada isi JSON menghasilkan nama berbeda, jadi sidik ini
/// tidak perlu kuat secara kriptografis — hanya perlu stabil dan cepat.
fn sidik_isi(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{buang_salinan_lama, rapikan_store, sidik_isi};
    use std::path::{Path, PathBuf};

    /// Folder uji unik per tes; isinya dibersihkan lagi di akhir.
    fn folder_uji(nama: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("perisai-uji-filter-{}-{nama}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn sentuh(path: &Path, isi: &str) {
        std::fs::write(path, isi).unwrap();
    }

    #[test]
    fn sidik_stabil_dan_berbeda_untuk_isi_berbeda() {
        assert_eq!(sidik_isi(b"abc"), sidik_isi(b"abc"));
        assert_ne!(sidik_isi(b"abc"), sidik_isi(b"abd"));
    }

    #[test]
    fn salinan_lama_dibuang_salinan_aktif_disimpan() {
        // Setiap isi filter baru meninggalkan 14 MB; tanpa pembersihan folder
        // cache tumbuh ratusan MB dan menekan page cache mesin kecil.
        let folder = folder_uji("salinan");
        let aktif = folder.join("perisai-baru.json");
        sentuh(&folder.join("perisai-lama1.json"), "x");
        sentuh(&folder.join("perisai-lama2.json"), "x");
        sentuh(&aktif, "x");
        // Berkas milik WebKit dan milik aplikasi lain tidak boleh disentuh.
        sentuh(&folder.join("filter.json"), "x");
        sentuh(&folder.join("easylist.txt"), "x");

        buang_salinan_lama(&folder, &aktif);

        assert!(aktif.exists());
        assert!(folder.join("filter.json").exists());
        assert!(folder.join("easylist.txt").exists());
        let sisa: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|nama| nama.starts_with("perisai-"))
            .collect();
        assert_eq!(sisa, vec!["perisai-baru.json".to_string()]);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn store_webkit_disusutkan_ke_dua_terbaru() {
        // WebKit tidak pernah menghapus hasil kompilasi lama (masing-masing
        // ~30 MB, nama acak), jadi pembersihannya harus dari sisi kita.
        let folder = folder_uji("store");
        for (i, nama) in ["a", "b", "c", "d"].iter().enumerate() {
            let path = folder.join(format!("ContentRuleList-{nama}"));
            sentuh(&path, "x");
            // Waktu ubah dibuat berbeda-beda supaya urutannya pasti.
            let waktu = std::time::SystemTime::now() - std::time::Duration::from_secs(600)
                + std::time::Duration::from_secs(i as u64 * 60);
            let berkas = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            berkas.set_modified(waktu).unwrap();
        }
        // Berkas lain di folder yang sama tidak boleh ikut terhapus.
        sentuh(&folder.join("catatan.txt"), "x");

        rapikan_store(&folder);

        assert!(!folder.join("ContentRuleList-a").exists());
        assert!(!folder.join("ContentRuleList-b").exists());
        assert!(folder.join("ContentRuleList-c").exists());
        assert!(folder.join("ContentRuleList-d").exists());
        assert!(folder.join("catatan.txt").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn store_dengan_dua_berkas_tidak_disentuh() {
        let folder = folder_uji("store-dua");
        sentuh(&folder.join("ContentRuleList-a"), "x");
        sentuh(&folder.join("ContentRuleList-b"), "x");
        rapikan_store(&folder);
        assert!(folder.join("ContentRuleList-a").exists());
        assert!(folder.join("ContentRuleList-b").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
