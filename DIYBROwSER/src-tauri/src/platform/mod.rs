//! Penempatan dan ukuran webview, dipisah per platform.
//!
//! Di Windows dan macOS, child webview Tauri mendukung posisi dan ukuran
//! absolut apa adanya. Di Linux tidak: `Window::add_child` menaruh webview ke
//! dalam `GtkBox` milik window, sehingga webview bertumpuk vertikal dan
//! `Webview::set_bounds` tidak berpengaruh (tauri-apps/tauri#10420). Untuk
//! Linux, modul `linux` memindahkan webview ke `GtkFixed` milik kita sendiri —
//! wadah yang memang didukung wry untuk penempatan absolut.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{atur_batas, siapkan_wadah};

#[cfg(not(target_os = "linux"))]
mod lainnya;
#[cfg(not(target_os = "linux"))]
pub use lainnya::{atur_batas, siapkan_wadah};
