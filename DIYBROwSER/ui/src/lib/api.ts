import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Sama dengan `EVENT_NAVIGASI` di src-tauri/src/tabs.rs */
export const EVENT_NAVIGASI = "tab:navigasi";

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
}

/** Perubahan sebagian: hanya field yang berubah yang diisi Rust. */
export interface PeristiwaNavigasi {
  tab: string;
  url?: string;
  title?: string;
  memuat?: boolean;
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
  onNavigasi: (handler: (peristiwa: PeristiwaNavigasi) => void): Promise<UnlistenFn> =>
    listen<PeristiwaNavigasi>(EVENT_NAVIGASI, (event) => handler(event.payload)),
};
