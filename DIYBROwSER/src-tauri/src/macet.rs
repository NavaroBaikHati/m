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
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// Main thread dianggap tertahan kalau tidak menjawab selama ini.
pub const AMBANG: Duration = Duration::from_millis(2000);
/// Jedanya antar pertanyaan; pertanyaan sendiri hanya satu closure kecil.
const JEDA: Duration = Duration::from_millis(1000);

/// Nama pekerjaan main thread yang sedang berjalan.
///
/// Tanpa ini laporan macet hanya berkata "main thread tertahan", dan pemilik
/// laporan tetap harus menebak pekerjaan mana. Dengan penanda, barisnya
/// langsung menyebut tersangka — `(pasang-filter)`, `(tidurkan-tab)`, dan
/// seterusnya. Nilainya harus pendek dan statis supaya penulisan penanda
/// sendiri tidak menambah pekerjaan.
static KERJA: Mutex<&'static str> = Mutex::new("");

/// Menandai pekerjaan main thread yang sedang berjalan. Penandanya dilepas saat
/// nilai kembaliannya dijatuhkan (termasuk saat fungsi keluar karena `?`), dan
/// penanda sebelumnya dipulihkan supaya pekerjaan bersarang tidak saling
/// menghapus.
#[must_use]
pub struct Penanda {
    sebelumnya: &'static str,
}

impl Drop for Penanda {
    fn drop(&mut self) {
        pasang_kerja(self.sebelumnya);
    }
}

/// Menandai awal pekerjaan berat di main thread.
pub fn kerja(nama: &'static str) -> Penanda {
    let sebelumnya = kerja_sekarang();
    pasang_kerja(nama);
    Penanda { sebelumnya }
}

fn pasang_kerja(nama: &'static str) {
    if let Ok(mut slot) = KERJA.try_lock() {
        *slot = nama;
    }
}

/// Nama pekerjaan yang sedang berjalan, atau string kosong kalau tidak ada.
/// `try_lock` dipakai supaya utas pengukur tidak pernah ikut menunggu.
fn kerja_sekarang() -> &'static str {
    KERJA.try_lock().map(|slot| *slot).unwrap_or("")
}

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
    let kerja = kerja_sekarang();
    let rincian = if kerja.is_empty() {
        String::new()
    } else {
        format!(" ({kerja})")
    };
    eprintln!("macet: main thread tidak menjawab {awalan} {ms} ms{rincian}");
    let Some(folder) = folder_log(app) else {
        return;
    };
    let _ = std::fs::create_dir_all(&folder);
    if let Ok(mut berkas) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join("macet.log"))
    {
        let _ = writeln!(berkas, "{awalan} {ms} ms{rincian}");
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
    use super::{kerja, kerja_sekarang, tertahan, Penanda, AMBANG};
    use std::time::Duration;

    /// Semua pemeriksaan penanda di satu tes: penandanya satu variabel global,
    /// jadi tes yang berjalan paralel bisa saling menimpa.
    #[test]
    fn penanda_kerja_dipulihkan_saat_guard_jatuh() {
        {
            let _kerja = kerja("pasang-filter");
            assert_eq!(kerja_sekarang(), "pasang-filter");
        }
        assert_eq!(kerja_sekarang(), "");

        // Pekerjaan bersarang (misalnya pemasangan filter saat tab dibuka)
        // harus kembali ke penanda luar, bukan mengosongkannya.
        {
            let _luar = kerja("buka-tab");
            {
                let _dalam = kerja("pasang-filter");
                assert_eq!(kerja_sekarang(), "pasang-filter");
            }
            assert_eq!(kerja_sekarang(), "buka-tab");
        }
        assert_eq!(kerja_sekarang(), "");

        {
            let _penanda: Penanda = kerja("");
            assert_eq!(kerja_sekarang(), "");
        }
    }

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
