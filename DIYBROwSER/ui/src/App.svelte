<script lang="ts">
  import { onMount } from "svelte";
  import { api, type PeristiwaNavigasi } from "./lib/api";

  let url = $state("");
  let isiAlamat = $state("");
  let judul = $state("");
  let memuat = $state(false);
  let sedangMengetik = $state(false);
  let galat = $state("");

  onMount(() => {
    let dibatalkan = false;
    let berhentiMendengar: (() => void) | undefined;

    void (async () => {
      try {
        const tab = await api.tabAktif();
        if (tab) {
          url = tab.url;
          judul = tab.title;
          if (!sedangMengetik) isiAlamat = tab.url;
        }
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
    if (peristiwa.url !== undefined) {
      url = peristiwa.url;
      if (!sedangMengetik) isiAlamat = peristiwa.url;
    }
    if (peristiwa.title !== undefined) judul = peristiwa.title;
    if (peristiwa.memuat !== undefined) memuat = peristiwa.memuat;
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
</script>

<header class="chrome">
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
      class:memuat
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

  <p class="judul" title={judul}>{judul || url}</p>
</header>

{#if galat}
  <p class="galat" role="alert">{galat}</p>
{/if}

<style>
  .chrome {
    display: flex;
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
    button.memuat svg {
      transition: none;
      animation: none;
    }
  }
</style>
