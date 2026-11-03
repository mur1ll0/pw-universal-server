<script lang="ts">
  /* Abas da G-Shop como o cliente as monta (`CDlgQShop`): 8 abas principais — o carregador lê exatamente 8
     (`globaldataman.cpp:643`) e o diálogo tem `Btn_MainType0..7` —, com o texto do `szName` do arquivo
     (`OnChangeLayoutEnd`, `DlgQShop.cpp:1287`) e a imagem de cada botão no arquivo de interface; subabas com o nome do
     arquivo (`ShowMainType`, :669) até o número de `Btn_SubType` do diálogo (8 nos dois clientes), mais o "todos"
     (`Btn_SubType-1`, :806-822). */
  import { avisar } from "../lib/estado.svelte";
  import { cabeEmWchar, mudarCategoria, trocarImagemDaAba, urlDaImagem, type FiltroDaVitrine, type TemaDasAbas } from "../lib/gshop";
  import Icone from "./Icone.svelte";

  let { arquivo, categorias, contagem, temas, filtro = $bindable(), editavel, aoMudar }: {
    arquivo: string; categorias: { nome: string; subcategorias: string[] }[]; contagem: Map<string, number>;
    temas: TemaDasAbas[]; filtro: FiltroDaVitrine; editavel: boolean; aoMudar: () => Promise<void> | void;
  } = $props();

  const botoesDeSubaba = $derived(temas.length ? Math.min(...temas.map((t) => t.botoes_de_subaba)) : 8);
  let renomeando = $state<{ c: number; s: number | null; nome: string } | null>(null);
  let versaoDasImagens = $state(0);

  async function pedir(p: Parameters<typeof mudarCategoria>[1], ok: string) {
    try { await mudarCategoria(arquivo, p); avisar("ok", ok); await aoMudar(); } catch (e) { avisar("erro", (e as Error).message); }
  }
  async function confirmarNome() {
    if (!renomeando) return;
    const { c, s, nome } = renomeando;
    if (!nome.trim()) { avisar("aviso", "O nome não pode ficar vazio."); return; }
    if (!cabeEmWchar(nome, 64)) { avisar("aviso", "No máximo 63 caracteres (wchar[64])."); return; }
    renomeando = null;
    await pedir({ acao: "renomear", categoria: c, ...(s === null ? {} : { sub: s }), nome }, "Nome gravado no rascunho.");
  }
  async function novaSub(c: number) {
    const nome = prompt("Nome da subaba nova (até 63 caracteres):");
    if (!nome?.trim()) return;
    if (!cabeEmWchar(nome, 64)) { avisar("aviso", "No máximo 63 caracteres."); return; }
    await pedir({ acao: "nova", categoria: c, nome: nome.trim() }, "Subaba criada no fim (nenhuma oferta mudou).");
  }
  async function trocarImagem(e: Event, aba: number, tema: number, estado: "normal" | "selecionada") {
    const f = (e.target as HTMLInputElement).files?.[0];
    (e.target as HTMLInputElement).value = "";
    if (!f) return;
    try {
      await trocarImagemDaAba(arquivo, aba, tema, estado, f);
      avisar("ok", "Imagem da aba gravada no rascunho (no formato do cliente).");
      versaoDasImagens = Date.now();
      await aoMudar();
    } catch (erro) { avisar("erro", (erro as Error).message); }
  }
  const imagemDaAba = (c: number) => temas[0]?.abas[c]?.imagens.normal;
</script>

<nav class="arvore" aria-label="Abas da loja">
  <button type="button" class="item todas" class:ativo={filtro.aba === null} onclick={() => { filtro.aba = null; filtro.sub = -1; }}>
    Todas as abas <span class="mini">{[...contagem.entries()].filter(([k]) => !k.includes("/")).reduce((a, [, n]) => a + n, 0)}</span>
  </button>
  {#each categorias as cat, c}
    {@const img = imagemDaAba(c)}
    <div class="aba" class:ativa={filtro.aba === c}>
      <div class="linha-aba">
        <button type="button" class="item principal" class:ativo={filtro.aba === c && filtro.sub === -1} onclick={() => { filtro.aba = c; filtro.sub = -1; }}>
          {#if img?.existe && !temas[0]?.imagem_compartilhada}<img src={urlDaImagem(img.caminho, versaoDasImagens)} alt="" class="mini-img" />{/if}
          <span class="n">{c}</span> {cat.nome || "(sem nome)"} <span class="mini">{contagem.get(`${c}`) ?? 0}</span>
        </button>
        <button class="botao fantasma pequeno" type="button" disabled={!editavel} title="Renomear (szName da aba, wchar[64])"
          onclick={() => (renomeando = { c, s: null, nome: cat.nome })}><Icone nome="editar" /></button>
      </div>
      {#if filtro.aba === c}
        <ul>
          {#each cat.subcategorias as sub, s}
            <li class="linha-aba">
              <button type="button" class="item" class:ativo={filtro.sub === s} class:sem-botao={s >= botoesDeSubaba}
                title={s >= botoesDeSubaba ? `O diálogo só tem ${botoesDeSubaba} botões de subaba: esta não aparece no jogo (as ofertas dela só no "todos")` : ""}
                onclick={() => (filtro.sub = s)}>
                <span class="n">{s}</span> {sub || "(sem nome)"} <span class="mini">{contagem.get(`${c}/${s}`) ?? 0}</span>
                {#if s >= botoesDeSubaba}<span class="mini erro">sem botão</span>{/if}
              </button>
              <button class="botao fantasma pequeno" type="button" disabled={!editavel} title="Renomear subaba"
                onclick={() => (renomeando = { c, s, nome: sub })}><Icone nome="editar" /></button>
            </li>
          {/each}
          <li class="acoes">
            <button class="botao pequeno" type="button" disabled={!editavel || cat.subcategorias.length >= botoesDeSubaba}
              title={cat.subcategorias.length >= botoesDeSubaba ? `O diálogo da loja tem ${botoesDeSubaba} botões de subaba` : "Subaba nova no fim"}
              onclick={() => novaSub(c)}><Icone nome="mais" /> Subaba</button>
            {#if cat.subcategorias.length}
              {@const ultima = cat.subcategorias.length - 1}
              <button class="botao pequeno perigo" type="button" disabled={!editavel || (contagem.get(`${c}/${ultima}`) ?? 0) > 0}
                title={(contagem.get(`${c}/${ultima}`) ?? 0) > 0 ? "A última subaba tem ofertas: mova-as antes" : "Tira a última subaba (só a última: as outras são índices)"}
                onclick={() => pedir({ acao: "remover", categoria: c }, "Última subaba removida.")}><Icone nome="lixeira" /> Última</button>
            {/if}
          </li>
        </ul>
        <!-- imagens da aba -->
        {#each temas as t}
          {@const a = t.abas[c]}
          {#if a}
            <div class="imagens">
              <span class="mini">Tema {t.tema} · <code>{t.dialogo}</code></span>
              {#if t.imagem_compartilhada}
                <p class="mini">Os botões de aba deste diálogo usam todos a mesma imagem ({a.imagens.normal?.caminho}): não há imagem por aba para trocar.</p>
              {:else}
                {#each [["normal", "solta"], ["selecionada", "apertada"]] as [estado, rotulo]}
                  {@const im = a.imagens[estado as "normal" | "selecionada"]}
                  {#if im}
                    <div class="imagem">
                      {#if im.existe}<img src={urlDaImagem(im.caminho, versaoDasImagens)} alt="Imagem {rotulo} da aba {c}" />{:else}<span class="mini erro">não existe no cliente</span>{/if}
                      <span class="mini">{rotulo}{#if im.medida} · {im.medida.largura}×{im.medida.altura}{/if}</span>
                      <span class="linha">
                        {#if im.existe}<a class="botao pequeno" href={urlDaImagem(im.caminho, versaoDasImagens)} download="aba{c}_{estado}.png" title="Baixar PNG para editar">PNG</a>{/if}
                        <label class="botao pequeno" class:desligado={!editavel || !im.existe} title={im.medida ? `PNG de ${im.medida.largura}×${im.medida.altura}` : ""}>
                          Trocar<input type="file" accept="image/png" hidden disabled={!editavel || !im.existe}
                            onchange={(e) => trocarImagem(e, c, t.tema, estado as "normal" | "selecionada")} />
                        </label>
                      </span>
                    </div>
                  {/if}
                {/each}
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  {/each}
  <details class="nota">
    <summary class="mini">Por que não dá para criar aba nem reordenar?</summary>
    <p class="mini">O cliente lê <b>exatamente 8</b> abas do arquivo (`globaldataman.cpp:643`) e o diálogo tem 8 botões
      (`Btn_MainType0..7`). Aba e subaba são <b>índices</b> (`main_type`/`sub_type`, `globaldataman.h:103-104`): reordenar
      não muda o goods_index, mas quebraria o que aponta para o número — o `configs\qshopitem.ini` [Category] (pares
      aba/subaba fixos, `EC_UIConfigs.cpp:855-870`), a aba 5 da loja VIP (`DlgQShop.cpp:1705`) e a imagem de cada botão
      pelo número. Por isso: renomear, subaba nova no fim e remover só a última vazia.</p>
  </details>
</nav>

{#if renomeando}
  {@const r = renomeando}
  <div class="fundo" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) renomeando = null; }}>
    <form class="caixa renomear" onsubmit={(e) => { e.preventDefault(); confirmarNome(); }}>
      <h3>{r.s === null ? `Aba ${r.c}` : `Subaba ${r.c}/${r.s}`}</h3>
      <input class="campo" maxlength="63" bind:value={r.nome} aria-label="Nome" />
      <span class="mini">{[...r.nome].length}/63 · o cliente mostra este texto no botão</span>
      <div class="linha"><button class="botao" type="button" onclick={() => (renomeando = null)}>Cancelar</button><button class="botao primario" type="submit">Gravar</button></div>
    </form>
  </div>
{/if}

<style>
  .arvore { display: flex; flex-direction: column; gap: 4px; }
  .item { display: flex; align-items: center; gap: 6px; width: 100%; text-align: left; padding: 6px 8px; border-radius: 8px; border: 1px solid transparent;
    background: transparent; color: var(--texto); font: inherit; font-size: 13px; cursor: pointer; }
  .item:hover { background: var(--superficie-2); }
  .item.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .item .mini { margin-left: auto; }
  .item.sem-botao { opacity: .7; }
  .principal { font-weight: 600; }
  .n { color: var(--texto-3); font-variant-numeric: tabular-nums; width: 14px; }
  .linha-aba { display: flex; align-items: center; gap: 2px; }
  .aba { border-radius: 10px; }
  .aba.ativa { background: var(--superficie); border: 1px solid var(--borda); padding: 4px; }
  ul { list-style: none; margin: 2px 0 4px 14px; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .acoes { display: flex; gap: 6px; margin-top: 4px; }
  .mini-img { width: 22px; height: 22px; object-fit: cover; object-position: left; border-radius: 4px; }
  .imagens { display: flex; flex-wrap: wrap; gap: 8px; padding: 6px 4px; border-top: 1px solid var(--borda); }
  .imagens > .mini { width: 100%; }
  .imagem { display: flex; flex-direction: column; gap: 4px; align-items: flex-start; }
  .imagem img { image-rendering: pixelated; max-width: 120px; background: repeating-conic-gradient(#333 0 25%, #222 0 50%) 0 0 / 10px 10px; border-radius: 4px; }
  .desligado { opacity: .5; pointer-events: none; }
  .nota { margin-top: 8px; }
  .fundo { position: fixed; inset: 0; background: rgba(0, 0, 0, .5); display: grid; place-items: center; z-index: 60; }
  .renomear { display: flex; flex-direction: column; gap: 8px; width: min(380px, calc(100vw - 32px)); }
</style>
