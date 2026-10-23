<script lang="ts">
  /* B371: habilidades do monstro — pares (id, nível) e, nas faixas de vida, a chance (`skill_hp75/50/25`, normalizada
     pelo servidor ao carregar, `elementdataman.cpp`, MONSTER_ESSENCE: escolhe uma das 5 quando a vida cai abaixo da
     faixa). O GS só usa no monstro habilidade de tipo 1 ataque, 2 bênção e 3 maldição (`npcgenerator.cpp:250-275`); as
     outras ele ignora com aviso — a tela mostra. */
  import { buscarHabilidades, habilidadePorId, type Habilidade } from "../lib/api";
  import { acrescentar, ajustar, porcentagens, remover, type LinhaDeHabilidade } from "../lib/chances";
  import Icone from "./Icone.svelte";
  import Modal from "./Modal.svelte";

  let { titulo, explica = "", maximo, linhas, comChance = false, editavel, aoMudar }:
    { titulo: string; explica?: string; maximo: number; linhas: LinhaDeHabilidade[]; comChance?: boolean; editavel: boolean;
      aoMudar: (l: LinhaDeHabilidade[]) => void } = $props();

  const TIPOS: Record<number, string> = { 1: "ataque", 2: "bênção", 3: "maldição" };
  let info = $state<Record<number, Habilidade | null>>({});
  $effect(() => { for (const l of linhas) if (!(l.id in info)) habilidadePorId(l.id).then((h) => (info[l.id] = h)); });
  const pct = $derived(comChance ? porcentagens(linhas.map((l) => l.peso ?? 0)) : []);

  let seletor = $state<null | { indice: number | null }>(null);
  let busca = $state("");
  let achadas = $state<Habilidade[]>([]);
  let espera: ReturnType<typeof setTimeout> | undefined;
  function buscar() { clearTimeout(espera); espera = setTimeout(async () => { achadas = (await buscarHabilidades(busca.trim()).catch(() => ({ habilidades: [] }))).habilidades; }, 250); }
  $effect(() => { if (seletor) buscar(); });

  function escolher(h: Habilidade) {
    if (!seletor) return;
    info[h.id] = h;
    if (seletor.indice === null) {
      const pesos = comChance ? acrescentar(linhas.map((l) => l.peso ?? 0)) : [];
      aoMudar([...linhas, { id: h.id, nivel: 1 }].map((l, k) => (comChance ? { ...l, peso: pesos[k] } : l)));
    } else {
      const i = seletor.indice;
      aoMudar(linhas.map((l, k) => (k === i ? { ...l, id: h.id } : l)));
    }
    seletor = null;
  }
  function tirar(i: number) {
    const pesos = comChance ? remover(linhas.map((l) => l.peso ?? 0), i) : [];
    aoMudar(linhas.filter((_, k) => k !== i).map((l, k) => (comChance ? { ...l, peso: pesos[k] } : l)));
  }
  const fmt = (x: number) => x.toLocaleString("pt-BR", { maximumFractionDigits: 1 });
</script>

<div class="bloco">
  <h4>{titulo} <span class="mini">{linhas.length}/{maximo}</span></h4>
  {#if explica}<p class="mini">{explica}</p>{/if}
  <ul>
    {#each linhas as l, i (i)}
      {@const h = info[l.id]}
      <li class:alerta={h && !TIPOS[h.type]}>
        <button class="nome" type="button" disabled={!editavel} onclick={() => { busca = ""; seletor = { indice: i }; }}>
          <span>{h?.nome ?? `habilidade ${l.id}`}</span>
          <span class="mini">id {l.id}{h ? ` · ${TIPOS[h.type] ?? `tipo ${h.type}: o servidor ignora no monstro`}` : ""}</span>
        </button>
        <label class="mini">nível <input class="campo n" type="number" min="1" max={h?.max_level ?? 99} step="1" value={l.nivel} disabled={!editavel}
          onchange={(e) => aoMudar(linhas.map((x, k) => (k === i ? { ...x, nivel: Number((e.target as HTMLInputElement).value) } : x)))} /></label>
        {#if comChance}
          <input type="range" min="0" max="1000" value={Math.round(pct[i] * 10)} disabled={!editavel || linhas.length < 2} aria-label="Chance"
            oninput={(e) => { const p = ajustar(linhas.map((x) => x.peso ?? 0), i, Number((e.target as HTMLInputElement).value) / 1000); aoMudar(linhas.map((x, k) => ({ ...x, peso: p[k] }))); }} />
          <span class="pct">{fmt(pct[i])}%</span>
        {/if}
        <button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Remover" onclick={() => tirar(i)}><Icone nome="lixeira" tamanho={16} /></button>
      </li>
    {:else}
      <li class="suave">Nenhuma.</li>
    {/each}
  </ul>
  <button class="botao pequeno" type="button" disabled={!editavel || linhas.length >= maximo} onclick={() => { busca = ""; seletor = { indice: null }; }}><Icone nome="mais" tamanho={16} /> Adicionar</button>
</div>

{#if seletor}
  <Modal titulo="{titulo}: escolher habilidade" aoFechar={() => (seletor = null)} largo>
    <input class="campo cheio" placeholder="nome ou id (ex.: Bola de Fogo, 30)" bind:value={busca} oninput={buscar} aria-label="Buscar habilidade" />
    <ul class="achadas">
      {#each achadas as h (h.id)}
        <li><button type="button" ondblclick={() => escolher(h)} onclick={() => escolher(h)}>
          <span>{h.nome}</span><span class="mini">id {h.id} · {TIPOS[h.type] ?? `tipo ${h.type} (o servidor ignora no monstro)`} · até nível {h.max_level}</span></button></li>
      {:else}
        <li class="suave">Nada encontrado.</li>
      {/each}
    </ul>
  </Modal>
{/if}

<style>
  .bloco { display: flex; flex-direction: column; gap: 6px; }
  h4 { margin: 4px 0 0; font-size: 13px; color: var(--texto-2); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 5px; }
  li { display: flex; align-items: center; gap: 8px; padding: 5px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  li.alerta { border-color: var(--vermelho); }
  .nome { flex: 1; display: flex; flex-direction: column; align-items: flex-start; background: none; border: 0; color: var(--texto); font: inherit; cursor: pointer; text-align: left; min-width: 0; }
  .n { width: 64px; height: 26px; }
  .pct { width: 52px; text-align: right; font-variant-numeric: tabular-nums; }
  input[type="range"] { width: 110px; accent-color: var(--ouro); }
  .cheio { width: 100%; }
  .achadas { max-height: 50vh; overflow: auto; margin-top: 8px; }
  .achadas button { width: 100%; display: flex; flex-direction: column; align-items: flex-start; background: none; border: 0; color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
</style>
