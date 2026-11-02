//! Tab manager: satu tab = satu child webview di dalam window utama.
//!
//! Catatan desain: child webview tidak ikut menyesuaikan ukuran sendiri saat
//! window di-resize (perilakunya memang fixed position/size), jadi batas tiap
//! tab dihitung ulang di sini setiap kali window berubah ukuran. Hanya tab aktif
//! yang diposisikan; tab lain disembunyikan supaya tidak menutupi chrome UI dan
//! tidak memakan gambar.
//!
//! Tab manager juga pemilik hal-hal yang menempel pada siklus hidup tab:
//! pemasangan content blocker, scriptlet per halaman, pencatatan riwayat,
//! penyimpanan sesi, dan penerimaan unduhan.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use storage::Storage;
use tauri::{
    webview::{DownloadEvent, PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect, Url, Webview,
    WebviewUrl, Window,
};

use crate::adblock::{Adblock, Perlindungan};

/// Tinggi chrome browser (toolbar) dalam logical pixel.
/// Harus sama dengan variabel `--tinggi-chrome` di `ui/src/app.css`.
pub const TINGGI_CHROME: f64 = 52.0;

/// Lebar sidebar vertikal saat terbuka, dalam logical pixel.
/// Harus sama dengan variabel `--lebar-sidebar` di `ui/src/app.css`.
pub const LEBAR_SIDEBAR: f64 = 232.0;

/// Nama event yang dipakai Rust untuk memberi tahu webview chrome.
pub const EVENT_NAVIGASI: &str = "tab:navigasi";
pub const EVENT_UNDUHAN: &str = "unduhan:berubah";
/// Dikirim saat daftar tab berubah karena aksi backend (pintasan keyboard).
pub const EVENT_DAFTAR: &str = "tab:daftar";

/// Label webview chrome; lihat `tauri.conf.json` dan `capabilities/default.json`.
pub const LABEL_CHROME: &str = "main";

/// Batas jumlah tab yang dipulihkan saat aplikasi dibuka lagi.
pub const BATAS_SESI: usize = 20;
/// Batas tab yang dipulihkan saat start (dipakai `lib.rs`).
pub const BATAS_PULIH: usize = 20;

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
    pub zoom: f64,
    pub tema: String,
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

#[derive(Default, Serialize, Deserialize)]
struct PengaturanZoom {
    per_host: HashMap<String, f64>,
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
    adblock: Arc<Adblock>,
    db: Arc<Mutex<Storage>>,
    /// Halaman tab baru; `file://` hasil generate saat start.
    beranda: Url,
    /// Tab yang baru ditutup, untuk Ctrl+Shift+T.
    ditutup: Mutex<Vec<String>>,
    /// Zoom tab aktif, dicerminkan ke UI.
    zoom_aktif: Mutex<f64>,
    /// Lebar area panel samping yang dipesan UI (0 = tidak ada panel).
    panel: Mutex<f64>,
}

impl TabManager {
    pub fn new(
        app: AppHandle,
        window: Window,
        adblock: Arc<Adblock>,
        db: Arc<Mutex<Storage>>,
        beranda: Url,
    ) -> Self {
        let sidebar_terbuka = db
            .lock()
            .ok()
            .and_then(|db| db.get_pengaturan("sidebar").ok().flatten())
            .map(|nilai| nilai != "tutup")
            .unwrap_or(true);
        Self {
            app,
            window,
            isi: Mutex::new(Isi {
                tabs: Vec::new(),
                aktif: None,
                sidebar_terbuka,
            }),
            urutan: AtomicUsize::new(0),
            adblock,
            db,
            beranda,
            ditutup: Mutex::new(Vec::new()),
            zoom_aktif: Mutex::new(1.0),
            panel: Mutex::new(0.0),
        }
    }

    /// Membuat tab baru dan menjadikannya tab aktif.
    pub fn buka(&self, url: Url) -> Result<KeadaanTab, String> {
        let id = format!("tab-{}", self.urutan.fetch_add(1, Ordering::SeqCst) + 1);
        // Halaman `file://` tidak boleh tercatat sebagai riwayat atau sesi.
        let lokal = url.scheme() == "file";

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
                let adblock = self.adblock.clone();
                move |webview, payload| {
                    let url = payload.url().to_string();
                    match payload.event() {
                        PageLoadEvent::Started => {
                            // Scriptlet dijalankan sedini mungkin: sebagian
                            // bergantung pada pendengaran event halaman.
                            let skrip = adblock.skrip_untuk(&url);
                            if !skrip.is_empty() {
                                let _ = webview.eval(&skrip);
                            }
                            if let Some(tabs) = app.try_state::<TabManager>() {
                                tabs.terapkan_zoom_url(&url);
                            }
                        }
                        PageLoadEvent::Finished => {
                            catat_riwayat(&app, &url);
                        }
                    }
                    kirim(
                        &app,
                        PeristiwaNavigasi {
                            tab: id.clone(),
                            url: Some(url),
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
                        let url = if let Ok(mut isi) = tabs.isi.lock() {
                            let url = isi.tabs.iter_mut().find(|t| t.id == id).map(|tab| {
                                tab.title = judul.clone();
                                tab.url.clone()
                            });
                            if isi.aktif.as_deref() == Some(id.as_str()) {
                                url
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(url) = url {
                            catat_riwayat_judul(&app, &url, &judul);
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
            })
            .on_download({
                let app = self.app.clone();
                let db = self.db.clone();
                move |_webview, event| {
                    tangani_unduhan(&app, &db, event);
                    true
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
        // Di Linux ukuran di atas diabaikan (lihat src/platform/linux), jadi
        // batas asli selalu ditetapkan lewat helper platform.
        crate::platform::atur_batas(&webview, self.rect_konten()?)?;

        // Perlindungan dipasang sebelum halaman pertama dimuat bila filter sudah
        // siap. Kalau belum, `pasang_perlindungan` menyusul setelah selesai.
        if self
            .adblock
            .aktif_untuk(&adblock_core::host_dari(url.as_str()).unwrap_or_default())
            && crate::platform::filter_siap()
        {
            crate::platform::pasang_filter_ke(&webview);
        }

        let _ = webview.show();
        let _ = webview.set_focus();
        self.terapkan_zoom_url(url.as_str());

        let keadaan = KeadaanTab {
            id: id.clone(),
            url: url.to_string(),
            title: String::new(),
        };
        let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
        isi.tabs.push(keadaan.clone());
        isi.aktif = Some(id);
        drop(isi);

        if crate::platform::filter_siap()
            && !self
                .adblock
                .aktif_untuk(&adblock_core::host_dari(url.as_str()).unwrap_or_default())
        {
            crate::platform::lepas_filter(&webview);
        }
        if !lokal {
            self.simpan_sesi();
        }
        beri_tahu_daftar(&self.app);
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
            zoom: self.zoom(),
            tema: self.tema(),
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
        if let Ok(url) = webview.url() {
            self.terapkan_zoom_url(url.as_str());
        }
        self.simpan_sesi();
        beri_tahu_daftar(&self.app);
        Ok(())
    }

    /// Berpindah tab relatif terhadap tab aktif (dipakai Ctrl+Tab).
    pub fn pindah(&self, langkah: isize) -> Result<(), String> {
        let (tabs, aktif) = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            (
                isi.tabs.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
                isi.aktif.clone(),
            )
        };
        if tabs.len() < 2 {
            return Ok(());
        }
        let posisi = aktif
            .and_then(|id| tabs.iter().position(|t| *t == id))
            .unwrap_or(0) as isize;
        let baru = (posisi + langkah).rem_euclid(tabs.len() as isize) as usize;
        self.pilih(&tabs[baru])
    }

    /// Membuka tab baru di halaman beranda.
    pub fn tab_baru(&self) -> Result<KeadaanTab, String> {
        self.buka(self.beranda.clone())
    }

    /// Membuka kembali tab terakhir yang ditutup (Ctrl+Shift+T).
    pub fn buka_kembali(&self) -> Result<Option<KeadaanTab>, String> {
        let url = self
            .ditutup
            .lock()
            .map_err(|_| "stack tab tertutup rusak".to_string())?
            .pop();
        let Some(url) = url else {
            return Ok(None);
        };
        let url = Url::parse(&url).map_err(|e| e.to_string())?;
        self.buka(url).map(Some)
    }

    /// Menutup tab `id`. Kalau tab terakhir ditutup, tab baru langsung dibuka
    /// supaya jendela tidak pernah kosong.
    pub fn tutup(&self, id: &str) -> Result<(), String> {
        let (menutup_aktif, pengganti, terakhir, url_lama) = {
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
                isi.tabs[posisi].url.clone(),
            )
        };

        if let Some(webview) = self.app.get_webview(id) {
            webview.close().map_err(|e| e.to_string())?;
        }
        if let Ok(mut stack) = self.ditutup.lock() {
            if url_lama.starts_with("http") {
                stack.push(url_lama);
                if stack.len() > 25 {
                    stack.remove(0);
                }
            }
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
            self.tab_baru()?;
        }
        self.simpan_sesi();
        beri_tahu_daftar(&self.app);
        Ok(())
    }

    /// Membuka atau menciutkan sidebar vertikal.
    pub fn atur_sidebar(&self, terbuka: bool) -> Result<(), String> {
        {
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.sidebar_terbuka = terbuka;
        }
        if let Ok(db) = self.db.lock() {
            let _ = db.set_pengaturan("sidebar", if terbuka { "buka" } else { "tutup" });
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

    /// Memasang content blocker ke seluruh webview tab yang sudah ada.
    pub fn pasang_perlindungan(&self) {
        let ids: Vec<String> = match self.isi.lock() {
            Ok(isi) => isi.tabs.iter().map(|t| t.id.clone()).collect(),
            Err(_) => return,
        };
        for id in ids {
            if let Some(webview) = self.app.get_webview(&id) {
                let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
                let host = adblock_core::host_dari(&url).unwrap_or_default();
                if self.adblock.aktif_untuk(&host) {
                    crate::platform::pasang_filter_ke(&webview);
                } else {
                    crate::platform::lepas_filter(&webview);
                }
            }
        }
    }

    /// Menyalakan/mematikan perlindungan untuk situs satu tab, lalu menerapkannya
    /// ke semua tab di situs yang sama.
    pub fn atur_situs(&self, id: &str, aktif: bool) -> Result<Perlindungan, String> {
        let webview = self
            .app
            .get_webview(id)
            .ok_or_else(|| format!("webview tab {id} tidak ada"))?;
        let url = webview.url().map_err(|e| e.to_string())?.to_string();
        let host = adblock_core::host_dari(&url).unwrap_or_default();
        if host.is_empty() {
            return Err("halaman ini tidak punya host".into());
        }
        self.adblock.atur_situs(&host, aktif);

        let ids: Vec<String> = match self.isi.lock() {
            Ok(isi) => isi.tabs.iter().map(|t| t.id.clone()).collect(),
            Err(_) => Vec::new(),
        };
        let nonaktif = self.adblock.daftar_nonaktif();
        for tab_id in ids {
            if let Some(tab) = self.app.get_webview(&tab_id) {
                let Ok(tab_url) = tab.url() else { continue };
                let tab_host = adblock_core::host_dari(tab_url.as_str()).unwrap_or_default();
                if nonaktif.contains(&tab_host) {
                    crate::platform::lepas_filter(&tab);
                } else {
                    crate::platform::pasang_filter_ke(&tab);
                }
            }
        }
        self.simpan_pengaturan_perlindungan();
        // Perlindungan baru berlaku untuk request berikutnya.
        let _ = webview.reload();
        Ok(self.adblock.info(&url))
    }

    pub fn info_perlindungan(&self, id: Option<&str>) -> Perlindungan {
        let label = id
            .map(str::to_string)
            .or_else(|| self.label_aktif())
            .unwrap_or_default();
        let url = self
            .app
            .get_webview(&label)
            .and_then(|w| w.url().ok())
            .map(|u| u.to_string())
            .unwrap_or_default();
        self.adblock.info(&url)
    }

    pub fn simpan_pengaturan_perlindungan(&self) {
        let daftar: Vec<String> = self.adblock.daftar_nonaktif().into_iter().collect();
        if let Ok(db) = self.db.lock() {
            if let Ok(json) = serde_json::to_string(&daftar) {
                let _ = db.set_pengaturan("perlindungan.nonaktif", &json);
            }
        }
    }

    /// Zoom tab aktif.
    pub fn zoom(&self) -> f64 {
        self.zoom_aktif.lock().map(|z| *z).unwrap_or(1.0)
    }

    /// Mengatur zoom; host disimpan agar bertahan saat situs dibuka lagi.
    pub fn atur_zoom(&self, nilai: f64) -> Result<f64, String> {
        let nilai = nilai.clamp(0.3, 3.0);
        let webview = self
            .webview_aktif()
            .ok_or_else(|| "tidak ada tab aktif".to_string())?;
        webview.set_zoom(nilai).map_err(|e| e.to_string())?;
        if let Ok(mut slot) = self.zoom_aktif.lock() {
            *slot = nilai;
        }
        if let (Ok(url), Ok(db)) = (webview.url(), self.db.lock()) {
            let host = adblock_core::host_dari(url.as_str()).unwrap_or_default();
            let mut pengaturan = db
                .get_pengaturan("zoom")
                .ok()
                .flatten()
                .and_then(|json| serde_json::from_str::<PengaturanZoom>(&json).ok())
                .unwrap_or_default();
            pengaturan.per_host.insert(host, nilai);
            if let Ok(json) = serde_json::to_string(&pengaturan) {
                let _ = db.set_pengaturan("zoom", &json);
            }
        }
        Ok(nilai)
    }

    /// Menerapkan zoom tersimpan untuk host halaman yang baru dimuat.
    pub fn terapkan_zoom_url(&self, url: &str) {
        let host = adblock_core::host_dari(url).unwrap_or_default();
        let nilai = self
            .db
            .lock()
            .ok()
            .and_then(|db| db.get_pengaturan("zoom").ok().flatten())
            .and_then(|json| serde_json::from_str::<PengaturanZoom>(&json).ok())
            .and_then(|p| p.per_host.get(&host).copied());
        if let (Some(nilai), Some(webview)) = (nilai, self.webview_aktif()) {
            let _ = webview.set_zoom(nilai);
            if let Ok(mut slot) = self.zoom_aktif.lock() {
                *slot = nilai;
            }
        }
    }

    /// Tema UI: `sistem`, `terang`, atau `gelap`.
    pub fn tema(&self) -> String {
        self.db
            .lock()
            .ok()
            .and_then(|db| db.get_pengaturan("tema").ok().flatten())
            .unwrap_or_else(|| "sistem".into())
    }

    pub fn atur_tema(&self, tema: &str) -> Result<String, String> {
        let tema = match tema {
            "terang" | "gelap" => tema,
            _ => "sistem",
        };
        if let Ok(db) = self.db.lock() {
            db.set_pengaturan("tema", tema).map_err(|e| e.to_string())?;
        }
        Ok(tema.to_string())
    }

    /// Menyimpan daftar URL tab ke pengaturan, agar pulih setelah crash.
    fn simpan_sesi(&self) {
        let urls: Vec<String> = match self.isi.lock() {
            Ok(isi) => isi
                .tabs
                .iter()
                .filter(|t| t.url.starts_with("http"))
                .take(BATAS_SESI)
                .map(|t| t.url.clone())
                .collect(),
            Err(_) => return,
        };
        if let (Ok(db), Ok(json)) = (self.db.lock(), serde_json::to_string(&urls)) {
            let _ = db.set_pengaturan("sesi", &json);
        }
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

    /// Area isi tab: di bawah chrome, di kanan sidebar, dikurangi area panel.
    pub fn rect_konten(&self) -> Result<Rect, String> {
        let (lebar, tinggi) = self.ukuran_logis()?;
        let awal = self.sidebar_lebar()?;
        let panel = self.panel_lebar();
        Ok(Rect {
            position: LogicalPosition::new(awal, TINGGI_CHROME).into(),
            size: LogicalSize::new(
                (lebar - awal - panel).max(1.0),
                (tinggi - TINGGI_CHROME).max(1.0),
            )
            .into(),
        })
    }

    /// Memesan ruang di kanan untuk panel UI (bookmark, unduhan, dll).
    ///
    /// Child webview selalu di atas webview chrome, jadi panel yang melayang di
    /// atas konten akan tertutup. Dengan memesan ruang, area panel berada di
    /// luar batas webview tab dan tetap terlihat.
    pub fn atur_panel(&self, lebar: f64) -> Result<(), String> {
        {
            let mut slot = self
                .panel
                .lock()
                .map_err(|_| "state panel rusak".to_string())?;
            *slot = lebar.clamp(0.0, 640.0);
        }
        if let Some(webview) = self.webview_aktif() {
            crate::platform::atur_batas(&webview, self.rect_konten()?)?;
        }
        Ok(())
    }

    fn panel_lebar(&self) -> f64 {
        self.panel.lock().map(|slot| *slot).unwrap_or(0.0)
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

fn sekarang() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Mencatat kunjungan ke riwayat; halaman internal dan `file://` dilewati.
fn catat_riwayat(app: &AppHandle, url: &str) {
    if !url.starts_with("http") {
        return;
    }
    let judul = app
        .try_state::<TabManager>()
        .and_then(|tabs| {
            tabs.isi.lock().ok().and_then(|isi| {
                isi.tabs
                    .iter()
                    .find(|t| t.url == url)
                    .map(|t| t.title.clone())
            })
        })
        .unwrap_or_default();
    catat_riwayat_judul(app, url, &judul);
}

fn catat_riwayat_judul(app: &AppHandle, url: &str, judul: &str) {
    if !url.starts_with("http") {
        return;
    }
    if let Some(tabs) = app.try_state::<TabManager>() {
        if let Ok(db) = tabs.db.lock() {
            let _ = db.catat_kunjungan(url, judul, sekarang());
        }
    }
}

/// Menerima permintaan unduhan: tentukan tujuan, catat, dan beri tahu UI.
fn tangani_unduhan(app: &AppHandle, db: &Arc<Mutex<Storage>>, event: DownloadEvent<'_>) {
    match event {
        DownloadEvent::Requested { url, destination } => {
            let nama = nama_berkas(&url);
            let folder = app
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            // Jangan menimpa berkas lain dengan nama sama.
            let mut tujuan = folder.join(&nama);
            let mut nomor = 1;
            while tujuan.exists() {
                let batang = nama
                    .rsplit_once('.')
                    .map(|(a, b)| (a.to_string(), format!(".{b}")))
                    .unwrap_or_else(|| (nama.clone(), String::new()));
                tujuan = folder.join(format!("{} ({nomor}){}", batang.0, batang.1));
                nomor += 1;
            }
            *destination = tujuan.clone();

            if let Ok(db) = db.lock() {
                let _ = db.catat_unduhan(
                    url.as_str(),
                    &tujuan.to_string_lossy(),
                    &tujuan
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or(nama),
                    "berjalan",
                    sekarang(),
                );
            }
            beri_tahu_unduhan(app);
        }
        DownloadEvent::Finished { url, success, .. } => {
            if let Ok(db) = db.lock() {
                let _ = db.catat_unduhan(
                    url.as_str(),
                    "",
                    &nama_berkas(&url),
                    if success { "selesai" } else { "gagal" },
                    sekarang(),
                );
            }
            beri_tahu_unduhan(app);
        }
        // Varian baru di Tauri versi mendatang tidak boleh membuat build gagal.
        _ => {}
    }
}

/// Memberi tahu UI bahwa daftar tab berubah (mis. dari pintasan keyboard).
fn beri_tahu_daftar(app: &AppHandle) {
    let target = EventTarget::Webview {
        label: LABEL_CHROME.to_string(),
    };
    let _ = app.emit_to(target, EVENT_DAFTAR, ());
}

fn beri_tahu_unduhan(app: &AppHandle) {
    let target = EventTarget::Webview {
        label: LABEL_CHROME.to_string(),
    };
    let _ = app.emit_to(target, EVENT_UNDUHAN, ());
}

/// Nama berkas dari URL, dengan pengaman untuk URL tanpa nama.
fn nama_berkas(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segmen| segmen.next_back())
        .filter(|nama| !nama.is_empty() && !nama.contains('/'))
        .map(|nama| {
            percent_encoding::percent_decode_str(nama)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "unduhan".to_string())
}

#[cfg(test)]
mod tests {
    use super::{nama_berkas, tab_pengganti};
    use tauri::Url;

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

    #[test]
    fn nama_berkas_dari_url() {
        let url = Url::parse("https://example.com/unduh/berkas%20saya.zip").unwrap();
        assert_eq!(nama_berkas(&url), "berkas saya.zip");
        let url = Url::parse("https://example.com/").unwrap();
        assert_eq!(nama_berkas(&url), "unduhan");
    }
}
