//! Windows dan macOS: tidak perlu perlakuan khusus.
//!
//! Content blocker di kedua platform ini belum diimplementasikan: WebView2
//! (Windows) memakai `WebResourceRequested` yang perlu dibungkus wry, sedangkan
//! macOS perlu `WKContentRuleList` lewat `objc2`. Sampai itu ada, Perlindungan
//! Perisai hanya menyala di Linux. Lihat `docs/rencana-fitur.md` §6.1.

use std::path::PathBuf;

use tauri::{AppHandle, Rect, Webview};

/// Tidak ada wadah yang perlu diganti.
pub fn siapkan_wadah(_app: &AppHandle, _rect: Rect) -> Result<(), String> {
    Ok(())
}

pub fn atur_batas(webview: &Webview, rect: Rect) -> Result<(), String> {
    webview.set_bounds(rect).map_err(|e| e.to_string())
}

/// Belum ada jalur content blocker di platform ini.
pub fn pasang_filter(
    _json: String,
    _folder: PathBuf,
    _saat_siap: Box<dyn FnOnce()>,
) -> Result<(), String> {
    Ok(())
}

pub fn filter_siap() -> bool {
    false
}

pub fn pasang_filter_ke(_webview: &Webview) {}

pub fn lepas_filter(_webview: &Webview) {}

/// Pintasan keyboard lintas platform belum dikerjakan (lihat docs/rencana-fitur.md §6).
pub fn pasang_pintasan(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}
