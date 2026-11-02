//! Perlindungan Perisai: mesin adblock, daftar filter, dan pemasangan ke webview.
//!
//! Pembagian tugas:
//!
//! - `adblock-core` mengompilasi daftar filter menjadi content blocker JSON dan
//!   mesin cosmetic/scriptlet.
//! - Modul ini mengurus cache di disk (unduh EasyList/EasyPrivacy harian),
//!   menyimpan daftar situs yang dikecualikan, dan memberi tahu UI.
//! - `platform` yang benar-benar memasang filter ke WebKit.
//!
//! Content blocker WebKit tidak melaporkan request yang diblokir, jadi panel
//! Perlindungan menampilkan jumlah aturan yang aktif untuk halaman itu —
//! bukan angka "iklan diblokir" yang dikarang.

use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use adblock_core::{FilterList, Mesin};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// Event yang dikirim ke webview chrome saat filter selesai dikompilasi.
pub const EVENT_PERLINDUNGAN: &str = "perlindungan:siap";

/// Umur cache daftar filter sebelum diunduh ulang.
const UMUR_CACHE: Duration = Duration::from_secs(24 * 60 * 60);

/// Informasi perlindungan untuk satu halaman, dipakai panel shield.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Perlindungan {
    pub host: String,
    pub aktif: bool,
    pub siap: bool,
    pub jumlah_aturan: usize,
    /// Jumlah selector cosmetic yang berlaku untuk halaman ini.
    pub jumlah_selector: usize,
}

pub struct Adblock {
    folder: PathBuf,
    mesin: RwLock<Arc<Mesin>>,
    /// Host yang perlindungannya dimatikan pengguna.
    nonaktif: RwLock<HashSet<String>>,
    jumlah_aturan: AtomicUsize,
    siap: AtomicBool,
    /// Sidik JSON yang sedang terpasang, untuk menghindari pemasangan ganda.
    terpasang: RwLock<Option<u64>>,
}

impl Adblock {
    /// Membuat state kosong (belum ada aturan) dan menyiapkan folder cache.
    pub fn baru(app: &AppHandle) -> Arc<Self> {
        let folder = app
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("perisai"))
            .join("adblock");
        let _ = std::fs::create_dir_all(&folder);
        Arc::new(Self {
            folder,
            mesin: RwLock::new(Arc::new(adblock_core::kosong())),
            nonaktif: RwLock::new(HashSet::new()),
            jumlah_aturan: AtomicUsize::new(0),
            siap: AtomicBool::new(false),
            terpasang: RwLock::new(None),
        })
    }

    pub fn siap(&self) -> bool {
        self.siap.load(Ordering::Acquire)
    }

    pub fn jumlah_aturan(&self) -> usize {
        self.jumlah_aturan.load(Ordering::Acquire)
    }

    /// Memuat daftar situs yang dikecualikan dari pengaturan tersimpan.
    pub fn atur_nonaktif(&self, daftar: HashSet<String>) {
        if let Ok(mut slot) = self.nonaktif.write() {
            *slot = daftar;
        }
    }

    pub fn daftar_nonaktif(&self) -> HashSet<String> {
        self.nonaktif
            .read()
            .map(|slot| slot.clone())
            .unwrap_or_default()
    }

    pub fn aktif_untuk(&self, host: &str) -> bool {
        !self
            .nonaktif
            .read()
            .map(|slot| slot.contains(host))
            .unwrap_or(false)
    }

    /// Menyalakan/mematikan perlindungan untuk satu host.
    pub fn atur_situs(&self, host: &str, aktif: bool) {
        if let Ok(mut slot) = self.nonaktif.write() {
            if aktif {
                slot.remove(host);
            } else {
                slot.insert(host.to_string());
            }
        }
    }

    pub fn mesin(&self) -> Arc<Mesin> {
        self.mesin
            .read()
            .map(|slot| slot.clone())
            .unwrap_or_else(|_| Arc::new(adblock_core::kosong()))
    }

    /// Scriptlet untuk satu URL (kosong kalau perlindungan situs dimatikan).
    pub fn skrip_untuk(&self, url: &str) -> String {
        let host = adblock_core::host_dari(url).unwrap_or_default();
        if !self.aktif_untuk(&host) {
            return String::new();
        }
        self.mesin().skrip_untuk(url)
    }

    /// Ringkasan perlindungan untuk panel shield.
    pub fn info(&self, url: &str) -> Perlindungan {
        let host = adblock_core::host_dari(url).unwrap_or_default();
        let selector = if self.aktif_untuk(&host) {
            adblock_core::gabung_selector(&self.mesin(), url)
        } else {
            0
        };
        Perlindungan {
            aktif: self.aktif_untuk(&host),
            siap: self.siap(),
            jumlah_aturan: self.jumlah_aturan(),
            jumlah_selector: selector,
            host,
        }
    }

    /// Menjalankan pemuatan daftar filter di latar belakang.
    ///
    /// Kalau `filter.json` hasil kompilasi sebelumnya ada, filter itu langsung
    /// dipasang agar halaman pertama sudah terlindungi; sisanya (mesin cosmetic
    /// dan pembaruan daftar) menyusul tanpa memblokir pembukaan jendela.
    pub fn mulai(self: &Arc<Self>, app: AppHandle) {
        let ini = self.clone();
        let app_awal = app.clone();
        std::thread::Builder::new()
            .name("perisai-adblock".into())
            .spawn(move || {
                // 1. Filter yang sudah terkompilasi: pasang secepatnya.
                if let Some(json) = ini.baca_json_cache() {
                    ini.pasang(app_awal.clone(), json);
                }
                // 2. Mesin + pembaruan daftar.
                let hasil = ini.siapkan_mesin();
                match hasil {
                    Ok((mesin, json)) => {
                        if let Ok(mut slot) = ini.mesin.write() {
                            *slot = Arc::new(mesin);
                        }
                        ini.pasang(app.clone(), json);
                        let info = ini.info("https://example.com/");
                        let _ = app.emit(EVENT_PERLINDUNGAN, info);
                    }
                    Err(pesan) => {
                        eprintln!("adblock: daftar filter gagal disiapkan: {pesan}");
                    }
                }
            })
            .expect("gagal membuat thread adblock");
    }

    fn pasang(self: &Arc<Self>, app: AppHandle, json: String) {
        let sidik = sidik(&json);
        {
            let Ok(slot) = self.terpasang.read() else {
                return;
            };
            if *slot == Some(sidik) {
                return;
            }
        }
        let _ = std::fs::write(self.folder.join("filter.json"), &json);
        let _ = std::fs::write(
            self.folder.join("filter.sig"),
            format!("{:016x}", self.sidik_konfigurasi()),
        );

        let ini = self.clone();
        let folder = self.folder.clone();
        let app_siap = app.clone();

        // Objek WebKitGTK hanya boleh dibuat di thread utama GTK. Fungsi ini
        // dipanggil dari thread pekerja, jadi pemasangan dikirim ke main loop.
        let hasil = app.run_on_main_thread(move || {
            let ini_siap = ini.clone();
            let app_cb = app_siap.clone();
            let hasil = crate::platform::pasang_filter(
                json,
                folder,
                Box::new(move || {
                    ini_siap.siap.store(true, Ordering::Release);
                    let jumlah = ini_siap.mesin().jumlah_aturan();
                    eprintln!("adblock: siap {jumlah} aturan");
                    ini_siap.jumlah_aturan.store(jumlah, Ordering::Release);
                    let _ = app_cb.emit(
                        EVENT_PERLINDUNGAN,
                        Perlindungan {
                            host: String::new(),
                            aktif: true,
                            siap: true,
                            jumlah_aturan: jumlah,
                            jumlah_selector: 0,
                        },
                    );
                    // Webview yang sudah terbuka belum ikut terpasang.
                    if let Some(tabs) = app_cb.try_state::<crate::tabs::TabManager>() {
                        tabs.pasang_perlindungan();
                    }
                }),
            );
            if let Err(pesan) = hasil {
                eprintln!("adblock: gagal memasang filter: {pesan}");
            }
        });
        if let Err(pesan) = hasil {
            eprintln!("adblock: gagal menjadwalkan pemasangan filter: {pesan}");
        } else if let Ok(mut slot) = self.terpasang.write() {
            *slot = Some(sidik);
        }
    }

    /// Membaca filter hasil kompilasi sebelumnya, tapi hanya kalau daftar yang
    /// dipakai saat itu sama dengan daftar sekarang. Tanpa pemeriksaan ini,
    /// filter lama (misalnya tanpa aturan baru) sempat dipasang lebih dulu dan
    /// situs terbuka dengan perlindungan yang salah.
    fn baca_json_cache(&self) -> Option<String> {
        let tanda = std::fs::read_to_string(self.folder.join("filter.sig")).ok()?;
        if tanda.trim() != format!("{:016x}", self.sidik_konfigurasi()) {
            return None;
        }
        let teks = std::fs::read_to_string(self.folder.join("filter.json")).ok()?;
        if teks.trim().is_empty() {
            None
        } else {
            Some(teks)
        }
    }

    /// Sidik dari daftar filter yang tersedia: ukuran + waktu ubah tiap berkas
    /// cache, ditambah isi aturan uji (kalau ada). Perubahan apa pun membuat
    /// cache JSON dianggap basi.
    fn sidik_konfigurasi(&self) -> u64 {
        let mut bahan = String::new();
        for daftar in adblock_core::DEFAULT_FILTER_LISTS {
            let path = self.folder.join(format!("{}.txt", daftar.id));
            if let Ok(meta) = std::fs::metadata(&path) {
                let waktu = meta
                    .modified()
                    .ok()
                    .and_then(|w| w.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                bahan.push_str(&format!("{}:{}:{waktu};", daftar.id, meta.len()));
            }
        }
        if let Some(path) = std::env::var_os("PERISAI_FILTER_UJI") {
            if let Ok(isi) = std::fs::read_to_string(path) {
                bahan.push_str(&isi);
            }
        }
        sidik(&bahan)
    }

    /// Membaca cache daftar (mengunduh kalau kedaluwarsa), lalu mengompilasi.
    fn siapkan_mesin(&self) -> Result<(Mesin, String), String> {
        let mut teks = Vec::new();
        for daftar in adblock_core::DEFAULT_FILTER_LISTS {
            if let Some(isi) = self.daftar_terbaru(daftar) {
                teks.push(isi);
            }
        }
        // Hook pengujian GUI: aturan tambahan dari berkas, dipakai skenario Xvfb
        // untuk membuktikan content blocker benar-benar memblokir request.
        if let Some(path) = std::env::var_os("PERISAI_FILTER_UJI") {
            if let Ok(isi) = std::fs::read_to_string(path) {
                teks.push(isi);
            }
        }
        if teks.is_empty() {
            return Err("tidak ada daftar filter yang bisa dipakai".into());
        }
        let hasil = adblock_core::bangun(&teks)?;
        Ok((hasil.mesin, hasil.filter_json))
    }

    fn daftar_terbaru(&self, daftar: &FilterList) -> Option<String> {
        let path = self.folder.join(format!("{}.txt", daftar.id));
        let basi = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .map(|waktu| {
                SystemTime::now()
                    .duration_since(waktu)
                    .map(|umur| umur > UMUR_CACHE)
                    .unwrap_or(false)
            })
            .unwrap_or(true);

        if !basi {
            if let Ok(isi) = std::fs::read_to_string(&path) {
                if !isi.trim().is_empty() {
                    return Some(isi);
                }
            }
        }

        match unduh(daftar.url) {
            Ok(isi) if !isi.trim().is_empty() => {
                if let Err(e) = std::fs::write(&path, &isi) {
                    eprintln!("adblock: gagal menyimpan cache {}: {e}", daftar.id);
                }
                Some(isi)
            }
            Ok(_) => {
                eprintln!("adblock: {} kosong, memakai cache lama", daftar.name);
                std::fs::read_to_string(&path).ok()
            }
            Err(pesan) => {
                eprintln!(
                    "adblock: {} gagal diunduh ({pesan}), memakai cache lama",
                    daftar.name
                );
                std::fs::read_to_string(&path)
                    .ok()
                    .filter(|isi| !isi.trim().is_empty())
            }
        }
    }
}

/// Unduh satu daftar filter dengan batas waktu yang wajar.
fn unduh(url: &str) -> Result<String, String> {
    let respons = ureq::get(url)
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|e| e.to_string())?;
    let mut isi = String::new();
    respons
        .into_reader()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut isi)
        .map_err(|e| e.to_string())?;
    Ok(isi)
}

fn sidik(teks: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in teks.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::sidik;

    #[test]
    fn sidik_berbeda_untuk_filter_berbeda() {
        assert_eq!(sidik("a"), sidik("a"));
        assert_ne!(sidik("a"), sidik("b"));
    }
}
