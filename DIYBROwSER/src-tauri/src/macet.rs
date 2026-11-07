//! Pendeteksi main thread yang tertahan.
//!
//! Keluhan "browser freeze" bisa punya dua arti yang sangat berbeda: proses
//! WebKit berhenti (halamannya saja) atau main thread aplikasi berhenti
//! (seluruh jendela tidak menjawab klik). Yang kedua tidak meninggalkan jejak
//! apa pun di log, jadi di sini main thread ditanya tiap detik lewat
//! `run_on_main_thread`. Kalau jawabannya datang terlambat, berarti ada
//! pekerjaan di main thread yang terlalu lama — dan lamanya dicatat.

use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// Main thread dianggap tertahan kalau tidak menjawab selama ini.
pub const AMBANG: Duration = Duration::from_millis(2000);
/// Jedanya antar pertanyaan; pertanyaan sendiri hanya satu closure kecil.
const JEDA: Duration = Duration::from_millis(1000);

pub fn mulai(app: &AppHandle) {
    let app = app.clone();
    let (tx, rx) = mpsc::channel::<Instant>();
    thread::spawn(move || loop {
        thread::sleep(JEDA);
        let mulai = Instant::now();
        let tx = tx.clone();
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(Instant::now());
            })
            .is_err()
        {
            return;
        }
        match rx.recv_timeout(AMBANG) {
            Ok(selesai) => {
                let durasi = selesai.duration_since(mulai);
                if tertahan(durasi) {
                    catat(&app, durasi, false);
                }
            }
            // Belum menjawab sampai ambang: main thread masih di dalam
            // pekerjaan yang sama. Jangan menunggu selamanya.
            Err(_) => {
                catat(&app, AMBANG, true);
                thread::sleep(AMBANG);
            }
        }
    });
}

fn tertahan(durasi: Duration) -> bool {
    durasi >= AMBANG
}

/// Ditulis ke stderr (terlihat kalau dijalankan dari terminal) dan ke berkas
/// supaya tetap ada saat dijalankan dari peluncur aplikasi.
fn catat(app: &AppHandle, durasi: Duration, lebih_lama: bool) {
    let ms = durasi.as_millis();
    let awalan = if lebih_lama { ">" } else { "selama" };
    eprintln!("macet: main thread tidak menjawab {awalan} {ms} ms");
    let Some(folder) = folder_log(app) else {
        return;
    };
    let _ = std::fs::create_dir_all(&folder);
    if let Ok(mut berkas) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join("macet.log"))
    {
        let _ = writeln!(berkas, "{awalan} {ms} ms");
    }
}

fn folder_log(app: &AppHandle) -> Option<std::path::PathBuf> {
    if let Ok(folder) = app.path().app_log_dir() {
        return Some(folder);
    }
    let dasar = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state"))
        })?;
    Some(dasar.join("perisai"))
}

#[cfg(test)]
mod tests {
    use super::{tertahan, AMBANG};
    use std::time::Duration;

    #[test]
    fn jeda_pendek_bukan_macet() {
        assert!(!tertahan(Duration::from_millis(5)));
        assert!(!tertahan(AMBANG - Duration::from_millis(1)));
    }

    #[test]
    fn ambang_dan_lebih_dihitung_macet() {
        assert!(tertahan(AMBANG));
        assert!(tertahan(Duration::from_secs(30)));
    }

    #[test]
    fn ambang_cukup_besar_untuk_tidak_berisik() {
        // Satu frame lambat bukan freeze; ambang di bawah 1 detik akan
        // melaporkan hal-hal normal dan justru menyembunyikan yang penting.
        assert!(AMBANG >= Duration::from_secs(1));
    }
}
