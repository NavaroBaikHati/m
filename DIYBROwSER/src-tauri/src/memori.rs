//! Pemantauan memori untuk mode hemat.
//!
//! Laptop lama sering hanya punya 1–2 GB RAM. WebKitGTK memakai satu proses
//! untuk setiap halaman, jadi jumlah tab yang terbuka langsung menentukan
//! apakah mesin masih bisa bernapas atau mulai menukar (swap) ke disk — dan
//! begitu swap dipakai, memuat halaman terasa sepuluh kali lebih lambat.
//!
//! Karena itu Perisai perlu tahu berapa memori yang benar-benar tersedia,
//! bukan berapa yang "bebas": `MemFree` tidak menghitung cache halaman yang
//! bisa dibuang kernel, sedangkan `MemAvailable` menghitungnya. Angka yang
//! dipakai di sini adalah `MemAvailable`.

/// Ambang bawaan: di bawah 300 MB tersedia, tab yang tidak aktif ditidurkan.
/// Pada mesin 8 GB angka ini praktis tidak pernah tersentuh.
pub const AMBANG_BAWAAN_MB: u64 = 300;

/// Isi `/proc/meminfo` yang kita pakai.
pub fn baca_meminfo(isi: &str) -> Option<u64> {
    for baris in isi.lines() {
        let Some(sisa) = baris.strip_prefix("MemAvailable:") else {
            continue;
        };
        // Formatnya "MemAvailable:   1234567 kB".
        let angka = sisa.trim().strip_suffix("kB")?.trim();
        let kb: u64 = angka.parse().ok()?;
        return Some(kb / 1024);
    }
    None
}

/// Memori yang tersedia sekarang, dalam MB. `None` kalau tidak bisa dibaca
/// (bukan Linux, atau `/proc` tidak tersedia).
pub fn tersedia_mb() -> Option<u64> {
    baca_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

/// Ambang tidur dari `PERISAI_TIDUR_MB`; `None` berarti fitur dimatikan
/// (nilai `0`, atau tidak ada `/proc/meminfo`).
pub fn ambang_tidur() -> Option<u64> {
    let ambang = match std::env::var("PERISAI_TIDUR_MB") {
        Ok(nilai) if !nilai.trim().is_empty() => nilai.trim().parse().ok()?,
        _ => AMBANG_BAWAAN_MB,
    };
    if ambang == 0 {
        return None;
    }
    tersedia_mb()?;
    Some(ambang)
}

#[cfg(test)]
mod tests {
    use super::{baca_meminfo, AMBANG_BAWAAN_MB};

    #[test]
    fn meminfo_memakai_memavailable_dalam_mb() {
        let isi = "MemTotal:        1800000 kB\nMemFree:          100000 kB\n\
MemAvailable:     614400 kB\nBuffers:           10000 kB\n";
        assert_eq!(baca_meminfo(isi), Some(600));
    }

    #[test]
    fn meminfo_tanpa_memavailable_tidak_menebak() {
        // `MemFree` sengaja ada: memakainya akan melebih-lebihkan sisa memori
        // dan membuat tab ditidurkan justru saat mesin masih longgar.
        let isi = "MemTotal:        1800000 kB\nMemFree:          100000 kB\n";
        assert_eq!(baca_meminfo(isi), None);
    }

    #[test]
    fn meminfo_rusak_tidak_panik() {
        assert_eq!(baca_meminfo(""), None);
        assert_eq!(baca_meminfo("MemAvailable: banyak kB\n"), None);
        assert_eq!(baca_meminfo("MemAvailable: 123456 kB\n"), Some(120));
    }

    #[test]
    fn ambang_bawaan_masuk_akal_untuk_mesin_kecil() {
        // 1.7 GB total: 300 MB sisa berarti swap sudah dekat. Kalau angkanya
        // terlalu besar, tab ditidurkan saat mesin masih longgar; terlalu kecil,
        // peniduran datang setelah swap terlanjur dipakai.
        let jangkauan = 200..=500;
        assert!(jangkauan.contains(&AMBANG_BAWAAN_MB));
    }
}
