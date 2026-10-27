//! Menerjemahkan isi address bar menjadi URL yang bisa dinavigasi.

use tauri::Url;

/// Halaman pencarian. PRD tidak menyebut merek mesin pencari, jadi dipakai
/// DuckDuckGo sebagai asumsi paling sederhana (dan tanpa profil pengguna).
pub const HALAMAN_PENCARIAN: &str = "https://duckduckgo.com/";

/// Skema yang boleh dibuka langsung dari address bar.
/// Sisanya (mis. `javascript:`) diperlakukan sebagai teks pencarian.
const SKEMA_DIIZINKAN: &[&str] = &["http", "https", "file", "about"];

/// URL yang dibuka kalau aplikasi dijalankan tanpa argumen URL.
pub const URL_AWAL: &str = "https://example.com/";

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

    let mut url = Url::parse(HALAMAN_PENCARIAN).ok()?;
    url.query_pairs_mut().append_pair("q", bersih);
    Some(url)
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
    fn kata_kunci_jadi_pencarian() {
        let url = dari_input("kucing lucu").unwrap();
        assert_eq!(url.host_str(), Some("duckduckgo.com"));
        assert_eq!(url.query_pairs().next().unwrap().1, "kucing lucu");
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
        assert_eq!(url.host_str(), Some("duckduckgo.com"));
    }

    #[test]
    fn url_file_diizinkan() {
        let url = dari_input("file:///tmp/uji.html").unwrap();
        assert_eq!(url.scheme(), "file");
    }
}
