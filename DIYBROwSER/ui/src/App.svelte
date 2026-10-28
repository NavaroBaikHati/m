<script lang="ts">
  import { onMount } from "svelte";
  import { api, type DaftarTab, type PeristiwaNavigasi } from "./lib/api";

  let daftar = $state<DaftarTab>({ tabs: [], aktif: null, sidebarTerbuka: true });
  let isiAlamat = $state("");
  let sedangMengetik = $state(false);
  let memuat = $state<Record<string, boolean>>({});
  let galat = $state("");

  let tabAktif = $derived(daftar.tabs.find((tab) => tab.id === daftar.aktif));
  let url = $derived(tabAktif?.url ?? "");
  let judul = $derived(tabAktif?.title || url);
  let isiMemuat = $derived(daftar.aktif ? memuat[daftar.aktif] === true : false);

  $effect(() => {
    if (!sedangMengetik) isiAlamat = url;
  });

  onMount(() => {
    let dibatalkan = false;
    let berhentiMendengar: (() => void) | undefined;

    void (async () => {
      try {
        daftar = await api.daftarTab();
        const unlisten = await api.onNavigasi(tanganiPeristiwa);
        if (dibatalkan) unlisten();
        else berhentiMendengar = unlisten;
      } catch (error) {
        galat = pesanGalat(error);
      }
    })();

    return () => {
      dibatalkan = true;
      berhentiMendengar?.();
    };
  });

  function tanganiPeristiwa(peristiwa: PeristiwaNavigasi) {
    const tab = daftar.tabs.find((t) => t.id === peristiwa.tab);
    if (tab) {
      if (peristiwa.url !== undefined) {
        tab.url = peristiwa.url;
        if (!sedangMengetik && tab.id === daftar.aktif) isiAlamat = peristiwa.url;
      }
      if (peristiwa.title !== undefined) tab.title = peristiwa.title;
    }
    if (peristiwa.memuat !== undefined) {
      memuat = { ...memuat, [peristiwa.tab]: peristiwa.memuat };
    }
  }

  function kirim(event: SubmitEvent) {
    event.preventDefault();
    if (!isiAlamat.trim()) return;
    void jalankan(api.navigasi(isiAlamat), () => {
      (document.activeElement as HTMLElement | null)?.blur();
    });
  }

  function panggil(aksi: () => Promise<void>) {
    void jalankan(aksi(), () => {});
  }

  async function perbarui(aksi: Promise<DaftarTab>) {
    galat = "";
    try {
      daftar = await aksi;
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function jalankan(aksi: Promise<void>, selesai: () => void) {
    galat = "";
    try {
      await aksi;
      selesai();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  function pesanGalat(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function tutupTab(event: MouseEvent, id: string) {
    // Jangan sampai klik tombol tutup ikut memilih tabnya.
    event.stopPropagation();
    void perbarui(api.tutupTab(id));
  }

  function judulTab(url: string): string {
    try {
      return new URL(url).host || url;
    } catch {
      return url;
    }
  }
</script>

<div class="aplikasi">
  {#if daftar.sidebarTerbuka}
    <aside class="sidebar">
      <div class="sidebar-kepala">
        <span class="merek">DIY</span>
        <button
          type="button"
          class="ikon"
          title="Tab baru"
          aria-label="Tab baru"
          onclick={() => perbarui(api.tabBaru())}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 3v10M3 8h10" />
          </svg>
        </button>
        <button
          type="button"
          class="ikon"
          title="Ciutkan sidebar"
          aria-label="Ciutkan sidebar"
          onclick={() => perbarui(api.aturSidebar(false))}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M9.5 3 5 8l4.5 5" />
            <path d="M3 3v10" />
          </svg>
        </button>
      </div>

      <nav class="daftar-tab" aria-label="Tab">
        {#each daftar.tabs as tab (tab.id)}
          <div
            class="tab"
            class:aktif={tab.id === daftar.aktif}
            role="button"
            tabindex="0"
            aria-current={tab.id === daftar.aktif ? "page" : undefined}
            onclick={() => perbarui(api.pilihTab(tab.id))}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                void perbarui(api.pilihTab(tab.id));
              }
            }}
          >
            <span class="tab-judul">{tab.title || judulTab(tab.url)}</span>
            {#if memuat[tab.id]}
              <span class="tab-memuat" aria-label="Memuat"></span>
            {/if}
            <button
              type="button"
              class="tutup"
              title="Tutup tab"
              aria-label="Tutup tab"
              onclick={(event) => tutupTab(event, tab.id)}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M4 4l8 8M12 4l-8 8" />
              </svg>
            </button>
          </div>
        {/each}
      </nav>
    </aside>
  {/if}

  <div class="utama">
    <header class="chrome">
      {#if !daftar.sidebarTerbuka}
        <button
          type="button"
          class="ikon"
          title="Buka sidebar"
          aria-label="Buka sidebar"
          onclick={() => perbarui(api.aturSidebar(true))}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M6.5 3 11 8l-4.5 5" />
            <path d="M13 3v10" />
          </svg>
        </button>
      {/if}

      <div class="tombol">
        <button type="button" title="Mundur" aria-label="Mundur" onclick={() => panggil(api.mundur)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 2 4 8l6 6" /></svg>
        </button>
        <button type="button" title="Maju" aria-label="Maju" onclick={() => panggil(api.maju)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 2l6 6-6 6" /></svg>
        </button>
        <button
          type="button"
          title="Muat ulang"
          aria-label="Muat ulang"
          class:memuat={isiMemuat}
          onclick={() => panggil(api.muatUlang)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M13 8a5 5 0 1 1-1.6-3.7" />
            <path d="M13 2v3.2h-3.2" />
          </svg>
        </button>
      </div>

      <form class="alamat" onsubmit={kirim}>
        <input
          bind:value={isiAlamat}
          onfocus={() => (sedangMengetik = true)}
          onblur={() => (sedangMengetik = false)}
          aria-label="Address bar"
          placeholder="Ketik alamat atau kata kunci"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
        />
      </form>

      <p class="judul" title={judul}>{judul}</p>
    </header>
  </div>
</div>

{#if galat}
  <p class="galat" role="alert">{galat}</p>
{/if}

<style>
  .aplikasi {
    display: flex;
    height: 100vh;
  }

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
    gap: 4px;
    height: var(--tinggi-chrome);
    padding: 0 8px;
  }

  .merek {
    flex: 1;
    padding-left: 6px;
    color: var(--teks);
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .daftar-tab {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    padding: 6px 8px 10px;
  }

  .tab {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 4px 0 10px;
    border-radius: 9px;
    color: var(--teks);
    cursor: default;
    transition: background 120ms ease-out;
  }

  .tab:hover {
    background: color-mix(in srgb, var(--teks) 8%, transparent);
  }

  .tab:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--aksen) 60%, transparent);
    outline-offset: -1px;
  }

  .tab.aktif {
    background: color-mix(in srgb, var(--aksen) 16%, transparent);
  }

  .tab-judul {
    flex: 1;
    overflow: hidden;
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-memuat {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--aksen);
    animation: denyut 1s ease-in-out infinite;
  }

  @keyframes denyut {
    50% {
      opacity: 0.25;
    }
  }

  .tutup {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--teks-samar);
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

  .utama {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    background: var(--latar);
  }

  .chrome {
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    height: var(--tinggi-chrome);
    padding: 0 10px;
    background: var(--chrome);
    border-bottom: 1px solid var(--garis);
    -webkit-app-region: no-drag;
  }

  .tombol {
    display: flex;
    gap: 2px;
  }

  button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--teks);
    cursor: pointer;
    transition: background 120ms ease-out;
  }

  button:hover {
    background: color-mix(in srgb, var(--teks) 10%, transparent);
  }

  button svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  button.memuat svg {
    animation: putar 900ms linear infinite;
  }

  @keyframes putar {
    to {
      transform: rotate(360deg);
    }
  }

  .alamat {
    flex: 1;
    min-width: 0;
  }

  .alamat input {
    width: 100%;
    height: 32px;
    padding: 0 12px;
    border: 1px solid var(--garis);
    border-radius: 10px;
    background: var(--latar);
    color: var(--teks);
    font: inherit;
    user-select: text;
  }

  .alamat input:focus {
    outline: 2px solid color-mix(in srgb, var(--aksen) 60%, transparent);
    outline-offset: -1px;
    border-color: var(--aksen);
  }

  .judul {
    max-width: 280px;
    margin: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--teks-samar);
    font-size: 12px;
  }

  .galat {
    position: fixed;
    right: 12px;
    bottom: 12px;
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    background: #b3261e;
    color: #fff;
    font-size: 12px;
  }

  @media (prefers-reduced-motion: reduce) {
    button,
    .tab,
    .tutup,
    button.memuat svg,
    .tab-memuat {
      transition: none;
      animation: none;
    }
  }
</style>
