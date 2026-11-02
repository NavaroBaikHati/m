//! Aksi pintasan keyboard.
//!
//! Peta tombol ke aksi ada di platform (GTK di Linux; lihat
//! `src/platform/linux/pintasan.rs`), sedangkan aksinya di sini supaya logika
//! tidak bergantung pada toolkit.
//!
//! Perintah yang sebenarnya milik UI (buka panel unduhan, fokus address bar)
//! dikirim lewat event `chrome:perintah` ke webview chrome. Sisanya dikerjakan
//! langsung oleh `TabManager`.

use tauri::{AppHandle, Emitter, EventTarget, Manager};

use crate::tabs::{TabManager, LABEL_CHROME};

/// Event untuk meminta UI melakukan sesuatu (fokus address bar, buka panel).
pub const EVENT_PERINTAH: &str = "chrome:perintah";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aksi {
    TabBaru,
    TabTutup,
    TabBerikut,
    TabSebelum,
    TabPulihkan,
    MuatUlang,
    Mundur,
    Maju,
    ZoomMasuk,
    ZoomKeluar,
    ZoomNormal,
    FokusAlamat,
    Sidebar,
    SimpanBookmark,
    BukaUnduhan,
    BukaRiwayat,
    Menu,
    Keluar,
}

impl Aksi {
    /// Menjalankan aksi; mengembalikan `false` kalau aksi tidak dikenal.
    pub fn jalankan(self, app: &AppHandle) {
        let Some(tabs) = app.try_state::<TabManager>() else {
            return;
        };
        match self {
            Aksi::TabBaru => {
                let _ = tabs.tab_baru();
            }
            Aksi::TabTutup => {
                if let Some(aktif) = tabs.daftar().ok().and_then(|d| d.aktif) {
                    let _ = tabs.tutup(&aktif);
                }
            }
            Aksi::TabBerikut => {
                let _ = tabs.pindah(1);
            }
            Aksi::TabSebelum => {
                let _ = tabs.pindah(-1);
            }
            Aksi::TabPulihkan => {
                let _ = tabs.buka_kembali();
            }
            Aksi::MuatUlang => {
                if let Some(webview) = tabs.webview_aktif() {
                    let _ = webview.reload();
                }
            }
            Aksi::Mundur | Aksi::Maju => {
                if let Some(webview) = tabs.webview_aktif() {
                    let skrip = if self == Aksi::Mundur {
                        "window.history.back()"
                    } else {
                        "window.history.forward()"
                    };
                    let _ = webview.eval(skrip);
                }
            }
            Aksi::ZoomMasuk => {
                let _ = tabs.atur_zoom(tabs.zoom() + 0.1);
            }
            Aksi::ZoomKeluar => {
                let _ = tabs.atur_zoom(tabs.zoom() - 0.1);
            }
            Aksi::ZoomNormal => {
                let _ = tabs.atur_zoom(1.0);
            }
            Aksi::Sidebar => {
                let terbuka = tabs.daftar().map(|d| d.sidebar_terbuka).unwrap_or(true);
                let _ = tabs.atur_sidebar(!terbuka);
            }
            Aksi::Keluar => app.exit(0),
            Aksi::FokusAlamat => kirim_perintah(app, "fokus-alamat"),
            Aksi::SimpanBookmark => kirim_perintah(app, "bookmark"),
            Aksi::BukaUnduhan => kirim_perintah(app, "unduhan"),
            Aksi::BukaRiwayat => kirim_perintah(app, "riwayat"),
            Aksi::Menu => kirim_perintah(app, "menu"),
        }
    }
}

/// Mengirim perintah ke UI chrome (bukan ke halaman web).
pub fn kirim_perintah(app: &AppHandle, perintah: &str) {
    let target = EventTarget::Webview {
        label: LABEL_CHROME.to_string(),
    };
    let _ = app.emit_to(target, EVENT_PERINTAH, perintah);
}

#[cfg(test)]
mod tests {
    use super::Aksi;

    #[test]
    fn aksi_punya_identitas_stabil() {
        // Penjaga sederhana: varian baru harus tetap bisa dibandingkan.
        assert_eq!(Aksi::TabBaru, Aksi::TabBaru);
        assert_ne!(Aksi::TabBaru, Aksi::TabTutup);
    }
}
