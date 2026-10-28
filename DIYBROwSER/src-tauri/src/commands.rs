//! Perintah yang dipanggil webview chrome (UI Svelte).
//!
//! Semua perintah `async` supaya pembuatan/pengaturan webview tidak memblokir
//! thread utama (lihat catatan deadlock `WebviewBuilder` di dokumentasi Tauri).

use tauri::State;

use crate::tabs::{DaftarTab, TabManager};
use crate::url_input;

/// Daftar tab untuk sidebar; dipanggil UI saat pertama kali dimuat.
#[tauri::command]
pub async fn daftar_tab(tabs: State<'_, TabManager>) -> Result<DaftarTab, String> {
    tabs.daftar()
}

/// Membuka tab baru dengan `input` dari address bar, atau alamat awal kalau kosong.
#[tauri::command]
pub async fn tab_baru(
    tabs: State<'_, TabManager>,
    input: Option<String>,
) -> Result<DaftarTab, String> {
    let url = input
        .as_deref()
        .and_then(url_input::dari_input)
        .unwrap_or_else(url_input::url_awal);
    tabs.buka(url)?;
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

fn webview_aktif(tabs: &TabManager) -> Result<tauri::Webview, String> {
    tabs.webview_aktif()
        .ok_or_else(|| "tidak ada tab aktif".to_string())
}

/// Riwayat maju/mundur memakai API riwayat di dalam webview tab.
fn jalankan_skrip(tabs: &TabManager, skrip: &str) -> Result<(), String> {
    webview_aktif(tabs)?.eval(skrip).map_err(|e| e.to_string())
}
