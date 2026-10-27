//! Linux (WebKitGTK): penempatan absolut child webview lewat `GtkFixed`.
//!
//! Kenapa perlu: `tauri-runtime-wry` menambahkan child webview ke `GtkBox`
//! bawaan window (`webview_builder.build_gtk(window.default_vbox())`), dan
//! `GtkBox` menyusun anak-anaknya secara vertikal — jadi semua webview berbagi
//! tinggi window dan `Webview::set_bounds` tidak memindahkan apa pun
//! (tauri-apps/tauri#10420, masih terbuka).
//!
//! wry sendiri mendukung penempatan absolut: bila wadah webview berupa
//! `gtk::Fixed`, webview diletakkan pada koordinat yang diminta. Karena itu kita
//! menyisipkan `GtkFixed` ke dalam `GtkBox` window dan memindahkan seluruh
//! webview ke dalamnya. Modul ini tidak diperlukan lagi kalau Tauri/tao memakai
//! `GtkFixed` secara bawaan.

use std::cell::RefCell;

use gtk::prelude::*;
use tauri::{AppHandle, Manager, Rect, Webview};

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
            let widget: gtk::Widget = platform.inner().clone().upcast();
            tempatkan(&widget, rect);
        })
        .map_err(|e| e.to_string())
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
