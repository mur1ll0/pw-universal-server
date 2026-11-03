<script lang="ts">
  /* B394: busca de mascotes (`PET_ESSENCE`) pelo tipo e pelo ambiente — faixas sobre a tabela inteira
     (`GET tabelas/PET_ESSENCE?faixas=id_type:X:X`, B376). Novo = cópia do mascote de menor id do tipo escolhido, com o
     nome "Novo mascote" (fórmulas, modelo e ovo vêm junto: troque na tela). */
  import { clonar, listar, salvar, type Registro } from "../lib/api";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import { AMBIENTES, TIPOS_DE_MASCOTE, tipoDoMascote } from "../lib/mascote";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import Modal from "./Modal.svelte";

  let { aoFechar }: { aoFechar: () => void } = $props();
  const POR_PAGINA = 50;
  const COLUNAS = ["id_type", "inhabit_type", "level_require", "level_max"];
  const tipos = $derived(TIPOS_DE_MASCOTE.filter((t) => !t.so || t.so === estudio.versao));
  let busca = $state("");
  let tipo = $state<number | null>(8782);
  let ambiente = $state("");
  let pagina = $state(0);
  let total = $state(0);
  let registros = $state<Registro[]>([]);
  let ocupado = $state(false);
  let ordem = $state("level_require");

  async function buscar() {
    const faixas: Record<string, { min?: number; max?: number }> = {};
    if (tipo) faixas.id_type = { min: tipo, max: tipo };
    if (ambiente !== "") faixas.inhabit_type = { min: Number(ambiente), max: Number(ambiente) };
    try {
      const r = await listar("PET_ESSENCE", busca.trim(), pagina, POR_PAGINA, { colunas: COLUNAS, faixas, ordem });
      registros = r.registros;
      total = r.total;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { void tipo; void ambiente; void pagina; void ordem; void estudio.rascunho; buscar(); });
  let espera: ReturnType<typeof setTimeout> | undefined;
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 250); }
  function abrir(r: Registro) { aoFechar(); ir(`mascotes/PET_ESSENCE/${r.indice}`); }
  const ordenar = (c: string) => { ordem = ordem === c ? `-${c}` : c; pagina = 0; };
  const seta = (c: string) => (ordem === c ? " ▲" : ordem === `-${c}` ? " ▼" : "");

  async function copiar(base: Registro, nome?: string) {
    if (!estudio.rascunho) { avisar("aviso", "Escolha ou crie um rascunho no alto para criar."); return; }
    ocupado = true;
    try {
      const c = await clonar("PET_ESSENCE", base.indice);
      if (nome) await salvar("PET_ESSENCE", c.resultado.indice, { Name: nome });
      avisar("ok", `Mascote id ${c.resultado.id} no rascunho (cópia do ${base.id}). Crie o ovo dele na aba Ovo.`);
      abrir({ ...base, indice: c.resultado.indice, id: c.resultado.id });
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
  async function novo() {
    const t = tipo ?? 8782;
    const r = await listar("PET_ESSENCE", "", 0, 1, { ordem: "id", faixas: { id_type: { min: t, max: t } } }).catch(() => null);
    const base = r?.registros[0];
    if (!base) { avisar("aviso", "Nenhum mascote deste tipo para servir de base."); return; }
    await copiar(base, "Novo mascote");
  }
</script>

<Modal titulo="Mascotes · buscar" {aoFechar} largo>
  <div class="linha">
    <input class="campo busca" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar mascote" />
    <select class="campo" bind:value={ambiente} onchange={() => (pagina = 0)} aria-label="Ambiente">
      <option value="">Todos os ambientes</option>
      {#each AMBIENTES as a, k (k)}<option value={String(k)}>{a}</option>{/each}
    </select>
  </div>
  <div class="chips" role="group" aria-label="Tipo de mascote">
    <button type="button" class:on={tipo === null} aria-pressed={tipo === null} onclick={() => { tipo = null; pagina = 0; }}>Todos</button>
    {#each tipos as t (t.idType)}
      <button type="button" class:on={tipo === t.idType} aria-pressed={tipo === t.idType} title={t.explica} onclick={() => { tipo = t.idType; pagina = 0; }}>{t.nome}</button>
    {/each}
    <span class="mini">{total} mascote(s)</span>
  </div>
  {#if tipo}<p class="mini">{tipoDoMascote(tipo)?.explica}</p>{/if}
  <table class="tabela">
    <thead><tr>
      <th></th><th>Nome</th><th>Id</th><th>Tipo</th><th>Ambiente</th>
      <th><button class="ord" type="button" onclick={() => ordenar("level_require")}>Nível exigido{seta("level_require")}</button></th>
      <th><button class="ord" type="button" onclick={() => ordenar("level_max")}>Nível máx.{seta("level_max")}</button></th><th></th>
    </tr></thead>
    <tbody>
      {#each registros as r (r.indice)}
        <tr>
          <td class="ic"><IconeItem hex={r.icone} nome="" tamanho={28} discreto /></td>
          <td><button class="link" type="button" onclick={() => abrir(r)}>{r.nome || "(sem nome)"}</button></td>
          <td class="num">{r.id}</td>
          <td>{tipoDoMascote(Number(r.campos?.id_type ?? 0))?.nome ?? r.campos?.id_type}</td>
          <td>{AMBIENTES[Number(r.campos?.inhabit_type ?? 0)] ?? r.campos?.inhabit_type}</td>
          <td class="num">{r.campos?.level_require ?? ""}</td>
          <td class="num">{r.campos?.level_max ?? ""}</td>
          <td><button class="botao pequeno fantasma" type="button" disabled={ocupado} onclick={() => copiar(r)} aria-label="Clonar" title="Clonar"><Icone nome="copiar" tamanho={16} /></button></td>
        </tr>
      {:else}
        <tr><td colspan="8" class="suave">Nenhum mascote com esse filtro.</td></tr>
      {/each}
    </tbody>
  </table>
  {#snippet rodape()}
    <button class="botao" type="button" disabled={pagina === 0} onclick={() => pagina--}>Anterior</button>
    <span class="mini">página {pagina + 1} de {Math.max(1, Math.ceil(total / POR_PAGINA))}</span>
    <button class="botao" type="button" disabled={(pagina + 1) * POR_PAGINA >= total} onclick={() => pagina++}>Próxima</button>
    <button class="botao primario" type="button" disabled={ocupado} onclick={novo}><Icone nome="mais" /> Novo mascote</button>
  {/snippet}
</Modal>

<style>
  .busca { flex: 1; min-width: 200px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 10px 0 4px; }
  .chips button { padding: 4px 12px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto-2); font: inherit; font-size: 12px; cursor: pointer; }
  .chips button.on { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .tabela { width: 100%; border-collapse: collapse; font-size: 13px; margin-top: 8px; }
  .tabela th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--texto-3); padding: 6px; border-bottom: 1px solid var(--borda-forte); }
  .tabela td { padding: 4px 6px; border-bottom: 1px solid var(--borda); }
  .ic { width: 34px; }
  .num { font-variant-numeric: tabular-nums; color: var(--texto-2); }
  .link, .ord { background: none; border: 0; color: var(--texto); font: inherit; font-weight: 600; cursor: pointer; padding: 0; text-align: left; }
  .ord { font-size: 11px; text-transform: uppercase; color: var(--texto-3); }
  .link:hover, .ord:hover { color: var(--ouro); }
</style>
