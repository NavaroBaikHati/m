//! Cangkang browser: window utama (chrome UI) + satu child webview untuk isi tab.

mod commands;
mod platform;
mod tabs;
mod url_input;

use tauri::{Manager, Url, WindowEvent};

use tabs::TabManager;

/// Menjalankan aplikasi.
///
/// Argumen pertama yang bukan flag diperlakukan sebagai URL/alamat awal, jadi
/// `cargo tauri dev -- http://localhost:8123/` bisa dipakai untuk pengujian.
pub fn run() {
    let url_awal = url_awal();

    tauri::Builder::default()
        .setup(move |app| {
            let window = app
                .get_window("main")
                .ok_or("window \"main\" tidak ada di tauri.conf.json")?;

            let tabs = TabManager::new(app.handle().clone(), window.clone());
            // Di Linux, webview chrome dipindahkan ke GtkFixed milik kita agar
            // tata letak absolut bisa dipakai (lihat src/platform/linux.rs).
            platform::siapkan_wadah(app.handle(), tabs.rect_penuh()?)?;
            tabs.buka(url_awal.clone())?;

            // Child webview tidak ikut menyesuaikan diri saat window berubah ukuran.
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

            app.manage(tabs);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::tab_aktif,
            commands::navigasi,
            commands::mundur,
            commands::maju,
            commands::muat_ulang,
        ])
        .run(tauri::generate_context!())
        .expect("aplikasi gagal dijalankan");
}

fn url_awal() -> Url {
    std::env::args()
        .skip(1)
        .find(|argumen| !argumen.starts_with('-'))
        .and_then(|argumen| url_input::dari_input(&argumen))
        .unwrap_or_else(|| {
            Url::parse(url_input::URL_AWAL).expect("URL_AWAL harus berupa URL yang valid")
        })
}
