<script lang="ts">
  /* Ícone do item. O cliente não lê ícone em arquivo solto: só o atlas `Surfaces\IconSet\IconList_Ivtr{M,F}.dds` com o
     `.txt` de nomes (`EC_GameUIMan.cpp:588-640`), e acha a célula pelo nome do arquivo do `file_icon` em minúsculas
     (`SetCover`, `:6204-6208`). Escolher um ícone = trocar o nome no fim do `file_icon` por um que o atlas tem.
     Enviar ícone novo exigiria gravar uma célula nos dois atlas e o nome no `.txt` (item `cliente` do rascunho): fica
     para um bloco próprio, combinado com o Murillo em 2026-10-23. */
  import { buscarIcones } from "../lib/api";
  import { avisar } from "../lib/estado.svelte";
  import IconeItem from "./IconeItem.svelte";
  import Modal from "./Modal.svelte";

  let { caminho, aoEscolher, aoFechar }: { caminho: string; aoEscolher: (novo: string) => void; aoFechar: () => void } = $props();

  const pasta = $derived(caminho.includes("\\") ? caminho.slice(0, caminho.lastIndexOf("\\") + 1) : "");
  const arquivo = $derived(caminho.slice(caminho.lastIndexOf("\\") + 1));
  let busca = $state("");
  let total = $state(0);
  let icones = $state<{ arquivo: string; hex: string }[]>([]);
  let escolhido = $state<{ arquivo: string; hex: string } | null>(null);
  let espera: ReturnType<typeof setTimeout> | undefined;

  async function buscar() {
    try {
      const r = await buscarIcones(busca.trim(), 120);
      total = r.total;
      icones = r.icones;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { buscar(); });
  function digitar() { clearTimeout(espera); espera = setTimeout(buscar, 250); }
</script>

<Modal titulo="Ícone do item" {aoFechar} largo>
  <dl class="info">
    <dt>file_icon</dt><dd><code>{caminho || "(vazio)"}</code></dd>
    <dt>No cliente</dt><dd><code>Surfaces\IconSet\IconList_IvtrM.dds</code> / <code>IconList_IvtrF.dds</code>, célula <code>{arquivo.toLowerCase()}</code> (lista em <code>IconList_Ivtr*.txt</code>)</dd>
  </dl>
  <input class="campo cheio" placeholder="parte do nome do arquivo do ícone" bind:value={busca} oninput={digitar} aria-label="Buscar ícone" />
  <p class="mini">{total} ícone(s) no atlas{total > icones.length ? `, mostrando ${icones.length}` : ""}.</p>
  <ul class="grade">
    {#each icones as i (i.arquivo)}
      <li>
        <button type="button" class:ativo={escolhido === i} title={i.arquivo} onclick={() => (escolhido = i)}
          ondblclick={() => { aoEscolher(pasta + i.arquivo); aoFechar(); }}>
          <IconeItem hex={i.hex} nome={i.arquivo} />
        </button>
      </li>
    {/each}
  </ul>
  {#if escolhido}<p class="mini">Novo file_icon: <code>{pasta}{escolhido.arquivo}</code></p>{/if}
  <p class="mini">Enviar ícone novo: em breve (precisa gravar uma célula nos dois atlas e o nome na lista).</p>
  {#snippet rodape()}
    <button class="botao" type="button" disabled title="em breve">Enviar ícone…</button>
    <button class="botao primario" type="button" disabled={!escolhido} onclick={() => { if (escolhido) { aoEscolher(pasta + escolhido.arquivo); aoFechar(); } }}>Usar este</button>
  {/snippet}
</Modal>

<style>
  .info { display: grid; grid-template-columns: max-content 1fr; gap: 4px 12px; margin: 0 0 12px; font-size: 13px; }
  dt { color: var(--texto-2); }
  dd { margin: 0; word-break: break-all; }
  .cheio { width: 100%; }
  .grade { list-style: none; margin: 8px 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(44px, 1fr)); gap: 6px; max-height: 45vh; overflow: auto; }
  .grade button { padding: 4px; border-radius: 8px; border: 1px solid var(--borda); background: var(--fundo-2); cursor: pointer; display: grid; place-items: center; }
  .grade button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
</style>
