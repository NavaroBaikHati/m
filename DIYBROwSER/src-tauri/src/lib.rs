//! Cangkang browser: window utama (chrome UI) + satu child webview untuk isi tab.

mod adblock;
mod beranda;
mod commands;
mod lingkungan;
mod memori;
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
            commands::hentikan,
            commands::status_halaman,
            commands::buka_panel,
            commands::tutup_panel,
            commands::jenis_panel,
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
    if std::env::args().any(|argumen| argumen == "--diagnostik") {
        return halaman_diagnostik();
    }
    std::env::args()
        .skip(1)
        .find(|argumen| !argumen.starts_with('-'))
        .and_then(|argumen| url_input::dari_input(&argumen))
}

/// Halaman diagnostik media bawaan (`--diagnostik`).
///
/// Ketika video menolak diputar, pertanyaan pertamanya selalu "codec apa yang
/// sebenarnya didukung WebKit di mesin ini?". Halaman ini menanyakan hal itu ke
/// mesin WebKit yang sedang berjalan — MediaSource, `canPlayType`, EME — dan
/// menampilkan jawabannya. Versi AppImage yang lupa membawa plugin GStreamer
/// dulunya melaporkan semua codec `false`, dan halaman inilah yang membuatnya
/// kelihatan.
const HALAMAN_DIAGNOSTIK: &str = include_str!("../../scripts/smoke/kodek.html");

/// Setelan mesin yang hanya diketahui proses induk, ditempel ke halaman
/// diagnostik supaya laporan pengguna memuat jalur render yang sebenarnya.
fn info_mesin() -> serde_json::Value {
    let env = |nama: &str| std::env::var(nama).ok().filter(|nilai| !nilai.is_empty());
    let akselerasi = match env("PERISAI_AKSELERASI").as_deref() {
        Some("selalu") => "selalu".to_string(),
        Some("perangkat-lunak") => "perangkat-lunak".to_string(),
        _ => "otomatis (OnDemand)".to_string(),
    };
    let dmabuf = if env("WEBKIT_DISABLE_DMABUF_RENDERER").is_some() {
        "dimatikan (jalur SHM)"
    } else {
        "aktif"
    };
    let sesi = match env("XDG_SESSION_TYPE").as_deref() {
        Some("wayland") => "wayland".to_string(),
        Some("x11") => "x11".to_string(),
        _ if env("WAYLAND_DISPLAY").is_some() => "wayland (WAYLAND_DISPLAY)".to_string(),
        _ => "x11".to_string(),
    };
    // WebKit bisa berasal dari pustaka sistem atau dari dalam AppImage; bedanya
    // penting saat membandingkan kecepatan.
    let webkit = std::fs::read_to_string("/proc/self/maps")
        .ok()
        .and_then(|peta| {
            peta.lines()
                .find(|baris| baris.contains("libwebkit2gtk-4.1.so"))
                .and_then(|baris| baris.split_whitespace().last())
                .map(|jalur| {
                    if jalur.contains("AppImage") || jalur.contains("/tmp/.mount") {
                        format!("bundel AppImage ({jalur})")
                    } else {
                        format!("pustaka sistem ({jalur})")
                    }
                })
        })
        .unwrap_or_else(|| "tidak diketahui".to_string());

    // WebKit menyamarkan nama GPU di WebGL (selalu "Apple GPU"), jadi jalur
    // render dinilai dari perangkat DRM yang tersedia. Tanpa `/dev/dri`,
    // seluruh cat dilakukan CPU dan halaman pasti terasa berat.
    let gpu = std::fs::read_dir("/dev/dri")
        .map(|isi| {
            let mut simpul: Vec<String> = isi
                .flatten()
                .map(|entri| entri.file_name().to_string_lossy().into_owned())
                .filter(|nama| nama.starts_with("render") || nama.starts_with("card"))
                .collect();
            simpul.sort();
            if simpul.is_empty() {
                "tidak ada perangkat DRM (semua cat di CPU)".to_string()
            } else {
                format!("/dev/dri/{{{}}} naik/turun", simpul.join(","))
            }
        })
        .unwrap_or_else(|_| "tidak ada /dev/dri (semua cat di CPU)".to_string());
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|isi| {
            isi.lines()
                .find(|baris| baris.starts_with("model name"))
                .and_then(|baris| baris.split_once(':'))
                .map(|(_, nilai)| nilai.trim().to_string())
        })
        .unwrap_or_else(|| "tidak diketahui".to_string());

    serde_json::json!({
        "versi": env!("CARGO_PKG_VERSION"),
        "webkit": webkit,
        "gpu": gpu,
        "cpu": cpu,
        "akselerasi": akselerasi,
        "dmabuf": dmabuf,
        "gdk": env("GDK_BACKEND").unwrap_or_else(|| "bawaan".to_string()),
        "sesi": sesi,
        "appimage": if env("APPDIR").is_some() { "ya" } else { "tidak" },
        "memori": format!(
            "{} MB tersedia; tab tidur di bawah {} MB",
            crate::memori::tersedia_mb()
                .map(|mb| mb.to_string())
                .unwrap_or_else(|| "?".into()),
            crate::memori::ambang_tidur()
                .map(|mb| mb.to_string())
                .unwrap_or_else(|| "mati".into()),
        ),
    })
}

fn halaman_diagnostik() -> Option<Url> {
    let folder = std::env::temp_dir().join("perisai-diagnostik");
    std::fs::create_dir_all(&folder).ok()?;
    let berkas = folder.join("diagnostik.html");
    // `/*MESIN*/null` adalah penanda di halaman; tanpa penggantian halaman tetap
    // bisa dibuka sebagai berkas biasa.
    let isi = HALAMAN_DIAGNOSTIK.replace("/*MESIN*/null", &format!("/*MESIN*/{}", info_mesin()));
    std::fs::write(&berkas, isi).ok()?;
    Url::parse(&format!("file://{}", berkas.display())).ok()
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
