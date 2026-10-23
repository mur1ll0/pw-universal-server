<script lang="ts">
  /* Escolha de um registro de um espaço de id (`GET espacos/{E}`, o seletor de referência dos Editores). No espaço
     `ADDON` também busca pelo texto que o cliente mostra (`/api/efeitos`, B206) e mostra a descrição de cada efeito. */
  import { buscarEfeitosPorTexto, buscarNoEspaco, descreverEfeito, descreverEfeitos, ficha, type Registro } from "../lib/api";
  import { avisar } from "../lib/estado.svelte";
  import IconeItem from "./IconeItem.svelte";
  import Modal from "./Modal.svelte";

  let { titulo, espaco, tabela = "", aoEscolher, aoFechar, aoCriar }:
    { titulo: string; espaco: string; tabela?: string; aoEscolher: (r: Registro) => void; aoFechar: () => void; aoCriar?: () => void } = $props();

  let busca = $state("");
  let achados = $state<(Registro & { texto?: string })[]>([]);
  let escolhido = $state<(Registro & { texto?: string }) | null>(null);
  let descricao = $state<string[]>([]);
  let parametros = $state<number[] | null>(null);
  let espera: ReturnType<typeof setTimeout> | undefined;

  async function buscar() {
    try {
      let r = (await buscarNoEspaco(espaco, busca.trim(), 60)) as (Registro & { texto?: string })[];
      if (tabela) r = r.filter((x) => x.tabela === tabela);
      if (espaco === "ADDON" && busca.trim() && !/^\d+$/.test(busca.trim())) {
        // O texto do cliente acha efeitos que o nome interno do EQUIPMENT_ADDON não diz.
        const porTexto = await buscarEfeitosPorTexto(busca.trim()).catch(() => ({ efeitos: [] }));
        for (const e of porTexto.efeitos) {
          if (r.some((x) => x.id === e.id)) continue;
          const [x] = await buscarNoEspaco("ADDON", String(e.id), 1).catch(() => [] as Registro[]);
          if (x) r.push({ ...x, texto: e.texto });
        }
      }
      if (espaco === "ADDON" && r.length) {
        // Texto do cliente com o valor do efeito (o mesmo da edição de personagem, B209).
        const d = await descreverEfeitos(r.map((x) => x.id ?? 0), 1).catch(() => []);
        const porId = new Map(d.filter((x) => x.texto).map((x) => [x.id, x.texto]));
        r = r.map((x) => ({ ...x, texto: porId.get(x.id ?? 0) ?? x.texto }));
      }
      achados = r;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { buscar(); });
  function digitar() { clearTimeout(espera); espera = setTimeout(buscar, 250); }

  async function selecionar(r: Registro & { texto?: string }) {
    escolhido = r;
    descricao = [];
    parametros = null;
    if (espaco !== "ADDON" || r.indice === undefined || !r.tabela) return;
    try {
      const f = await ficha(r.tabela, r.indice);
      const v = (n: string) => Number(f.campos.find((c) => c.nome === n)?.valor ?? 0);
      const args = [v("param1"), v("param2"), v("param3")].slice(0, Math.max(0, v("num_params")));
      parametros = args;
      descricao = (await descreverEfeito(r.id!, args)).linhas;
    } catch { descricao = []; }
  }
</script>

<Modal {titulo} {aoFechar} largo>
  <input class="campo cheio" placeholder={espaco === "ADDON" ? "id, nome ou texto do efeito (ex.: ataque)" : "id ou nome"} bind:value={busca} oninput={digitar} aria-label="Buscar" />
  <ul class="lista">
    {#each achados as r (`${r.tabela}/${r.indice}`)}
      <li>
        <button type="button" class:ativo={escolhido === r} onclick={() => selecionar(r)} ondblclick={() => { aoEscolher(r); aoFechar(); }}>
          {#if espaco !== "ADDON"}<IconeItem hex={r.icone} nome={r.nome} />{/if}
          <span class="nome">{r.texto || r.nome || "(sem nome)"}{#if r.texto}<span class="mini"> · {r.nome}</span>{/if}</span>
          <span class="mini">{r.tabela} · id {r.id}</span>
        </button>
      </li>
    {:else}
      <li class="suave">Nada encontrado.</li>
    {/each}
  </ul>
  {#if escolhido && parametros}
    <div class="descricao">
      <strong>Como o cliente descreve:</strong> {descricao.length ? descricao.join(" ") : "sem texto no catálogo do cliente (item_ext_prop.txt) — ex.: o refino é descrito pela tabela de refino"}
      <div class="mini">EQUIPMENT_ADDON id {escolhido.id} · parâmetros: {parametros.length ? parametros.join(", ") : "nenhum"}</div>
    </div>
  {/if}
  {#snippet rodape()}
    {#if aoCriar}<button class="botao" type="button" onclick={aoCriar}>Criar…</button>{/if}
    <button class="botao primario" type="button" disabled={!escolhido} onclick={() => { if (escolhido) { aoEscolher(escolhido); aoFechar(); } }}>Escolher</button>
  {/snippet}
</Modal>

<style>
  .cheio { width: 100%; }
  .lista { list-style: none; margin: 12px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 50vh; overflow: auto; }
  .lista button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .lista button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .nome { flex: 1; }
  .descricao { margin-top: 10px; padding: 8px 10px; border-radius: 8px; background: var(--fundo-2); }
</style>
