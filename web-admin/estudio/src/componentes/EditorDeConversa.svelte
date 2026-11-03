<script lang="ts">
  /* B392: editor de conversa — fluxo (janelas em árvore), editor da janela (texto com cor e botões) e prévia como o
     jogo, em que se clica nos botões para navegar. Serve ao TALK_PROC do elements (estrutura livre: janelas novas,
     remover, reordenar, trocar destino) e aos diálogos de missão (`estruturaLivre` falso: o tasks.data só aceita editar
     textos, acrescentar botão — para uma função ou para uma janela nova — e remover botão, `tasks_editavel`, B266/B270).
     Lógica e validação em `lib/conversa.ts` (as regras do servidor). */
  import { arvore, copiar, destino, ehFuncao, FUNCOES_COM_MISSAO, novaJanela, removerJanela, TALK_EXIT, TALK_RETURN, TEXTO_DA_OPCAO,
    validar, type ConversaModelo, type Funcao, type JanelaModelo, type OpcaoModelo } from "../lib/conversa";
  import { semCodigos } from "../lib/marcacao";
  import EditorDeTexto from "./EditorDeTexto.svelte";
  import Icone from "./Icone.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { modelo, funcoes, aoSalvar, editavel = true, estruturaLivre = true, nomeNaPrevia = "NPC", comTitulo = true, rotuloDoSalvar = "Gravar conversa" }:
    { modelo: ConversaModelo; funcoes: Funcao[]; aoSalvar: (c: ConversaModelo, novas: Set<OpcaoModelo>) => Promise<void>; editavel?: boolean;
      estruturaLivre?: boolean; nomeNaPrevia?: string; comTitulo?: boolean; rotuloDoSalvar?: string } = $props();

  let c = $state<ConversaModelo>(copiar(modelo));
  let original = JSON.stringify(modelo);
  let selecionada = $state<number>(modelo.janelas[0]?.id ?? 0);
  let removendo = $state<number | null>(null);
  let gravando = $state(false);
  let acaoDaPrevia = $state("");
  /** Botões criados nesta edição (no modo de missão só estes podem ter o destino escolhido). */
  let novas = $state(new Set<OpcaoModelo>());

  const lista = $derived(arvore(c));
  const janela = $derived(c.janelas.find((j) => j.id === selecionada) ?? c.janelas[0]);
  const problemas = $derived(validar(c, funcoes));
  const sujo = $derived(JSON.stringify(c) !== original);
  const funcoesPorNome = $derived([...funcoes].sort((a, b) => a.rotulo.localeCompare(b.rotulo, "pt-BR")));

  function selecionar(id: number) { selecionada = id; acaoDaPrevia = ""; removendo = null; }

  function valorDoDestino(o: OpcaoModelo) { return ehFuncao(o.id) ? `f:${o.id >>> 0}` : `j:${o.id}`; }
  function mudarDestino(j: JanelaModelo, o: OpcaoModelo, v: string) {
    if (v === "nova") {
      const id = novaJanela(c, j.id, "");
      o.id = id;
      o.param = 0;
      // o botão "Até logo" da janela nova também é novo (no diálogo de missão vira uma operação própria)
      for (const x of c.janelas.find((w) => w.id === id)?.opcoes ?? []) novas.add(x);
      novas = new Set(novas);
      selecionar(id);
      return;
    }
    const [t, n] = v.split(":");
    o.id = t === "f" ? Number(n) : Number(n);
    if (t === "j") o.param = 0;
  }
  function novoBotao(j: JanelaModelo) {
    j.opcoes.push({ id: TALK_EXIT, texto: "Até logo", param: 0 });
    // o objeto guardado é o do estado (proxy), o mesmo que o `each` entrega
    novas.add(j.opcoes[j.opcoes.length - 1]);
    novas = new Set(novas);
  }
  function mover(j: JanelaModelo, k: number, d: number) {
    const t = k + d;
    if (t < 0 || t >= j.opcoes.length) return;
    [j.opcoes[k], j.opcoes[t]] = [j.opcoes[t], j.opcoes[k]];
  }
  function tirarBotao(j: JanelaModelo, k: number) {
    const [o] = j.opcoes.splice(k, 1);
    novas.delete(o);
    novas = new Set(novas);
    // No diálogo de missão, tirar o botão que abre janela tira a janela e as filhas (`tasks_editavel::remover_opcao`).
    if (!estruturaLivre && !ehFuncao(o.id)) removerJanela(c, o.id);
  }
  function janelaFilha() { if (janela) selecionar(novaJanela(c, janela.id, "")); }
  function janelaRaiz() {
    const id = novaJanela(c, -1, "");
    for (const x of c.janelas.find((w) => w.id === id)?.opcoes ?? []) novas.add(x);
    novas = new Set(novas);
    selecionar(id);
  }
  function remover(id: number) {
    removerJanela(c, id);
    removendo = null;
    selecionar(c.janelas[0]?.id ?? 0);
  }

  /** A prévia: clicar num botão faz o que o jogo faria. */
  function clicar(o: OpcaoModelo) {
    if (!ehFuncao(o.id)) { selecionar(o.id); return; }
    if ((o.id >>> 0) === TALK_EXIT) { acaoDaPrevia = "Fecha a janela."; return; }
    if ((o.id >>> 0) === TALK_RETURN && janela && janela.id_pai !== -1) { selecionar(janela.id_pai); return; }
    acaoDaPrevia = `Abre: ${destino(o, funcoes)}${o.param ? ` (parâmetro ${o.param})` : ""}.`;
  }

  async function gravar() {
    if (problemas.length) return;
    gravando = true;
    try {
      await aoSalvar(copiar(c), novas);
      original = JSON.stringify(c);
      novas = new Set();
      c = copiar(c); // reavalia `sujo`
    } finally { gravando = false; }
  }
  function descartar() { c = copiar(modelo); original = JSON.stringify(modelo); novas = new Set(); selecionar(modelo.janelas[0]?.id ?? 0); }
</script>

<div class="conversa">
  <div class="barra">
    {#if comTitulo}
      <label class="titulo"><span class="mini">Nome da conversa (só para achar; o jogador não vê)</span>
        <input class="campo" value={c.titulo} maxlength={TEXTO_DA_OPCAO} disabled={!editavel} oninput={(e) => (c.titulo = (e.target as HTMLInputElement).value)} /></label>
    {/if}
    <span class="espaco"></span>
    {#if problemas.length}<span class="erro mini">{problemas.length} problema(s) — corrija para gravar</span>{:else if sujo}<span class="mini">alterações por gravar</span>{/if}
    <button class="botao" type="button" disabled={!sujo || gravando} onclick={descartar}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || !sujo || gravando || problemas.length > 0} onclick={gravar}><Icone nome="check" /> {rotuloDoSalvar}</button>
  </div>
  {#if problemas.length}
    <ul class="problemas">{#each problemas.slice(0, 6) as p, k (k)}<li>{p}</li>{/each}{#if problemas.length > 6}<li>… e mais {problemas.length - 6}</li>{/if}</ul>
  {/if}

  <div class="tres">
    <!-- 1. Fluxo -->
    <section class="caixa fluxo">
      <h3>Fluxo</h3>
      <ol>
        {#each lista as { janela: j, nivel } (j.id)}
          <li style:margin-left="{nivel * 14}px">
            <button type="button" class="no" class:sel={j.id === janela?.id} onclick={() => selecionar(j.id)}>
              <span class="tp">{j.id_pai === -1 ? "raiz" : "janela"} {j.id} · {j.opcoes.length} botão(ões)</span>
              <span class="resumo">{semCodigos(j.texto).slice(0, 60) || "(sem texto)"}</span>
            </button>
          </li>
        {/each}
      </ol>
      {#if estruturaLivre || !c.janelas.length}
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={janelaRaiz}><Icone nome="mais" tamanho={16} /> Janela raiz</button>
      {/if}
    </section>

    <!-- 2. Editor da janela -->
    <section class="caixa editor">
      {#if janela}
        <div class="cab">
          <h3>Janela {janela.id}</h3>
          {#if estruturaLivre}
            <label class="mini pai">vem de
              <select class="campo" disabled={!editavel} value={String(janela.id_pai)} onchange={(e) => (janela.id_pai = Number((e.target as HTMLSelectElement).value))}>
                <option value="-1">— raiz —</option>
                {#each c.janelas.filter((x) => x.id !== janela.id) as x (x.id)}<option value={String(x.id)}>janela {x.id}</option>{/each}
              </select></label>
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={janelaFilha}><Icone nome="mais" tamanho={16} /> Janela filha</button>
            {#if removendo === janela.id}
              <button class="botao pequeno perigo" type="button" onclick={() => remover(janela.id)}>Confirmar: remover com as filhas</button>
              <button class="botao pequeno" type="button" onclick={() => (removendo = null)}>Cancelar</button>
            {:else}
              <button class="botao pequeno fantasma" type="button" disabled={!editavel || c.janelas.length < 2} onclick={() => (removendo = janela.id)} aria-label="Remover janela"><Icone nome="lixeira" tamanho={16} /></button>
            {/if}
          {/if}
        </div>
        <EditorDeTexto rotulo="Texto da janela" valor={janela.texto} limite={0} linhas={4} desabilitado={!editavel} previa={false}
          aoMudar={(v) => (janela.texto = v)} />

        <h4>Botões</h4>
        <ol class="opcoes">
          {#each janela.opcoes as o, k (o)}
            {@const livre = estruturaLivre || novas.has(o)}
            <li>
              <div class="linha-opcao">
                <span class="n">{k + 1}</span>
                <div class="texto-opcao">
                  <EditorDeTexto rotulo="" valor={o.texto} limite={TEXTO_DA_OPCAO} umaLinha previa={false} desabilitado={!editavel} aoMudar={(v) => (o.texto = v)} />
                </div>
              </div>
              <div class="linha-opcao">
                <span class="n"></span>
                <select class="campo destino" disabled={!editavel || !livre} value={valorDoDestino(o)} aria-label="O que o botão faz"
                  onchange={(e) => mudarDestino(janela, o, (e.target as HTMLSelectElement).value)}>
                  <optgroup label="Ir para outra janela">
                    <!-- no diálogo de missão, botão novo só abre janela nova (o arquivo não liga botão a janela existente) -->
                    {#each c.janelas.filter((x) => x.id !== janela.id && (estruturaLivre || !livre || x.id === o.id)) as x (x.id)}<option value="j:{x.id}">janela {x.id} — {semCodigos(x.texto).slice(0, 40) || "(sem texto)"}</option>{/each}
                    <option value="nova">+ janela nova (filha desta)</option>
                  </optgroup>
                  <optgroup label="Função do jogo">
                    {#each funcoesPorNome as f (f.codigo)}<option value="f:{f.codigo}">{f.rotulo}</option>{/each}
                  </optgroup>
                </select>
                {#if ehFuncao(o.id)}
                  <label class="mini param" title={FUNCOES_COM_MISSAO.has(o.id >>> 0) ? "id da missão" : "parâmetro da função"}>
                    {FUNCOES_COM_MISSAO.has(o.id >>> 0) ? "missão" : "param."}
                    <input class="campo" type="number" min="0" step="1" value={o.param} disabled={!editavel || !livre}
                      oninput={(e) => (o.param = Math.max(0, Number((e.target as HTMLInputElement).value) || 0))} /></label>
                {/if}
                {#if estruturaLivre}
                  <button class="botao pequeno fantasma" type="button" disabled={!editavel || k === 0} onclick={() => mover(janela, k, -1)} aria-label="Subir"><Icone nome="cima" tamanho={16} /></button>
                  <button class="botao pequeno fantasma" type="button" disabled={!editavel || k === janela.opcoes.length - 1} onclick={() => mover(janela, k, 1)} aria-label="Descer"><Icone nome="baixo" tamanho={16} /></button>
                {/if}
                <button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => tirarBotao(janela, k)} aria-label="Tirar botão"><Icone nome="lixeira" tamanho={16} /></button>
              </div>
            </li>
          {/each}
        </ol>
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => novoBotao(janela)}><Icone nome="mais" tamanho={16} /> Botão</button>
        {#if !estruturaLivre}
          <p class="mini">No diálogo de missão o arquivo só aceita: mudar textos, acrescentar botão (para uma função ou para uma janela nova) e tirar botão.</p>
        {/if}
      {:else}
        <p class="suave">Sem janela.</p>
      {/if}
    </section>

    <!-- 3. Prévia -->
    <section class="caixa previa">
      <h3>Como o jogador vê</h3>
      {#if janela}
        <div class="jogo">
          <div class="cab-jogo"><span>{nomeNaPrevia}</span><span aria-hidden="true">✕</span></div>
          <div class="texto"><TextoDoJogo texto={janela.texto} /></div>
          <ul>
            {#each janela.opcoes as o, k (k)}
              <li><button type="button" onclick={() => clicar(o)} title={destino(o, funcoes)}><TextoDoJogo texto={o.texto} padrao="#f6dc9a" /></button></li>
            {/each}
          </ul>
        </div>
        <p class="mini">{acaoDaPrevia || "Clique nos botões para navegar como no jogo."}</p>
      {/if}
    </section>
  </div>
</div>

<style>
  .conversa { display: flex; flex-direction: column; gap: 12px; }
  .barra { display: flex; align-items: flex-end; gap: 8px; flex-wrap: wrap; }
  .titulo { display: flex; flex-direction: column; gap: 4px; min-width: 260px; }
  .espaco { flex: 1; }
  .problemas { margin: 0; padding: 8px 12px 8px 28px; border: 1px solid var(--vermelho); border-radius: 10px; background: var(--vermelho-suave); color: var(--texto-erro); font-size: 12px; }
  .tres { display: grid; grid-template-columns: minmax(200px, 260px) minmax(0, 1fr) minmax(260px, 320px); gap: 12px; align-items: start; }
  .fluxo ol { list-style: none; margin: 0 0 10px; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 560px; overflow: auto; }
  .no { width: 100%; text-align: left; display: flex; flex-direction: column; gap: 2px; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda-forte);
    background: var(--fundo-2); color: var(--texto); font: inherit; font-size: 12px; cursor: pointer; }
  .no.sel { border-color: var(--ouro); background: var(--ouro-suave); }
  .tp { font-size: 10px; text-transform: uppercase; letter-spacing: .05em; color: var(--texto-3); }
  .resumo { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .editor { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .cab { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .cab h3 { margin: 0; flex: 1; }
  .pai { display: flex; align-items: center; gap: 6px; }
  .pai select { height: 28px; }
  h4 { margin: 8px 0 0; font-size: 13px; color: var(--texto-2); }
  .opcoes { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .opcoes > li { display: flex; flex-direction: column; gap: 4px; padding: 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .linha-opcao { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .n { width: 18px; text-align: center; font-size: 12px; color: var(--texto-3); }
  .texto-opcao { flex: 1; min-width: 0; }
  .destino { flex: 1; min-width: 180px; height: 30px; }
  .param { display: flex; align-items: center; gap: 4px; }
  .param input { width: 90px; height: 30px; }
  .previa { position: sticky; top: 70px; }
  .jogo { background: #0d1117; border: 2px solid #5a4a2a; border-radius: 6px; color: #f1e6c8; font-size: 13px; overflow: hidden; }
  .cab-jogo { background: linear-gradient(#3a2f1a, #22190b); padding: 6px 10px; font-weight: 600; color: #f6dc9a; display: flex; justify-content: space-between; }
  .texto { padding: 10px 12px; min-height: 90px; line-height: 1.5; }
  .jogo ul { list-style: none; margin: 0; padding: 4px 8px 10px; display: flex; flex-direction: column; gap: 2px; }
  .jogo li button { width: 100%; text-align: left; background: none; border: 0; padding: 3px 6px; border-radius: 4px; font: inherit; cursor: pointer; }
  .jogo li button:hover { background: rgba(246, 220, 154, .12); }
  .jogo li button::before { content: "◆ "; color: #b8923d; font-size: 10px; }
  @media (max-width: 1100px) { .tres { grid-template-columns: 1fr; } .previa { position: static; } }
</style>
