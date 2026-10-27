//! Inti adblock yang bebas dari Tauri dan UI.
//!
//! Semua yang berhubungan dengan penyaringan request, cosmetic filtering, dan
//! scriptlet ditaruh di sini supaya bisa dites penuh tanpa layar (dan mudah
//! dipindah kalau nanti pindah dari Tauri ke CEF).
//!
//! Tahap 1 baru berisi katalog daftar filter bawaan. Mesin pencocokannya
//! (crate `adblock`, yaitu adblock-rust milik Brave) menyusul di tahap 3.
#![forbid(unsafe_code)]

/// Deskripsi satu daftar filter yang bisa diunduh klien.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterList {
    /// Pengenal stabil, dipakai sebagai kunci di penyimpanan lokal.
    pub id: &'static str,
    /// Nama yang ditampilkan ke pengguna.
    pub name: &'static str,
    /// URL unduhan daftar filter.
    pub url: &'static str,
    /// Apakah daftar ini aktif secara bawaan.
    pub enabled_by_default: bool,
}

/// Daftar filter bawaan sesuai PRD: EasyList untuk iklan, EasyPrivacy untuk tracker.
pub const DEFAULT_FILTER_LISTS: &[FilterList] = &[
    FilterList {
        id: "easylist",
        name: "EasyList",
        url: "https://easylist.to/easylist/easylist.txt",
        enabled_by_default: true,
    },
    FilterList {
        id: "easyprivacy",
        name: "EasyPrivacy",
        url: "https://easylist.to/easylist/easyprivacy.txt",
        enabled_by_default: true,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daftar_bawaan_berisi_easylist_dan_easyprivacy() {
        let ids: Vec<&str> = DEFAULT_FILTER_LISTS.iter().map(|l| l.id).collect();
        assert!(
            ids.contains(&"easylist"),
            "EasyList hilang dari daftar bawaan"
        );
        assert!(
            ids.contains(&"easyprivacy"),
            "EasyPrivacy hilang dari daftar bawaan"
        );
    }

    #[test]
    fn id_daftar_unik_dan_url_https() {
        let mut ids: Vec<&str> = DEFAULT_FILTER_LISTS.iter().map(|l| l.id).collect();
        let jumlah = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), jumlah, "ada id daftar filter yang duplikat");

        for list in DEFAULT_FILTER_LISTS {
            assert!(
                list.url.starts_with("https://"),
                "daftar {} tidak memakai HTTPS",
                list.id
            );
        }
    }

    #[test]
    fn daftar_bawaan_aktif_semua() {
        assert!(DEFAULT_FILTER_LISTS.iter().all(|l| l.enabled_by_default));
    }
}
