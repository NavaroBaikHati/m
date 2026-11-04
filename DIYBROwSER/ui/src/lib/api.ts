import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Sama dengan konstanta di src-tauri/src/tabs.rs dan src-tauri/src/pintasan.rs */
export const EVENT_NAVIGASI = "tab:navigasi";
export const EVENT_PERINTAH = "chrome:perintah";
export const EVENT_UNDUHAN = "unduhan:berubah";
export const EVENT_PERLINDUNGAN = "perlindungan:siap";
export const EVENT_DAFTAR = "tab:daftar";
export const EVENT_PROGRES = "tab:progres";
export const EVENT_LAYAR_PENUH = "tab:layar-penuh";

export interface KeadaanTab {
  id: string;
  url: string;
  title: string;
}

/** Sama dengan `DaftarTab` di src-tauri/src/tabs.rs */
export interface DaftarTab {
  tabs: KeadaanTab[];
  aktif: string | null;
  sidebarTerbuka: boolean;
  zoom: number;
  tema: string;
}

/** Perubahan sebagian: hanya field yang berubah yang diisi Rust. */
export interface PeristiwaNavigasi {
  tab: string;
  url?: string;
  title?: string;
  memuat?: boolean;
}

/** Sama dengan `Perlindungan` di src-tauri/src/adblock.rs */
export interface Perlindungan {
  host: string;
  aktif: boolean;
  siap: boolean;
  jumlahAturan: number;
  jumlahSelector: number;
}

/** Kemajuan muat satu tab; `progres` 0..1. */
export interface PeristiwaProgres {
  tab: string;
  progres: number;
}

export interface StatusHalaman {
  url: string;
  bookmark: boolean;
  zoom: number;
  tema: string;
  beranda: boolean;
  perlindungan: Perlindungan;
}

export interface PilihanMesin {
  slug: string;
  nama: string;
  aktif: boolean;
}

export interface Bookmark {
  url: string;
  judul: string;
  dibuat: number;
}

export interface Riwayat {
  url: string;
  judul: string;
  dikunjungi: number;
}

export interface Unduhan {
  url: string;
  path: string;
  nama: string;
  status: string;
  waktu: number;
}

export const api = {
  daftarTab: () => invoke<DaftarTab>("daftar_tab"),
  tabBaru: (input?: string) => invoke<DaftarTab>("tab_baru", { input: input ?? null }),
  pilihTab: (id: string) => invoke<DaftarTab>("pilih_tab", { id }),
  tutupTab: (id: string) => invoke<DaftarTab>("tutup_tab", { id }),
  aturSidebar: (terbuka: boolean) => invoke<DaftarTab>("atur_sidebar", { terbuka }),
  navigasi: (input: string) => invoke<void>("navigasi", { input }),
  mundur: () => invoke<void>("mundur"),
  maju: () => invoke<void>("maju"),
  muatUlang: () => invoke<void>("muat_ulang"),
  hentikan: () => invoke<void>("hentikan"),
  status: (id?: string) => invoke<StatusHalaman>("status_halaman", { id: id ?? null }),
  aturPanel: (lebar: number) => invoke<void>("atur_panel", { lebar }),
  aturPerlindungan: (id: string, aktif: boolean) =>
    invoke<Perlindungan>("atur_perlindungan", { id, aktif }),
  aturZoom: (nilai: number) => invoke<number>("atur_zoom", { nilai }),
  aturTema: (tema: string) => invoke<string>("atur_tema", { tema }),
  daftarMesinPencarian: () => invoke<PilihanMesin[]>("daftar_mesin_pencarian"),
  aturMesinPencarian: (slug: string) => invoke<PilihanMesin[]>("atur_mesin_pencarian", { slug }),
  daftarBookmark: () => invoke<Bookmark[]>("daftar_bookmark"),
  toggleBookmark: (url: string, judul: string) =>
    invoke<boolean>("toggle_bookmark", { url, judul }),
  daftarRiwayat: (batas?: number) => invoke<Riwayat[]>("daftar_riwayat", { batas: batas ?? null }),
  cariRiwayat: (kata: string, batas?: number) =>
    invoke<Riwayat[]>("cari_riwayat", { kata, batas: batas ?? null }),
  hapusRiwayat: () => invoke<void>("hapus_riwayat"),
  daftarUnduhan: () => invoke<Unduhan[]>("daftar_unduhan"),
  onNavigasi: (handler: (peristiwa: PeristiwaNavigasi) => void): Promise<UnlistenFn> =>
    listen<PeristiwaNavigasi>(EVENT_NAVIGASI, (event) => handler(event.payload)),
  onPerintah: (handler: (perintah: string) => void): Promise<UnlistenFn> =>
    listen<string>(EVENT_PERINTAH, (event) => handler(event.payload)),
  onUnduhan: (handler: () => void): Promise<UnlistenFn> => listen(EVENT_UNDUHAN, () => handler()),
  onProgres: (handler: (peristiwa: PeristiwaProgres) => void): Promise<UnlistenFn> =>
    listen<PeristiwaProgres>(EVENT_PROGRES, (event) => handler(event.payload)),
  onLayarPenuh: (handler: (penuh: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>(EVENT_LAYAR_PENUH, (event) => handler(event.payload)),
  onPerlindungan: (handler: () => void): Promise<UnlistenFn> =>
    listen(EVENT_PERLINDUNGAN, () => handler()),
  onDaftarBerubah: (handler: () => void): Promise<UnlistenFn> =>
    listen(EVENT_DAFTAR, () => handler()),
};
