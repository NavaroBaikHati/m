//! Linux (WebKitGTK): penempatan absolut child webview lewat `GtkFixed`,
//! content blocker WebKit, dan setelan render.
//!
//! Kenapa perlu `GtkFixed`: `tauri-runtime-wry` menambahkan child webview ke
//! `GtkBox` bawaan window (`webview_builder.build_gtk(window.default_vbox())`),
//! dan `GtkBox` menyusun anak-anaknya secara vertikal — jadi semua webview
//! berbagi tinggi window dan `Webview::set_bounds` tidak memindahkan apa pun
//! (tauri-apps/tauri#10420, masih terbuka).
//!
//! wry sendiri mendukung penempatan absolut: bila wadah webview berupa
//! `gtk::Fixed`, webview diletakkan pada koordinat yang diminta. Karena itu kita
//! menyisipkan `GtkFixed` ke dalam `GtkBox` window dan memindahkan seluruh
//! webview ke dalamnya. Modul ini tidak diperlukan lagi kalau Tauri/tao memakai
//! `GtkFixed` secara bawaan.

mod filter;
mod pintasan;

use std::cell::RefCell;
use std::path::PathBuf;

use gtk::prelude::*;
use tauri::{AppHandle, Manager, Rect, Webview};
use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

use crate::tabs::LABEL_CHROME;

thread_local! {
    /// Wadah milik kita. Objek GTK bukan `Send`, jadi hanya disimpan di thread
    /// utama dan hanya disentuh dari closure `with_webview`.
    static WADAH: RefCell<Option<gtk::Fixed>> = const { RefCell::new(None) };
}

/// Memindahkan webview chrome ke `GtkFixed` buatan kita. Webview tab yang
/// dibuat setelah ini dipindahkan oleh [`atur_batas`].
pub fn siapkan_wadah(app: &AppHandle, rect: Rect) -> Result<(), String> {
    let chrome = app
        .get_webview(LABEL_CHROME)
        .ok_or_else(|| format!("webview {LABEL_CHROME} tidak ditemukan"))?;
    pindahkan(&chrome, rect)
}

/// Menempatkan satu webview pada posisi dan ukuran yang diminta.
pub fn atur_batas(webview: &Webview, rect: Rect) -> Result<(), String> {
    pindahkan(webview, rect)
}

fn pindahkan(webview: &Webview, rect: Rect) -> Result<(), String> {
    webview
        .with_webview(move |platform| {
            let view = platform.inner();
            let view = &view;
            terapkan_setelan(view);
            let widget: gtk::Widget = view.clone().upcast();
            tempatkan(&widget, rect);
        })
        .map_err(|e| e.to_string())
}

/// Setelan WebKit yang menentukan pengalaman "berat/ringan" browser.
///
/// Akselerasi perangkat keras dinyalakan secara eksplisit: WebKitGTK memakai
/// `OnDemand` secara bawaan, yang membuat halaman ringan tidak memakai GPU dan
/// kadang membuat animasi tersendat. Kalau pengguna memaksa render perangkat
/// lunak (mesin tanpa GPU, driver rusak, atau sesi uji Xvfb), kita hormati
/// variabel lingkungan alih-alih memaksa GPU.
fn terapkan_setelan(view: &webkit2gtk::WebView) {
    let lunak = std::env::var_os("PERISAI_RENDER_SOFTWARE").is_some()
        || std::env::var_os("LIBGL_ALWAYS_SOFTWARE").is_some()
        || std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_some();

    let Some(setelan) = WebViewExt::settings(view) else {
        return;
    };
    setelan.set_hardware_acceleration_policy(if lunak {
        HardwareAccelerationPolicy::Never
    } else {
        HardwareAccelerationPolicy::Always
    });
    setelan.set_enable_webgl(true);
    setelan.set_enable_smooth_scrolling(true);
    setelan.set_enable_media_stream(true);
    setelan.set_enable_webrtc(true);
    // Devtools hanya saat build debug; di rilis tidak ada gunanya dan menambah
    // permukaan serangan.
    setelan.set_enable_developer_extras(cfg!(debug_assertions));
    // Latar gelap netral: mengurangi kedipan putih saat halaman baru dimuat,
    // terutama pada tema gelap.
    view.set_background_color(&gtk::gdk::RGBA::new(0.09, 0.09, 0.1, 1.0));
}

fn tempatkan(widget: &gtk::Widget, rect: Rect) {
    let wadah = wadah();

    if wadah.parent().is_none() {
        match widget
            .parent()
            .and_then(|p| p.dynamic_cast::<gtk::Box>().ok())
        {
            Some(vbox) => {
                vbox.add(&wadah);
                wadah.show();
            }
            None => eprintln!("wadah GtkBox window tidak ditemukan; tata letak tab bisa salah"),
        }
    }

    if widget.parent().as_ref() != Some(wadah.upcast_ref::<gtk::Widget>()) {
        if let Some(induk) = widget
            .parent()
            .and_then(|p| p.dynamic_cast::<gtk::Container>().ok())
        {
            induk.remove(widget);
        }
        wadah.add(widget);
    }

    let skala = f64::from(widget.scale_factor());
    let (x, y) = rect.position.to_logical::<i32>(skala).into();
    let (lebar, tinggi) = rect.size.to_logical::<i32>(skala).into();

    widget.set_size_request(lebar, tinggi);
    wadah.move_(widget, x, y);
    widget.show();
}

fn wadah() -> gtk::Fixed {
    if let Some(fixed) = WADAH.with(|slot| slot.borrow().clone()) {
        return fixed;
    }
    let fixed = gtk::Fixed::new();
    WADAH.with(|slot| *slot.borrow_mut() = Some(fixed.clone()));
    fixed
}

/// Mengompilasi dan memasang content blocker; lihat `filter.rs`.
pub fn pasang_filter(
    json: String,
    folder: PathBuf,
    saat_siap: Box<dyn FnOnce()>,
) -> Result<(), String> {
    filter::pasang(json, &folder, saat_siap)
}

pub fn filter_siap() -> bool {
    filter::siap()
}

/// Memasang filter ke satu webview; tidak melakukan apa pun kalau belum siap.
pub fn pasang_filter_ke(webview: &Webview) {
    let _ = webview.with_webview(|platform| {
        filter::pasang_ke(&platform.inner());
    });
}

/// Melepas filter dari satu webview.
pub fn lepas_filter(webview: &Webview) {
    let _ = webview.with_webview(|platform| {
        filter::lepas_dari(&platform.inner());
    });
}

/// Memasang pintasan keyboard lewat handler GTK; lihat `pintasan.rs`.
pub fn pasang_pintasan(app: &AppHandle) -> Result<(), String> {
    pintasan::pasang(app)
}
