<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    type Bookmark,
    type DaftarTab,
    type PeristiwaNavigasi,
    type PilihanMesin,
    type Riwayat,
    type StatusHalaman,
    type Unduhan,
  } from "./lib/api";

  type Panel = "tidak" | "perlindungan" | "pustaka" | "unduhan" | "menu";

  let daftar = $state<DaftarTab>({
    tabs: [],
    aktif: null,
    sidebarTerbuka: true,
    zoom: 1,
    tema: "gelap",
  });
  let status = $state<StatusHalaman | null>(null);
  let isiAlamat = $state("");
  let sedangMengetik = $state(false);
  let memuat = $state<Record<string, boolean>>({});
  let galat = $state("");
  let panel = $state<Panel>("tidak");
  let pustakaTab = $state<"bookmark" | "riwayat">("bookmark");
  let bookmarkList = $state<Bookmark[]>([]);
  let riwayatList = $state<Riwayat[]>([]);
  let unduhanList = $state<Unduhan[]>([]);
  let cariPustaka = $state("");
  let mesinPencarian = $state<PilihanMesin[]>([]);
  /** Kemajuan muat per tab (0..1); tidak ada entri berarti belum ada laporan. */
  let progres = $state<Record<string, number>>({});
  /** Halaman sedang layar penuh: chrome disembunyikan oleh Rust. */
  let layarPenuh = $state(false);

  let inputAlamat: HTMLInputElement;
  let permintaanStatus = 0;
  let jadwalStatus: ReturnType<typeof setTimeout> | undefined;

  let tabAktif = $derived(daftar.tabs.find((tab) => tab.id === daftar.aktif));
  let urlAktif = $derived(status?.url || tabAktif?.url || "");
  let beranda = $derived(status?.beranda ?? urlAktif.startsWith("file:"));
  let isiMemuat = $derived(daftar.aktif ? memuat[daftar.aktif] === true : false);
  let jumlahTab = $derived(daftar.tabs.length);
  let nilaiProgres = $derived(daftar.aktif ? progres[daftar.aktif] : undefined);

  const namaTema = $derived.by(() => {
    const tema = status?.tema ?? daftar.tema;
    if (tema === "terang") return "Terang";
    if (tema === "gelap") return "Gelap";
    return "Mengikuti sistem";
  });

  let mesinAktif = $derived(mesinPencarian.find((mesin) => mesin.aktif));

  const riwayatTersaring = $derived(
    cariPustaka.trim()
      ? riwayatList.filter((item) =>
          `${item.judul} ${item.url}`.toLowerCase().includes(cariPustaka.toLowerCase()),
        )
      : riwayatList,
  );
  const bookmarkTersaring = $derived(
    cariPustaka.trim()
      ? bookmarkList.filter((item) =>
          `${item.judul} ${item.url}`.toLowerCase().includes(cariPustaka.toLowerCase()),
        )
      : bookmarkList,
  );

  $effect(() => {
    if (!sedangMengetik) isiAlamat = beranda ? "" : urlAktif;
  });

  // Panel memesan ruang di kanan secukupnya; webview tab adalah child native
  // yang selalu berada di atas chrome, jadi kartu melayang tidak bisa menutupi
  // halaman. Kartunya dibuat ramping supaya halaman hanya menyempit sedikit.
  $effect(() => {
    const lebarPanel = panel === "tidak" ? 0 : panel === "pustaka" ? 372 : 344;
    void api.aturPanel(lebarPanel).catch((error) => (galat = pesanGalat(error)));
  });

  // Pindah halaman berarti urusan panel sudah selesai.
  $effect(() => {
    void urlAktif;
    panel = "tidak";
  });

  $effect(() => {
    if (!galat) return;
    const timer = setTimeout(() => (galat = ""), 6000);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const tema = status?.tema ?? daftar.tema;
    if (tema === "sistem") delete document.documentElement.dataset.tema;
    else document.documentElement.dataset.tema = tema;
  });

  onMount(() => {
    let dibatalkan = false;
    let bersih: (() => void)[] = [];

    void (async () => {
      try {
        daftar = await api.daftarTab();
        await segarkanStatus();
        mesinPencarian = await api.daftarMesinPencarian();
        const lepas = await Promise.all([
          api.onNavigasi(tanganiNavigasi),
          api.onPerintah(tanganiPerintah),
          api.onUnduhan(() => void segarkanUnduhan()),
          api.onPerlindungan(() => void segarkanStatus()),
          api.onProgres((peristiwa) => {
            progres = { ...progres, [peristiwa.tab]: peristiwa.progres };
          }),
          api.onLayarPenuh((penuh) => {
            layarPenuh = penuh;
            if (penuh) panel = "tidak";
          }),
          api.onDaftarBerubah(() => {
            void segarkanDaftar();
            void segarkanStatus();
          }),
        ]);
        if (dibatalkan) lepas.forEach((f) => f());
        else bersih = lepas;
      } catch (error) {
        galat = pesanGalat(error);
      }
    })();

    const tombol = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (panel !== "tidak") {
        panel = "tidak";
        event.preventDefault();
      } else if (sedangMengetik) {
        sedangMengetik = false;
        inputAlamat?.blur();
      }
    };
    window.addEventListener("keydown", tombol);

    return () => {
      dibatalkan = true;
      bersih.forEach((f) => f());
      window.removeEventListener("keydown", tombol);
    };
  });

  function tanganiNavigasi(peristiwa: PeristiwaNavigasi) {
    const tab = daftar.tabs.find((t) => t.id === peristiwa.tab);
    if (tab) {
      if (peristiwa.url !== undefined) tab.url = peristiwa.url;
      if (peristiwa.title !== undefined) tab.title = peristiwa.title;
    }
    if (peristiwa.memuat !== undefined) {
      memuat = { ...memuat, [peristiwa.tab]: peristiwa.memuat };
      if (peristiwa.memuat) {
        progres = { ...progres, [peristiwa.tab]: 0 };
      } else {
        const sisa = { ...progres };
        delete sisa[peristiwa.tab];
        progres = sisa;
      }
    }
    if (peristiwa.tab === daftar.aktif) {
      segarkanStatusTertunda();
      if (peristiwa.memuat === false) {
        // Judul halaman biasanya datang setelah muat selesai.
        setTimeout(() => void segarkanDaftar(), 250);
      }
    }
  }

  function tanganiPerintah(perintah: string) {
    if (perintah === "fokus-alamat") fokusAlamat();
    else if (perintah === "bookmark") void toggleBookmark();
    else if (perintah === "unduhan") void bukaPanel("unduhan");
    else if (perintah === "riwayat") void bukaPustaka("riwayat");
    else if (perintah === "menu") void bukaPanel("menu");
  }

  /**
   * Satu halaman memicu beberapa event (mulai navigasi, mulai muat, judul,
   * selesai). Status hanya diminta sekali per gelombang, supaya query basis
   * data dan pencarian selector tidak diulang-ulang tanpa guna.
   */
  function segarkanStatusTertunda() {
    clearTimeout(jadwalStatus);
    jadwalStatus = setTimeout(() => void segarkanStatus(), 120);
  }

  async function segarkanStatus() {
    const nomor = ++permintaanStatus;
    try {
      const baru = await api.status(daftar.aktif ?? undefined);
      if (nomor === permintaanStatus) status = baru;
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function segarkanDaftar() {
    try {
      daftar = await api.daftarTab();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function perbarui(aksi: Promise<DaftarTab>) {
    galat = "";
    try {
      daftar = await aksi;
      await segarkanStatus();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function jalankan(aksi: Promise<unknown>) {
    galat = "";
    try {
      await aksi;
      await segarkanStatus();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  function kirim(event: SubmitEvent) {
    event.preventDefault();
    const nilai = isiAlamat.trim();
    if (!nilai) return;
    void jalankan(api.navigasi(nilai));
    sedangMengetik = false;
    inputAlamat?.blur();
  }

  function fokusAlamat() {
    panel = "tidak";
    sedangMengetik = true;
    inputAlamat?.focus();
    inputAlamat?.select();
  }

  function hentikan() {
    void jalankan(api.hentikan());
    if (daftar.aktif) memuat = { ...memuat, [daftar.aktif]: false };
  }

  function pilihTab(id: string) {
    panel = "tidak";
    void perbarui(api.pilihTab(id));
  }

  function tutupTab(event: MouseEvent, id: string) {
    event.stopPropagation();
    void perbarui(api.tutupTab(id));
    void segarkanStatus();
  }

  async function toggleBookmark() {
    if (!urlAktif || beranda) return;
    try {
      const sekarang = await api.toggleBookmark(urlAktif, tabAktif?.title || urlAktif);
      if (status) status = { ...status, bookmark: sekarang };
      if (panel === "pustaka") await muatPustaka();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function toggleShield() {
    if (!daftar.aktif || !status) return;
    try {
      const baru = await api.aturPerlindungan(daftar.aktif, !status.perlindungan.aktif);
      status = { ...status, perlindungan: baru };
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function ubahZoom(nilai: number) {
    try {
      const zoom = await api.aturZoom(nilai);
      if (status) status = { ...status, zoom };
      daftar = { ...daftar, zoom };
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function gantiTema() {
    const urutan = ["sistem", "terang", "gelap"];
    const sekarang = status?.tema ?? daftar.tema;
    const berikut = urutan[(urutan.indexOf(sekarang) + 1) % urutan.length];
    await pasangTema(berikut);
  }

  async function pasangTema(tema: string) {
    try {
      const baru = await api.aturTema(tema);
      if (status) status = { ...status, tema: baru };
      daftar = { ...daftar, tema: baru };
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function pasangMesin(slug: string) {
    try {
      mesinPencarian = await api.aturMesinPencarian(slug);
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function muatPustaka() {
    try {
      const [bookmark, riwayat] = await Promise.all([api.daftarBookmark(), api.daftarRiwayat()]);
      bookmarkList = bookmark;
      riwayatList = riwayat;
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function muatUnduhan() {
    try {
      unduhanList = await api.daftarUnduhan();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function segarkanUnduhan() {
    if (panel === "unduhan") await muatUnduhan();
  }

  async function bukaPanel(baru: Panel) {
    panel = panel === baru ? "tidak" : baru;
    if (panel === "pustaka") await muatPustaka();
    if (panel === "unduhan") await muatUnduhan();
  }

  async function bukaPustaka(tab: "bookmark" | "riwayat") {
    pustakaTab = tab;
    cariPustaka = "";
    panel = "pustaka";
    await muatPustaka();
  }

  async function hapusRiwayat() {
    try {
      await api.hapusRiwayat();
      await muatPustaka();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  function bukaUrl(url: string) {
    panel = "tidak";
    void jalankan(api.navigasi(url));
  }

  function pesanGalat(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function judulTab(url: string): string {
    if (url.startsWith("file:")) return "Tab baru";
    try {
      return new URL(url).host.replace(/^www\./, "") || url;
    } catch {
      return url;
    }
  }

  /** Host tanpa `www.` untuk tampilan address bar yang lebih rapi. */
  function pecahAlamat(url: string): { host: string; jalur: string } {
    try {
      const u = new URL(url);
      const host = u.host.replace(/^www\./, "");
      const jalur = `${u.pathname === "/" ? "" : u.pathname}${u.search}${u.hash}`;
      return { host, jalur };
    } catch {
      return { host: url, jalur: "" };
    }
  }

  function inisial(url: string, judul: string): string {
    if (url.startsWith("file:")) return "P";
    const host = judulTab(url).replace(/^www\./, "");
    // Alamat IP atau port lokal tidak punya huruf yang bermakna; pakai judul.
    if (/^\d/.test(host)) return (judul.trim()[0] || "#").toUpperCase();
    const bagian = host.split(".");
    const inti = bagian.length > 1 ? bagian[bagian.length - 2] : bagian[0];
    return (inti || "?").slice(0, 1).toUpperCase();
  }

  function waktuRelatif(detik: number): string {
    const selisih = Math.max(0, Math.floor(Date.now() / 1000) - detik);
    if (selisih < 60) return "baru saja";
    if (selisih < 3600) return `${Math.floor(selisih / 60)} menit lalu`;
    if (selisih < 86400) return `${Math.floor(selisih / 3600)} jam lalu`;
    return `${Math.floor(selisih / 86400)} hari lalu`;
  }

  const alamat = $derived(pecahAlamat(urlAktif));
  const lencanaShield = $derived(status?.perlindungan);
</script>

<div class="aplikasi" class:layar-penuh={layarPenuh}>
  {#if daftar.sidebarTerbuka}
    <aside class="sidebar">
      <div class="sidebar-kepala">
        <span class="merek" aria-label="Perisai">
          <svg viewBox="0 0 24 24" aria-hidden="true" class="lambang">
            <path
              d="M12 2.6 4.8 5.6v5.3c0 4.7 3 8.4 7.2 10 4.2-1.6 7.2-5.3 7.2-10V5.6L12 2.6Z"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linejoin="round"
            />
            <path
              d="m9 12 2.1 2.1 4-4.2"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <b>Perisai</b>
        </span>
        <button
          type="button"
          class="ikon"
          title="Tab baru (Ctrl+T)"
          aria-label="Tab baru"
          onclick={() => perbarui(api.tabBaru())}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3.5v9M3.5 8h9" /></svg>
        </button>
        <button
          type="button"
          class="ikon"
          title="Ciutkan sidebar (Ctrl+B)"
          aria-label="Ciutkan sidebar"
          onclick={() => perbarui(api.aturSidebar(false))}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect x="2.4" y="3.2" width="11.2" height="9.6" rx="2.2" />
            <path d="M6.4 3.2v9.6" />
          </svg>
        </button>
      </div>

      <nav class="daftar-tab" aria-label="Tab">
        {#each daftar.tabs as tab, urutan (tab.id)}
          <div
            class="tab"
            class:aktif={tab.id === daftar.aktif}
            role="button"
            tabindex="0"
            aria-current={tab.id === daftar.aktif ? "page" : undefined}
            onclick={() => pilihTab(tab.id)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                pilihTab(tab.id);
              }
            }}
          >
            <span class="avatar" aria-hidden="true">
              {#if memuat[tab.id]}
                <span class="denyut"></span>
              {:else}
                {inisial(tab.url, tab.title)}
              {/if}
            </span>
            <span class="tab-teks">
              <span class="tab-judul">{tab.title || judulTab(tab.url)}</span>
              <span class="tab-host">{judulTab(tab.url)}</span>
            </span>
            {#if urutan < 9}
              <span class="nomor" aria-hidden="true">{urutan + 1}</span>
            {/if}
            <button
              type="button"
              class="tutup"
              title="Tutup tab (Ctrl+W)"
              aria-label="Tutup tab"
              tabindex="-1"
              onclick={(event) => tutupTab(event, tab.id)}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
            </button>
          </div>
        {/each}
      </nav>

      <div class="sidebar-kaki">
        <button
          type="button"
          class="ikon"
          class:aktif={panel === "pustaka"}
          title="Bookmark"
          aria-label="Bookmark"
          onclick={() => bukaPustaka("bookmark")}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M4.5 2.5h7v11l-3.5-2.4-3.5 2.4Z" />
          </svg>
        </button>
        <button
          type="button"
          class="ikon"
          class:aktif={panel === "pustaka" && pustakaTab === "riwayat"}
          title="Riwayat (Ctrl+H)"
          aria-label="Riwayat"
          onclick={() => bukaPustaka("riwayat")}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="5.6" />
            <path d="M8 4.8V8l2.2 1.4" />
          </svg>
        </button>
        <button
          type="button"
          class="ikon"
          class:aktif={panel === "unduhan"}
          title="Unduhan (Ctrl+J)"
          aria-label="Unduhan"
          onclick={() => bukaPanel("unduhan")}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 2.8v7M5 7l3 3 3-3" />
            <path d="M3 12.5h10" />
          </svg>
        </button>
        <button
          type="button"
          class="ikon"
          title={`Tema: ${namaTema}`}
          aria-label="Ganti tema"
          onclick={gantiTema}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="5.4" />
            <path d="M8 2.6v10.8" />
          </svg>
        </button>
      </div>
    </aside>
  {/if}

  <div class="utama">
    <header class="chrome">
      {#if !daftar.sidebarTerbuka}
        <button
          type="button"
          class="ikon"
          title="Buka sidebar (Ctrl+B)"
          aria-label="Buka sidebar"
          onclick={() => perbarui(api.aturSidebar(true))}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect x="2.4" y="3.2" width="11.2" height="9.6" rx="2.2" />
            <path d="M6.4 3.2v9.6" />
          </svg>
        </button>
      {/if}

      <div class="tombol">
        <button type="button" title="Mundur (Alt+←)" aria-label="Mundur" onclick={() => void jalankan(api.mundur())}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 2.5 4.5 8l5.5 5.5" /></svg>
        </button>
        <button type="button" title="Maju (Alt+→)" aria-label="Maju" onclick={() => void jalankan(api.maju())}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 2.5 11.5 8 6 13.5" /></svg>
        </button>
        {#if isiMemuat}
          <button type="button" title="Hentikan pemuatan" aria-label="Hentikan" onclick={hentikan}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.6 4.6l6.8 6.8M11.4 4.6l-6.8 6.8" /></svg>
          </button>
        {:else}
          <button
            type="button"
            title="Muat ulang (Ctrl+R)"
            aria-label="Muat ulang"
            onclick={() => void jalankan(api.muatUlang())}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M13 8a5 5 0 1 1-1.6-3.7" />
              <path d="M13 2.4v3.2h-3.2" />
            </svg>
          </button>
        {/if}
      </div>

      <form class="alamat" class:fokus={sedangMengetik} onsubmit={kirim}>
        <button
          type="button"
          class="pelindung"
          class:mati={lencanaShield ? !lencanaShield.aktif : false}
          title="Perlindungan Perisai"
          aria-label="Perlindungan Perisai"
          tabindex="-1"
          onclick={() => bukaPanel("perlindungan")}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 1.7 3.6 3.5v3.4c0 3 2 5.5 4.4 6.5 2.4-1 4.4-3.5 4.4-6.5V3.5L8 1.7Z" />
          </svg>
          {#if lencanaShield?.siap && lencanaShield.aktif}
            <span class="titik" aria-hidden="true"></span>
          {/if}
        </button>

        {#if !sedangMengetik}
          <div class="tampilan" role="presentation" onclick={fokusAlamat}>
            {#if beranda}
              <span class="placeholder">Cari atau ketik alamat</span>
            {:else}
              <span class="host">{alamat.host}</span>
              {#if alamat.jalur}<span class="jalur">{alamat.jalur}</span>{/if}
            {/if}
          </div>
        {/if}
        <input
          bind:this={inputAlamat}
          bind:value={isiAlamat}
          class:hilang={!sedangMengetik}
          onfocus={() => (sedangMengetik = true)}
          onblur={() => (sedangMengetik = false)}
          aria-label="Address bar"
          placeholder="Cari atau ketik alamat"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
        />
        <button
          type="button"
          class="bintang"
          class:tersimpan={status?.bookmark}
          title="Simpan bookmark (Ctrl+D)"
          aria-label="Simpan bookmark"
          tabindex="-1"
          disabled={beranda}
          onclick={toggleBookmark}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M4.6 2.5h6.8v11L8 11.1l-3.4 2.4Z" />
          </svg>
        </button>
      </form>

      <div class="aksi-kanan">
        {#if (status?.zoom ?? 1) !== 1}
          <button
            type="button"
            class="lencana-zoom"
            title="Kembalikan zoom ke 100% (Ctrl+0)"
            onclick={() => ubahZoom(1)}
          >
            {Math.round((status?.zoom ?? 1) * 100)}%
          </button>
        {/if}
        <button
          type="button"
          class="ikon"
          class:aktif={panel === "menu"}
          title="Menu (Ctrl+K)"
          aria-label="Menu"
          onclick={() => bukaPanel("menu")}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="3.4" cy="8" r="1.1" fill="currentColor" stroke="none" />
            <circle cx="8" cy="8" r="1.1" fill="currentColor" stroke="none" />
            <circle cx="12.6" cy="8" r="1.1" fill="currentColor" stroke="none" />
          </svg>
        </button>
      </div>

      {#if isiMemuat}
        <div class="progres" class:tak-tentu={nilaiProgres === undefined} aria-hidden="true">
          <span
            style:width={nilaiProgres === undefined
              ? undefined
              : `${Math.max(3, Math.min(100, nilaiProgres * 100))}%`}
          ></span>
        </div>
      {/if}
    </header>

    {#if jumlahTab === 0}
      <div class="kosong"><p>Belum ada tab.</p></div>
    {/if}
  </div>
</div>

{#if panel !== "tidak"}
  <button
    type="button"
    class="penutup"
    aria-label="Tutup panel"
    onclick={() => (panel = "tidak")}
  ></button>
{/if}

{#if panel === "perlindungan" && lencanaShield}
  <section class="panel" aria-label="Perlindungan">
    <header>
      <span class="judul-panel">
        <svg viewBox="0 0 16 16" aria-hidden="true" class="ikon-panel">
          <path d="M8 1.7 3.6 3.5v3.4c0 3 2 5.5 4.4 6.5 2.4-1 4.4-3.5 4.4-6.5V3.5L8 1.7Z" />
        </svg>
        Perlindungan
      </span>
      <span class="status-chip" class:mati={!lencanaShield.aktif}>
        {lencanaShield.aktif ? "Aktif" : "Mati"}
      </span>
    </header>

    <p class="situs">{lencanaShield.host || "Halaman ini"}</p>

    <div class="angka">
      <b>{lencanaShield.jumlahAturan.toLocaleString("id-ID")}</b>
      <span>aturan penyaring siap memblokir iklan dan pelacak</span>
    </div>

    <div class="baris">
      <div>
        <strong>Blokir iklan & pelacak</strong>
        <small>
          {#if lencanaShield.siap}
            {lencanaShield.jumlahSelector} selector khusus untuk situs ini
          {:else}
            Daftar filter masih disiapkan
          {/if}
        </small>
      </div>
      <button
        type="button"
        class="sakelar"
        class:nyala={lencanaShield.aktif}
        role="switch"
        aria-checked={lencanaShield.aktif}
        aria-label="Blokir iklan di situs ini"
        onclick={toggleShield}
      >
        <span></span>
      </button>
    </div>

    <p class="catatan">
      WebKit memblokir request sebelum keluar dari perangkat. Browser tidak
      menerima laporan per-request, jadi yang ditampilkan adalah jumlah aturan
      yang menyaring halaman ini, bukan angka iklan yang dikarang.
    </p>
  </section>
{:else if panel === "pustaka"}
  <section class="panel lebar" aria-label="Pustaka">
    <header>
      <div class="segmen">
        <button type="button" class:aktif={pustakaTab === "bookmark"} onclick={() => bukaPustaka("bookmark")}>
          Bookmark
        </button>
        <button type="button" class:aktif={pustakaTab === "riwayat"} onclick={() => bukaPustaka("riwayat")}>
          Riwayat
        </button>
      </div>
      <button type="button" class="ikon" aria-label="Tutup" onclick={() => (panel = "tidak")}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>

    <input class="cari" bind:value={cariPustaka} placeholder="Cari…" autocomplete="off" />

    <div class="daftar-panel">
      {#if pustakaTab === "bookmark"}
        {#each bookmarkTersaring as item (item.url)}
          <button type="button" class="baris-panel" onclick={() => bukaUrl(item.url)}>
            <span class="avatar kecil">{inisial(item.url, item.judul)}</span>
            <span class="teks-panel">
              <strong>{item.judul || judulTab(item.url)}</strong>
              <small>{item.url}</small>
            </span>
          </button>
        {:else}
          <p class="kosong-panel">Belum ada bookmark. Tekan Ctrl+D di halaman mana pun.</p>
        {/each}
      {:else}
        {#each riwayatTersaring as item (item.url)}
          <button type="button" class="baris-panel" onclick={() => bukaUrl(item.url)}>
            <span class="avatar kecil">{inisial(item.url, item.judul)}</span>
            <span class="teks-panel">
              <strong>{item.judul || judulTab(item.url)}</strong>
              <small>{judulTab(item.url)} · {waktuRelatif(item.dikunjungi)}</small>
            </span>
          </button>
        {:else}
          <p class="kosong-panel">Belum ada riwayat.</p>
        {/each}
      {/if}
    </div>

    {#if pustakaTab === "riwayat" && riwayatList.length > 0}
      <footer class="panel-kaki">
        <button type="button" class="tautan" onclick={hapusRiwayat}>Hapus semua riwayat</button>
      </footer>
    {/if}
  </section>
{:else if panel === "unduhan"}
  <section class="panel" aria-label="Unduhan">
    <header>
      <span class="judul-panel">Unduhan</span>
      <button type="button" class="ikon" aria-label="Tutup" onclick={() => (panel = "tidak")}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>
    <div class="daftar-panel">
      {#each unduhanList as item (item.url + item.waktu)}
        <div class="baris-panel statis">
          <span class="avatar kecil netral" aria-hidden="true">
            <svg viewBox="0 0 16 16"><path d="M8 2.8v7M5 7l3 3 3-3M3 12.5h10" /></svg>
          </span>
          <span class="teks-panel">
            <strong>{item.nama}</strong>
            <small title={item.path}>{item.status} · {waktuRelatif(item.waktu)}</small>
          </span>
        </div>
      {:else}
        <p class="kosong-panel">Belum ada unduhan.</p>
      {/each}
    </div>
  </section>
{:else if panel === "menu"}
  <section class="panel" aria-label="Menu">
    <header>
      <span class="judul-panel">Menu</span>
      <button type="button" class="ikon" aria-label="Tutup" onclick={() => (panel = "tidak")}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>

    <div class="baris">
      <div><strong>Tema</strong><small>{namaTema}</small></div>
      <div class="segmen kecil">
        {#each [["sistem", "Sistem"], ["terang", "Terang"], ["gelap", "Gelap"]] as [nilai, label]}
          <button
            type="button"
            class:aktif={(status?.tema ?? daftar.tema) === nilai}
            onclick={() => pasangTema(nilai)}
          >{label}</button>
        {/each}
      </div>
    </div>

    <div class="baris">
      <div><strong>Mesin pencari</strong><small>{mesinAktif?.nama ?? "—"}</small></div>
      <div class="segmen kecil">
        {#each mesinPencarian as mesin (mesin.slug)}
          <button
            type="button"
            class:aktif={mesin.aktif}
            onclick={() => pasangMesin(mesin.slug)}
          >{mesin.nama}</button>
        {/each}
      </div>
    </div>

    <div class="baris">
      <div><strong>Zoom</strong><small>{Math.round((status?.zoom ?? 1) * 100)}%</small></div>
      <div class="zoom">
        <button type="button" aria-label="Perkecil" onclick={() => ubahZoom((status?.zoom ?? 1) - 0.1)}>−</button>
        <button type="button" aria-label="Perbesar" onclick={() => ubahZoom((status?.zoom ?? 1) + 0.1)}>+</button>
      </div>
    </div>

    <div class="pintasan">
      <p>Pintasan</p>
      <ul>
        <li><kbd>Ctrl</kbd><kbd>T</kbd><span>Tab baru</span></li>
        <li><kbd>Ctrl</kbd><kbd>Shift</kbd><kbd>T</kbd><span>Buka tab tertutup</span></li>
        <li><kbd>Ctrl</kbd><kbd>Tab</kbd><span>Tab berikutnya</span></li>
        <li><kbd>Ctrl</kbd><kbd>L</kbd><span>Fokus address bar</span></li>
        <li><kbd>Ctrl</kbd><kbd>D</kbd><span>Bookmark</span></li>
        <li><kbd>Ctrl</kbd><kbd>J</kbd><span>Unduhan</span></li>
        <li><kbd>Ctrl</kbd><kbd>=</kbd><span>Perbesar</span></li>
      </ul>
    </div>
  </section>
{/if}

{#if galat}
  <p class="galat" role="alert">{galat}</p>
{/if}

<style>
  .aplikasi {
    display: flex;
    height: 100vh;
    transition: opacity 120ms ease-out;
  }

  /* Rust menyembunyikan webview chrome saat video layar penuh; kelas ini
     mencegah chrome sempat berkedip sebelum perintah itu tiba. */
  .aplikasi.layar-penuh {
    opacity: 0;
  }

  /* ---------- sidebar + tab manager ---------- */

  .sidebar {
    display: flex;
    flex: none;
    flex-direction: column;
    width: var(--lebar-sidebar);
    background: var(--sidebar);
    border-right: 1px solid var(--garis);
  }

  .sidebar-kepala {
    display: flex;
    align-items: center;
    gap: 1px;
    height: var(--tinggi-chrome);
    padding: 0 6px 0 10px;
  }

  .merek {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .merek b {
    overflow: hidden;
    color: var(--teks);
    font-size: 11.5px;
    font-weight: 650;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .lambang {
    flex: none;
    width: 17px;
    height: 17px;
    color: var(--teks);
  }

  .daftar-tab {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
    padding: 3px 7px 10px;
  }

  .tab {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 4px 0 7px;
    border-radius: 9px;
    color: var(--teks);
    cursor: default;
    transition: background 130ms ease-out, box-shadow 130ms ease-out;
  }

  .tab:hover {
    background: var(--kartu-lembut);
  }

  .tab:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--teks) 34%, transparent);
    outline-offset: -1px;
  }

  .tab.aktif {
    background: var(--kartu);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .tab.aktif::before {
    position: absolute;
    top: 9px;
    bottom: 9px;
    left: 0;
    width: 2px;
    border-radius: 0 2px 2px 0;
    background: var(--teks);
    content: "";
  }

  .avatar {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 1px solid var(--garis);
    border-radius: 6px;
    background: var(--kartu-lembut);
    color: var(--samar);
    font-size: 10px;
    font-weight: 650;
  }

  .avatar.kecil {
    width: 19px;
    height: 19px;
    font-size: 9.5px;
  }

  .avatar.netral {
    background: var(--kartu-lembut);
    color: var(--samar);
  }

  .avatar.netral svg,
  .avatar svg {
    width: 13px;
    height: 13px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .denyut {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #fff;
    animation: denyut 1s ease-in-out infinite;
  }

  @keyframes denyut {
    50% {
      opacity: 0.3;
    }
  }

  .tab-teks {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .tab-judul {
    overflow: hidden;
    font-size: 12px;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-host {
    overflow: hidden;
    color: var(--samar);
    font-size: 10px;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.85;
  }

  .nomor {
    flex: none;
    color: var(--samar);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  .tab:hover .nomor,
  .tab.aktif .nomor {
    display: none;
  }

  .tutup {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--samar);
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease-out, background 120ms ease-out;
  }

  .tab:hover .tutup,
  .tab.aktif .tutup,
  .tutup:focus-visible {
    opacity: 1;
  }

  .tutup:hover {
    background: color-mix(in srgb, var(--teks) 12%, transparent);
    color: var(--teks);
  }

  .sidebar-kaki {
    display: flex;
    flex: none;
    gap: 2px;
    padding: 8px 10px 10px;
    border-top: 1px solid var(--garis);
  }

  /* ---------- chrome ---------- */

  .utama {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    background: var(--latar);
  }

  .chrome {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    height: var(--tinggi-chrome);
    padding: 0 7px;
    background: var(--chrome);
    border-bottom: 1px solid var(--garis);
  }

  .tombol {
    display: flex;
    gap: 1px;
  }

  button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--teks);
    cursor: pointer;
    transition: background 120ms ease-out, color 120ms ease-out;
  }

  button:hover {
    background: color-mix(in srgb, var(--teks) 9%, transparent);
  }

  button.ikon.aktif {
    background: color-mix(in srgb, var(--teks) 12%, transparent);
    color: var(--teks);
  }

  button svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .aksi-kanan {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .lencana-zoom {
    display: grid;
    width: auto;
    min-width: 38px;
    height: 21px;
    padding: 0 8px;
    border-radius: 999px;
    background: var(--kartu-lembut);
    color: var(--samar);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }

  /* ---------- garis progres ---------- */

  .progres {
    position: absolute;
    right: 0;
    bottom: -1px;
    left: 0;
    height: 2px;
    overflow: hidden;
    pointer-events: none;
  }

  .progres span {
    display: block;
    height: 100%;
    background: var(--teks);
    transition: width 160ms ease-out;
  }

  /* Tanpa laporan progres dari platform, garis bergerak sendiri. */
  .progres.tak-tentu span {
    width: 26%;
    animation: geser-progres 1.15s ease-in-out infinite;
  }

  @keyframes geser-progres {
    0% {
      transform: translateX(-110%);
    }
    100% {
      transform: translateX(410%);
    }
  }

  /* ---------- address bar ---------- */

  .alamat {
    position: relative;
    display: flex;
    flex: 1;
    align-items: center;
    gap: 4px;
    height: 26px;
    max-width: 600px;
    margin: 0 auto;
    padding: 0 4px 0 3px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--kartu-lembut);
    transition: border-color 150ms ease-out, box-shadow 150ms ease-out,
      background 150ms ease-out, max-width 180ms ease-out;
  }

  .alamat:hover {
    border-color: color-mix(in srgb, var(--teks) 13%, transparent);
  }

  .alamat.fokus {
    max-width: 720px;
    border-color: color-mix(in srgb, var(--teks) 26%, transparent);
    background: var(--kartu);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--teks) 7%, transparent);
  }

  .pelindung {
    position: relative;
    flex: none;
    width: 21px;
    height: 21px;
    border-radius: 50%;
    color: var(--teks);
    background: transparent;
  }

  .pelindung:hover {
    background: color-mix(in srgb, var(--teks) 11%, transparent);
  }

  .pelindung.mati {
    color: var(--samar);
  }

  .pelindung svg {
    width: 12px;
    height: 12px;
    stroke-width: 1.7;
  }

  .pelindung .titik {
    position: absolute;
    right: 1px;
    bottom: 2px;
    width: 5px;
    height: 5px;
    border: 1px solid var(--chrome);
    border-radius: 50%;
    background: var(--teks);
  }

  .alamat.fokus .pelindung .titik {
    border-color: var(--kartu);
  }

  .tampilan {
    display: flex;
    flex: 1;
    align-items: baseline;
    gap: 2px;
    min-width: 0;
    padding: 0 4px;
    cursor: text;
  }

  .tampilan .host {
    overflow: hidden;
    color: var(--teks);
    font-size: 12.5px;
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tampilan .jalur {
    overflow: hidden;
    color: var(--samar);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .placeholder {
    color: var(--samar);
    font-size: 12px;
  }

  .alamat input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--teks);
    font: inherit;
    font-size: 12.5px;
    outline: none;
    user-select: text;
  }

  .alamat input.hilang {
    position: absolute;
    width: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .bintang {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: var(--samar);
  }

  .bintang:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .bintang.tersimpan {
    color: var(--teks);
    background: color-mix(in srgb, var(--teks) 11%, transparent);
  }

  .bintang.tersimpan svg {
    fill: currentColor;
    stroke-width: 1.4;
  }

  .bintang svg {
    width: 13px;
    height: 13px;
  }

  .kosong {
    display: grid;
    flex: 1;
    place-items: center;
    color: var(--samar);
    font-size: 13px;
  }

  /* ---------- panel ---------- */

  .penutup {
    position: fixed;
    inset: 0;
    width: auto;
    height: auto;
    border-radius: 0;
    background: transparent;
    z-index: 20;
  }

  /* Kartu melayang: ramping, sudut membulat, dan tidak menempel ke tepi. */
  .panel {
    position: fixed;
    z-index: 21;
    top: calc(var(--tinggi-chrome) + 6px);
    right: 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 320px;
    max-height: calc(100vh - var(--tinggi-chrome) - 22px);
    padding: 12px;
    overflow-y: auto;
    border: 1px solid var(--garis-kuat);
    border-radius: 14px;
    background: var(--panel);
    box-shadow: var(--bayang-panel);
    backdrop-filter: blur(18px);
  }

  .panel.lebar {
    width: 348px;
  }

  .panel header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .judul-panel {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--teks);
    font-size: 12.5px;
    font-weight: 650;
  }

  .ikon-panel {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: var(--teks);
    stroke-width: 1.7;
    stroke-linejoin: round;
  }

  .status-chip {
    padding: 2px 8px;
    border: 1px solid var(--garis-kuat);
    border-radius: 999px;
    color: var(--teks);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .status-chip.mati {
    border-color: var(--garis);
    color: var(--samar);
  }

  .situs {
    margin: 0;
    overflow: hidden;
    color: var(--samar);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .angka {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px;
    border: 1px solid var(--garis);
    border-radius: 11px;
    background: var(--kartu-lembut);
  }

  .angka b {
    color: var(--teks);
    font-size: 24px;
    font-variant-numeric: tabular-nums;
    line-height: 1.05;
  }

  .angka span {
    color: var(--samar);
    font-size: 11.5px;
  }

  .baris {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .baris div strong {
    display: block;
    color: var(--teks);
    font-size: 12.5px;
    font-weight: 600;
  }

  .baris div small {
    color: var(--samar);
    font-size: 11px;
  }

  .sakelar {
    position: relative;
    flex: none;
    width: 40px;
    height: 23px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--teks) 18%, transparent);
    transition: background 140ms ease-out;
  }

  .sakelar span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    transition: transform 140ms ease-out;
  }

  .sakelar.nyala {
    background: var(--teks);
  }

  .sakelar.nyala span {
    background: var(--chrome);
  }

  .sakelar.nyala span {
    transform: translateX(17px);
  }

  .catatan {
    margin: 0;
    color: var(--samar);
    font-size: 11px;
    line-height: 1.45;
  }

  .segmen {
    display: flex;
    gap: 3px;
    padding: 3px;
    border-radius: 999px;
    background: var(--kartu-lembut);
  }

  .segmen button {
    width: auto;
    height: 24px;
    padding: 0 12px;
    border-radius: 999px;
    color: var(--samar);
    font-size: 11.5px;
    font-weight: 600;
  }

  .segmen button.aktif {
    background: var(--kartu);
    color: var(--teks);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .segmen.kecil button {
    height: 22px;
    padding: 0 9px;
    font-size: 11px;
  }

  .cari {
    width: 100%;
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--garis);
    border-radius: 10px;
    background: var(--latar);
    color: var(--teks);
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }

  .cari:focus {
    border-color: color-mix(in srgb, var(--teks) 30%, transparent);
  }

  .daftar-panel {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .baris-panel {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: auto;
    padding: 7px 8px;
    border-radius: 10px;
    text-align: left;
  }

  .baris-panel.statis {
    cursor: default;
  }

  .baris-panel.statis:hover {
    background: transparent;
  }

  .teks-panel {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .teks-panel strong {
    overflow: hidden;
    color: var(--teks);
    font-size: 12.5px;
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .teks-panel small {
    overflow: hidden;
    color: var(--samar);
    font-size: 10.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kosong-panel {
    margin: 6px 8px;
    color: var(--samar);
    font-size: 11.5px;
    line-height: 1.5;
  }

  .panel-kaki {
    display: flex;
    justify-content: flex-end;
  }

  .tautan {
    width: auto;
    height: auto;
    padding: 4px 8px;
    color: var(--teks);
    font-size: 11.5px;
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .zoom {
    display: flex;
    gap: 4px;
  }

  .zoom button {
    width: 28px;
    height: 28px;
    border-radius: 9px;
    background: var(--kartu-lembut);
    font-size: 14px;
  }

  .pintasan p {
    margin: 0 0 6px;
    color: var(--samar);
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .pintasan ul {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .pintasan li {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--samar);
    font-size: 11.5px;
  }

  .pintasan li span {
    margin-left: auto;
  }

  kbd {
    display: grid;
    place-items: center;
    min-width: 20px;
    height: 19px;
    padding: 0 5px;
    border: 1px solid var(--garis);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--latar);
    color: var(--teks);
    font-family: inherit;
    font-size: 10px;
    font-weight: 600;
  }

  .galat {
    position: fixed;
    bottom: 14px;
    left: 14px;
    width: auto;
    height: auto;
    max-width: 60vw;
    padding: 9px 13px;
    border: 1px solid var(--garis-kuat);
    border-radius: 10px;
    background: #171717;
    color: #f5f5f5;
    font-size: 12px;
    text-align: left;
    z-index: 30;
  }

  @media (prefers-reduced-motion: reduce) {
    button,
    .tab,
    .tutup,
    .alamat,
    .sakelar,
    .sakelar span,
    .progres span,
    .denyut {
      animation: none;
      transition: none;
    }
  }
</style>
