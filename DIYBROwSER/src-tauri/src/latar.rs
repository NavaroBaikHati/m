//! Skrip kecil yang menjaga tab aktif tetap dianggap "terlihat".
//!
//! WebKitGTK 2.46+ meneruskan keadaan jendela ke halaman: begitu jendela
//! di-minimize, berpindah workspace, atau tertutup jendela lain, WebKit
//! mengirim `visibilitychange` dan menyetel `document.hidden`. Itu benar untuk
//! animasi (menghemat CPU), tetapi situs memakainya untuk **menghentikan
//! pemutaran**: YouTube dan hampir semua pemutar musik memanggil
//! `audio.pause()` saat `document.hidden` menjadi true.
//!
//! Chrome memilih lain: jendela yang di-minimize tidak dianggap menyembunyikan
//! tab yang sedang aktif, jadi musik tetap jalan. Perisai mengikuti pilihan itu
//! — tetapi hanya untuk tab yang sedang aktif. Tab lain yang memang
//! disembunyikan Perisai tetap menerima `visibilitychange` seperti biasa,
//! sehingga hanya satu tab yang bersuara.
//!
//! Caranya: user script dipasang sebelum dokumen pertama dimuat (lihat
//! `platform::pasang_skrip_latar`). Skrip itu membungkus `hidden` /
//! `visibilityState` dengan pengecekan satu variabel global, dan menahan event
//! `visibilitychange` selama variabel itu menyala. Variabelnya dinyalakan dan
//! dimatikan Rust lewat `webview.eval` setiap kali tab menjadi aktif atau
//! tidak; dokumen baru selalu mulai dengan variabel menyala saat tab aktif
//! selesai dimuat (lihat `TabManager`).
//!
//! Yang tidak bisa diperbaiki dari sini: WebKit tetap menurunkan frekuensi
//! timer halaman yang jendelanya tidak terlihat. Bunyi tetap keluar karena
//! pemutaran berjalan di proses WebKit, bukan di timer JavaScript.

/// Skrip yang dipasang sebagai user script pada setiap dokumen.
pub const SKRIP: &str = r#"(function () {
  if (window.__perisaiVisibilitas) return;
  window.__perisaiVisibilitas = true;
  window.__perisaiPaksaTerlihat = false;

  function dipaksa() {
    return window.__perisaiPaksaTerlihat === true;
  }

  // Disimpan sebelum propertinya ditimpa: dipakai untuk tahu apakah halaman
  // benar-benar sedang disembunyikan.
  var asliHidden = Object.getOwnPropertyDescriptor(Document.prototype, "hidden");

  function alihkan(nama, nilaiPaksa) {
    var asli = Object.getOwnPropertyDescriptor(Document.prototype, nama);
    if (!asli || !asli.get) return;
    try {
      Object.defineProperty(Document.prototype, nama, {
        configurable: true,
        enumerable: asli.enumerable,
        get: function () {
          if (dipaksa()) return nilaiPaksa;
          return asli.get.call(this);
        },
      });
    } catch (e) {
      /* Properti tidak bisa ditimpa: halaman berperilaku seperti biasa. */
    }
  }

  alihkan("hidden", false);
  alihkan("visibilityState", "visible");
  alihkan("webkitHidden", false);
  alihkan("webkitVisibilityState", "visible");

  function sedangDisembunyikan() {
    try {
      return !!(asliHidden && asliHidden.get && asliHidden.get.call(document));
    } catch (e) {
      return false;
    }
  }

  // Hanya event "disembunyikan" yang ditahan. Event "terlihat lagi" harus
  // tetap sampai ke halaman, kalau tidak situs yang sempat berhenti tidak
  // pernah tahu ia boleh melanjutkan saat tabnya dipilih kembali.
  function tahan(event) {
    if (dipaksa() && sedangDisembunyikan()) event.stopImmediatePropagation();
  }

  // Ditangkap lebih dulu daripada pendengar halaman (dipasang sebelum dokumen
  // dimuat), jadi pendengar milik situs tidak pernah menerima event saat tab
  // aktif dan jendela disembunyikan.
  var sasaran = [window, document];
  for (var i = 0; i < sasaran.length; i++) {
    sasaran[i].addEventListener("visibilitychange", tahan, true);
    sasaran[i].addEventListener("webkitvisibilitychange", tahan, true);
  }
})();"#;

/// Apakah paksaan diaktifkan? Bisa dimatikan lewat `PERISAI_TETAP_LATAR=0`
/// bagi yang lebih memilih animasi berhenti saat jendela disembunyikan.
pub fn aktif() -> bool {
    aktif_dari(std::env::var("PERISAI_TETAP_LATAR").ok().as_deref())
}

fn aktif_dari(nilai: Option<&str>) -> bool {
    !matches!(nilai, Some("0") | Some("mati"))
}

/// Perintah eval untuk menyalakan atau mematikan paksaan pada satu halaman.
pub fn perintah(nyala: bool) -> &'static str {
    if nyala {
        "window.__perisaiPaksaTerlihat = true"
    } else {
        "window.__perisaiPaksaTerlihat = false"
    }
}

#[cfg(test)]
mod tests {
    use super::{aktif_dari, perintah, SKRIP};

    #[test]
    fn skrip_mengganti_kedua_nama_properti_visibilitas() {
        assert!(SKRIP.contains("\"hidden\""));
        assert!(SKRIP.contains("\"visibilityState\""));
        // Safari/WebKit lama memakai awalan webkit.
        assert!(SKRIP.contains("webkitHidden"));
        assert!(SKRIP.contains("webkitVisibilityState"));
    }

    #[test]
    fn skrip_menahan_event_di_fase_tangkap() {
        assert!(SKRIP.contains("stopImmediatePropagation"));
        assert!(SKRIP.contains("addEventListener(\"visibilitychange\", tahan, true)"));
    }

    #[test]
    fn hanya_event_disembunyikan_yang_ditahan() {
        // Event "terlihat lagi" harus lolos, kalau tidak halaman tidak pernah
        // tahu ia boleh melanjutkan setelah tabnya dipilih kembali.
        assert!(SKRIP.contains("dipaksa() && sedangDisembunyikan()"));
    }

    #[test]
    fn paksaan_bisa_dimatikan_lewat_lingkungan() {
        assert!(aktif_dari(None));
        assert!(aktif_dari(Some("1")));
        assert!(!aktif_dari(Some("0")));
        assert!(!aktif_dari(Some("mati")));
    }

    #[test]
    fn perintah_menyalakan_dan_mematikan() {
        assert_eq!(perintah(true), "window.__perisaiPaksaTerlihat = true");
        assert_eq!(perintah(false), "window.__perisaiPaksaTerlihat = false");
    }
}
