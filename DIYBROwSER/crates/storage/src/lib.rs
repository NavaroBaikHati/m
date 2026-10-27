//! Penyimpanan lokal berbasis SQLite.
//!
//! Sesuai PRD, semua data pengguna tinggal di SQLite lokal tanpa telemetri.
//! Tahap 1 baru menyediakan tabel `pengaturan`; tabel bookmark dan history
//! ditambahkan pada tahap 4.
//!
//! Catatan threading: `rusqlite::Connection` bersifat `Send` tapi tidak `Sync`,
//! jadi pemakainya (misalnya state Tauri) perlu membungkusnya dengan `Mutex`.
#![forbid(unsafe_code)]

use rusqlite::{params, OptionalExtension};
use std::path::Path;

pub use rusqlite::{Connection, Error, Result};

/// Skema versi saat ini, dipakai untuk migrasi bertahap.
const SKEMA_VERSI: i64 = 1;

pub struct Storage {
    conn: Connection,
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
        let storage = Self { conn };
        storage.migrasi()?;
        Ok(storage)
    }

    fn migrasi(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pengaturan (
                 kunci TEXT PRIMARY KEY,
                 nilai TEXT NOT NULL
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pengaturan_disimpan_dan_dibaca_kembali() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.get_pengaturan("tema").unwrap(), None);

        storage.set_pengaturan("tema", "gelap").unwrap();
        assert_eq!(
            storage.get_pengaturan("tema").unwrap(),
            Some("gelap".to_string())
        );
    }

    #[test]
    fn pengaturan_ditimpa_bukan_diduplikasi() {
        let storage = Storage::open_in_memory().unwrap();
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
            "diybrowser-storage-{}-{:?}.sqlite",
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
}
