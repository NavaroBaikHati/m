//! Content blocker WebKitGTK (`WebKitUserContentFilter`).
//!
//! WebKitGTK 2.24+ bisa mencocokkan request sendiri memakai JSON content
//! blocker (format yang sama dengan Safari). Sayangnya binding Rust
//! `webkit2gtk` sengaja tidak membungkus `WebKitUserContentFilter` (tipe
//! `Ignored` di file hasil generate), jadi bagian ini memakai FFI mentah.
//!
//! Alur: JSON disimpan lewat `WebKitUserContentFilterStore` (asinkron, karena
//! hanya itu cara membuat objek filter), lalu hasilnya dipasang ke
//! `WebKitUserContentManager` tiap webview. Store dan filter hidup selama
//! proses; jumlahnya satu per aplikasi, jadi tidak perlu dibersihkan.
//!
//! Semua fungsi di sini **hanya boleh dipanggil dari thread utama GTK**. Itu
//! sebabnya hasilnya disimpan di `thread_local`, bukan `static` biasa.

use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::path::Path;
use std::ptr;

use webkit2gtk::glib::translate::ToGlibPtr;
use webkit2gtk::{ffi, gio, glib, WebView, WebViewExt};

thread_local! {
    /// Filter yang sudah selesai dikompilasi WebKit. `None` berarti belum siap.
    static FILTER: RefCell<Option<*mut ffi::WebKitUserContentFilter>> = const { RefCell::new(None) };
    /// Store harus hidup selama pemuatan; disimpan sekali per proses.
    static STORE: RefCell<Option<*mut ffi::WebKitUserContentFilterStore>> = const { RefCell::new(None) };
}

struct Permintaan {
    store: *mut ffi::WebKitUserContentFilterStore,
    saat_siap: Option<Box<dyn FnOnce()>>,
}

/// Apakah filter sudah terpasang dan bisa dipakai webview baru.
pub fn siap() -> bool {
    FILTER.with(|slot| slot.borrow().is_some())
}

/// Menyimpan `json` sebagai content blocker dan memanggil `saat_siap` (di thread
/// utama) setelah WebKit selesai mengompilasinya.
///
/// `folder` dipakai untuk menyimpan salinan JSON dan basis data filter WebKit.
/// Nama filter diturunkan dari isi JSON, jadi filter yang sama tidak
/// dikompilasi ulang saat aplikasi dibuka lagi.
pub fn pasang(json: String, folder: &Path, saat_siap: Box<dyn FnOnce()>) -> Result<(), String> {
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

    let store = store(&store_dir)?;
    let id = CString::new(format!("perisai-{sidik:016x}")).map_err(|_| "id filter tidak valid")?;
    let berkas = CString::new(sumber.to_string_lossy().as_bytes())
        .map_err(|_| "path filter mengandung NUL".to_string())?;

    // GFile dan store hidup selama operasi asinkron; keduanya sengaja tidak
    // di-unref karena filter dipakai sampai proses berakhir.
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
    use super::sidik_isi;

    #[test]
    fn sidik_stabil_dan_berbeda_untuk_isi_berbeda() {
        assert_eq!(sidik_isi(b"abc"), sidik_isi(b"abc"));
        assert_ne!(sidik_isi(b"abc"), sidik_isi(b"abd"));
    }
}
