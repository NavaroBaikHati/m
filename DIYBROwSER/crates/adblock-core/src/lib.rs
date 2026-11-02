//! Inti adblock yang bebas dari Tauri dan UI.
//!
//! Semua yang berhubungan dengan penyaringan request, cosmetic filtering, dan
//! scriptlet ditaruh di sini supaya bisa dites penuh tanpa layar (dan mudah
//! dipindah kalau nanti pindah dari Tauri ke CEF).
//!
//! Mesinnya adalah `adblock` (adblock-rust milik Brave). Dua keluaran yang
//! dipakai cangkang browser:
//!
//! 1. **Content blocker JSON** — aturan jaringan dan penyembunyian elemen dalam
//!    format yang dimengerti WebKit (`WebKitUserContentFilter` di Linux,
//!    `WKContentRuleList` di macOS). WebKit yang mencocokkan request, jadi tidak
//!    ada biaya per-request di sisi kita.
//! 2. **Scriptlet** — potongan JS per situs untuk iklan yang tidak bisa
//!    diblokir di lapisan jaringan (iklan video YouTube adalah contohnya).
#![forbid(unsafe_code)]

use std::collections::HashSet;

use adblock::lists::{FilterSet, ParseOptions};

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

/// Aturan tambahan milik Perisai.
///
/// Isinya sengaja kecil dan hanya menutup celah yang tidak ditangani EasyList:
/// permukaan iklan YouTube yang dirender sebagai elemen pihak pertama, dan
/// cosmetic filter untuk halaman tab baru YouTube.
pub const ATURAN_TAMBAHAN: &str = r#"! Title: Perisai tambahan
! Iklan yang dirender sebagai elemen pihak pertama (tidak bisa diblokir jaringan)
youtube.com##.ytp-ad-module
youtube.com##.ytp-ad-overlay-container
youtube.com##.ytp-ad-progress-list
youtube.com##ytd-display-ad-renderer
youtube.com##ytd-ad-slot-renderer
youtube.com##ytd-promoted-sparkles-web-renderer
youtube.com##ytd-promoted-video-renderer
youtube.com##ytm-promoted-sparkles-web-renderer
youtube.com###masthead-ad
youtube.com##ytd-statement-banner-renderer
youtube.com##ytd-in-feed-ad-layout-renderer
youtube.com###player-ads
youtube.com###related ytd-ad-slot-renderer
youtube.com##.ytp-suggested-action-badge
! Endpoint statistik iklan YouTube
||youtube.com/api/stats/ads
||youtube.com/pagead/
||youtube.com/ptracking
! Jaringan iklan yang kadang lolos karena disajikan pihak pertama
||doubleclick.net^$third-party
||googlesyndication.com^$third-party
||googleadservices.com^$third-party
||adservice.google.com^$third-party
||amazon-adsystem.com^$third-party
! End
"#;

/// Hasil kompilasi daftar filter: mesin untuk query cosmetic/scriptlet dan
/// JSON content blocker untuk dipasang ke WebKit.
pub struct Hasil {
    pub mesin: Mesin,
    pub filter_json: String,
}

/// Mesin pencocokan + kumpulan aturan cosmetic.
pub struct Mesin {
    engine: adblock::Engine,
    jumlah_aturan: usize,
}

impl Mesin {
    /// Jumlah aturan content blocker yang aktif.
    pub fn jumlah_aturan(&self) -> usize {
        self.jumlah_aturan
    }

    /// Selector yang harus disembunyikan di `url`, hasil cosmetic filter.
    pub fn css_untuk(&self, url: &str) -> Vec<String> {
        let sumber = self.engine.url_cosmetic_resources(url);
        let mut selector: Vec<String> = sumber.hide_selectors.into_iter().collect();
        selector.sort();
        selector
    }

    /// Scriptlet (JS) yang harus dijalankan di `url`, termasuk perkakas
    /// tambahan Perisai untuk iklan video YouTube.
    pub fn skrip_untuk(&self, url: &str) -> String {
        let sumber = self.engine.url_cosmetic_resources(url);
        let mut skrip = sumber.injected_script;
        if host_dari(url)
            .is_some_and(|host| host == "youtube.com" || host.ends_with(".youtube.com"))
        {
            skrip.push_str(SCRIPTLET_YOUTUBE);
        }
        skrip
    }
}

/// Scriptlet pelewat iklan video YouTube.
///
/// Iklan video datang dari host yang sama dengan video biasa, jadi tidak bisa
/// diblokir di lapisan jaringan tanpa ikut memblokir videonya. Yang dilakukan
/// di sini: saat elemen pemutar berstatus `.ad-showing`, video iklan dilompati
/// ke akhir dan tombol "Lewati" diklik. Ini pendekatan best-effort yang sama
/// dipakai banyak blocker; hasilnya bisa berubah kalau YouTube mengubah DOM.
pub const SCRIPTLET_YOUTUBE: &str = r##"
;(() => {
  if (window.__perisaiYoutube) return;
  window.__perisaiYoutube = true;
  const lewati = () => {
    const pemutar = document.querySelector("#movie_player, .html5-video-player");
    if (!pemutar || !pemutar.classList.contains("ad-showing")) return;
    const video = pemutar.querySelector("video");
    if (video && Number.isFinite(video.duration) && video.duration > 0) {
      try { video.currentTime = video.duration; } catch (_) {}
    }
    const tombol = pemutar.querySelector(
      ".ytp-ad-skip-button-modern, .ytp-ad-skip-button, .ytp-skip-ad-button"
    );
    if (tombol) tombol.click();
  };
  setInterval(lewati, 350);
  document.addEventListener("DOMContentLoaded", lewati, { once: true });
})();
"##;

/// Mengompilasi daftar filter menjadi mesin + content blocker JSON.
///
/// `daftar` berisi teks mentah tiap daftar filter (EasyList, EasyPrivacy, ...).
/// Aturan tambahan Perisai selalu disertakan.
pub fn bangun(daftar: &[String]) -> Result<Hasil, String> {
    let mut semua: Vec<String> = daftar.to_vec();
    semua.push(ATURAN_TAMBAHAN.to_string());

    let mut set_filter = FilterSet::new(true);
    for teks in &semua {
        set_filter.add_filter_list(teks.clone(), ParseOptions::default());
    }
    let (aturan, dipakai) = set_filter
        .into_content_blocking()
        .map_err(|_| "konversi content blocker gagal".to_string())?;
    let filter_json =
        serde_json::to_string(&aturan).map_err(|e| format!("serialisasi filter gagal: {e}"))?;

    // Engine terpisah untuk query cosmetic dan scriptlet; FilterSet di atas
    // sudah habis dipakai oleh konversi.
    let mut set_mesin = FilterSet::new(false);
    for teks in &semua {
        set_mesin.add_filter_list(teks.clone(), ParseOptions::default());
    }
    let engine = adblock::Engine::new_with_filter_set(set_mesin);

    Ok(Hasil {
        mesin: Mesin {
            engine,
            jumlah_aturan: aturan.len(),
        },
        filter_json: {
            // `dipakai` hanya dipakai untuk diagnostik; jangan sampai compiler
            // menganggapnya tidak terpakai.
            let _ = dipakai.len();
            filter_json
        },
    })
}

/// Mesin tanpa aturan apa pun; dipakai sebelum daftar filter selesai dimuat
/// atau saat pengguna mematikan perlindungan.
pub fn kosong() -> Mesin {
    Mesin {
        engine: adblock::Engine::new_with_filter_set(FilterSet::new(false)),
        jumlah_aturan: 0,
    }
}

/// Host (tanpa `www.`) dari URL, untuk keputusan per-situs.
pub fn host_dari(url: &str) -> Option<String> {
    let setelah_skema = url.split_once("://")?.1;
    let host = setelah_skema
        .split(['/', '?', '#'])
        .next()?
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host.strip_prefix("www.").unwrap_or(&host).to_string())
    }
}

/// Selector cosmetic unik dari beberapa URL, dipakai untuk menghitung
/// perkiraan elemen yang akan disembunyikan (dipakai UI panel shield).
pub fn gabung_selector(mesin: &Mesin, url: &str) -> usize {
    let selector: HashSet<String> = mesin.css_untuk(url).into_iter().collect();
    selector.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UJI: &str = r#"
! uji
||iklan.example.com^
||tracker.example.com^$third-party
example.com##.spanduk-iklan
example.com##div[id="promo"]
"#;

    fn mesin_uji() -> Hasil {
        bangun(&[UJI.to_string()]).unwrap()
    }

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

    #[test]
    fn filter_json_valid_dan_berisi_blokir_jaringan() {
        let hasil = mesin_uji();
        let nilai: serde_json::Value = serde_json::from_str(&hasil.filter_json).unwrap();
        let aturan = nilai
            .as_array()
            .expect("content blocker harus berupa array");
        assert!(!aturan.is_empty(), "content blocker kosong");
        assert!(
            aturan.iter().any(|r| r["action"]["type"] == "block"
                && r["trigger"]["url-filter"]
                    .as_str()
                    .is_some_and(|f| f.contains("iklan") && f.contains("example"))),
            "aturan blokir iklan.example.com tidak ada di JSON"
        );
        assert!(hasil.mesin.jumlah_aturan() > 0);
    }

    #[test]
    fn cosmetic_filter_menghasilkan_selector_untuk_host_yang_benar() {
        let hasil = mesin_uji();
        let selector = hasil.mesin.css_untuk("https://example.com/artikel");
        assert!(
            selector.iter().any(|s| s.contains("spanduk-iklan")),
            "selector cosmetic tidak muncul: {selector:?}"
        );
        // Host lain tidak boleh ikut menghukum halaman yang tidak terkait.
        let lain = hasil.mesin.css_untuk("https://situslain.example/");
        assert!(!lain.iter().any(|s| s.contains("spanduk-iklan")));
    }

    #[test]
    fn aturan_tambahan_youtube_ada_di_content_blocker() {
        let hasil = mesin_uji();
        assert!(
            hasil.filter_json.contains("ytp-ad-module"),
            "aturan cosmetic YouTube tidak ikut terkompilasi"
        );
    }

    #[test]
    fn scriptlet_youtube_hanya_untuk_youtube() {
        let hasil = mesin_uji();
        let di_youtube = hasil
            .mesin
            .skrip_untuk("https://www.youtube.com/watch?v=abc");
        assert!(di_youtube.contains("__perisaiYoutube"));
        let di_lain = hasil.mesin.skrip_untuk("https://example.com/");
        assert!(!di_lain.contains("__perisaiYoutube"));
    }

    #[test]
    fn url_tanpa_skema_tidak_panic() {
        let hasil = mesin_uji();
        assert!(hasil.mesin.css_untuk("bukan url").is_empty());
        assert_eq!(host_dari("bukan url"), None);
    }

    #[test]
    fn host_dinormalkan() {
        assert_eq!(
            host_dari("https://www.Example.com:443/a"),
            Some("example.com".into())
        );
        assert_eq!(
            host_dari("http://localhost:1420/"),
            Some("localhost".into())
        );
        assert_eq!(
            host_dari("https://user@sub.example.com/x"),
            Some("sub.example.com".into())
        );
    }

    #[test]
    fn mesin_kosong_tidak_punya_aturan() {
        let kosong = kosong();
        assert_eq!(kosong.jumlah_aturan(), 0);
        assert!(kosong.css_untuk("https://example.com/").is_empty());
        assert!(kosong.skrip_untuk("https://example.com/").is_empty());
    }
}
