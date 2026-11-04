//! Perintah yang dipanggil webview chrome (UI Svelte).
//!
//! Semua perintah `async` supaya pembuatan/pengaturan webview tidak memblokir
//! thread utama (lihat catatan deadlock `WebviewBuilder` di dokumentasi Tauri).

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::adblock::Perlindungan;
use crate::beranda;
use crate::tabs::{DaftarTab, TabManager};
use crate::url_input;
use crate::Database;

/// Kunci pengaturan untuk pilihan mesin pencari.
const KUNCI_MESIN: &str = "mesin_pencarian";

/// Ringkasan halaman aktif untuk chrome UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusHalaman {
    pub url: String,
    pub bookmark: bool,
    pub zoom: f64,
    pub tema: String,
    /// Halaman tab baru (`file://`), dipakai UI untuk mengosongkan address bar.
    pub beranda: bool,
    pub perlindungan: Perlindungan,
}

/// Daftar tab untuk sidebar; dipanggil UI saat pertama kali dimuat.
#[tauri::command]
pub async fn daftar_tab(tabs: State<'_, TabManager>) -> Result<DaftarTab, String> {
    tabs.daftar()
}

/// Membuka tab baru dengan `input` dari address bar, atau halaman tab baru
/// kalau kosong.
#[tauri::command]
pub async fn tab_baru(
    tabs: State<'_, TabManager>,
    input: Option<String>,
) -> Result<DaftarTab, String> {
    match input.as_deref().and_then(url_input::dari_input) {
        Some(url) => {
            tabs.buka(url)?;
        }
        None => {
            tabs.tab_baru()?;
        }
    }
    tabs.daftar()
}

#[tauri::command]
pub async fn pilih_tab(tabs: State<'_, TabManager>, id: String) -> Result<DaftarTab, String> {
    tabs.pilih(&id)?;
    tabs.daftar()
}

#[tauri::command]
pub async fn tutup_tab(tabs: State<'_, TabManager>, id: String) -> Result<DaftarTab, String> {
    tabs.tutup(&id)?;
    tabs.daftar()
}

/// Membuka (`true`) atau menciutkan (`false`) sidebar vertikal.
#[tauri::command]
pub async fn atur_sidebar(tabs: State<'_, TabManager>, terbuka: bool) -> Result<DaftarTab, String> {
    tabs.atur_sidebar(terbuka)?;
    tabs.daftar()
}

/// Menavigasi tab aktif ke `input` yang diketik di address bar.
#[tauri::command]
pub async fn navigasi(tabs: State<'_, TabManager>, input: String) -> Result<(), String> {
    let url = url_input::dari_input(&input).ok_or_else(|| "alamat kosong".to_string())?;
    webview_aktif(&tabs)?
        .navigate(url)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mundur(tabs: State<'_, TabManager>) -> Result<(), String> {
    jalankan_skrip(&tabs, "window.history.back()")
}

#[tauri::command]
pub async fn maju(tabs: State<'_, TabManager>) -> Result<(), String> {
    jalankan_skrip(&tabs, "window.history.forward()")
}

#[tauri::command]
pub async fn muat_ulang(tabs: State<'_, TabManager>) -> Result<(), String> {
    webview_aktif(&tabs)?.reload().map_err(|e| e.to_string())
}

/// Menghentikan pemuatan tab aktif; dipakai tombol yang sama dengan muat ulang.
#[tauri::command]
pub async fn hentikan(tabs: State<'_, TabManager>) -> Result<(), String> {
    jalankan_skrip(&tabs, "window.stop()")
}

/// Status halaman (`id` opsional; kalau kosong memakai tab aktif).
#[tauri::command]
pub async fn status_halaman(
    tabs: State<'_, TabManager>,
    db: State<'_, Database>,
    id: Option<String>,
) -> Result<StatusHalaman, String> {
    let daftar = tabs.daftar()?;
    let label = id.clone().or_else(|| daftar.aktif.clone());
    let tab = label
        .as_ref()
        .and_then(|label| daftar.tabs.iter().find(|t| &t.id == label));
    let url = tab.map(|t| t.url.clone()).unwrap_or_default();

    let bookmark =
        db.0.lock()
            .ok()
            .and_then(|db| db.bookmark_ada(&url).ok())
            .unwrap_or(false);

    Ok(StatusHalaman {
        beranda: url.starts_with("file:"),
        bookmark,
        zoom: tabs.zoom(),
        tema: tabs.tema(),
        perlindungan: tabs.info_perlindungan(label.as_deref()),
        url,
    })
}

/// Memesan ruang panel samping (0 = tutup). Dipanggil UI saat panel dibuka.
#[tauri::command]
pub async fn atur_panel(tabs: State<'_, TabManager>, lebar: f64) -> Result<(), String> {
    tabs.atur_panel(lebar)
}

/// Menyalakan/mematikan Perlindungan untuk situs pada tab `id`.
#[tauri::command]
pub async fn atur_perlindungan(
    tabs: State<'_, TabManager>,
    id: String,
    aktif: bool,
) -> Result<Perlindungan, String> {
    tabs.atur_situs(&id, aktif)
}

#[tauri::command]
pub async fn atur_zoom(tabs: State<'_, TabManager>, nilai: f64) -> Result<f64, String> {
    tabs.atur_zoom(nilai)
}

#[tauri::command]
pub async fn atur_tema(tabs: State<'_, TabManager>, tema: String) -> Result<String, String> {
    tabs.atur_tema(&tema)
}

/// Satu pilihan mesin pencari untuk menu.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PilihanMesin {
    pub slug: &'static str,
    pub nama: &'static str,
    pub aktif: bool,
}

fn daftar_mesin(mesin_aktif: url_input::MesinPencarian) -> Vec<PilihanMesin> {
    url_input::MesinPencarian::SEMUA
        .into_iter()
        .map(|mesin| PilihanMesin {
            slug: mesin.slug(),
            nama: mesin.nama(),
            aktif: mesin == mesin_aktif,
        })
        .collect()
}

#[tauri::command]
pub async fn daftar_mesin_pencarian() -> Vec<PilihanMesin> {
    daftar_mesin(url_input::mesin_aktif())
}

/// Menyimpan pilihan mesin pencari dan menulis ulang halaman tab baru.
#[tauri::command]
pub async fn atur_mesin_pencarian(
    app: AppHandle,
    db: State<'_, Database>,
    slug: String,
) -> Result<Vec<PilihanMesin>, String> {
    let mesin = url_input::atur_mesin(&slug)
        .ok_or_else(|| format!("mesin pencari tidak dikenal: {slug}"))?;
    db.0.lock()
        .map_err(|_| "basis data terkunci".to_string())?
        .set_pengaturan(KUNCI_MESIN, mesin.slug())
        .map_err(|e| e.to_string())?;
    beranda::siapkan(&app)?;
    Ok(daftar_mesin(mesin))
}

#[tauri::command]
pub async fn daftar_bookmark(db: State<'_, Database>) -> Result<Vec<storage::Bookmark>, String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    db.daftar_bookmark().map_err(|e| e.to_string())
}

/// Menyimpan atau menghapus bookmark; mengembalikan keadaan akhir.
#[tauri::command]
pub async fn toggle_bookmark(
    db: State<'_, Database>,
    url: String,
    judul: String,
) -> Result<bool, String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    if db.bookmark_ada(&url).map_err(|e| e.to_string())? {
        db.hapus_bookmark(&url).map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        db.tambah_bookmark(&url, &judul, waktu_sekarang())
            .map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn daftar_riwayat(
    db: State<'_, Database>,
    batas: Option<usize>,
) -> Result<Vec<storage::Riwayat>, String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    db.daftar_riwayat(batas.unwrap_or(300))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cari_riwayat(
    db: State<'_, Database>,
    kata: String,
    batas: Option<usize>,
) -> Result<Vec<storage::Riwayat>, String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    db.cari_riwayat(&kata, batas.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hapus_riwayat(db: State<'_, Database>) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    db.hapus_riwayat().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn daftar_unduhan(db: State<'_, Database>) -> Result<Vec<storage::Unduhan>, String> {
    let db = db.0.lock().map_err(|_| "basis data terkunci".to_string())?;
    db.daftar_unduhan(100).map_err(|e| e.to_string())
}

fn webview_aktif(tabs: &TabManager) -> Result<tauri::Webview, String> {
    tabs.webview_aktif()
        .ok_or_else(|| "tidak ada tab aktif".to_string())
}

/// Riwayat maju/mundur memakai API riwayat di dalam webview tab.
fn jalankan_skrip(tabs: &TabManager, skrip: &str) -> Result<(), String> {
    webview_aktif(tabs)?.eval(skrip).map_err(|e| e.to_string())
}

fn waktu_sekarang() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
