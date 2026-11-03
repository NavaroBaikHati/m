//! Menerjemahkan isi address bar menjadi URL yang bisa dinavigasi.

use std::sync::RwLock;

use tauri::Url;

/// Mesin pencari yang bisa dipakai. PRD tidak menyebut merek, jadi daftar ini
/// sengaja kecil dan bisa diganti dari menu.
///
/// Bawaan **Google**: DuckDuckGo sering diblokir/intersepsi penyedia internet
/// di Indonesia, dan intersepsi itu muncul sebagai galat TLS di WebKit
/// ("Unacceptable TLS certificate") sehingga terlihat seperti kerusakan
/// browser. Mesin pencari yang bisa diganti membuat kasus itu tidak lagi
/// memblokir pencarian.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MesinPencarian {
    Google,
    Bing,
    DuckDuckGo,
    Brave,
}

/// Skema yang boleh dibuka langsung dari address bar.
/// Sisanya (mis. `javascript:`) diperlakukan sebagai teks pencarian.
const SKEMA_DIIZINKAN: &[&str] = &["http", "https", "file", "about"];

impl MesinPencarian {
    /// Semua pilihan, urut seperti yang tampil di menu.
    pub const SEMUA: [MesinPencarian; 4] =
        [Self::Google, Self::Bing, Self::DuckDuckGo, Self::Brave];

    /// Nilai yang disimpan di basis data.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Bing => "bing",
            Self::DuckDuckGo => "duckduckgo",
            Self::Brave => "brave",
        }
    }

    /// Label pendek untuk tombol di menu.
    pub fn nama(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Bing => "Bing",
            Self::DuckDuckGo => "DDG",
            Self::Brave => "Brave",
        }
    }

    /// Awalan URL pencarian; kata kunci ditambahkan sebagai `q`.
    pub fn templat(self) -> &'static str {
        match self {
            Self::Google => "https://www.google.com/search?q=",
            Self::Bing => "https://www.bing.com/search?q=",
            Self::DuckDuckGo => "https://duckduckgo.com/?q=",
            Self::Brave => "https://search.brave.com/search?q=",
        }
    }

    /// Mesin dari nilai tersimpan; `None` kalau tidak dikenal.
    pub fn dari_slug(slug: &str) -> Option<Self> {
        Self::SEMUA.into_iter().find(|mesin| mesin.slug() == slug)
    }

    /// URL halaman hasil pencarian untuk `kata`.
    pub fn url_pencarian(self, kata: &str) -> Option<Url> {
        // Awalan diakhiri `?q=`, jadi parameternya dibuang dulu supaya kata
        // kunci tidak menjadi pasangan `q` kedua yang kosong.
        let dasar = self.templat().trim_end_matches("?q=");
        let mut url = Url::parse(dasar).ok()?;
        url.query_pairs_mut().append_pair("q", kata);
        Some(url)
    }
}

/// Mesin pencari yang sedang dipakai proses ini. Disimpan sebagai nilai global
/// supaya address bar, tab baru, dan halaman beranda memakai pilihan yang sama
/// tanpa harus membawa basis data ke mana-mana.
static MESIN_AKTIF: RwLock<MesinPencarian> = RwLock::new(MesinPencarian::Google);

pub fn mesin_aktif() -> MesinPencarian {
    MESIN_AKTIF
        .read()
        .map(|mesin| *mesin)
        .unwrap_or(MesinPencarian::Google)
}

/// Mengganti mesin pencari; mengembalikan mesin baru, atau `None` kalau `slug`
/// tidak dikenal.
pub fn atur_mesin(slug: &str) -> Option<MesinPencarian> {
    let mesin = MesinPencarian::dari_slug(slug)?;
    if let Ok(mut aktif) = MESIN_AKTIF.write() {
        *aktif = mesin;
    }
    Some(mesin)
}

/// Mengubah input address bar menjadi URL.
///
/// Urutan keputusan: URL lengkap yang skemanya dikenal -> dianggap host (mis.
/// `example.com`) -> diperlakukan sebagai kata kunci pencarian.
/// Mengembalikan `None` untuk input kosong.
pub fn dari_input(input: &str) -> Option<Url> {
    let bersih = input.trim();
    if bersih.is_empty() {
        return None;
    }

    if let Ok(url) = Url::parse(bersih) {
        if SKEMA_DIIZINKAN.contains(&url.scheme()) {
            return Some(url);
        }
    }

    if terlihat_seperti_host(bersih) {
        let skema = if host_lokal(bersih) { "http" } else { "https" };
        if let Ok(url) = Url::parse(&format!("{skema}://{bersih}")) {
            return Some(url);
        }
    }

    mesin_aktif().url_pencarian(bersih)
}

/// Tanpa spasi, mengandung titik, atau host lokal: itulah tanda orang menulis alamat.
fn terlihat_seperti_host(bersih: &str) -> bool {
    !bersih.chars().any(char::is_whitespace)
        && (bersih.contains('.') || bersih.starts_with("localhost"))
}

/// Server pengembangan di mesin sendiri hampir selalu HTTP, bukan HTTPS.
fn host_lokal(bersih: &str) -> bool {
    let host = bersih
        .split(['/', ':'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_lengkap_dibiarkan() {
        let url = dari_input("https://example.com/berita?q=1").unwrap();
        assert_eq!(url.as_str(), "https://example.com/berita?q=1");
    }

    #[test]
    fn domain_tanpa_skema_dilengkapi_https() {
        let url = dari_input("example.com").unwrap();
        assert_eq!(url.as_str(), "https://example.com/");
    }

    #[test]
    fn host_lokal_pakai_http() {
        let url = dari_input("localhost:1420").unwrap();
        assert_eq!(url.as_str(), "http://localhost:1420/");
    }

    #[test]
    fn kata_kunci_jadi_pencarian_di_mesin_bawaan() {
        assert_eq!(mesin_aktif(), MesinPencarian::Google);
        let url = dari_input("kucing lucu").unwrap();
        assert_eq!(url.host_str(), Some("www.google.com"));
        assert_eq!(url.query_pairs().next().unwrap().1, "kucing lucu");
    }

    #[test]
    fn tiap_mesin_mencari_di_hostnya_sendiri() {
        assert_eq!(
            MesinPencarian::Bing
                .url_pencarian("kucing lucu")
                .unwrap()
                .host_str(),
            Some("www.bing.com")
        );
        assert_eq!(
            MesinPencarian::DuckDuckGo
                .url_pencarian("kucing lucu")
                .unwrap()
                .host_str(),
            Some("duckduckgo.com")
        );
        assert_eq!(
            MesinPencarian::Brave
                .url_pencarian("kucing lucu")
                .unwrap()
                .host_str(),
            Some("search.brave.com")
        );
    }

    #[test]
    fn slug_tidak_dikenal_ditolak() {
        assert_eq!(atur_mesin("mesin-asing"), None);
        assert_eq!(mesin_aktif(), MesinPencarian::Google);
    }

    #[test]
    fn setiap_mesin_punya_templat_https_dengan_q() {
        for mesin in MesinPencarian::SEMUA {
            assert!(mesin.templat().starts_with("https://"));
            assert!(mesin.templat().ends_with("?q="));
            assert_eq!(MesinPencarian::dari_slug(mesin.slug()), Some(mesin));
            let url = mesin.url_pencarian("a b").unwrap();
            assert_eq!(
                url.query_pairs().next().unwrap(),
                ("q".into(), "a b".into())
            );
        }
    }

    #[test]
    fn input_kosong_tidak_dinavigasi() {
        assert!(dari_input("").is_none());
        assert!(dari_input("   ").is_none());
    }

    #[test]
    fn skema_berbahaya_tidak_dibuka_langsung() {
        let url = dari_input("javascript:alert(1)").unwrap();
        assert_ne!(url.scheme(), "javascript");
        assert_eq!(url.host_str(), Some("www.google.com"));
    }

    #[test]
    fn url_file_diizinkan() {
        let url = dari_input("file:///tmp/uji.html").unwrap();
        assert_eq!(url.scheme(), "file");
    }
}
