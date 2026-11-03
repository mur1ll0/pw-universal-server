<script lang="ts">
  /* B392: busca de NPCs pelo que fazem. Os filtros são faixas sobre a tabela inteira (`GET tabelas/NPC_ESSENCE?faixas=`,
     B376): "Lojas" = `id_sell_service` ≥ 1 etc. (`lib/npc.ts::FILTROS_DE_NPC`); o tipo é `id_type`. Novo = clone do NPC
     de menor id, com o nome "Novo NPC" (modelo, monstro de base e serviços vêm junto: troque na tela). */
  import { clonar, listar, salvar, type Registro } from "../lib/api";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import { FILTROS_DE_NPC } from "../lib/npc";
  import Icone from "./Icone.svelte";
  import Modal from "./Modal.svelte";

  let { aoFechar }: { aoFechar: () => void } = $props();
  const POR_PAGINA = 50;
  let busca = $state("");
  let filtros = $state<Set<string>>(new Set());
  let tipo = $state("");
  let tipos = $state<{ id: number; nome: string; registros: number }[]>([]);
  let pagina = $state(0);
  let total = $state(0);
  let registros = $state<Registro[]>([]);
  let ocupado = $state(false);
  const COLUNAS = [...FILTROS_DE_NPC.map((f) => f.campo), "id_type"];

  async function buscar() {
    const faixas: Record<string, { min?: number; max?: number }> = {};
    for (const f of FILTROS_DE_NPC) if (filtros.has(f.chave)) faixas[f.campo] = { min: 1 };
    if (tipo) faixas.id_type = { min: Number(tipo), max: Number(tipo) };
    try {
      const r = await listar("NPC_ESSENCE", busca.trim(), pagina, POR_PAGINA, { colunas: COLUNAS, faixas });
      registros = r.registros;
      total = r.total;
      if (!tipos.length && r.opcoes?.id_type) tipos = r.opcoes.id_type.map((o) => ({ id: o.valor, nome: o.nome ?? `tipo ${o.valor}`, registros: o.registros }));
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { void filtros; void tipo; void pagina; void estudio.rascunho; buscar(); });
  let espera: ReturnType<typeof setTimeout> | undefined;
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 250); }
  function alternar(k: string) { const s = new Set(filtros); if (s.has(k)) s.delete(k); else s.add(k); filtros = s; pagina = 0; }
  const faz = (r: Registro) => FILTROS_DE_NPC.filter((f) => Number(r.campos?.[f.campo] ?? 0) > 0).map((f) => f.nome);
  function abrir(r: Registro, extra: Record<string, string | number> = {}) { aoFechar(); ir(`npcs/NPC_ESSENCE/${r.indice}`, extra); }
  async function novo() {
    if (!estudio.rascunho) { avisar("aviso", "Escolha ou crie um rascunho no alto para criar."); return; }
    ocupado = true;
    try {
      const r = await listar("NPC_ESSENCE", "", 0, 1, { ordem: "id" });
      const base = r.registros[0];
      if (!base) throw new Error("Sem NPC para servir de base.");
      const c = await clonar("NPC_ESSENCE", base.indice);
      await salvar("NPC_ESSENCE", c.resultado.indice, { Name: "Novo NPC" });
      avisar("ok", `NPC novo id ${c.resultado.id} no rascunho (cópia do ${base.id}).`);
      abrir({ ...base, indice: c.resultado.indice, id: c.resultado.id });
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
  async function clonarEste(r: Registro) {
    if (!estudio.rascunho) { avisar("aviso", "Escolha ou crie um rascunho no alto para clonar."); return; }
    try {
      const c = await clonar("NPC_ESSENCE", r.indice);
      avisar("ok", `Cópia id ${c.resultado.id} no rascunho.`);
      abrir({ ...r, indice: c.resultado.indice, id: c.resultado.id });
    } catch (e) { avisar("erro", (e as Error).message); }
  }
</script>

<Modal titulo="NPCs · buscar" {aoFechar} largo>
  <div class="linha">
    <input class="campo busca" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar NPC" />
    <select class="campo" bind:value={tipo} onchange={() => (pagina = 0)} aria-label="Tipo">
      <option value="">Todos os tipos</option>
      {#each tipos as t (t.id)}<option value={String(t.id)}>{t.nome} ({t.registros})</option>{/each}
    </select>
  </div>
  <div class="chips" role="group" aria-label="O que o NPC faz">
    {#each FILTROS_DE_NPC as f (f.chave)}
      <button type="button" class:on={filtros.has(f.chave)} aria-pressed={filtros.has(f.chave)} onclick={() => alternar(f.chave)}>{f.nome}</button>
    {/each}
    <span class="mini">{total} NPC(s)</span>
  </div>
  <table class="tabela">
    <thead><tr><th>Nome</th><th>Id</th><th>Tipo</th><th>O que faz</th><th></th></tr></thead>
    <tbody>
      {#each registros as r (r.indice)}
        <tr>
          <td><button class="link" type="button" onclick={() => abrir(r)}>{r.nome || "(sem nome)"}</button></td>
          <td class="num">{r.id}</td>
          <td>{r.nomes?.id_type ?? r.campos?.id_type ?? ""}</td>
          <td><div class="faz">{#each faz(r) as x (x)}<span>{x}</span>{/each}</div></td>
          <td><button class="botao pequeno fantasma" type="button" onclick={() => clonarEste(r)} aria-label="Clonar"><Icone nome="copiar" tamanho={16} /></button></td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#snippet rodape()}
    <button class="botao" type="button" disabled={pagina === 0} onclick={() => pagina--}>Anterior</button>
    <span class="mini">página {pagina + 1} de {Math.max(1, Math.ceil(total / POR_PAGINA))}</span>
    <button class="botao" type="button" disabled={(pagina + 1) * POR_PAGINA >= total} onclick={() => pagina++}>Próxima</button>
    <button class="botao primario" type="button" disabled={ocupado} onclick={novo}><Icone nome="mais" /> Novo NPC</button>
  {/snippet}
</Modal>

<style>
  .busca { flex: 1; min-width: 200px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 10px 0; }
  .chips button { padding: 4px 10px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto-2); font: inherit; font-size: 12px; cursor: pointer; }
  .chips button.on { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .tabela { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabela th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--texto-3); padding: 6px; border-bottom: 1px solid var(--borda-forte); }
  .tabela td { padding: 5px 6px; border-bottom: 1px solid var(--borda); }
  .num { font-variant-numeric: tabular-nums; color: var(--texto-2); }
  .link { background: none; border: 0; color: var(--texto); font: inherit; font-weight: 600; cursor: pointer; padding: 0; text-align: left; }
  .link:hover { color: var(--ouro); }
  .faz { display: flex; flex-wrap: wrap; gap: 4px; }
  .faz span { font-size: 11px; padding: 1px 7px; border-radius: 999px; border: 1px solid var(--borda-forte); color: var(--texto-2); }
</style>
