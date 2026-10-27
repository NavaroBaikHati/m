// Jangan tampilkan jendela konsol tambahan di Windows saat mode release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    diy_browser::run()
}
