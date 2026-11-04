//! Cangkang browser: window utama (chrome UI) + satu child webview untuk isi tab.

mod adblock;
mod beranda;
mod commands;
mod lingkungan;
mod pintasan;
mod platform;
mod tabs;
mod url_input;

use std::sync::{Arc, Mutex};

use storage::Storage;
use tauri::{Manager, Url, WindowEvent};

use adblock::Adblock;
use tabs::TabManager;

/// Akses basis data bersama untuk perintah UI dan tab manager.
pub struct Database(pub Arc<Mutex<Storage>>);

/// Menjalankan aplikasi.
///
/// Argumen pertama yang bukan flag diperlakukan sebagai URL/alamat awal, jadi
/// `cargo tauri dev -- http://localhost:8123/` bisa dipakai untuk pengujian.
pub fn run() {
    // Harus sebelum GTK/WebKit diinisialisasi: memperbaiki variabel lingkungan
    // AppImage yang membuat semua sertifikat HTTPS ditolak.
    lingkungan::rapikan();

    tauri::Builder::default()
        .setup(|app| {
            let window = app
                .get_window("main")
                .ok_or("window \"main\" tidak ada di tauri.conf.json")?;

            let folder_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&folder_data)?;
            let storage = Storage::open(folder_data.join("perisai.sqlite"))
                .map_err(|e| format!("basis data gagal dibuka: {e}"))?;
            let db = Arc::new(Mutex::new(storage));

            let adblock = Adblock::baru(app.handle());
            if let Some(daftar) = db
                .lock()
                .ok()
                .and_then(|db| db.get_pengaturan("perlindungan.nonaktif").ok().flatten())
                .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
            {
                adblock.atur_nonaktif(daftar.into_iter().collect());
            }

            // Mesin pencari harus diketahui sebelum halaman tab baru ditulis.
            if let Some(slug) = db
                .lock()
                .ok()
                .and_then(|db| db.get_pengaturan("mesin_pencarian").ok().flatten())
            {
                url_input::atur_mesin(&slug);
            }

            let beranda_url = beranda::siapkan(app.handle())?;
            let tabs = TabManager::new(
                app.handle().clone(),
                window.clone(),
                adblock.clone(),
                db.clone(),
                beranda_url.clone(),
            );
            // Di Linux, webview chrome dipindahkan ke GtkFixed milik kita agar
            // tata letak absolut bisa dipakai (lihat src/platform/linux).
            platform::siapkan_wadah(app.handle(), tabs.rect_penuh()?)?;

            let awal = url_awal().unwrap_or_else(|| beranda_url.clone());
            let sesi = if awal == beranda_url {
                pulihkan_sesi(&db)
            } else {
                Vec::new()
            };
            let jalankan_sesi = !sesi.is_empty();
            app.manage(tabs);
            app.manage(Database(db.clone()));

            {
                let tabs = app.state::<TabManager>();
                if jalankan_sesi {
                    for url in sesi {
                        let _ = tabs.buka(url);
                    }
                } else {
                    tabs.buka(awal)?;
                }
            }

            platform::pasang_pintasan(app.handle())?;

            // Daftar filter dimuat di latar belakang; jendela tidak menunggu.
            // Kait diagnostik: `PERISAI_TANPA_ADBLOCK=1` melewati perlindungan
            // sepenuhnya, untuk memisahkan masalah halaman dari content blocker.
            if std::env::var_os("PERISAI_TANPA_ADBLOCK").is_none() {
                adblock.mulai(app.handle().clone());
            }

            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                ) {
                    if let Some(tabs) = handle.try_state::<TabManager>() {
                        if let Err(e) = tabs.ukuran_berubah() {
                            eprintln!("gagal menyesuaikan ukuran tab: {e}");
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::daftar_tab,
            commands::tab_baru,
            commands::pilih_tab,
            commands::tutup_tab,
            commands::atur_sidebar,
            commands::navigasi,
            commands::mundur,
            commands::maju,
            commands::muat_ulang,
            commands::status_halaman,
            commands::atur_panel,
            commands::atur_perlindungan,
            commands::atur_zoom,
            commands::atur_tema,
            commands::daftar_mesin_pencarian,
            commands::atur_mesin_pencarian,
            commands::daftar_bookmark,
            commands::toggle_bookmark,
            commands::daftar_riwayat,
            commands::cari_riwayat,
            commands::hapus_riwayat,
            commands::daftar_unduhan,
        ])
        .run(tauri::generate_context!())
        .expect("aplikasi gagal dijalankan");
}

/// URL dari argumen baris perintah, kalau ada.
fn url_awal() -> Option<Url> {
    std::env::args()
        .skip(1)
        .find(|argumen| !argumen.starts_with('-'))
        .and_then(|argumen| url_input::dari_input(&argumen))
}

/// Membaca tab yang tersisa dari sesi terakhir, tanpa membuka halaman internal.
fn pulihkan_sesi(db: &Arc<Mutex<Storage>>) -> Vec<Url> {
    db.lock()
        .ok()
        .and_then(|db| db.get_pengaturan("sesi").ok().flatten())
        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
        .map(|daftar| {
            daftar
                .into_iter()
                .filter(|url| url.starts_with("http"))
                .filter_map(|url| Url::parse(&url).ok())
                .take(tabs::BATAS_PULIH)
                .collect()
        })
        .unwrap_or_default()
}
