//! Windows dan macOS: tidak perlu perlakuan khusus.

use tauri::{AppHandle, Rect, Webview};

/// Tidak ada wadah yang perlu diganti.
pub fn siapkan_wadah(_app: &AppHandle, _rect: Rect) -> Result<(), String> {
    Ok(())
}

pub fn atur_batas(webview: &Webview, rect: Rect) -> Result<(), String> {
    webview.set_bounds(rect).map_err(|e| e.to_string())
}
