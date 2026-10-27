//! Tab manager: satu tab = satu child webview di dalam window utama.
//!
//! Catatan desain: child webview tidak ikut menyesuaikan ukuran sendiri saat
//! window di-resize (perilakunya memang fixed position/size), jadi batas tiap
//! tab dihitung ulang di sini setiap kali window berubah ukuran.

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
            }),
            urutan: AtomicUsize::new(0),
        }
    }

    /// Membuat tab baru dan menjadikannya tab aktif.
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

        let webview = self
            .window
            .add_child(
                builder,
                LogicalPosition::new(0.0, TINGGI_CHROME),
                self.ukuran_konten()?,
            )
            .map_err(|e| format!("gagal membuat webview {id}: {e}"))?;
        // Di Linux ukuran di atas diabaikan (lihat src/platform/linux.rs), jadi
        // batas asli selalu ditetapkan lewat helper platform.
        crate::platform::atur_batas(&webview, self.rect_konten()?)?;
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

    /// Keadaan tab aktif, dengan URL diambil langsung dari webview agar tetap
    /// benar setelah pengalihan (redirect) atau navigasi di dalam halaman.
    pub fn aktif(&self) -> Option<KeadaanTab> {
        let label = self.label_aktif()?;
        let webview = self.app.get_webview(&label)?;
        let isi = self.isi.lock().ok()?;
        let mut tab = isi.tabs.iter().find(|t| t.id == label)?.clone();
        if let Ok(url) = webview.url() {
            tab.url = url.to_string();
        }
        Some(tab)
    }

    pub fn webview_aktif(&self) -> Option<Webview> {
        let label = self.label_aktif()?;
        self.app.get_webview(&label)
    }

    /// Menyesuaikan batas webview chrome dan semua tab setelah window berubah ukuran.
    pub fn ukuran_berubah(&self) -> Result<(), String> {
        let konten = self.rect_konten()?;
        let labels: Vec<String> = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.tabs.iter().map(|t| t.id.clone()).collect()
        };
        for label in labels {
            if let Some(webview) = self.app.get_webview(&label) {
                crate::platform::atur_batas(&webview, konten)
                    .map_err(|e| format!("gagal menyesuaikan {label}: {e}"))?;
            }
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

    /// Area isi tab: di bawah chrome, selebar window.
    /// Sidebar vertikal (tahap 2) akan memakai `x` dan mengurangi `lebar` di sini.
    pub fn rect_konten(&self) -> Result<Rect, String> {
        Ok(Rect {
            position: LogicalPosition::new(0.0, TINGGI_CHROME).into(),
            size: self.ukuran_konten()?.into(),
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

    fn ukuran_konten(&self) -> Result<LogicalSize<f64>, String> {
        let (lebar, tinggi) = self.ukuran_logis()?;
        Ok(LogicalSize::new(lebar, (tinggi - TINGGI_CHROME).max(0.0)))
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
