<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    EVENT_PANEL,
    type Bookmark,
    type PilihanMesin,
    type Riwayat,
    type StatusHalaman,
    type Unduhan,
  } from "./lib/api";

  type Jenis = "" | "pustaka" | "unduhan" | "menu" | "perlindungan";

  let jenis = $state<Jenis>("");
  let pustakaTab = $state<"bookmark" | "riwayat">("bookmark");
  let cari = $state("");
  let bookmarkList = $state<Bookmark[]>([]);
  let riwayatList = $state<Riwayat[]>([]);
  let unduhanList = $state<Unduhan[]>([]);
  let mesinPencarian = $state<PilihanMesin[]>([]);
  let status = $state<StatusHalaman | null>(null);
  let tabAktif = $state<string | null>(null);
  let galat = $state("");

  const lencana = $derived(status?.perlindungan);

  const riwayatTersaring = $derived(
    cari.trim()
      ? riwayatList.filter((item) =>
          `${item.judul} ${item.url}`.toLowerCase().includes(cari.toLowerCase()),
        )
      : riwayatList,
  );
  const bookmarkTersaring = $derived(
    cari.trim()
      ? bookmarkList.filter((item) =>
          `${item.judul} ${item.url}`.toLowerCase().includes(cari.toLowerCase()),
        )
      : bookmarkList,
  );

  onMount(() => {
    let dibatalkan = false;
    let bersih: (() => void)[] = [];

    void (async () => {
      try {
        await segarkan();
        const lepas = await Promise.all([
          api.onPanel((baru) => {
            if (!baru) {
              jenis = "";
              return;
            }
            if (baru !== jenis) cari = "";
            jenis = baru as Jenis;
            void segarkan();
          }),
          api.onUnduhan(() => {
            if (jenis === "unduhan") void muatUnduhan();
          }),
          api.onPerlindungan(() => void segarkanStatus()),
        ]);
        if (dibatalkan) lepas.forEach((f) => f());
        else bersih = lepas;
        // Webview panel dibuat tepat sebelum dibuka, jadi event pertama bisa
        // tiba sebelum pendengar terpasang; tanyakan keadaan terakhirnya.
        const awal = await api.jenisPanel();
        if (awal && !dibatalkan) {
          jenis = awal as Jenis;
          await segarkan();
        }
      } catch (error) {
        galat = pesanGalat(error);
      }
    })();

    const tombol = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void tutup();
      }
    };
    window.addEventListener("keydown", tombol);

    return () => {
      dibatalkan = true;
      bersih.forEach((f) => f());
      window.removeEventListener("keydown", tombol);
    };
  });

  async function segarkan() {
    await segarkanStatus();
    if (jenis === "pustaka") await muatPustaka();
    if (jenis === "unduhan") await muatUnduhan();
    if (jenis === "menu") await muatMesin();
  }

  async function segarkanStatus() {
    try {
      status = await api.status();
      const tema = status.tema;
      if (tema === "sistem") delete document.documentElement.dataset.tema;
      else document.documentElement.dataset.tema = tema;
      const daftar = await api.daftarTab();
      tabAktif = daftar.aktif;
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

  async function muatMesin() {
    try {
      mesinPencarian = await api.daftarMesinPencarian();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function tutup() {
    try {
      await api.tutupPanel();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function bukaUrl(url: string) {
    try {
      await api.navigasi(url);
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function hapusRiwayat() {
    try {
      await api.hapusRiwayat();
      await muatPustaka();
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function toggleShield() {
    if (!tabAktif || !lencana) return;
    try {
      const baru = await api.aturPerlindungan(tabAktif, !lencana.aktif);
      if (status) status = { ...status, perlindungan: baru };
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  async function pasangTema(tema: string) {
    try {
      const baru = await api.aturTema(tema);
      if (status) status = { ...status, tema: baru };
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

  async function ubahZoom(nilai: number) {
    try {
      const zoom = await api.aturZoom(nilai);
      if (status) status = { ...status, zoom };
    } catch (error) {
      galat = pesanGalat(error);
    }
  }

  function pesanGalat(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  function judulTab(url: string): string {
    try {
      return new URL(url).host.replace(/^www\./, "") || url;
    } catch {
      return url;
    }
  }

  function inisial(url: string, judul: string): string {
    const host = judulTab(url);
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

  const namaTema = $derived.by(() => {
    const tema = status?.tema;
    if (tema === "terang") return "Terang";
    if (tema === "gelap") return "Gelap";
    return "Mengikuti sistem";
  });

  const mesinAktif = $derived(mesinPencarian.find((mesin) => mesin.aktif));
</script>

<section class="kartu" class:hilang={jenis === ""} aria-label="Panel">
  {#if jenis === "perlindungan" && lencana}
    <header>
      <span class="judul">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M8 1.7 3.6 3.5v3.4c0 3 2 5.5 4.4 6.5 2.4-1 4.4-3.5 4.4-6.5V3.5L8 1.7Z" />
        </svg>
        Perlindungan
      </span>
      <span class="chip" class:mati={!lencana.aktif}>{lencana.aktif ? "Aktif" : "Mati"}</span>
      <button type="button" class="ikon" aria-label="Tutup" onclick={tutup}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>

    <p class="situs">{lencana.host || "Halaman ini"}</p>

    <div class="angka">
      <b>{lencana.jumlahAturan.toLocaleString("id-ID")}</b>
      <span>aturan penyaring siap memblokir iklan dan pelacak</span>
    </div>

    <div class="baris">
      <div>
        <strong>Blokir iklan & pelacak</strong>
        <small>
          {#if lencana.siap}
            {lencana.jumlahSelector} selector khusus untuk situs ini
          {:else}
            Daftar filter masih disiapkan
          {/if}
        </small>
      </div>
      <button
        type="button"
        class="sakelar"
        class:nyala={lencana.aktif}
        role="switch"
        aria-checked={lencana.aktif}
        aria-label="Blokir iklan di situs ini"
        onclick={toggleShield}
      >
        <span></span>
      </button>
    </div>

    <p class="catatan">
      WebKit memblokir request sebelum keluar dari perangkat. Browser tidak menerima laporan
      per-request, jadi yang ditampilkan adalah jumlah aturan yang menyaring halaman ini.
    </p>
  {:else if jenis === "pustaka"}
    <header>
      <div class="segmen">
        <button type="button" class:aktif={pustakaTab === "bookmark"} onclick={() => (pustakaTab = "bookmark")}>
          Bookmark
        </button>
        <button type="button" class:aktif={pustakaTab === "riwayat"} onclick={() => (pustakaTab = "riwayat")}>
          Riwayat
        </button>
      </div>
      <button type="button" class="ikon" aria-label="Tutup" onclick={tutup}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>

    <input class="cari" bind:value={cari} placeholder="Cari…" autocomplete="off" />

    <div class="daftar">
      {#if pustakaTab === "bookmark"}
        {#each bookmarkTersaring as item (item.url)}
          <button type="button" class="baris-panel" onclick={() => bukaUrl(item.url)}>
            <span class="avatar">{inisial(item.url, item.judul)}</span>
            <span class="teks">
              <strong>{item.judul || judulTab(item.url)}</strong>
              <small>{item.url}</small>
            </span>
          </button>
        {:else}
          <p class="kosong">Belum ada bookmark. Tekan Ctrl+D di halaman mana pun.</p>
        {/each}
      {:else}
        {#each riwayatTersaring as item (item.url)}
          <button type="button" class="baris-panel" onclick={() => bukaUrl(item.url)}>
            <span class="avatar">{inisial(item.url, item.judul)}</span>
            <span class="teks">
              <strong>{item.judul || judulTab(item.url)}</strong>
              <small>{judulTab(item.url)} · {waktuRelatif(item.dikunjungi)}</small>
            </span>
          </button>
        {:else}
          <p class="kosong">Belum ada riwayat.</p>
        {/each}
      {/if}
    </div>

    {#if pustakaTab === "riwayat" && riwayatList.length > 0}
      <footer>
        <button type="button" class="tautan" onclick={hapusRiwayat}>Hapus semua riwayat</button>
      </footer>
    {/if}
  {:else if jenis === "unduhan"}
    <header>
      <span class="judul">Unduhan</span>
      <button type="button" class="ikon" aria-label="Tutup" onclick={tutup}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>
    <div class="daftar">
      {#each unduhanList as item (item.url + item.waktu)}
        <div class="baris-panel statis">
          <span class="avatar netral" aria-hidden="true">
            <svg viewBox="0 0 16 16"><path d="M8 2.8v7M5 7l3 3 3-3M3 12.5h10" /></svg>
          </span>
          <span class="teks">
            <strong>{item.nama}</strong>
            <small title={item.path}>{item.status} · {waktuRelatif(item.waktu)}</small>
          </span>
        </div>
      {:else}
        <p class="kosong">Belum ada unduhan.</p>
      {/each}
    </div>
  {:else if jenis === "menu"}
    <header>
      <span class="judul">Menu</span>
      <button type="button" class="ikon" aria-label="Tutup" onclick={tutup}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
    </header>

    <div class="baris">
      <div><strong>Tema</strong><small>{namaTema}</small></div>
      <div class="segmen kecil">
        {#each [["sistem", "Sistem"], ["terang", "Terang"], ["gelap", "Gelap"]] as [nilai, label]}
          <button type="button" class:aktif={(status?.tema ?? "gelap") === nilai} onclick={() => pasangTema(nilai)}>
            {label}
          </button>
        {/each}
      </div>
    </div>

    <div class="baris">
      <div><strong>Mesin pencari</strong><small>{mesinAktif?.nama ?? "—"}</small></div>
      <div class="segmen kecil">
        {#each mesinPencarian as mesin (mesin.slug)}
          <button type="button" class:aktif={mesin.aktif} onclick={() => pasangMesin(mesin.slug)}>
            {mesin.nama}
          </button>
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
  {/if}

  {#if galat}
    <p class="galat" role="alert">{galat}</p>
  {/if}
</section>

<style>
  .kartu {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 12px 14px;
    overflow: hidden;
    border: 1px solid var(--garis);
    border-radius: 14px;
    background: var(--panel);
    color: var(--teks);
    font-size: 13px;
    opacity: 1;
    transform: translateY(0);
    transition: opacity 110ms ease-out, transform 110ms ease-out;
  }

  /* Panel lama sempat terlihat sebelum data jenis baru tiba. */
  .kartu.hilang {
    opacity: 0;
    transform: translateY(-4px);
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    margin-bottom: 10px;
  }

  .judul {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12.5px;
    font-weight: 650;
    letter-spacing: 0.01em;
  }

  .judul svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linejoin: round;
  }

  .chip {
    padding: 2px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--teks) 10%, transparent);
    color: var(--teks);
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .chip.mati {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--garis-kuat);
    color: var(--samar);
  }

  .ikon {
    display: grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--samar);
    cursor: default;
  }

  .ikon:hover {
    background: var(--kartu-lembut);
    color: var(--teks);
  }

  .ikon svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
  }

  .situs {
    margin: 0 0 10px;
    overflow: hidden;
    color: var(--samar);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .angka {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--kartu-lembut);
  }

  .angka b {
    font-size: 19px;
    font-weight: 640;
    font-variant-numeric: tabular-nums;
  }

  .angka span,
  .baris small {
    color: var(--samar);
    font-size: 11.5px;
    line-height: 1.35;
  }

  .baris {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 0;
    border-top: 1px solid var(--garis);
  }

  .baris:first-of-type {
    border-top: 0;
  }

  .baris strong {
    display: block;
    font-size: 12.5px;
    font-weight: 600;
  }

  .sakelar {
    flex: none;
    width: 38px;
    height: 22px;
    padding: 2px;
    border: 0;
    border-radius: 999px;
    background: var(--garis-kuat);
    cursor: default;
    transition: background 130ms ease-out;
  }

  .sakelar span {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--kartu);
    transition: transform 130ms ease-out;
  }

  .sakelar.nyala {
    background: var(--aksen);
  }

  .sakelar.nyala span {
    transform: translateX(16px);
  }

  .catatan {
    margin: 10px 0 0;
    color: var(--samar);
    font-size: 11px;
    line-height: 1.45;
  }

  .segmen {
    display: flex;
    flex: 1;
    gap: 2px;
    padding: 2px;
    border-radius: 8px;
    background: var(--kartu-lembut);
  }

  /* Mesin pencari bisa lebih banyak dari lebar panel: biarkan bergulir
     mendatar, jangan sampai terpotong. */
  .segmen.kecil {
    flex: 1 1 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .segmen.kecil::-webkit-scrollbar {
    display: none;
  }

  .segmen.kecil button {
    flex: none;
  }

  .segmen button {
    flex: 1;
    padding: 4px 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--samar);
    font: inherit;
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
    cursor: default;
  }

  .segmen button.aktif {
    background: var(--kartu);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18);
    color: var(--teks);
  }

  .cari {
    flex: none;
    margin-bottom: 6px;
    padding: 6px 9px;
    border: 1px solid var(--garis);
    border-radius: 8px;
    background: var(--kartu-lembut);
    color: var(--teks);
    font: inherit;
    font-size: 12px;
    outline: none;
  }

  .cari:focus {
    border-color: var(--garis-kuat);
  }

  .daftar {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    margin: 0 -6px;
    padding: 0 6px;
    overflow-y: auto;
  }

  .baris-panel {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 6px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--teks);
    text-align: left;
    cursor: default;
  }

  .baris-panel:hover {
    background: var(--kartu-lembut);
  }

  .baris-panel.statis {
    cursor: default;
  }

  .avatar {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--kartu-lembut);
    color: var(--teks);
    font-size: 10.5px;
    font-weight: 650;
  }

  .avatar.netral svg {
    width: 13px;
    height: 13px;
    fill: none;
    stroke: var(--samar);
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .teks {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }

  .teks strong {
    overflow: hidden;
    font-size: 12px;
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .teks small {
    overflow: hidden;
    color: var(--samar);
    font-size: 10.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kosong {
    margin: 8px 6px;
    color: var(--samar);
    font-size: 11.5px;
    line-height: 1.45;
  }

  footer {
    flex: none;
    padding-top: 8px;
    border-top: 1px solid var(--garis);
  }

  .tautan {
    padding: 2px;
    border: 0;
    background: transparent;
    color: var(--samar);
    font: inherit;
    font-size: 11.5px;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: default;
  }

  .tautan:hover {
    color: var(--teks);
  }

  .zoom {
    display: flex;
    flex: none;
    gap: 4px;
  }

  .zoom button {
    width: 26px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--garis);
    border-radius: 7px;
    background: transparent;
    color: var(--teks);
    font: inherit;
    cursor: default;
  }

  .zoom button:hover {
    background: var(--kartu-lembut);
  }

  .pintasan {
    margin-top: 4px;
    overflow-y: auto;
  }

  .pintasan p {
    margin: 0 0 6px;
    color: var(--samar);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
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
    font-size: 11.5px;
  }

  .pintasan li span {
    margin-left: auto;
    color: var(--samar);
  }

  kbd {
    padding: 1px 5px;
    border: 1px solid var(--garis);
    border-radius: 4px;
    background: var(--kartu-lembut);
    font-family: inherit;
    font-size: 10.5px;
  }

  .galat {
    flex: none;
    margin: 8px 0 0;
    padding: 6px 8px;
    border-radius: 7px;
    background: var(--kartu-lembut);
    color: var(--teks);
    font-size: 11px;
  }
</style>
