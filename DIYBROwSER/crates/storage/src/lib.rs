//! Penyimpanan lokal berbasis SQLite.
//!
//! Sesuai PRD, semua data pengguna tinggal di SQLite lokal tanpa telemetri:
//! pengaturan, riwayat, bookmark, dan catatan unduhan.
//!
//! Catatan threading: `rusqlite::Connection` bersifat `Send` tapi tidak `Sync`,
//! jadi pemakainya (misalnya state Tauri) perlu membungkusnya dengan `Mutex`.
#![forbid(unsafe_code)]

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::path::Path;

pub use rusqlite::{Connection, Error, Result};

/// Skema versi saat ini, dipakai untuk migrasi bertahap.
const SKEMA_VERSI: i64 = 2;

pub struct Storage {
    conn: Connection,
}

/// Satu kunjungan halaman.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Riwayat {
    pub url: String,
    pub judul: String,
    /// Waktu kunjungan dalam detik sejak epoch.
    pub dikunjungi: i64,
}

/// Satu halaman yang disimpan pengguna.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub url: String,
    pub judul: String,
    pub dibuat: i64,
}

/// Satu berkas yang pernah diunduh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unduhan {
    pub url: String,
    pub path: String,
    pub nama: String,
    /// `berjalan`, `selesai`, atau `gagal`.
    pub status: String,
    pub waktu: i64,
}

impl Storage {
    /// Membuka (atau membuat) basis data di `path`, lalu menjalankan migrasi.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::dari_connection(conn)
    }

    /// Basis data sementara di memori; dipakai unit test dan mode privat nanti.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::dari_connection(conn)
    }

    fn dari_connection(conn: Connection) -> Result<Self> {
        // WAL + synchronous NORMAL: riwayat dan sesi ditulis dari thread latar
        // (lihat `src-tauri/src/penulis.rs`), dan tanpa ini setiap commit
        // menunggu fsync sekaligus memblokir pembaca lain. Keduanya hanya
        // pengaturan, jadi kegagalannya tidak fatal.
        let _ = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0));
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        // Kalau penulis latar dan perintah UI bertabrakan, tunggu sebentar
        // alih-alih langsung gagal dengan SQLITE_BUSY.
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let storage = Self { conn };
        storage.migrasi()?;
        Ok(storage)
    }

    fn migrasi(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pengaturan (
                 kunci TEXT PRIMARY KEY,
                 nilai TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS riwayat (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 url TEXT NOT NULL,
                 judul TEXT NOT NULL DEFAULT '',
                 dikunjungi INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS riwayat_waktu ON riwayat (dikunjungi DESC);
             CREATE INDEX IF NOT EXISTS riwayat_url ON riwayat (url);

             CREATE TABLE IF NOT EXISTS bookmark (
                 url TEXT PRIMARY KEY,
                 judul TEXT NOT NULL,
                 dibuat INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS unduhan (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 url TEXT NOT NULL,
                 path TEXT NOT NULL,
                 nama TEXT NOT NULL,
                 status TEXT NOT NULL,
                 waktu INTEGER NOT NULL
             );",
        )?;
        let versi: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if versi < SKEMA_VERSI {
            self.conn.pragma_update(None, "user_version", SKEMA_VERSI)?;
        }
        Ok(())
    }

    /// Mengembalikan `None` kalau kunci belum pernah disimpan.
    pub fn get_pengaturan(&self, kunci: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT nilai FROM pengaturan WHERE kunci = ?1",
                params![kunci],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn set_pengaturan(&self, kunci: &str, nilai: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO pengaturan (kunci, nilai) VALUES (?1, ?2)
             ON CONFLICT(kunci) DO UPDATE SET nilai = excluded.nilai",
            params![kunci, nilai],
        )?;
        Ok(())
    }

    pub fn hapus_pengaturan(&self, kunci: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM pengaturan WHERE kunci = ?1", params![kunci])?;
        Ok(())
    }

    /// Mencatat kunjungan. Kunjungan berulang ke URL yang sama tetap dicatat,
    /// tapi entri lama dengan URL sama dipangkas agar riwayat tidak menggelembung:
    /// yang disimpan hanya waktu terakhir.
    pub fn catat_kunjungan(&self, url: &str, judul: &str, dikunjungi: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM riwayat WHERE url = ?1", params![url])?;
        self.conn.execute(
            "INSERT INTO riwayat (url, judul, dikunjungi) VALUES (?1, ?2, ?3)",
            params![url, judul, dikunjungi],
        )?;
        Ok(())
    }

    /// Riwayat terbaru lebih dulu, maksimal `batas` entri.
    pub fn daftar_riwayat(&self, batas: usize) -> Result<Vec<Riwayat>> {
        let mut stmt = self.conn.prepare(
            "SELECT url, judul, dikunjungi FROM riwayat
             ORDER BY dikunjungi DESC LIMIT ?1",
        )?;
        let baris = stmt.query_map(params![batas as i64], |row| {
            Ok(Riwayat {
                url: row.get(0)?,
                judul: row.get(1)?,
                dikunjungi: row.get(2)?,
            })
        })?;
        baris.collect()
    }

    /// Cari riwayat berdasarkan potongan judul atau URL.
    pub fn cari_riwayat(&self, kata: &str, batas: usize) -> Result<Vec<Riwayat>> {
        let pola = format!("%{}%", kata.replace('%', "\\%"));
        let mut stmt = self.conn.prepare(
            "SELECT url, judul, dikunjungi FROM riwayat
             WHERE url LIKE ?1 ESCAPE '\\' OR judul LIKE ?1 ESCAPE '\\'
             ORDER BY dikunjungi DESC LIMIT ?2",
        )?;
        let baris = stmt.query_map(params![pola, batas as i64], |row| {
            Ok(Riwayat {
                url: row.get(0)?,
                judul: row.get(1)?,
                dikunjungi: row.get(2)?,
            })
        })?;
        baris.collect()
    }

    pub fn hapus_riwayat(&self) -> Result<()> {
        self.conn.execute("DELETE FROM riwayat", [])?;
        Ok(())
    }

    /// Menyimpan bookmark; URL yang sama diperbarui judulnya.
    pub fn tambah_bookmark(&self, url: &str, judul: &str, dibuat: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO bookmark (url, judul, dibuat) VALUES (?1, ?2, ?3)
             ON CONFLICT(url) DO UPDATE SET judul = excluded.judul",
            params![url, judul, dibuat],
        )?;
        Ok(())
    }

    pub fn hapus_bookmark(&self, url: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM bookmark WHERE url = ?1", params![url])?;
        Ok(())
    }

    pub fn bookmark_ada(&self, url: &str) -> Result<bool> {
        let ada: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM bookmark WHERE url = ?1",
                params![url],
                |row| row.get(0),
            )
            .optional()?;
        Ok(ada.is_some())
    }

    pub fn daftar_bookmark(&self) -> Result<Vec<Bookmark>> {
        let mut stmt = self
            .conn
            .prepare("SELECT url, judul, dibuat FROM bookmark ORDER BY dibuat DESC")?;
        let baris = stmt.query_map([], |row| {
            Ok(Bookmark {
                url: row.get(0)?,
                judul: row.get(1)?,
                dibuat: row.get(2)?,
            })
        })?;
        baris.collect()
    }

    /// Mencatat unduhan baru dan mengembalikan id-nya.
    pub fn catat_unduhan(
        &self,
        url: &str,
        path: &str,
        nama: &str,
        status: &str,
        waktu: i64,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO unduhan (url, path, nama, status, waktu)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![url, path, nama, status, waktu],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn perbarui_status_unduhan(&self, id: i64, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE unduhan SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn daftar_unduhan(&self, batas: usize) -> Result<Vec<Unduhan>> {
        let mut stmt = self.conn.prepare(
            "SELECT url, path, nama, status, waktu FROM unduhan
             ORDER BY waktu DESC LIMIT ?1",
        )?;
        let baris = stmt.query_map(params![batas as i64], |row| {
            Ok(Unduhan {
                url: row.get(0)?,
                path: row.get(1)?,
                nama: row.get(2)?,
                status: row.get(3)?,
                waktu: row.get(4)?,
            })
        })?;
        baris.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> Storage {
        Storage::open_in_memory().unwrap()
    }

    #[test]
    fn pengaturan_disimpan_dan_dibaca_kembali() {
        let storage = storage();
        assert_eq!(storage.get_pengaturan("tema").unwrap(), None);

        storage.set_pengaturan("tema", "gelap").unwrap();
        assert_eq!(
            storage.get_pengaturan("tema").unwrap(),
            Some("gelap".to_string())
        );
    }

    #[test]
    fn pengaturan_ditimpa_bukan_diduplikasi() {
        let storage = storage();
        storage.set_pengaturan("tema", "gelap").unwrap();
        storage.set_pengaturan("tema", "terang").unwrap();

        assert_eq!(
            storage.get_pengaturan("tema").unwrap(),
            Some("terang".to_string())
        );
        let jumlah: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pengaturan WHERE kunci = 'tema'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(jumlah, 1);
    }

    #[test]
    fn data_bertahan_setelah_dibuka_ulang() {
        let path = std::env::temp_dir().join(format!(
            "perisai-storage-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        {
            let storage = Storage::open(&path).unwrap();
            storage.set_pengaturan("bahasa", "id").unwrap();
        }
        {
            // Migrasi harus idempoten saat file yang sama dibuka lagi.
            let storage = Storage::open(&path).unwrap();
            assert_eq!(
                storage.get_pengaturan("bahasa").unwrap(),
                Some("id".to_string())
            );
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn riwayat_terbaru_di_atas_dan_url_sama_tidak_duplikat() {
        let storage = storage();
        storage
            .catat_kunjungan("https://a.example/", "A", 100)
            .unwrap();
        storage
            .catat_kunjungan("https://b.example/", "B", 200)
            .unwrap();
        storage
            .catat_kunjungan("https://a.example/", "A baru", 300)
            .unwrap();

        let riwayat = storage.daftar_riwayat(10).unwrap();
        assert_eq!(riwayat.len(), 2);
        assert_eq!(riwayat[0].url, "https://a.example/");
        assert_eq!(riwayat[0].judul, "A baru");
        assert_eq!(riwayat[1].url, "https://b.example/");

        assert_eq!(storage.cari_riwayat("baru", 10).unwrap().len(), 1);
        storage.hapus_riwayat().unwrap();
        assert!(storage.daftar_riwayat(10).unwrap().is_empty());
    }

    #[test]
    fn bookmark_ditambah_dihapus_dan_dicek() {
        let storage = storage();
        storage
            .tambah_bookmark("https://a.example/", "A", 10)
            .unwrap();
        assert!(storage.bookmark_ada("https://a.example/").unwrap());
        // URL sama, judul diperbarui, bukan ditambah.
        storage
            .tambah_bookmark("https://a.example/", "A2", 11)
            .unwrap();
        let daftar = storage.daftar_bookmark().unwrap();
        assert_eq!(daftar.len(), 1);
        assert_eq!(daftar[0].judul, "A2");

        storage.hapus_bookmark("https://a.example/").unwrap();
        assert!(!storage.bookmark_ada("https://a.example/").unwrap());
    }

    #[test]
    fn unduhan_dicatat_dan_statusnya_diperbarui() {
        let storage = storage();
        let id = storage
            .catat_unduhan(
                "https://a.example/f.zip",
                "/tmp/f.zip",
                "f.zip",
                "berjalan",
                5,
            )
            .unwrap();
        storage.perbarui_status_unduhan(id, "selesai").unwrap();

        let daftar = storage.daftar_unduhan(5).unwrap();
        assert_eq!(daftar.len(), 1);
        assert_eq!(daftar[0].status, "selesai");
        assert_eq!(daftar[0].nama, "f.zip");
    }
}
