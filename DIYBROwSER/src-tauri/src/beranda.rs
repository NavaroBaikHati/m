//! Halaman tab baru Perisai.
//!
//! Halaman ini sengaja berupa berkas HTML biasa di folder cache, bukan halaman
//! bertaut IPC: child webview untuk isi tab tidak punya izin Tauri, jadi
//! halaman statis adalah cara paling sederhana yang tetap aman. Pencarian dan
//! tautan cukup memakai navigasi biasa.

use tauri::{AppHandle, Manager, Url};

/// Nama berkas di folder cache.
const NAMA: &str = "beranda.html";

/// Menulis halaman tab baru (kalau belum ada) dan mengembalikan URL-nya.
pub fn siapkan(app: &AppHandle) -> Result<Url, String> {
    let folder = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("folder cache tidak tersedia: {e}"))?;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(NAMA);
    std::fs::write(&path, HTML).map_err(|e| format!("gagal menulis halaman beranda: {e}"))?;
    let url = Url::from_file_path(&path).map_err(|_| "path beranda tidak valid".to_string())?;
    Ok(url)
}

const HTML: &str = r##"<!doctype html>
<html lang="id">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Tab Baru</title>
<style>
  :root {
    color-scheme: light dark;
    --latar: #f7f5f2;
    --latar2: #efe9ff;
    --teks: #17161c;
    --samar: #6b6875;
    --kartu: rgba(255, 255, 255, 0.72);
    --garis: rgba(23, 22, 28, 0.1);
    --aksen: #6b4dfb;
    --aksen2: #ff7a59;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --latar: #131218;
      --latar2: #1d1830;
      --teks: #f2f0f7;
      --samar: #a09cab;
      --kartu: rgba(38, 35, 48, 0.72);
      --garis: rgba(242, 240, 247, 0.1);
      --aksen: #9b86ff;
      --aksen2: #ff9678;
    }
  }
  * { box-sizing: border-box; }
  html, body { margin: 0; height: 100%; }
  body {
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      radial-gradient(60rem 40rem at 20% -10%, var(--latar2), transparent 60%),
      radial-gradient(50rem 30rem at 110% 10%, color-mix(in srgb, var(--aksen2) 18%, transparent), transparent 60%),
      var(--latar);
    color: var(--teks);
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  main { width: min(560px, calc(100vw - 48px)); padding-bottom: 12vh; }
  .merek { display: flex; align-items: center; justify-content: center; gap: 12px; margin-bottom: 28px; }
  .merek svg { width: 34px; height: 34px; }
  .merek b { font-size: 19px; letter-spacing: 0.06em; font-weight: 650; }
  .salam { margin: 0 0 22px; text-align: center; font-size: 15px; color: var(--samar); }
  form { position: relative; }
  form input {
    width: 100%;
    height: 54px;
    padding: 0 54px 0 50px;
    border: 1px solid var(--garis);
    border-radius: 999px;
    background: var(--kartu);
    backdrop-filter: blur(18px);
    color: var(--teks);
    font: inherit;
    font-size: 15px;
    outline: none;
    box-shadow: 0 18px 40px -24px rgba(0, 0, 0, 0.45);
    transition: border-color 160ms ease, box-shadow 160ms ease;
  }
  form input:focus {
    border-color: color-mix(in srgb, var(--aksen) 55%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--aksen) 16%, transparent), 0 18px 40px -24px rgba(0,0,0,0.45);
  }
  form .kaca { position: absolute; left: 19px; top: 50%; translate: 0 -50%; color: var(--samar); display: grid; }
  form button {
    position: absolute; right: 8px; top: 50%; translate: 0 -50%;
    width: 38px; height: 38px; border: 0; border-radius: 50%;
    background: linear-gradient(135deg, var(--aksen), var(--aksen2));
    color: #fff; cursor: pointer; display: grid; place-items: center;
    transition: filter 160ms ease;
  }
  form button:hover { filter: brightness(1.08); }
  .pintasan { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-top: 30px; }
  .pintasan a {
    display: flex; flex-direction: column; align-items: center; gap: 9px;
    padding: 15px 8px; border: 1px solid transparent; border-radius: 18px;
    color: var(--teks); text-decoration: none; font-size: 12px;
    transition: background 160ms ease, border-color 160ms ease, transform 160ms ease;
  }
  .pintasan a:hover { background: var(--kartu); border-color: var(--garis); transform: translateY(-2px); }
  .pintasan .huruft { display: grid; place-items: center; width: 40px; height: 40px; border-radius: 13px; font-weight: 650; font-size: 16px; color: #fff; }
  .catatan { margin: 34px 0 0; text-align: center; font-size: 12px; color: var(--samar); display: flex; align-items: center; justify-content: center; gap: 7px; }
  .catatan svg { width: 13px; height: 13px; }
  @media (prefers-reduced-motion: reduce) { * { transition: none !important; } }
</style>
</head>
<body>
<main>
  <div class="merek">
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M12 2.5 4.5 5.6v5.5c0 4.9 3.2 8.7 7.5 10.4 4.3-1.7 7.5-5.5 7.5-10.4V5.6L12 2.5Z" stroke="url(#g)" stroke-width="1.7" stroke-linejoin="round"/>
      <path d="m8.8 12 2.2 2.2 4.2-4.4" stroke="url(#g)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
      <defs><linearGradient id="g" x1="4" y1="2" x2="20" y2="21"><stop stop-color="var(--aksen)"/><stop offset="1" stop-color="var(--aksen2)"/></linearGradient></defs>
    </svg>
    <b>PERISAI</b>
  </div>
  <p class="salam">Mau ke mana hari ini?</p>
  <form id="cari" autocomplete="off">
    <span class="kaca"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.6-3.6"/></svg></span>
    <input id="q" type="text" placeholder="Cari atau ketik alamat" spellcheck="false" autofocus />
    <button type="submit" aria-label="Cari">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h13M13 6l6 6-6 6"/></svg>
    </button>
  </form>
  <nav class="pintasan">
    <a href="https://www.youtube.com/"><span class="huruft" style="background:#ff3b30">Y</span>YouTube</a>
    <a href="https://github.com/"><span class="huruft" style="background:#24292f">G</span>GitHub</a>
    <a href="https://id.wikipedia.org/"><span class="huruft" style="background:#54595d">W</span>Wikipedia</a>
    <a href="https://news.ycombinator.com/"><span class="huruft" style="background:#ff6600">H</span>Hacker News</a>
  </nav>
  <p class="catatan">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M12 2.5 4.5 5.6v5.5c0 4.9 3.2 8.7 7.5 10.4 4.3-1.7 7.5-5.5 7.5-10.4V5.6L12 2.5Z"/></svg>
    Iklan dan pelacak disaring sebelum halaman dimuat.
  </p>
</main>
<script>
  const form = document.getElementById("cari");
  const q = document.getElementById("q");
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    const nilai = q.value.trim();
    if (!nilai) return;
    const alamat = /^[a-z]+:\/\//i.test(nilai)
      ? nilai
      : (/^[^\s]+\.[^\s]{2,}$/.test(nilai)
          ? "https://" + nilai
          : "https://duckduckgo.com/?q=" + encodeURIComponent(nilai));
    location.href = alamat;
  });
</script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halaman_berisi_pencarian_dan_merek() {
        assert!(HTML.contains("PERISAI"));
        assert!(HTML.contains("duckduckgo.com"));
        assert!(HTML.contains("prefers-color-scheme: dark"));
        // Halaman tab baru tidak boleh memuat sumber luar selain tautan biasa.
        assert!(!HTML.contains("http://"));
    }
}
