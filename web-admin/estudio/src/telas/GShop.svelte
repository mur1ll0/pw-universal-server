<script lang="ts">
  /* Estúdio G-Shop (ESTUDIO_DO_PAINEL.md §4.6): a Loja Gold como o jogador vê — abas à esquerda, vitrine em grade no
     meio, oferta à direita. Tudo no rascunho escolhido no alto, pelas rotas `estudio/loja/{arquivo}` do pw-recursos.
     O `gshop*.data` é o mesmo arquivo no cliente e no servidor (item `ambos`): a posição da oferta é o `goods_index`
     da compra (`MALL_SHOPPING`), por isso oferta nova vai para o fim e remover avisa que as seguintes sobem. */
  import { untrack } from "svelte";
  import type { Registro } from "../lib/api";
  import { avisar, estudio, rota } from "../lib/estado.svelte";
  import {
    DESCRICAO_DA_LOJA, FILTRO_INICIAL, LOJAS, SELOS, contarPorAba, filtrarVitrine, lerAbas, lerLoja, novaOferta,
    type FiltroDaVitrine, type Loja, type TemaDasAbas,
  } from "../lib/gshop";
  import ArvoreDeAbas from "../componentes/ArvoreDeAbas.svelte";
  import CartaoDaOferta from "../componentes/CartaoDaOferta.svelte";
  import Icone from "../componentes/Icone.svelte";
  import PainelDaOferta from "../componentes/PainelDaOferta.svelte";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";

  const lojas = $derived(estudio.versao === "1.2.6" ? ["gshop.data"] : [...LOJAS]);
  let arquivo = $state(rota.params.get("loja") || "gshop.data");
  let loja = $state<Loja | null>(null);
  let temas = $state<TemaDasAbas[]>([]);
  let erro = $state("");
  let filtro = $state<FiltroDaVitrine>({ ...FILTRO_INICIAL });
  let escolhida = $state<number | null>(null);
  let adicionando = $state(false);
  let limite = $state(120);

  async function carregar(selecionar?: number | null) {
    erro = "";
    try {
      const [l, a] = await Promise.all([lerLoja(arquivo), lerAbas(arquivo).catch(() => ({ temas: [] as TemaDasAbas[] }))]);
      loja = l;
      temas = a.temas;
      if (selecionar !== undefined) escolhida = selecionar;
      if (escolhida !== null && escolhida >= l.ofertas.length) escolhida = null;
      if (selecionar != null) {
        const o = l.ofertas[selecionar];
        if (o && filtro.aba !== null && (o.aba !== filtro.aba || (filtro.sub >= 0 && o.sub !== filtro.sub))) { filtro.aba = o.aba; filtro.sub = -1; }
      }
    } catch (e) { erro = (e as Error).message; loja = null; }
  }
  $effect(() => { arquivo; estudio.rascunho; untrack(() => carregar()); });

  const contagem = $derived(loja ? contarPorAba(loja.ofertas) : new Map<string, number>());
  const vitrine = $derived(loja ? filtrarVitrine(loja.ofertas, filtro, loja.conferencias) : []);
  const comErro = $derived(loja ? Object.values(loja.conferencias).filter((c) => c.erros.length).length : 0);
  const desativadas = $derived(loja ? loja.ofertas.filter((o) => o.mostrado === 0).length : 0);
  const linha = $derived(loja && escolhida !== null ? loja.ofertas[escolhida] : null);

  /* Oferta nova a partir de um item: cópia de uma oferta da mesma aba (preços, selo e imagem vêm dela), com o item,
     a quantidade 1, o nome do item, a aba e a subaba escolhidas e sem descrição. Vai para o fim. */
  async function adicionar(r: Registro) {
    adicionando = false;
    if (!loja) return;
    const aba = filtro.aba ?? 0;
    const sub = filtro.sub >= 0 ? filtro.sub : 0;
    const base = (escolhida !== null && loja.ofertas[escolhida]?.aba === aba ? escolhida : null)
      ?? loja.ofertas.find((o) => o.aba === aba && o.mostrado > 0)?.indice ?? loja.ofertas.find((o) => o.mostrado > 0)?.indice ?? 0;
    try {
      const n = await novaOferta(arquivo, base, {
        id: r.id ?? 0, num: 1, szName: [...(r.nome ?? "")].slice(0, 31).join(""), main_type: aba, sub_type: sub, desc: "",
      });
      avisar("ok", `Oferta #${n.resultado.indice} criada no fim (cópia da #${base}; nenhuma oferta mudou de índice).`);
      await carregar(n.resultado.indice);
    } catch (e) { avisar("erro", (e as Error).message); }
  }

  function trocarArquivo(a: string) {
    arquivo = a;
    escolhida = null;
    filtro = { ...FILTRO_INICIAL };
    const q = new URLSearchParams(rota.params);
    q.set("loja", a);
    history.replaceState(null, "", `#/gshop?${q}`);
  }
</script>

<section class="topo-loja">
  <div class="linha">
    <h1><Icone nome="loja" /> G-Shop</h1>
    <select class="campo" value={arquivo} onchange={(e) => trocarArquivo((e.target as HTMLSelectElement).value)} aria-label="Arquivo da loja">
      {#each lojas as a}<option value={a}>{a} — {DESCRICAO_DA_LOJA[a as keyof typeof DESCRICAO_DA_LOJA]}</option>{/each}
    </select>
    {#if loja}
      <span class="mini">{loja.ofertas.length} ofertas · layout {loja.formato} · carimbo {loja.carimbo}{#if loja.no_rascunho} · <b>editado no rascunho</b>{/if}</span>
    {/if}
  </div>
  <div class="linha filtros">
    <input class="campo busca" placeholder="Buscar por nome, item, id ou #índice" bind:value={filtro.busca} aria-label="Buscar" />
    <select class="campo" value={filtro.selo ?? ""} onchange={(e) => (filtro.selo = (e.target as HTMLSelectElement).value === "" ? null : Number((e.target as HTMLSelectElement).value))} aria-label="Selo">
      <option value="">todos os selos</option>
      {#each SELOS.slice(0, 4) as s}<option value={s.valor}>{s.nome}</option>{/each}
      <option value={4}>Descontos (4–12)</option><option value={13}>Relâmpago</option><option value={14}>VIP 1–6 (14–19)</option>
    </select>
    <label class="mini"><input type="checkbox" bind:checked={filtro.soVip} /> só VIP</label>
    <label class="mini"><input type="checkbox" bind:checked={filtro.soBrinde} /> com brinde</label>
    <label class="mini"><input type="checkbox" bind:checked={filtro.desativadas} /> desativadas ({desativadas})</label>
    <label class="mini"><input type="checkbox" bind:checked={filtro.soComProblema} /> só com erro ({comErro})</label>
    <select class="campo" bind:value={filtro.ordem} aria-label="Ordem">
      <option value="cliente">ordem da vitrine do jogo</option><option value="arquivo">ordem do arquivo (goods_index)</option>
    </select>
  </div>
  <p class="mini nota">O mesmo <code>data\{arquivo}</code> vai ao cliente e ao servidor (item <code>ambos</code>): a posição de cada
    oferta é o <b>goods_index</b> da compra. Publicar manda os dois juntos; o launcher atualiza o cliente.
    {#if estudio.versao !== "1.2.6"}A ordem "do jogo" agrupa pelo selo da 1ª opção com preço (`DlgQShop.cpp:170-232`); o
      cliente ainda esconde ofertas pelo nível do jogador e pelo NPC que abriu a loja (`ShouldIgnore`, :847-851).{:else}No
      1.2.6 a ordem "do jogo" usa a regra do cliente 1.5.5 (não conferida no binário 1.2.6).{/if}</p>
</section>

{#if erro}
  <p class="caixa erro">{erro}</p>
{:else if !loja}
  <p class="suave">Carregando a loja…</p>
{:else}
  <div class="tres">
    <aside class="coluna esquerda">
      <ArvoreDeAbas {arquivo} categorias={loja.categorias} {contagem} {temas} bind:filtro editavel={!!estudio.rascunho} aoMudar={() => carregar()} />
    </aside>

    <section class="coluna meio">
      <div class="linha cabeca-vitrine">
        <strong>{filtro.aba === null ? "Todas as abas" : loja.categorias[filtro.aba]?.nome}{#if filtro.aba !== null && filtro.sub >= 0} › {loja.categorias[filtro.aba]?.subcategorias[filtro.sub]}{/if}</strong>
        <span class="mini">{vitrine.length} ofertas</span>
        <button class="botao primario pequeno direita" type="button" disabled={!estudio.rascunho || filtro.aba === null}
          title={filtro.aba === null ? "Escolha uma aba à esquerda" : "Oferta nova no fim, nesta aba e subaba"} onclick={() => (adicionando = true)}>
          <Icone nome="mais" /> Nova oferta</button>
      </div>
      <div class="grade-vitrine">
        {#each vitrine.slice(0, limite) as o (o.indice)}
          {@const c = loja.conferencias[String(o.indice)]}
          <CartaoDaOferta oferta={o} escolhida={escolhida === o.indice} problemas={c?.erros.length ?? 0} avisos={c?.avisos.length ?? 0}
            aoEscolher={() => (escolhida = o.indice)} />
        {/each}
      </div>
      {#if vitrine.length > limite}
        <button class="botao mais" type="button" onclick={() => (limite += 120)}>Mostrar mais ({vitrine.length - limite})</button>
      {/if}
      {#if !vitrine.length}<p class="suave">Nenhuma oferta com esses filtros.</p>{/if}
    </section>

    <aside class="coluna direita-col">
      {#if linha && escolhida !== null}
        {#key `${arquivo}/${escolhida}`}
          <PainelDaOferta {arquivo} formato={loja.formato} indice={escolhida} total={loja.ofertas.length} {linha}
            categorias={loja.categorias} aoMudar={(i) => carregar(i)} />
        {/key}
      {:else}
        <p class="suave">Escolha uma oferta na vitrine.</p>
        <p class="mini">Os cartões mostram o que o jogador vê: o ícone do item (passe o mouse para a dica), o nome da oferta,
          o preço da 1ª opção com preço em gold (o arquivo guarda centavos: 100 = 1 gold, `EC_GameUIMan.cpp:7951`) e o
          selo.</p>
      {/if}
    </aside>
  </div>
{/if}

{#if adicionando}
  <SeletorDeEspaco titulo="Item da oferta nova" espaco="ESSENCE" aoEscolher={adicionar} aoFechar={() => (adicionando = false)} />
{/if}

<style>
  .topo-loja { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
  h1 { margin: 0; font-size: 20px; display: inline-flex; align-items: center; gap: 8px; }
  .filtros .busca { min-width: 260px; flex: 1; }
  .filtros label { display: inline-flex; align-items: center; gap: 4px; }
  .nota { margin: 0; }
  .tres { display: grid; grid-template-columns: 260px minmax(0, 1fr) 440px; gap: 14px; align-items: start; }
  .coluna { min-width: 0; }
  .esquerda, .direita-col { position: sticky; top: 64px; max-height: calc(100vh - 80px); overflow: auto; }
  .direita-col { background: var(--superficie); border: 1px solid var(--borda); border-radius: var(--raio-g); padding: 12px 14px; }
  .cabeca-vitrine { margin-bottom: 8px; }
  .direita { margin-left: auto; }
  .grade-vitrine { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 10px; }
  .mais { margin: 12px auto; display: flex; }
  @media (max-width: 1200px) {
    .tres { grid-template-columns: 220px minmax(0, 1fr); }
    .direita-col { grid-column: 1 / -1; position: static; max-height: none; }
  }
  @media (max-width: 760px) {
    .tres { grid-template-columns: 1fr; }
    .esquerda { position: static; max-height: none; }
  }
</style>
