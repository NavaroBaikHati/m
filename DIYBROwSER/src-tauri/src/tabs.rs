//! Tab manager: satu tab = satu child webview di dalam window utama.
//!
//! Catatan desain: child webview tidak ikut menyesuaikan ukuran sendiri saat
//! window di-resize (perilakunya memang fixed position/size), jadi batas tiap
//! tab dihitung ulang di sini setiap kali window berubah ukuran. Hanya tab aktif
//! yang diposisikan; tab lain disembunyikan supaya tidak menutupi chrome UI dan
//! tidak memakan gambar.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect, Url, Webview,
    WebviewUrl, Window,
};

/// Tinggi chrome browser (toolbar) dalam logical pixel.
/// Harus sama dengan variabel `--tinggi-chrome` di `ui/src/app.css`.
pub const TINGGI_CHROME: f64 = 48.0;

/// Lebar sidebar vertikal saat terbuka, dalam logical pixel.
/// Harus sama dengan variabel `--lebar-sidebar` di `ui/src/app.css`.
pub const LEBAR_SIDEBAR: f64 = 216.0;

/// Nama event yang dipakai Rust untuk memberi tahu webview chrome.
pub const EVENT_NAVIGASI: &str = "tab:navigasi";

/// Label webview chrome; lihat `tauri.conf.json` dan `capabilities/default.json`.
pub const LABEL_CHROME: &str = "main";

/// Keadaan satu tab seperti yang diketahui Rust.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeadaanTab {
    pub id: String,
    pub url: String,
    pub title: String,
}

/// Daftar tab yang ditampilkan sidebar, plus tab mana yang aktif.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaftarTab {
    pub tabs: Vec<KeadaanTab>,
    pub aktif: Option<String>,
    pub sidebar_terbuka: bool,
}

/// Perubahan sebagian: hanya field yang berubah yang dikirim.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PeristiwaNavigasi {
    tab: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memuat: Option<bool>,
}

struct Isi {
    tabs: Vec<KeadaanTab>,
    aktif: Option<String>,
    sidebar_terbuka: bool,
}

pub struct TabManager {
    app: AppHandle,
    window: Window,
    isi: Mutex<Isi>,
    urutan: AtomicUsize,
}

impl TabManager {
    pub fn new(app: AppHandle, window: Window) -> Self {
        Self {
            app,
            window,
            isi: Mutex::new(Isi {
                tabs: Vec::new(),
                aktif: None,
                sidebar_terbuka: true,
            }),
            urutan: AtomicUsize::new(0),
        }
    }

    /// Membuat tab baru dan menjadikannya tab aktif.
    ///
    /// Tab yang sebelumnya aktif disembunyikan, jadi hanya satu webview yang
    /// terlihat pada satu waktu.
    pub fn buka(&self, url: Url) -> Result<KeadaanTab, String> {
        let id = format!("tab-{}", self.urutan.fetch_add(1, Ordering::SeqCst) + 1);

        let builder = WebviewBuilder::new(id.clone(), WebviewUrl::External(url.clone()))
            .on_navigation({
                let app = self.app.clone();
                let id = id.clone();
                move |url| {
                    kirim(
                        &app,
                        PeristiwaNavigasi {
                            tab: id.clone(),
                            url: Some(url.to_string()),
                            title: None,
                            memuat: Some(true),
                        },
                    );
                    true
                }
            })
            .on_page_load({
                let app = self.app.clone();
                let id = id.clone();
                move |_webview, payload| {
                    kirim(
                        &app,
                        PeristiwaNavigasi {
                            tab: id.clone(),
                            url: Some(payload.url().to_string()),
                            title: None,
                            memuat: Some(matches!(payload.event(), PageLoadEvent::Started)),
                        },
                    );
                }
            })
            .on_document_title_changed({
                let app = self.app.clone();
                let id = id.clone();
                move |_webview, judul| {
                    // Simpan juga di state Rust supaya daftar tab tetap benar
                    // saat sidebar dimuat ulang atau berpindah tab.
                    if let Some(tabs) = app.try_state::<TabManager>() {
                        if let Ok(mut isi) = tabs.isi.lock() {
                            if let Some(tab) = isi.tabs.iter_mut().find(|t| t.id == id) {
                                tab.title = judul.clone();
                            }
                        }
                    }
                    kirim(
                        &app,
                        PeristiwaNavigasi {
                            tab: id.clone(),
                            url: None,
                            title: Some(judul),
                            memuat: None,
                        },
                    );
                }
            });

        if let Some(lama) = self.label_aktif() {
            if let Some(webview) = self.app.get_webview(&lama) {
                let _ = webview.hide();
            }
        }

        let (lebar, tinggi) = self.ukuran_logis()?;
        let awal = self.sidebar_lebar()?;
        let ukuran = LogicalSize::new((lebar - awal).max(1.0), (tinggi - TINGGI_CHROME).max(1.0));
        let webview = self
            .window
            .add_child(builder, LogicalPosition::new(awal, TINGGI_CHROME), ukuran)
            .map_err(|e| format!("gagal membuat webview {id}: {e}"))?;
        // Di Linux ukuran di atas diabaikan (lihat src/platform/linux.rs), jadi
        // batas asli selalu ditetapkan lewat helper platform.
        crate::platform::atur_batas(&webview, self.rect_konten()?)?;
        let _ = webview.show();
        let _ = webview.set_focus();

        let keadaan = KeadaanTab {
            id: id.clone(),
            url: url.to_string(),
            title: String::new(),
        };
        let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
        isi.tabs.push(keadaan.clone());
        isi.aktif = Some(id);
        drop(isi);

        Ok(keadaan)
    }

    /// Daftar tab untuk sidebar. URL diambil langsung dari webview agar tetap
    /// benar setelah pengalihan (redirect) atau navigasi di dalam halaman.
    pub fn daftar(&self) -> Result<DaftarTab, String> {
        let (tabs, aktif, sidebar_terbuka) = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            (isi.tabs.clone(), isi.aktif.clone(), isi.sidebar_terbuka)
        };

        let mut tabs = tabs;
        for tab in &mut tabs {
            if let Some(webview) = self.app.get_webview(&tab.id) {
                if let Ok(url) = webview.url() {
                    tab.url = url.to_string();
                }
            }
        }

        Ok(DaftarTab {
            tabs,
            aktif,
            sidebar_terbuka,
        })
    }

    /// Mengaktifkan tab `id` dan menampilkannya kembali.
    pub fn pilih(&self, id: &str) -> Result<(), String> {
        let lama = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            if !isi.tabs.iter().any(|tab| tab.id == id) {
                return Err(format!("tab {id} tidak ada"));
            }
            isi.aktif.clone()
        };

        let webview = self
            .app
            .get_webview(id)
            .ok_or_else(|| format!("webview tab {id} tidak ada"))?;

        if lama.as_deref() != Some(id) {
            if let Some(lama) = lama {
                if let Some(lama) = self.app.get_webview(&lama) {
                    let _ = lama.hide();
                }
            }
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.aktif = Some(id.to_string());
        }

        crate::platform::atur_batas(&webview, self.rect_konten()?)?;
        webview.show().map_err(|e| e.to_string())?;
        let _ = webview.set_focus();
        Ok(())
    }

    /// Menutup tab `id`. Kalau tab terakhir ditutup, tab baru langsung dibuka
    /// supaya jendela tidak pernah kosong.
    pub fn tutup(&self, id: &str) -> Result<(), String> {
        let (menutup_aktif, pengganti, terakhir) = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            let posisi = isi
                .tabs
                .iter()
                .position(|tab| tab.id == id)
                .ok_or_else(|| format!("tab {id} tidak ada"))?;
            let pengganti = tab_pengganti(isi.tabs.len(), posisi).map(|i| isi.tabs[i].id.clone());
            (
                isi.aktif.as_deref() == Some(id),
                pengganti,
                isi.tabs.len() == 1,
            )
        };

        if let Some(webview) = self.app.get_webview(id) {
            webview.close().map_err(|e| e.to_string())?;
        }

        {
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.tabs.retain(|tab| tab.id != id);
            if menutup_aktif {
                isi.aktif = pengganti.clone();
            }
        }

        if menutup_aktif {
            if let Some(pengganti) = pengganti {
                self.pilih(&pengganti)?;
            }
        }

        if terakhir {
            self.buka(crate::url_input::url_awal())?;
        }

        Ok(())
    }

    /// Membuka atau menciutkan sidebar vertikal.
    pub fn atur_sidebar(&self, terbuka: bool) -> Result<(), String> {
        {
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.sidebar_terbuka = terbuka;
        }
        if let Some(webview) = self.webview_aktif() {
            crate::platform::atur_batas(&webview, self.rect_konten()?)?;
            let _ = webview.show();
        }
        Ok(())
    }

    pub fn webview_aktif(&self) -> Option<Webview> {
        let label = self.label_aktif()?;
        self.app.get_webview(&label)
    }

    /// Menyesuaikan batas webview chrome dan tab aktif setelah window berubah ukuran.
    pub fn ukuran_berubah(&self) -> Result<(), String> {
        if let Some(webview) = self.webview_aktif() {
            crate::platform::atur_batas(&webview, self.rect_konten()?)?;
        }
        if let Some(chrome) = self.app.get_webview(LABEL_CHROME) {
            crate::platform::atur_batas(&chrome, self.rect_penuh()?)?;
        }
        Ok(())
    }

    fn label_aktif(&self) -> Option<String> {
        self.isi.lock().ok()?.aktif.clone()
    }

    /// Seluruh area window; dipakai webview chrome (UI).
    pub fn rect_penuh(&self) -> Result<Rect, String> {
        let (lebar, tinggi) = self.ukuran_logis()?;
        Ok(Rect {
            position: LogicalPosition::new(0.0, 0.0).into(),
            size: LogicalSize::new(lebar, tinggi).into(),
        })
    }

    /// Area isi tab: di bawah chrome, di sebelah kanan sidebar (kalau terbuka).
    pub fn rect_konten(&self) -> Result<Rect, String> {
        let (lebar, tinggi) = self.ukuran_logis()?;
        let awal = self.sidebar_lebar()?;
        Ok(Rect {
            position: LogicalPosition::new(awal, TINGGI_CHROME).into(),
            size: LogicalSize::new((lebar - awal).max(1.0), (tinggi - TINGGI_CHROME).max(1.0))
                .into(),
        })
    }

    fn sidebar_lebar(&self) -> Result<f64, String> {
        let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
        Ok(if isi.sidebar_terbuka {
            LEBAR_SIDEBAR
        } else {
            0.0
        })
    }

    fn ukuran_logis(&self) -> Result<(f64, f64), String> {
        let skala = self.window.scale_factor().map_err(|e| e.to_string())?;
        let ukuran = self.window.inner_size().map_err(|e| e.to_string())?;
        Ok((
            (ukuran.width as f64 / skala).max(1.0),
            (ukuran.height as f64 / skala).max(1.0),
        ))
    }
}

/// Posisi tab yang diaktifkan setelah tab di `posisi` ditutup: tab di kanan
/// kalau ada, kalau tidak tab di kiri. `None` kalau tidak ada tab lain.
fn tab_pengganti(jumlah: usize, posisi: usize) -> Option<usize> {
    if jumlah <= 1 {
        return None;
    }
    if posisi + 1 < jumlah {
        Some(posisi + 1)
    } else {
        Some(posisi - 1)
    }
}

/// Mengirim perubahan keadaan ke webview chrome saja.
///
/// Webview tab yang memuat situs web tidak punya izin IPC, tapi menargetkan
/// label chrome membuat niatnya eksplisit.
fn kirim(app: &AppHandle, peristiwa: PeristiwaNavigasi) {
    let target = EventTarget::Webview {
        label: LABEL_CHROME.to_string(),
    };
    if let Err(e) = app.emit_to(target, EVENT_NAVIGASI, peristiwa) {
        eprintln!("gagal mengirim {EVENT_NAVIGASI}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::tab_pengganti;

    #[test]
    fn memilih_tab_kanan_setelah_menutup() {
        assert_eq!(tab_pengganti(3, 0), Some(1));
        assert_eq!(tab_pengganti(3, 1), Some(2));
    }

    #[test]
    fn memilih_tab_kiri_kalau_menutup_tab_terakhir() {
        assert_eq!(tab_pengganti(3, 2), Some(1));
    }

    #[test]
    fn tidak_ada_pengganti_kalau_hanya_satu_tab() {
        assert_eq!(tab_pengganti(1, 0), None);
    }
}
