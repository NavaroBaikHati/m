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

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
pub const TINGGI_CHROME: f64 = 40.0;

/// Jeda sebelum peniduran tab dikerjakan, supaya tidak bertabrakan dengan
/// pembukaan halaman yang baru saja diminta.
const JEDA_TIDUR: Duration = Duration::from_millis(1500);

/// Lebar sidebar vertikal saat terbuka, dalam logical pixel.
/// Harus sama dengan variabel `--lebar-sidebar` di `ui/src/app.css`.
pub const LEBAR_SIDEBAR: f64 = 224.0;

/// Label webview panel melayang (pustaka, unduhan, menu, perlindungan).
pub const LABEL_PANEL: &str = "panel";

/// Lebar panel melayang; pustaka butuh lebih banyak ruang untuk daftar URL.
pub const LEBAR_PANEL: f64 = 344.0;
pub const LEBAR_PANEL_PUSTAKA: f64 = 376.0;

/// Jarak panel dari tepi kanan, atas (di bawah toolbar), dan bawah jendela.
pub const JARAK_PANEL: f64 = 10.0;

/// Pilihan panel yang dikenal backend.
const JENIS_PANEL: &[&str] = &["perlindungan", "pustaka", "unduhan", "menu"];

/// Panel dibuka/ditutup; `jenis` kosong berarti ditutup.
pub const EVENT_PANEL: &str = "panel:jenis";

/// Nama event yang dipakai Rust untuk memberi tahu webview chrome.
pub const EVENT_NAVIGASI: &str = "tab:navigasi";
pub const EVENT_UNDUHAN: &str = "unduhan:berubah";
/// Kemajuan muat halaman (0.0–1.0), dipakai garis progres di bawah toolbar.
pub const EVENT_PROGRES: &str = "tab:progres";
/// Halaman masuk/keluar layar penuh; chrome UI ikut disembunyikan.
pub const EVENT_LAYAR_PENUH: &str = "tab:layar-penuh";
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
    /// Webview tab ini dilepas untuk menghemat memori (lihat
    /// [`TabManager::tidurkan_satu_tab`]); URL di atas tetap bisa dibuka lagi.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub tidur: bool,
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
struct PeristiwaProgres {
    tab: String,
    progres: f64,
}

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
    /// Jenis panel melayang yang sedang terbuka (`None` = tertutup).
    panel: Mutex<Option<String>>,
    /// Tab aktif sedang memutar video layar penuh (chrome disembunyikan).
    layar_penuh: Mutex<bool>,
    /// Tab yang sedang mengeluarkan suara, diperbarui dari sinyal WebKit
    /// `is-playing-audio`. Dipakai agar peniduran tab tidak memotong musik.
    suara: Mutex<HashSet<String>>,
    /// Urutan tab dipakai, terlama di depan. Peniduran tab memakai urutan ini
    /// supaya yang paling lama tidak disentuh yang tidur lebih dulu.
    urutan_pakai: Mutex<Vec<String>>,
    /// Riwayat dan sesi ditulis di thread sendiri; main thread tidak menunggu
    /// disk (lihat `src/penulis.rs`).
    penulis: crate::penulis::Penulis,
    /// Peniduran tab sedang dijadwalkan atau berjalan di latar belakang.
    tidur_berjalan: std::sync::atomic::AtomicBool,
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
            penulis: crate::penulis::Penulis::mulai(db.clone()),
            db,
            beranda,
            ditutup: Mutex::new(Vec::new()),
            zoom_aktif: Mutex::new(1.0),
            panel: Mutex::new(None),
            layar_penuh: Mutex::new(false),
            suara: Mutex::new(HashSet::new()),
            urutan_pakai: Mutex::new(Vec::new()),
            tidur_berjalan: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Mencatat tab `id` sebagai yang paling baru dipakai.
    fn tandai_dipakai(&self, id: &str) {
        if let Ok(mut urutan) = self.urutan_pakai.lock() {
            urutan.retain(|lain| lain != id);
            urutan.push(id.to_string());
        }
    }

    /// Mencatat perubahan status suara satu tab (dipanggil `platform::pantau`).
    pub fn catat_suara(&self, id: &str, memutar: bool) {
        if let Ok(mut suara) = self.suara.lock() {
            if memutar {
                suara.insert(id.to_string());
            } else {
                suara.remove(id);
            }
        }
        // Tab yang baru mulai bersuara harus segera dianggap terlihat, kalau
        // tidak situsnya sendiri yang mematikan pemutaran karena "tersembunyi".
        self.segarkan_visibilitas();
    }

    /// Apakah tab ini sedang memutar suara? Kalau statusnya tidak diketahui,
    /// dianggap memutar: lebih baik tidak menidurkan tab daripada memotong
    /// musik yang sedang diputar.
    fn sedang_putar_suara(&self, id: &str) -> bool {
        self.suara
            .lock()
            .map(|suara| suara.contains(id))
            .unwrap_or(true)
    }

    /// Membuat tab baru dan menjadikannya tab aktif.
    pub fn buka(&self, url: Url) -> Result<KeadaanTab, String> {
        let _kerja = crate::macet::kerja("buka-tab");
        let id = format!("tab-{}", self.urutan.fetch_add(1, Ordering::SeqCst) + 1);
        // Halaman `file://` tidak boleh tercatat sebagai riwayat atau sesi.
        let lokal = url.scheme() == "file";

        self.bangun_webview(&id, &url)?;

        let keadaan = KeadaanTab {
            id: id.clone(),
            url: url.to_string(),
            title: String::new(),
            tidur: false,
        };
        {
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.tabs.push(keadaan.clone());
            isi.aktif = Some(id);
        }

        self.tandai_dipakai(&keadaan.id);
        if !lokal {
            self.simpan_sesi();
        }
        self.segarkan_visibilitas();
        beri_tahu_daftar(&self.app);
        self.jadwalkan_tidur();
        Ok(keadaan)
    }

    /// Menyalakan atau mematikan paksaan "tab ini tetap dianggap terlihat".
    ///
    /// Lihat `src/latar.rs` untuk alasannya.
    fn paksa_terlihat(&self, id: &str, nyala: bool) {
        let nyala = nyala && crate::latar::aktif();
        if let Some(webview) = self.app.get_webview(id) {
            let _ = webview.eval(crate::latar::perintah(nyala));
        }
    }

    /// Apakah tab ini sedang benar-benar mengeluarkan suara (tanpa menebak).
    fn sedang_putar_suara_pasti(&self, id: &str) -> bool {
        self.suara
            .lock()
            .map(|suara| suara.contains(id))
            .unwrap_or(false)
    }

    /// Menyelaraskan skrip visibilitas di seluruh tab.
    ///
    /// Dua jenis tab tetap dianggap terlihat: tab aktif (jendela yang
    /// di-minimize atau ditinggal ke workspace lain tidak boleh menghentikan
    /// halaman yang sedang dipakai) dan tab yang sedang bersuara (musik latar
    /// tetap jalan saat pengguna pindah tab, seperti browser lain). Tab lain
    /// dibiarkan tahu ia tersembunyi supaya animasinya berhenti dan CPU tetap
    /// lega — mesin 1,7 GB tidak kuat menyalakan semua tab sekaligus.
    fn segarkan_visibilitas(&self) {
        let (tabs, aktif) = match self.isi.lock() {
            Ok(isi) => (
                isi.tabs.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
                isi.aktif.clone(),
            ),
            Err(_) => return,
        };
        for id in tabs {
            let terlihat =
                aktif.as_deref() == Some(id.as_str()) || self.sedang_putar_suara_pasti(&id);
            self.paksa_terlihat(&id, terlihat);
        }
    }

    /// Membuat webview untuk tab `id` dengan URL `url`.
    ///
    /// Dipakai untuk tab baru maupun untuk membangunkan tab yang ditidurkan:
    /// keduanya sama-sama butuh WebKit, pemantau layar penuh, dan content
    /// blocker yang terpasang sebelum halaman pertama dimuat.
    fn bangun_webview(&self, id: &str, url: &Url) -> Result<Webview, String> {
        let id = id.to_string();
        let url = url.clone();

        let builder = WebviewBuilder::new(id.clone(), WebviewUrl::External(url.clone()))
            .on_navigation({
                let app = self.app.clone();
                let id = id.clone();
                move |url| {
                    // Hanya URL yang dilaporkan di sini. Status "sedang
                    // memuat" sengaja tidak: event navigasi ikut menyala untuk
                    // iframe, jadi memakainya membuat garis progres dan tombol
                    // berhenti menyala selama iframe mana pun belum selesai.
                    kirim(
                        &app,
                        PeristiwaNavigasi {
                            tab: id.clone(),
                            url: Some(url.to_string()),
                            title: None,
                            memuat: None,
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
                            // Dokumen baru mulai dengan paksaan mati (variabel
                            // globalnya baru), jadi selaraskan lagi.
                            if let Some(tabs) = app.try_state::<TabManager>() {
                                tabs.segarkan_visibilitas();
                            }
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
            // Diputuskan sebelum `hide`: tab yang bersuara tetap dianggap
            // terlihat, tab sunyi menerima "hidden" sehingga animasinya berhenti.
            self.paksa_terlihat(&lama, self.sedang_putar_suara_pasti(&lama));
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
        // Webview baru ditaruh paling atas di wadah; panel melayang harus
        // kembali menimpa halaman.
        if self.panel_terbuka() {
            if let Some(panel) = self.app.get_webview(LABEL_PANEL) {
                let _ = crate::platform::naikkan(&panel);
            }
        }
        // Layar penuh HTML5 dan progres muat hanya bisa dipantau dari webview
        // platform; helper ini no-op di sistem operasi lain.
        crate::platform::pantau(&webview, self.app.clone(), id.clone())?;

        // Perlindungan dipasang sebelum halaman pertama dimuat bila filter sudah
        // siap. Kalau belum, `pasang_perlindungan` menyusul setelah selesai.
        if crate::platform::filter_siap() {
            if self
                .adblock
                .aktif_untuk(&adblock_core::host_dari(url.as_str()).unwrap_or_default())
            {
                crate::platform::pasang_filter_ke(&webview);
            } else {
                crate::platform::lepas_filter(&webview);
            }
        }

        // Skrip visibilitas harus terpasang sebelum dokumen pertama dimuat,
        // jadi dipasang di sini — bukan di `pantau` yang berjalan setelahnya.
        crate::platform::pasang_skrip_latar(&webview);

        let _ = webview.show();
        let _ = webview.set_focus();
        self.terapkan_zoom_url(url.as_str());
        Ok(webview)
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
        // Berpindah tab saat video layar penuh harus mengembalikan chrome,
        // kalau tidak toolbar tetap tersembunyi di tab berikutnya.
        if self.layar_penuh.lock().map(|slot| *slot).unwrap_or(false) {
            self.keluar_layar_penuh()?;
        }
        // Panel melayang milik tab sebelumnya; tutup sebelum pindah.
        self.tutup_panel()?;
        let (lama, url_tersimpan) = {
            let isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            let tab = isi
                .tabs
                .iter()
                .find(|tab| tab.id == id)
                .ok_or_else(|| format!("tab {id} tidak ada"))?;
            (isi.aktif.clone(), tab.url.clone())
        };

        let webview = match self.app.get_webview(id) {
            Some(webview) => webview,
            None => {
                // Tab yang ditidurkan kehilangan webview-nya (memori), tetapi
                // URL-nya tersimpan: bangun lagi dari situ. Halaman memang
                // dimuat ulang, dan hanya terjadi saat memori sedang sempit.
                let url = Url::parse(&url_tersimpan)
                    .map_err(|e| format!("URL tab {id} tidak bisa dibuka lagi: {e}"))?;
                let webview = self.bangun_webview(id, &url)?;
                if let Ok(mut isi) = self.isi.lock() {
                    if let Some(tab) = isi.tabs.iter_mut().find(|tab| tab.id == id) {
                        tab.tidur = false;
                    }
                }
                webview
            }
        };

        if lama.as_deref() != Some(id) {
            if let Some(lama) = lama.as_deref() {
                // Sama seperti di `bangun_webview`: putuskan sebelum hide.
                self.paksa_terlihat(lama, self.sedang_putar_suara_pasti(lama));
                if let Some(lama) = self.app.get_webview(lama) {
                    let _ = lama.hide();
                }
            }
            let mut isi = self.isi.lock().map_err(|_| "state tab rusak".to_string())?;
            isi.aktif = Some(id.to_string());
        }

        crate::platform::atur_batas(&webview, self.rect_konten()?)?;
        // Sebelum `show`: event "terlihat lagi" yang dikirim WebKit nanti
        // memang tidak ditahan, tetapi flag ini juga yang menentukan perilaku
        // halaman kalau ia memeriksa keadaannya sendiri lebih dulu.
        self.paksa_terlihat(id, true);
        webview.show().map_err(|e| e.to_string())?;
        self.segarkan_visibilitas();
        let _ = webview.set_focus();
        if let Ok(url) = webview.url() {
            self.terapkan_zoom_url(url.as_str());
        }
        self.tandai_dipakai(id);
        self.simpan_sesi();
        beri_tahu_daftar(&self.app);
        self.jadwalkan_tidur();
        Ok(())
    }

    /// Menidurkan tab yang tidak aktif saat memori tersedia menipis.
    ///
    /// WebKitGTK memakai satu proses untuk setiap halaman, jadi tab yang cuma
    /// menempel di sidebar tetap memakan 80–200 MB. Pada mesin 1–2 GB, membuka
    /// beberapa tab saja sudah cukup untuk memaksa kernel menukar halaman ke
    /// disk — dan setelah itu semua terasa lambat, bukan hanya tab baru.
    ///
    /// Tab yang ditidurkan melepas webview-nya (proses WebKit ikut berhenti)
    /// tetapi tetap ada di daftar dengan URL terakhir; memilihnya akan memuat
    /// ulang halaman. Tab yang sedang memutar suara dilewati supaya musik dari
    /// tab lain tidak terpotong. Kalau memori sudah longgar, fungsi ini tidak
    /// melakukan apa pun.
    ///
    /// Ambang diatur `PERISAI_TIDUR_MB` (bawaan 300, `0` mematikan).
    ///
    /// **Satu tab per pemanggilan, dengan sengaja.** `webview.close()`
    /// menghancurkan widget GTK lalu menghentikan proses WebKit-nya, dan semua
    /// itu di main thread. Dulu keempat tab dilepas dalam satu closure; di
    /// mesin 1,7 GB itu terukur menahan jendela lebih dari dua detik. Sekarang
    /// [`Self::jadwalkan_tidur`] memanggil ini sekali, membiarkan main thread
    /// bernapas, lalu menjadwalkan putaran berikutnya kalau memori masih
    /// sempit. Mengembalikan `true` kalau ada tab yang ditidurkan.
    ///
    /// Jangan dipanggil langsung di tengah pembukaan tab; gunakan
    /// [`Self::jadwalkan_tidur`].
    pub fn tidurkan_satu_tab(&self) -> bool {
        let Some(ambang) = crate::memori::ambang_tidur() else {
            return false;
        };
        if crate::memori::tersedia_mb().unwrap_or(u64::MAX) >= ambang {
            return false;
        }
        let kandidat: Vec<String> = match self.isi.lock() {
            Ok(isi) => {
                // Urutkan berdasarkan kapan terakhir dipakai: tab yang paling
                // lama tidak disentuh ditidurkan lebih dulu, sehingga tab yang
                // barusan ditinggal masih hangat kalau pengguna kembali.
                let mut urutan = self
                    .urutan_pakai
                    .lock()
                    .map(|u| u.clone())
                    .unwrap_or_default();
                let hidup: Vec<String> = isi
                    .tabs
                    .iter()
                    .filter(|tab| Some(tab.id.as_str()) != isi.aktif.as_deref())
                    .map(|tab| tab.id.clone())
                    .collect();
                urutan.retain(|id| hidup.contains(id));
                let mut hasil = urutan.clone();
                for id in hidup {
                    if !hasil.contains(&id) {
                        hasil.push(id);
                    }
                }
                hasil
            }
            Err(_) => return false,
        };

        for id in kandidat {
            let Some(webview) = self.app.get_webview(&id) else {
                continue;
            };
            if self.sedang_putar_suara(&id) {
                continue;
            }
            let url = webview.url().map(|u| u.to_string()).ok();
            if webview.close().is_err() {
                continue;
            }
            if let Ok(mut isi) = self.isi.lock() {
                if let Some(tab) = isi.tabs.iter_mut().find(|tab| tab.id == id) {
                    if let Some(url) = url {
                        tab.url = url;
                    }
                    tab.tidur = true;
                }
            }
            beri_tahu_daftar(&self.app);
            return true;
        }
        false
    }

    /// Apakah memori masih di bawah ambang tidur setelah satu tab dilepas?
    fn masih_sempit(&self) -> bool {
        crate::memori::ambang_tidur()
            .is_some_and(|ambang| crate::memori::tersedia_mb().unwrap_or(u64::MAX) < ambang)
    }

    /// Menjadwalkan peniduran tab setelah halaman baru tenang.
    ///
    /// Melepas webview menghancurkan widget GTK sekaligus menghentikan proses
    /// WebKit-nya. Dikerjakan tepat saat halaman baru dibuka, itu terasa
    /// sebagai jendela yang membeku — dan di mesin 1–2 GB ambangnya sering
    /// tersentuh justru ketika tab baru dibuka. Karena itu pekerjaannya ditunda
    /// sebentar dan hanya dijadwalkan sekali per gelombang.
    pub fn jadwalkan_tidur(&self) {
        if crate::memori::ambang_tidur().is_none() {
            return;
        }
        if self.tidur_berjalan.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = self.app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(JEDA_TIDUR);
            let untuk_main = app.clone();
            let hasil = app.run_on_main_thread(move || {
                if let Some(tabs) = untuk_main.try_state::<TabManager>() {
                    let _kerja = crate::macet::kerja("tidurkan-tab");
                    tabs.tidur_berjalan.store(false, Ordering::SeqCst);
                    // Masih sempit setelah satu tab dilepas: jadwalkan putaran
                    // berikutnya agar main thread sempat melayani klik dulu.
                    // Kalau tidak ada tab yang bisa ditidurkan lagi, berhenti —
                    // jangan menjadwalkan ulang selamanya.
                    if tabs.tidurkan_satu_tab() && tabs.masih_sempit() {
                        tabs.jadwalkan_tidur();
                    }
                }
            });
            if hasil.is_err() {
                // Jendela sudah ditutup; jangan biarkan penanda menyala
                // sehingga peniduran tidak pernah dijadwalkan lagi.
                if let Some(tabs) = app.try_state::<TabManager>() {
                    tabs.tidur_berjalan.store(false, Ordering::SeqCst);
                }
            }
        });
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
        // Tab yang sedang diputar layar penuh tidak mungkin dipakai lagi
        // setelah ditutup, jadi chrome harus muncul kembali.
        if self.layar_penuh.lock().map(|slot| *slot).unwrap_or(false) {
            self.keluar_layar_penuh()?;
        }
        self.simpan_sesi();
        self.segarkan_visibilitas();
        beri_tahu_daftar(&self.app);
        Ok(())
    }

    /// Mengembalikan tampilan normal tanpa menunggu sinyal `leave-fullscreen`.
    fn keluar_layar_penuh(&self) -> Result<(), String> {
        if let Ok(mut slot) = self.layar_penuh.lock() {
            *slot = false;
        }
        let _ = self.window.set_fullscreen(false);
        if let Some(chrome) = self.app.get_webview(LABEL_CHROME) {
            crate::platform::atur_batas(&chrome, self.rect_penuh()?)?;
            let _ = chrome.show();
        }
        if let Some(webview) = self.webview_aktif() {
            crate::platform::atur_batas(&webview, self.rect_konten()?)?;
            let _ = webview.show();
            let _ = webview.set_focus();
        }
        let target = EventTarget::Webview {
            label: LABEL_CHROME.to_string(),
        };
        let _ = self.app.emit_to(target, EVENT_LAYAR_PENUH, false);
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
        // Panel melayang menempel ke tepi kanan, jadi ikut bergeser saat
        // jendela berubah ukuran.
        let jenis = self.panel.lock().ok().and_then(|slot| slot.clone());
        if let (Some(jenis), Some(panel)) = (jenis, self.app.get_webview(LABEL_PANEL)) {
            crate::platform::atur_batas(&panel, self.rect_panel(&jenis)?)?;
            crate::platform::naikkan(&panel)?;
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
            // Bawaan gelap: palet monokrom Perisai dirancang untuk latar hitam.
            .unwrap_or_else(|| "gelap".into())
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
        if let Ok(json) = serde_json::to_string(&urls) {
            self.penulis.pengaturan("sesi", &json);
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

    /// Area isi tab: di bawah chrome dan di kanan sidebar.
    ///
    /// Saat halaman meminta layar penuh, area isi memakai seluruh jendela dan
    /// chrome UI disembunyikan, jadi video tidak lagi terpotong toolbar.
    ///
    /// Panel melayang tidak lagi memotong area ini: panel adalah webview
    /// sendiri yang digambar di atasnya, jadi halaman tetap selebar jendela
    /// seperti browser lain.
    pub fn rect_konten(&self) -> Result<Rect, String> {
        let layar_penuh = self.layar_penuh.lock().map(|slot| *slot).unwrap_or(false);
        if layar_penuh {
            return self.rect_penuh();
        }
        let (lebar, tinggi) = self.ukuran_logis()?;
        let awal = self.sidebar_lebar()?;
        Ok(Rect {
            position: LogicalPosition::new(awal, TINGGI_CHROME).into(),
            size: LogicalSize::new((lebar - awal).max(1.0), (tinggi - TINGGI_CHROME).max(1.0))
                .into(),
        })
    }

    /// Area panel melayang: menempel ke tepi kanan, di bawah toolbar.
    fn rect_panel(&self, jenis: &str) -> Result<Rect, String> {
        let (lebar, tinggi) = self.ukuran_logis()?;
        let lebar_panel = match jenis {
            "pustaka" => LEBAR_PANEL_PUSTAKA,
            _ => LEBAR_PANEL,
        };
        let y = TINGGI_CHROME + JARAK_PANEL;
        Ok(Rect {
            position: LogicalPosition::new((lebar - lebar_panel - JARAK_PANEL).max(0.0), y).into(),
            size: LogicalSize::new(lebar_panel, (tinggi - y - JARAK_PANEL).max(1.0)).into(),
        })
    }

    /// Membuka panel melayang `jenis` di atas halaman.
    ///
    /// Panel adalah child webview tersendiri yang dimuat dari `panel.html`.
    /// Ini satu-satunya cara membuatnya benar-benar melayang: webview tab
    /// selalu berada di atas webview chrome, jadi panel yang dirender di
    /// chrome akan tertutup halaman. Ruang halaman tidak dikorbankan.
    pub fn buka_panel(&self, jenis: &str) -> Result<(), String> {
        if !JENIS_PANEL.contains(&jenis) {
            return Err(format!("panel {jenis} tidak dikenal"));
        }
        if self.layar_penuh.lock().map(|slot| *slot).unwrap_or(false) {
            return Ok(());
        }
        {
            let mut slot = self
                .panel
                .lock()
                .map_err(|_| "state panel rusak".to_string())?;
            *slot = Some(jenis.to_string());
        }

        let webview = self.wadah_panel()?;
        crate::platform::atur_batas(&webview, self.rect_panel(jenis)?)?;
        // Tab yang dibuat setelah panel bisa menimpanya; pastikan panel di atas.
        crate::platform::naikkan(&webview)?;
        webview.show().map_err(|e| e.to_string())?;
        let _ = webview.set_focus();
        let _ = self.app.emit_to(
            EventTarget::Webview {
                label: LABEL_PANEL.to_string(),
            },
            EVENT_PANEL,
            jenis,
        );
        Ok(())
    }

    /// Menutup panel melayang dan memberi tahu chrome agar sorotan tombolnya
    /// ikut padam.
    pub fn tutup_panel(&self) -> Result<(), String> {
        let terbuka = self
            .panel
            .lock()
            .map(|mut slot| slot.take().is_some())
            .unwrap_or(false);
        if let Some(webview) = self.app.get_webview(LABEL_PANEL) {
            let _ = webview.hide();
        }
        if terbuka {
            // Fokus kembali ke halaman supaya pintasan keyboard tetap jalan.
            if let Some(webview) = self.webview_aktif() {
                let _ = webview.set_focus();
            }
            let _ = self.app.emit_to(
                EventTarget::Webview {
                    label: LABEL_CHROME.to_string(),
                },
                EVENT_PANEL,
                "",
            );
        }
        Ok(())
    }

    /// Jenis panel yang sedang terbuka; kosong kalau tertutup.
    ///
    /// Dipakai webview panel saat baru dimuat: event `EVENT_PANEL` bisa tiba
    /// sebelum halaman panel sempat memasang pendengarnya, jadi ia menanyakan
    /// sendiri keadaan terakhir.
    pub fn jenis_panel(&self) -> String {
        let jenis = self
            .panel
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_default();
        jenis
    }

    pub fn panel_terbuka(&self) -> bool {
        self.panel
            .lock()
            .map(|slot| slot.is_some())
            .unwrap_or(false)
    }

    /// Webview panel, dibuat sekali lalu dipakai ulang.
    ///
    /// Latar transparan supaya sudut membulat kartu tidak menampilkan kotak
    /// gelap di atas halaman.
    fn wadah_panel(&self) -> Result<Webview, String> {
        if let Some(webview) = self.app.get_webview(LABEL_PANEL) {
            return Ok(webview);
        }
        let builder = WebviewBuilder::new(LABEL_PANEL, WebviewUrl::App("panel.html".into()))
            .transparent(true);
        let ukuran = LogicalSize::new(LEBAR_PANEL, 200.0);
        let webview = self
            .window
            .add_child(builder, LogicalPosition::new(0.0, 0.0), ukuran)
            .map_err(|e| format!("gagal membuat webview panel: {e}"))?;
        let _ = webview.hide();
        Ok(webview)
    }

    /// Masuk/keluar layar penuh untuk tab `id` (dipicu sinyal WebKit).
    ///
    /// WebKitGTK menyerahkan layar penuh HTML5 ke aplikasi: tanpa penanganan,
    /// video hanya memenuhi area isi tab dan toolbar tetap menutupi bagian
    /// atas. Di sini tab dinaikkan ke seluruh jendela dan webview chrome
    /// disembunyikan, jadi hasilnya layar penuh seperti browser lain.
    pub fn atur_layar_penuh(&self, id: &str, penuh: bool) -> Result<(), String> {
        {
            let mut slot = self
                .layar_penuh
                .lock()
                .map_err(|_| "state layar penuh rusak".to_string())?;
            *slot = penuh;
        }
        let Some(webview) = self.app.get_webview(id) else {
            return Ok(());
        };

        // Jendela ikut penuh layar supaya bilah judul sistem tidak tersisa di
        // atas video. WebKitGTK sendiri tidak melakukannya untuk kita.
        let _ = self.window.set_fullscreen(penuh);

        if penuh {
            // Panel melayang tidak boleh menutupi video layar penuh.
            let _ = self.tutup_panel();
            // Tab yang meminta layar penuh selalu jadi tab aktif.
            if self.label_aktif().as_deref() != Some(id) {
                if let Some(lama) = self.label_aktif() {
                    if let Some(lama) = self.app.get_webview(&lama) {
                        let _ = lama.hide();
                    }
                }
                if let Ok(mut isi) = self.isi.lock() {
                    isi.aktif = Some(id.to_string());
                }
            }
            crate::platform::atur_batas(&webview, self.rect_penuh()?)?;
            if let Some(chrome) = self.app.get_webview(LABEL_CHROME) {
                let _ = chrome.hide();
            }
            let _ = webview.show();
            let _ = webview.set_focus();
        } else {
            if let Some(chrome) = self.app.get_webview(LABEL_CHROME) {
                crate::platform::atur_batas(&chrome, self.rect_penuh()?)?;
                let _ = chrome.show();
            }
            crate::platform::atur_batas(&webview, self.rect_konten()?)?;
            let _ = webview.show();
            let _ = webview.set_focus();
        }

        let target = EventTarget::Webview {
            label: LABEL_CHROME.to_string(),
        };
        let _ = self.app.emit_to(target, EVENT_LAYAR_PENUH, penuh);
        beri_tahu_daftar(&self.app);
        Ok(())
    }

    /// Meneruskan keadaan "sedang memuat" tab `id` ke chrome UI.
    ///
    /// Dipanggil dari `platform::pantau` (sinyal `is-loading` WebKit), jadi
    /// nilainya mengikuti dokumen utama saja — iframe tidak ikut dihitung —
    /// dan ikut padam saat pemuatan dihentikan.
    pub fn laporkan_memuat(&self, id: &str, memuat: bool) {
        let target = EventTarget::Webview {
            label: LABEL_CHROME.to_string(),
        };
        let _ = self.app.emit_to(
            target,
            EVENT_NAVIGASI,
            PeristiwaNavigasi {
                tab: id.to_string(),
                url: None,
                title: None,
                memuat: Some(memuat),
            },
        );
    }

    /// Meneruskan kemajuan muat halaman ke chrome UI.
    pub fn laporkan_progres(&self, id: &str, progres: f64) {
        let target = EventTarget::Webview {
            label: LABEL_CHROME.to_string(),
        };
        let peristiwa = PeristiwaProgres {
            tab: id.to_string(),
            progres: progres.clamp(0.0, 1.0),
        };
        let _ = self.app.emit_to(target, EVENT_PROGRES, peristiwa);
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
        tabs.penulis.riwayat(url, judul, sekarang());
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
