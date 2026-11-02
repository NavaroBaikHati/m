//! Pintasan keyboard di Linux.
//!
//! Tauri tidak punya API pintasan yang berlaku saat fokus ada di webview isi
//! tab. Menu aplikasi bisa dipakai, tapi di GTK menu itu memunculkan menu bar
//! di atas jendela — bertentangan dengan UI tanpa menu bar. Karena itu tombol
//! ditangkap di level `GtkWindow`: event keyboard naik dari widget yang fokus
//! (webview) ke jendela, jadi handler di sini melihat Ctrl+T dan kawan-kawan
//! sebelum WebKit sempat memakainya.

use gtk::prelude::*;
use tauri::{AppHandle, Manager};

use crate::pintasan::Aksi;

/// Konstanta keyval GTK; di gdk 0.18 letaknya di submodul `constants`.
use gtk::gdk::keys::constants as k;

/// Memasang penangkap tombol pada jendela utama. Dipanggil sekali saat setup.
pub fn pasang(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_window("main")
        .ok_or_else(|| "window main tidak ditemukan".to_string())?;
    let gtk_window = window
        .gtk_window()
        .map_err(|e| format!("window GTK tidak tersedia: {e}"))?;

    let app = app.clone();
    gtk_window.connect_key_press_event(move |_, event| {
        let state = event.state();
        let ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
        let alt = state.contains(gtk::gdk::ModifierType::MOD1_MASK);
        let shift = state.contains(gtk::gdk::ModifierType::SHIFT_MASK);
        let tombol = event.keyval();

        let aksi = if alt {
            match tombol {
                k::Left => Some(Aksi::Mundur),
                k::Right => Some(Aksi::Maju),
                _ => None,
            }
        } else if ctrl {
            match tombol {
                k::t if shift => Some(Aksi::TabPulihkan),
                k::t => Some(Aksi::TabBaru),
                k::w => Some(Aksi::TabTutup),
                k::Tab => Some(Aksi::TabBerikut),
                k::ISO_Left_Tab => Some(Aksi::TabSebelum),
                k::r => Some(Aksi::MuatUlang),
                k::l => Some(Aksi::FokusAlamat),
                k::b => Some(Aksi::Sidebar),
                k::d => Some(Aksi::SimpanBookmark),
                k::j => Some(Aksi::BukaUnduhan),
                k::h => Some(Aksi::BukaRiwayat),
                k::k => Some(Aksi::Menu),
                k::q => Some(Aksi::Keluar),
                k::equal | k::plus | k::KP_Add => Some(Aksi::ZoomMasuk),
                k::minus | k::KP_Subtract => Some(Aksi::ZoomKeluar),
                k::_0 | k::KP_0 => Some(Aksi::ZoomNormal),
                _ => None,
            }
        } else {
            None
        };

        match aksi {
            Some(aksi) => {
                aksi.jalankan(&app);
                gtk::glib::Propagation::Stop
            }
            None => gtk::glib::Propagation::Proceed,
        }
    });
    Ok(())
}
