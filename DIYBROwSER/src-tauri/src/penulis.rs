//! Penulis basis data di latar belakang.
//!
//! Main thread GTK/WebKit tidak boleh menunggu disk. Sebelumnya riwayat dan
//! sesi ditulis langsung saat event WebKit datang — termasuk pada setiap
//! perubahan judul — sehingga mesin dengan disk lambat berhenti sejenak tiap
//! kali halaman dibuka. Sekarang tulisan dikirim ke satu thread penulis yang
//! menggabungkan tugas yang mengantre menjadi satu putaran tulis.
//!
//! Tulisan juga dirapikan: judul yang berubah berkali-kali pada URL yang sama
//! hanya ditulis sekali per nilai akhir. YouTube dan situs SPA lain mengubah
//! judul puluhan kali per menit; tanpa penyaringan itu, riwayat menulis ke
//! disk jauh lebih sering daripada yang berguna.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use storage::Storage;

/// Pekerjaan tulis yang boleh menunggu; tidak ada yang dibaca balik.
enum Tugas {
    Riwayat {
        url: String,
        judul: String,
        waktu: i64,
    },
    Pengaturan {
        kunci: String,
        nilai: String,
    },
}

/// Jumlah tugas yang digabung dalam satu putaran tulis.
const TUMPUKAN: usize = 64;

pub struct Penulis {
    /// `None` kalau thread penulis gagal dibuat; tulisan lalu jatuh ke jalur
    /// langsung supaya data tidak hilang diam-diam.
    tx: Option<Sender<Tugas>>,
    db: Arc<Mutex<Storage>>,
    /// Pasangan (url, judul) terakhir yang sudah dikirim, untuk penyaringan.
    terakhir: Mutex<Option<(String, String)>>,
}

impl Penulis {
    pub fn mulai(db: Arc<Mutex<Storage>>) -> Self {
        let (tx, rx) = mpsc::channel::<Tugas>();
        let untuk_thread = db.clone();
        let thread = std::thread::Builder::new()
            .name("perisai-tulis".into())
            .spawn(move || jalankan(rx, untuk_thread));
        Self {
            tx: thread.ok().map(|_| tx),
            db,
            terakhir: Mutex::new(None),
        }
    }

    /// Mengirim satu kunjungan ke riwayat; judul sama persis tidak ditulis dua
    /// kali berturut-turut.
    pub fn riwayat(&self, url: &str, judul: &str, waktu: i64) {
        {
            let mut terakhir = match self.terakhir.lock() {
                Ok(slot) => slot,
                Err(_) => return,
            };
            if terakhir.as_ref() == Some(&(url.to_string(), judul.to_string())) {
                return;
            }
            *terakhir = Some((url.to_string(), judul.to_string()));
        }
        self.kirim(Tugas::Riwayat {
            url: url.to_string(),
            judul: judul.to_string(),
            waktu,
        });
    }

    pub fn pengaturan(&self, kunci: &str, nilai: &str) {
        self.kirim(Tugas::Pengaturan {
            kunci: kunci.to_string(),
            nilai: nilai.to_string(),
        });
    }

    fn kirim(&self, mut tugas: Tugas) {
        if let Some(tx) = &self.tx {
            match tx.send(tugas) {
                Ok(()) => return,
                // Tugas dikembalikan saat kanal putus; tulis langsung di bawah.
                Err(kembali) => tugas = kembali.0,
            }
        }
        // Penulis tidak tersedia: tulis langsung. Lebih baik main thread
        // menunggu sekali daripada riwayat dan sesi hilang diam-diam.
        let Ok(db) = self.db.lock() else {
            return;
        };
        match tugas {
            Tugas::Riwayat { url, judul, waktu } => {
                let _ = db.catat_kunjungan(&url, &judul, waktu);
            }
            Tugas::Pengaturan { kunci, nilai } => {
                let _ = db.set_pengaturan(&kunci, &nilai);
            }
        }
    }
}

fn jalankan(rx: Receiver<Tugas>, db: Arc<Mutex<Storage>>) {
    while let Ok(pertama) = rx.recv() {
        let mut tumpukan = vec![pertama];
        loop {
            match rx.try_recv() {
                Ok(tugas) => {
                    tumpukan.push(tugas);
                    if tumpukan.len() >= TUMPUKAN {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        let Ok(db) = db.lock() else {
            continue;
        };
        for tugas in tumpukan {
            match tugas {
                Tugas::Riwayat { url, judul, waktu } => {
                    let _ = db.catat_kunjungan(&url, &judul, waktu);
                }
                Tugas::Pengaturan { kunci, nilai } => {
                    let _ = db.set_pengaturan(&kunci, &nilai);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Penulis;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn penulis() -> (Penulis, Arc<Mutex<storage::Storage>>) {
        let db = Arc::new(Mutex::new(storage::Storage::open_in_memory().unwrap()));
        (Penulis::mulai(db.clone()), db)
    }

    fn tunggu_sampai(f: impl Fn() -> bool) -> bool {
        let batas = Instant::now() + Duration::from_secs(2);
        while Instant::now() < batas {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn riwayat_sampai_ke_basis_data() {
        let (penulis, db) = penulis();
        penulis.riwayat("https://contoh.test/a", "Halaman A", 10);
        assert!(tunggu_sampai(|| {
            db.lock().unwrap().daftar_riwayat(10).unwrap().len() == 1
        }));
    }

    #[test]
    fn judul_sama_tidak_ditulis_dua_kali() {
        let (penulis, db) = penulis();
        penulis.riwayat("https://contoh.test/a", "Halaman A", 10);
        penulis.riwayat("https://contoh.test/a", "Halaman A", 11);
        assert!(tunggu_sampai(|| {
            db.lock().unwrap().daftar_riwayat(10).unwrap().len() == 1
        }));
        // Yang tersimpan adalah kunjungan pertama; yang kedua disaring.
        let riwayat = db.lock().unwrap().daftar_riwayat(10).unwrap();
        assert_eq!(riwayat[0].dikunjungi, 10);
    }

    #[test]
    fn judul_berubah_tetap_ditulis() {
        let (penulis, db) = penulis();
        penulis.riwayat("https://contoh.test/a", "Awal", 10);
        penulis.riwayat("https://contoh.test/a", "Judul Baru", 11);
        assert!(tunggu_sampai(|| {
            db.lock()
                .unwrap()
                .daftar_riwayat(10)
                .unwrap()
                .first()
                .map(|r| r.judul == "Judul Baru")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn pengaturan_tersimpan() {
        let (penulis, db) = penulis();
        penulis.pengaturan("sesi", "[\"https://contoh.test\"]");
        assert!(tunggu_sampai(|| {
            db.lock().unwrap().get_pengaturan("sesi").unwrap().is_some()
        }));
    }
}
