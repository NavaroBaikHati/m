//! Halaman tab baru Perisai.
//!
//! Halaman ini sengaja berupa berkas HTML biasa di folder cache, bukan halaman
//! bertaut IPC: child webview untuk isi tab tidak punya izin Tauri, jadi
//! halaman statis adalah cara paling sederhana yang tetap aman. Pencarian dan
//! tautan cukup memakai navigasi biasa.
//!
//! Rancangannya "lembar indeks": kertas bergaris buku besar, judul serif
//! besar, label monospace huruf kecil semua, dan tautan bernomor dengan titik
//! pemandu seperti daftar isi. Tidak ada kartu membulat, tidak ada gradien
//! berwarna, tidak ada fon dari internet — halaman ini harus tetap ringan di
//! mesin 1,7 GB dan tetap bisa dibuka tanpa jaringan.

use tauri::{AppHandle, Manager, Url};

/// Nama berkas di folder cache.
const NAMA: &str = "beranda.html";

/// Menulis halaman tab baru dan mengembalikan URL-nya.
///
/// Ditulis ulang setiap dipanggil supaya mesin pencari yang baru dipilih di
/// menu langsung dipakai tab berikutnya. `tema` adalah tema aplikasi
/// (`gelap`, `terang`, atau `sistem`); halaman ini tidak punya akses ke
/// pengaturan Rust, jadi nilainya ditempel saat berkas ditulis.
pub fn siapkan(app: &AppHandle, tema: &str) -> Result<Url, String> {
    let folder = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("folder cache tidak tersedia: {e}"))?;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(NAMA);
    let isi = halaman(crate::url_input::mesin_aktif().templat(), tema);
    std::fs::write(&path, isi).map_err(|e| format!("gagal menulis halaman beranda: {e}"))?;
    let url = Url::from_file_path(&path).map_err(|_| "path beranda tidak valid".to_string())?;
    Ok(url)
}

/// Isi halaman dengan awalan URL mesin pencari dan tema terpasang.
pub fn halaman(templat: &str, tema: &str) -> String {
    let tema = match tema {
        "gelap" | "terang" => tema,
        _ => "sistem",
    };
    HTML.replace("__MESIN_PENCARIAN__", templat)
        .replace("__TEMA__", tema)
}

const HTML: &str = r##"<!doctype html>
<html lang="id" data-tema="__TEMA__">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Tab Baru</title>
<style>
  :root {
    color-scheme: light;
    --kertas: #f4f2ec;
    --tinta: #14130f;
    --samar: #6c685e;
    --garis: rgba(20, 19, 15, 0.14);
    --garis-tipis: rgba(20, 19, 15, 0.07);
    --sorot: rgba(20, 19, 15, 0.045);
    --serif: "Iowan Old Style", "Palatino Linotype", Palatino, "Book Antiqua",
      Georgia, "Times New Roman", serif;
    --mono: ui-monospace, "SF Mono", "JetBrains Mono", "Cascadia Mono", Menlo,
      Consolas, "Liberation Mono", monospace;
  }
  /* Tema aplikasi menang; "sistem" mengikuti preferensi desktop. */
  :root[data-tema="gelap"] {
    color-scheme: dark;
    --kertas: #0d0e10;
    --tinta: #ecebe6;
    --samar: #8b887f;
    --garis: rgba(236, 235, 230, 0.16);
    --garis-tipis: rgba(236, 235, 230, 0.06);
    --sorot: rgba(236, 235, 230, 0.05);
  }
  @media (prefers-color-scheme: dark) {
    :root[data-tema="sistem"] {
      color-scheme: dark;
      --kertas: #0d0e10;
      --tinta: #ecebe6;
      --samar: #8b887f;
      --garis: rgba(236, 235, 230, 0.16);
      --garis-tipis: rgba(236, 235, 230, 0.06);
      --sorot: rgba(236, 235, 230, 0.05);
    }
  }
  * { box-sizing: border-box; }
  html, body { margin: 0; min-height: 100%; }
  body {
    background: var(--kertas);
    color: var(--tinta);
    font-family: var(--serif);
    -webkit-font-smoothing: antialiased;
    font-kerning: normal;
    font-variant-ligatures: common-ligatures;
  }
  /* Kertas bergaris: satu garis rambut tiap 34px, seperti buku besar. */
  body::before {
    content: "";
    position: fixed;
    inset: 0;
    background-image: repeating-linear-gradient(
      to bottom,
      var(--garis-tipis) 0 1px,
      transparent 1px 34px
    );
    pointer-events: none;
  }
  /* Label vertikal di tepi kiri, hanya kalau layar cukup lebar. */
  .tepi {
    position: fixed;
    top: 50%;
    left: 18px;
    translate: 0 -50%;
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.34em;
    text-transform: uppercase;
    color: var(--samar);
    writing-mode: vertical-rl;
    rotate: 180deg;
    opacity: 0.75;
  }
  main {
    position: relative;
    width: min(880px, calc(100vw - 76px));
    margin: 0 auto;
    padding: 9vh 0 7vh;
  }
  .kepala {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 24px;
    padding-bottom: 13px;
    border-bottom: 1px solid var(--garis);
  }
  .merek {
    display: flex;
    align-items: center;
    gap: 9px;
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
  }
  .merek svg { width: 17px; height: 17px; flex: none; }
  .merek b { font-weight: 500; text-transform: uppercase; }
  .merek span { color: var(--samar); }
  .jam {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.1em;
    font-variant-numeric: tabular-nums;
    color: var(--samar);
    text-transform: uppercase;
    text-align: right;
  }
  .jam b { display: block; color: var(--tinta); font-size: 12px; letter-spacing: 0.16em; }
  .isi { padding-top: 46px; }
  .etiket {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: var(--samar);
    margin: 0 0 18px;
  }
  h1 {
    margin: 0 0 10px;
    font-size: clamp(44px, 7.6vw, 82px);
    font-weight: 400;
    line-height: 0.92;
    letter-spacing: -0.032em;
    text-wrap: balance;
  }
  h1 em {
    display: block;
    font-style: italic;
    letter-spacing: -0.038em;
    text-indent: 0.06em;
  }
  .garis-bawah {
    height: 1px;
    background: var(--garis);
    margin: 34px 0 30px;
  }
  form { margin: 0 0 40px; }
  .label {
    display: block;
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.26em;
    text-transform: uppercase;
    color: var(--samar);
    margin-bottom: 8px;
  }
  .baris {
    display: flex;
    align-items: center;
    gap: 14px;
    border-bottom: 1px solid var(--garis);
    transition: border-color 140ms linear;
  }
  .baris:focus-within { border-color: var(--tinta); }
  input {
    flex: 1;
    min-width: 0;
    height: 46px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--tinta);
    font-family: var(--serif);
    font-size: 19px;
    letter-spacing: -0.01em;
    outline: none;
  }
  input::placeholder {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--samar);
    opacity: 1;
  }
  form button {
    flex: none;
    width: 34px;
    height: 34px;
    margin-bottom: 6px;
    border: 1px solid var(--garis);
    border-radius: 0;
    background: none;
    color: var(--tinta);
    font-family: var(--mono);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    transition: background 140ms linear, border-color 140ms linear;
  }
  form button:hover { background: var(--sorot); border-color: var(--tinta); }

  .indeks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 40px;
    border-top: 1px solid var(--garis);
  }
  .indeks a {
    display: flex;
    align-items: baseline;
    gap: 11px;
    padding: 12px 2px;
    border-bottom: 1px solid var(--garis-tipis);
    color: var(--tinta);
    text-decoration: none;
    transition: background 140ms linear, padding-left 140ms linear;
  }
  .indeks a:hover { background: var(--sorot); padding-left: 8px; }
  .no {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.12em;
    color: var(--samar);
    font-variant-numeric: tabular-nums;
  }
  .nama { font-size: 16.5px; letter-spacing: -0.005em; white-space: nowrap; }
  .pemandu {
    flex: 1;
    min-width: 14px;
    border-bottom: 1px dotted var(--garis);
    translate: 0 -3px;
  }
  .host {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.08em;
    color: var(--samar);
    white-space: nowrap;
  }
  .kaki {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin-top: 34px;
    padding-top: 13px;
    border-top: 1px solid var(--garis);
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--samar);
  }
  .kaki .kiri { display: flex; align-items: center; gap: 7px; }
  .kaki svg { width: 12px; height: 12px; flex: none; }
  @media (max-width: 1000px) {
    .tepi { display: none; }
  }
  @media (max-width: 720px) {
    main { width: calc(100vw - 44px); padding-top: 6vh; }
    .indeks { grid-template-columns: 1fr; }
    .kaki { flex-direction: column; align-items: flex-start; gap: 8px; }
  }
  @media (prefers-reduced-motion: reduce) { * { transition: none !important; } }
</style>
</head>
<body>
<p class="tepi" aria-hidden="true">Perisai — lembar indeks</p>
<main>
  <header class="kepala">
    <div class="merek">
      <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path d="M12 2.5 4.5 5.6v5.5c0 4.9 3.2 8.7 7.5 10.4 4.3-1.7 7.5-5.5 7.5-10.4V5.6L12 2.5Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>
        <path d="m8.8 12 2.2 2.2 4.2-4.4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <b>Perisai</b>
      <span>· lembar indeks</span>
    </div>
    <p class="jam" aria-live="off">
      <b id="jam">--:--</b>
      <span id="tanggal">memuat tanggal</span>
    </p>
  </header>

  <div class="isi">
    <p class="etiket">Tab baru · tanpa iklan</p>
    <h1>Mau ke mana<em>hari ini?</em></h1>

    <div class="garis-bawah"></div>

    <form id="cari" autocomplete="off">
      <label class="label" for="q">Cari atau ketik alamat</label>
      <div class="baris">
        <input id="q" type="text" placeholder="kata kunci / domain" spellcheck="false" autofocus />
        <button type="submit" aria-label="Buka" title="Buka (Enter)">&#8629;</button>
      </div>
    </form>

    <nav class="indeks" aria-label="Sering dikunjungi">
      <a href="https://www.youtube.com/">
        <span class="no">01</span><span class="nama">YouTube</span><span class="pemandu"></span><span class="host">youtube.com</span>
      </a>
      <a href="https://github.com/">
        <span class="no">02</span><span class="nama">GitHub</span><span class="pemandu"></span><span class="host">github.com</span>
      </a>
      <a href="https://id.wikipedia.org/">
        <span class="no">03</span><span class="nama">Wikipedia</span><span class="pemandu"></span><span class="host">wikipedia.org</span>
      </a>
      <a href="https://news.ycombinator.com/">
        <span class="no">04</span><span class="nama">Hacker News</span><span class="pemandu"></span><span class="host">ycombinator.com</span>
      </a>
    </nav>

    <footer class="kaki">
      <span class="kiri">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 2.5 4.5 5.6v5.5c0 4.9 3.2 8.7 7.5 10.4 4.3-1.7 7.5-5.5 7.5-10.4V5.6L12 2.5Z"/>
        </svg>
        Iklan dan pelacak disaring sebelum halaman dimuat
      </span>
      <span>Ctrl+L untuk alamat</span>
    </footer>
  </div>
</main>
<script>
  (function () {
    var jam = document.getElementById("jam");
    var tanggal = document.getElementById("tanggal");
    function perbarui() {
      var kini = new Date();
      try {
        jam.textContent = kini.toLocaleTimeString("id-ID", {
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
        });
        tanggal.textContent = kini.toLocaleDateString("id-ID", {
          weekday: "long",
          day: "numeric",
          month: "long",
        });
      } catch (e) {
        jam.textContent = kini.getHours() + ":" + String(kini.getMinutes()).padStart(2, "0");
        tanggal.textContent = "";
      }
    }
    perbarui();
    setInterval(perbarui, 15000);

    var form = document.getElementById("cari");
    var q = document.getElementById("q");
    form.addEventListener("submit", function (e) {
      e.preventDefault();
      var nilai = q.value.trim();
      if (!nilai) return;
      var alamat = /^[a-z]+:\/\//i.test(nilai)
        ? nilai
        : /^[^\s]+\.[^\s]{2,}$/.test(nilai)
          ? "https://" + nilai
          : "__MESIN_PENCARIAN__" + encodeURIComponent(nilai);
      location.href = alamat;
    });
  })();
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halaman_berisi_pencarian_dan_merek() {
        assert!(HTML.to_lowercase().contains("perisai"));
        assert!(HTML.contains("__MESIN_PENCARIAN__"));
        assert!(HTML.contains("prefers-color-scheme: dark"));
        // Halaman tab baru tidak boleh memuat sumber luar selain tautan biasa.
        assert!(!HTML.contains("http://"));
        assert!(!HTML.contains("@import"));
    }

    #[test]
    fn mesin_pencarian_terpasang_di_halaman() {
        let isi = halaman(crate::url_input::MesinPencarian::Bing.templat(), "gelap");
        assert!(isi.contains("\"https://www.bing.com/search?q=\" + encodeURIComponent"));
        assert!(!isi.contains("__MESIN_PENCARIAN__"));
    }

    #[test]
    fn tema_aplikasi_ditempel_dan_nilai_asing_jadi_sistem() {
        assert!(halaman("x", "gelap").contains("data-tema=\"gelap\""));
        assert!(halaman("x", "terang").contains("data-tema=\"terang\""));
        assert!(halaman("x", "ngawur").contains("data-tema=\"sistem\""));
        assert!(!halaman("x", "gelap").contains("__TEMA__"));
    }

    #[test]
    fn tipografi_memakai_fon_sistem() {
        // Tanpa fon dari internet: halaman harus tampil sama saat offline.
        assert!(HTML.contains("Iowan Old Style"));
        assert!(HTML.contains("ui-monospace"));
        assert!(!HTML.contains("fonts.googleapis"));
    }
}
