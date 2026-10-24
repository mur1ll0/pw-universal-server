<script lang="ts">
  /* Escolha de um registro de um espaço de id (`GET espacos/{E}`, o seletor de referência dos Editores). No espaço
     `ADDON` também busca pelo texto que o cliente mostra (`/api/efeitos`, B206) e mostra a descrição de cada efeito. */
  import { untrack } from "svelte";
  import { buscarEfeitosPorTexto, buscarNoEspaco, descreverEfeito, descreverEfeitos, ficha, listar, tabelas, type Pagina, type Registro } from "../lib/api";
  import { COLUNAS_DO_SELETOR, tabelaComColunas } from "../lib/catalogo";
  import { esconderDica, mostrarDica } from "../lib/dica.svelte";
  import { avisar } from "../lib/estado.svelte";
  import IconeItem from "./IconeItem.svelte";
  import Modal from "./Modal.svelte";

  let { titulo, espaco, tabela = "", categoriaInicial = "", aoEscolher, aoFechar, aoCriar }:
    { titulo: string; espaco: string; tabela?: string; categoriaInicial?: string; aoEscolher: (r: Registro) => void; aoFechar: () => void; aoCriar?: () => void } = $props();

  let busca = $state("");
  /* B372: filtro por categoria (a tabela do elements). Com categoria, a busca é a listagem da tabela (`GET tabelas/{T}`),
     sem o limite de 60 do espaço inteiro. Os nomes das categorias mais usadas vêm traduzidos; o resto aparece pelo nome
     da tabela. */
  const NOMES_DE_CATEGORIA: Record<string, string> = {
    WEAPON_ESSENCE: "Armas", ARMOR_ESSENCE: "Armaduras", DECORATION_ESSENCE: "Ornamentos", FASHION_ESSENCE: "Moda",
    MATERIAL_ESSENCE: "Materiais", MEDICINE_ESSENCE: "Remédios", STONE_ESSENCE: "Pedras", TASKMATTER_ESSENCE: "Itens de missão",
    TASKNORMALMATTER_ESSENCE: "Itens de missão (comuns)", SKILLTOME_ESSENCE: "Livros de habilidade", FLYSWORD_ESSENCE: "Voo",
    WINGMANWING_ESSENCE: "Asas", PROJECTILE_ESSENCE: "Munição", QUIVER_ESSENCE: "Aljavas", ARMORRUNE_ESSENCE: "Runas de defesa",
    DAMAGERUNE_ESSENCE: "Runas de ataque", TOWNSCROLL_ESSENCE: "Pergaminhos de cidade", REVIVESCROLL_ESSENCE: "Pergaminhos de reviver",
    TRANSMITSCROLL_ESSENCE: "Pergaminhos de teleporte", ELEMENT_ESSENCE: "Pedras elementais", BIBLE_ESSENCE: "Livros",
    PET_EGG_ESSENCE: "Ovos de mascote", PET_FOOD_ESSENCE: "Comida de mascote", REFINE_TICKET_ESSENCE: "Amuletos de refino",
    DYE_TICKET_ESSENCE: "Tinturas", POKER_ESSENCE: "Cartas de General", GOBLIN_EQUIP_ESSENCE: "Equip. de Daimon",
    MONSTER_ESSENCE: "Monstros", NPC_ESSENCE: "NPCs", MINE_ESSENCE: "Minas",
  };
  /* Tabelas do ESSENCE que não são item (o GS não gera exemplo delas): sem prévia. */
  const SEM_PREVIA = /^(MONSTER|NPC|MINE)_ESSENCE$|_(TYPE|SERVICE)$/;
  const POR_PAGINA = 100;

  let categoria = $state(categoriaInicial);
  let categorias = $state<{ tabela: string; nome: string }[]>([]);
  if (espaco !== "ADDON") tabelas().then((t) => {
    const l = t.tabelas.filter((x) => x.espaco === espaco && x.registros > 0 && !/_(SERVICE|TYPE)$/.test(x.nome))
      .map((x) => ({ tabela: x.nome, nome: NOMES_DE_CATEGORIA[x.nome] ?? x.nome }));
    l.sort((a, b) => Number(!NOMES_DE_CATEGORIA[a.tabela]) - Number(!NOMES_DE_CATEGORIA[b.tabela]) || a.nome.localeCompare(b.nome, "pt-BR"));
    categorias = l;
  }).catch(() => {});
  let achados = $state<(Registro & { texto?: string })[]>([]);
  let escolhido = $state<(Registro & { texto?: string }) | null>(null);
  let descricao = $state<string[]>([]);
  let parametros = $state<number[] | null>(null);
  let espera: ReturnType<typeof setTimeout> | undefined;

  /* B376: colunas, filtro por faixa e ordenação — pedidos ao pw-recursos, valem sobre a tabela inteira. */
  let colunas = $state<string[]>([]);
  let opcoes = $state<NonNullable<Pagina["opcoes"]>>({});
  let ordem = $state("");
  let faixas = $state<Record<string, { min: number | null; max: number | null }>>({});
  let pagina = $state(0);
  let total = $state(0);
  let pedido = 0;
  const tituloDe = (c: string) => COLUNAS_DO_SELETOR.find((x) => x.campo === c)?.titulo ?? c;
  const comPrevia = (r: Registro) => espaco === "ESSENCE" && !!r.id && !!r.tabela && !SEM_PREVIA.test(r.tabela);

  async function buscar() {
    const meu = ++pedido;
    try {
      let r: (Registro & { texto?: string })[];
      if (categoria) {
        const comColunas = tabelaComColunas(categoria);
        const p = await listar(categoria, busca.trim(), pagina, POR_PAGINA, comColunas ? {
          colunas: COLUNAS_DO_SELETOR.map((x) => x.campo), ordem,
          faixas,
        } : {});
        if (meu !== pedido) return;
        r = p.registros.map((x) => ({ ...x, tabela: categoria }));
        total = p.total;
        colunas = p.colunas ?? [];
        opcoes = p.opcoes ?? {};
      } else {
        r = await buscarNoEspaco(espaco, busca.trim(), 60) as (Registro & { texto?: string })[];
        if (meu !== pedido) return;
        total = r.length;
        colunas = [];
        opcoes = {};
      }
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
      if (meu === pedido) achados = r;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { untrack(buscar); });
  $effect(() => esconderDica); // ao fechar o modal com o mouse sobre um ícone
  function depois() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 250); }
  function trocarCategoria() { pagina = 0; ordem = ""; faixas = {}; colunas = []; buscar(); }
  function irPara(p: number) { pagina = p; buscar(); }

  /* Clique no cabeçalho: crescente → decrescente → sem ordem. */
  function ordenar(c: string) {
    ordem = ordem === c ? `-${c}` : ordem === `-${c}` ? "" : c;
    pagina = 0;
    buscar();
  }
  const seta = (c: string) => (ordem === c ? "▲" : ordem === `-${c}` ? "▼" : "");
  const ariaSort = (c: string) => (ordem === c ? "ascending" : ordem === `-${c}` ? "descending" : "none");

  function limite(c: string, lado: "min" | "max", texto: string) {
    const v = texto.trim() === "" ? null : Number(texto);
    if (v !== null && !Number.isFinite(v)) return;
    faixas[c] = { ...(faixas[c] ?? { min: null, max: null }), [lado]: v };
    depois();
  }
  function escolherOpcao(c: string, texto: string) {
    if (texto === "") delete faixas[c];
    else faixas[c] = { min: Number(texto), max: Number(texto) };
    pagina = 0;
    buscar();
  }
  const valorDe = (r: Registro, c: string) => {
    const v = r.campos?.[c];
    if (v == null) return "—";
    return r.nomes?.[c] ?? v.toLocaleString("pt-BR");
  };

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
  const paginas = $derived(categoria ? Math.max(1, Math.ceil(total / POR_PAGINA)) : 1);
</script>

<Modal {titulo} {aoFechar} largo>
  {#if categorias.length}
    <div class="linha cat">
      <label class="mini" for="categoria">Categoria</label>
      <select id="categoria" class="campo" bind:value={categoria} onchange={trocarCategoria}>
        <option value="">Todas</option>
        {#each categorias as c (c.tabela)}<option value={c.tabela}>{c.nome}</option>{/each}
      </select>
    </div>
  {/if}
  <input class="campo cheio" placeholder={espaco === "ADDON" ? "id, nome ou texto do efeito (ex.: ataque)" : "id ou nome"} bind:value={busca} oninput={depois} aria-label="Buscar" />
  <div class="rolagem">
    <table class="tabela">
      <thead>
        <tr>
          {#if espaco !== "ADDON"}<th class="ic"></th>{/if}
          <th aria-sort={categoria ? ariaSort("nome") : undefined}>
            {#if categoria}<button class="cab" type="button" onclick={() => ordenar("nome")}>Nome {seta("nome")}</button>{:else}Nome{/if}
          </th>
          <th class="num" aria-sort={categoria ? ariaSort("id") : undefined}>
            {#if categoria}<button class="cab" type="button" onclick={() => ordenar("id")}>id {seta("id")}</button>{:else}id{/if}
          </th>
          {#each colunas as c (c)}
            <th class:num={!c.startsWith("id_")} aria-sort={ariaSort(c)}>
              <button class="cab" type="button" onclick={() => ordenar(c)} title="Ordenar por {tituloDe(c).toLowerCase()} ({c})">{tituloDe(c)} {seta(c)}</button>
            </th>
          {/each}
        </tr>
        {#if colunas.length}
          <tr class="filtros">
            {#if espaco !== "ADDON"}<td></td>{/if}<td></td><td></td>
            {#each colunas as c (c)}
              <td>
                {#if opcoes[c]}
                  <select class="campo mini-campo" aria-label="Filtrar {tituloDe(c)}" value={faixas[c] ? String(faixas[c].min) : ""}
                    onchange={(e) => escolherOpcao(c, (e.target as HTMLSelectElement).value)}>
                    <option value="">Todas</option>
                    {#each opcoes[c] as o (o.valor)}<option value={String(o.valor)}>{o.nome || `id ${o.valor}`} ({o.registros})</option>{/each}
                  </select>
                {:else}
                  <span class="faixa">
                    <input class="campo mini-campo" inputmode="numeric" placeholder="mín." aria-label="{tituloDe(c)} mínimo"
                      value={faixas[c]?.min ?? ""} oninput={(e) => limite(c, "min", (e.target as HTMLInputElement).value)} />
                    <input class="campo mini-campo" inputmode="numeric" placeholder="máx." aria-label="{tituloDe(c)} máximo"
                      value={faixas[c]?.max ?? ""} oninput={(e) => limite(c, "max", (e.target as HTMLInputElement).value)} />
                  </span>
                {/if}
              </td>
            {/each}
          </tr>
        {/if}
      </thead>
      <tbody>
        {#each achados as r (`${r.tabela}/${r.indice}`)}
          <tr class:ativo={escolhido === r}>
            {#if espaco !== "ADDON"}
              <td class="ic">
                {#if comPrevia(r)}
                  <!-- B376: a prévia do item (a mesma dos cartões de chance), logo acima do ícone. -->
                  <span class="icone-dica" role="img" aria-label="Prévia de {r.nome}"
                    onmouseenter={(e) => mostrarDica(r.id!, r.nome, e.currentTarget as HTMLElement)} onmouseleave={esconderDica}>
                    <IconeItem hex={r.icone} nome={r.nome} tamanho={28} />
                  </span>
                {:else}
                  <IconeItem hex={r.icone} nome={r.nome} tamanho={28} />
                {/if}
              </td>
            {/if}
            <td>
              <button class="linha-botao" type="button" onclick={() => selecionar(r)} ondblclick={() => { esconderDica(); aoEscolher(r); aoFechar(); }}>
                <span class="nome">{r.texto || r.nome || "(sem nome)"}</span>
                {#if r.texto}<span class="mini">{r.nome}</span>{/if}
                {#if !categoria}<span class="mini">{r.tabela}</span>{/if}
              </button>
            </td>
            <td class="num mini">{r.id}</td>
            {#each colunas as c (c)}<td class:num={!c.startsWith("id_")}>{valorDe(r, c)}</td>{/each}
          </tr>
        {:else}
          <tr><td class="suave" colspan={3 + colunas.length}>Nada encontrado.</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if categoria && total > POR_PAGINA}
    <div class="paginacao mini">
      <button class="botao" type="button" disabled={pagina === 0} onclick={() => irPara(pagina - 1)}>‹</button>
      <span>página {pagina + 1} de {paginas} · {total.toLocaleString("pt-BR")} registros</span>
      <button class="botao" type="button" disabled={pagina + 1 >= paginas} onclick={() => irPara(pagina + 1)}>›</button>
    </div>
  {:else if categoria}
    <div class="paginacao mini">{total.toLocaleString("pt-BR")} registros</div>
  {/if}
  {#if escolhido && parametros}
    <div class="descricao">
      <strong>Como o cliente descreve:</strong> {descricao.length ? descricao.join(" ") : "sem texto no catálogo do cliente (item_ext_prop.txt) — ex.: o refino é descrito pela tabela de refino"}
      <div class="mini">EQUIPMENT_ADDON id {escolhido.id} · parâmetros: {parametros.length ? parametros.join(", ") : "nenhum"}</div>
    </div>
  {/if}
  {#snippet rodape()}
    {#if aoCriar}<button class="botao" type="button" onclick={aoCriar}>Criar…</button>{/if}
    <button class="botao primario" type="button" disabled={!escolhido} onclick={() => { if (escolhido) { esconderDica(); aoEscolher(escolhido); aoFechar(); } }}>Escolher</button>
  {/snippet}
</Modal>

<style>
  .cheio { width: 100%; }
  .cat { margin-bottom: 8px; }
  .rolagem { margin-top: 12px; max-height: 52vh; overflow: auto; border: 1px solid var(--borda); border-radius: 8px; }
  .tabela { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabela th, .tabela td { padding: 4px 8px; border-bottom: 1px solid var(--borda); text-align: left; vertical-align: middle; }
  .tabela thead th { position: sticky; top: 0; background: var(--superficie); z-index: 1; white-space: nowrap; }
  .tabela thead tr.filtros td { position: sticky; top: 29px; background: var(--superficie); z-index: 1; }
  .tabela .num { text-align: right; white-space: nowrap; }
  .tabela .ic { width: 36px; }
  .tabela tbody tr:hover { background: var(--fundo-2); }
  .tabela tbody tr.ativo { background: var(--ouro-suave); }
  .cab { background: none; border: 0; padding: 0; color: var(--texto); font: inherit; font-weight: 600; cursor: pointer; white-space: nowrap; }
  .linha-botao { width: 100%; display: flex; flex-direction: column; align-items: flex-start; gap: 1px; padding: 2px 0; background: none; border: 0;
    color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .faixa { display: inline-flex; gap: 4px; }
  .mini-campo { width: 64px; padding: 2px 6px; font-size: 12px; }
  select.mini-campo { width: 140px; }
  .icone-dica { display: inline-flex; cursor: help; flex: none; }
  .paginacao { display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 8px; }
  .descricao { margin-top: 10px; padding: 8px 10px; border-radius: 8px; background: var(--fundo-2); }
</style>
