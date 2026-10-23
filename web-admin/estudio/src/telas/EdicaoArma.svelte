<script lang="ts">
  /* Edição de Arma (`WEAPON_ESSENCE`, `ExpTypes.h:177-305`), B366. Os campos são os aprovados pelo Murillo em
     2026-10-23 (`ESTUDIO_DO_PAINEL.md` §9); o que não existe no layout da versão (v156 / v7) não aparece, e o resto
     fica nos Editores. Gravar = `PUT tabelas/WEAPON_ESSENCE/{i}` com os campos mudados, no rascunho do alto. */
  import { onDestroy } from "svelte";
  import { buscarIcones, buscarNoEspaco, definirTexto, descreverEfeitos, ficha as lerFicha, linkDosEditores, listar, novoTexto, salvar, textoDaChave, type Campo, type Ficha, type Registro } from "../lib/api";
  import { BOXES_DA_ARMA, CHANCES_SIMPLES, DISTANCIAS, ITENS_LIGADOS, LISTAS_DA_ARMA, REGRAS_DO_ITEM, classesDaVersao, subcategoriaPorTabela } from "../lib/catalogo";
  import { acharLista, gravarLista, lerLista, paraGravar, type Cartao, type Lista } from "../lib/chances";
  import { NOME_DO_CANAL, avisar, estudio, ir, parametros, rota, type MensagemDeItem } from "../lib/estado.svelte";
  import ChancesSimples from "../componentes/ChancesSimples.svelte";
  import Icone from "../componentes/Icone.svelte";
  import IconeItem from "../componentes/IconeItem.svelte";
  import ListaDeChances from "../componentes/ListaDeChances.svelte";
  import ModalCategoria from "../componentes/ModalCategoria.svelte";
  import ModalIcone from "../componentes/ModalIcone.svelte";
  import ModalMascara from "../componentes/ModalMascara.svelte";
  import Modal from "../componentes/Modal.svelte";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";
  import Visual3D from "../componentes/Visual3D.svelte";

  let { tabela, indice }: { tabela: string; indice: number } = $props();
  const sub = subcategoriaPorTabela(tabela);

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let nomes = $state<Record<string, string>>({}); // nome do registro citado por um campo (id → nome)
  let iconeHex = $state<string | null>(null);
  let descricao = $state("");
  let descricaoOriginal = $state("");
  let descricaoExiste = false;
  let gravando = $state(false);
  let modal = $state<"" | "icone" | "categoria" | "classes" | "regras" | "refino" | string>("");

  const editavel = $derived(!!estudio.rascunho);
  const existe = (n: string) => n in porNome;
  const num = (n: string) => Number(valores[n] ?? 0);

  async function carregar() {
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      const n: Record<string, string> = {};
      for (const c of x.campos) if (c.referencia?.nome) n[`${c.nome}`] = c.referencia.nome;
      nomes = n;
      iconeHex = x.icone;
      // O `levelup_addon` não está no índice de referências (`specs/estudio/referencias.json`): o nome sai do espaço ADDON.
      const refino = Number(originais.levelup_addon ?? 0);
      if (refino) buscarNoEspaco("ADDON", String(refino), 5)
        .then((r) => { const a = r.find((y) => y.id === refino); if (a) nomes.levelup_addon = a.nome; }).catch(() => {});
      f = x;
      await carregarDescricao(x.id ?? 0);
      if (rota.params.get("novo")) {
        for (const l of listas) Object.assign(valores, gravarLista(l.lista, []));
        avisar("aviso", "Registro novo: as listas de efeitos foram limpas na tela. Grave para confirmar.");
      }
    } catch (e) { erro = (e as Error).message; }
  }

  async function carregarDescricao(id: number) {
    try {
      const t = await textoDaChave("item_ext_desc.txt", id);
      descricaoExiste = t !== null;
      descricaoOriginal = t ?? "";
    } catch { descricaoOriginal = ""; descricaoExiste = false; }
    descricao = descricaoOriginal;
  }

  carregar();

  /* ---- descrição dos efeitos como o cliente mostra (B209: o GS dá o valor do EQUIPMENT_ADDON) ---- */
  let efeitosTexto = $state<Record<number, { texto: string; args: number[] | null }>>({});
  const pedidos = new Set<number>();
  $effect(() => {
    if (!f) return;
    const ids = [...listas.flatMap(({ lista }) => lerLista(lista, num).map((l) => l.id)), num("levelup_addon")]
      .filter((id) => id && !pedidos.has(id));
    if (!ids.length) return;
    ids.forEach((id) => pedidos.add(id));
    descreverEfeitos(ids, f.id ?? 1)
      .then((d) => { for (const e of d) efeitosTexto[e.id] = { texto: e.texto, args: e.args }; })
      .catch(() => ids.forEach((id) => pedidos.delete(id)));
  });

  /* ---- tipo de munição exigido: `require_projectile` escolhe um `PROJECTILE_TYPE` (`ExpTypes.h:185`) ---- */
  let tiposDeMunicao = $state<{ id: number; nome: string }[]>([]);
  listar("PROJECTILE_TYPE", "", 0, 200)
    .then((r) => (tiposDeMunicao = r.registros.filter((x) => x.id != null).map((x) => ({ id: x.id!, nome: x.nome }))))
    .catch(() => {});
  const nomeDaMunicao = (id: number) => (id ? tiposDeMunicao.find((t) => t.id === id)?.nome ?? `id ${id} (não existe)` : "nenhuma (não exige)");

  /* ---- listas (id, chance) ---- */
  const listas = $derived.by(() => {
    const todos = Object.keys(porNome);
    return LISTAS_DA_ARMA.map((d) => ({ d, lista: acharLista(todos, d.prefixo, d.id, d.chance) }))
      .filter((x): x is { d: (typeof LISTAS_DA_ARMA)[number]; lista: Lista } => !!x.lista);
  });
  function cartoes(lista: Lista): Cartao[] {
    return lerLista(lista, num).map((l) => ({ ...l, nome: nomeDoEfeito(lista, l.id), descricao: efeitosTexto[l.id]?.texto || "" }));
  }
  function nomeDoEfeito(lista: Lista, id: number) {
    const campo = lista.ids.find((c) => num(c) === id);
    return (campo && nomes[campo]) || nomes[`efeito:${id}`] || "";
  }
  function mudarLista(lista: Lista, linhas: Cartao[]) {
    for (const l of linhas) if (l.nome) nomes[`efeito:${l.id}`] = l.nome;
    Object.assign(valores, gravarLista(lista, linhas));
  }

  /* ---- chances simples (furos, quantidade de efeitos) ---- */
  function simples(prefixo: string) {
    const r: string[] = [];
    for (let k = 0; existe(`${prefixo}${k}`); k++) r.push(`${prefixo}${k}`);
    return r;
  }
  function mudarSimples(campos: string[], pesos: number[]) {
    const g = paraGravar(pesos);
    campos.forEach((c, k) => (valores[c] = g[k]));
  }

  /* ---- gravar ---- */
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));

  function validar(): Record<string, number | string> | null {
    const campos: Record<string, number | string> = {};
    for (const n of mudados) {
      const c = porNome[n];
      const v = valores[n];
      if (c.tipo === "int32") {
        const x = Number(v);
        if (v === "" || !Number.isInteger(x) || x < -2147483648 || x > 2147483647) { avisar("aviso", `${c.rotulo || n}: número inteiro inválido.`); return null; }
        campos[n] = x;
      } else if (c.tipo === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x)) { avisar("aviso", `${c.rotulo || n}: número inválido.`); return null; }
        campos[n] = x;
      } else campos[n] = String(v);
    }
    return campos;
  }

  async function gravar() {
    if (!f) return;
    const campos = validar();
    if (!campos) return;
    const textoMudou = descricao !== descricaoOriginal;
    if (!Object.keys(campos).length && !textoMudou) { avisar("aviso", "Nada mudou."); return; }
    gravando = true;
    try {
      if (Object.keys(campos).length) {
        const r = await salvar(tabela, indice, campos);
        avisar("ok", r.igual_ao_publicado ? "Igual ao publicado: o elements saiu do rascunho." : `${r.resultado?.campos_mudados ?? Object.keys(campos).length} campo(s) gravados no rascunho.`);
      }
      if (textoMudou && f.id) {
        if (descricaoExiste) await definirTexto("item_ext_desc.txt", f.id, descricao);
        else await novoTexto("item_ext_desc.txt", f.id, descricao);
        avisar("ok", "Descrição gravada no rascunho (item_ext_desc.txt).");
      }
      const semNovo = new URLSearchParams(rota.params); semNovo.delete("novo");
      if (rota.params.get("novo")) history.replaceState(null, "", `#/${rota.caminho.join("/")}?${semNovo}`);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }
  function descartar() { valores = { ...originais }; descricao = descricaoOriginal; iconeHex = f?.icone ?? null; }

  /** O hex GBK do ícone escolhido, para a imagem (o atlas devolve nome e hex). */
  async function buscarIconeHex(nome: string) {
    try {
      const r = await buscarIcones(nome, 20);
      iconeHex = r.icones.find((i) => i.arquivo === nome)?.hex ?? null;
    } catch { iconeHex = null; }
  }

  /* ---- referências por nome ---- */
  async function resolverNome(campo: string, id: number, tabelaAlvo?: string) {
    if (!id) { delete nomes[campo]; return; }
    try {
      const r = (await buscarNoEspaco("ESSENCE", String(id), 5)).find((x) => x.id === id && (!tabelaAlvo || x.tabela === tabelaAlvo));
      nomes[campo] = r ? r.nome : `id ${id} (não existe)`;
      if (r) nomes[`icone:${campo}`] = r.icone ?? "";
    } catch { nomes[campo] = `id ${id}`; }
  }

  /* ---- item ligado: criar em outra aba (BroadcastChannel) ---- */
  const canal = typeof BroadcastChannel !== "undefined" ? new BroadcastChannel(NOME_DO_CANAL) : null;
  const esperando = new Map<string, string>(); // código de retorno → campo
  if (canal) canal.onmessage = (e: MessageEvent<MensagemDeItem>) => {
    const m = e.data;
    if (m?.tipo !== "item-criado" || !esperando.has(m.retorno)) return;
    const campo = esperando.get(m.retorno)!;
    esperando.delete(m.retorno);
    valores[campo] = m.id;
    nomes[campo] = m.nome;
    resolverNome(campo, m.id);
    avisar("ok", `Item ${m.id} criado na outra aba e escolhido aqui (grave para confirmar).`);
  };
  onDestroy(() => canal?.close());
  function criarEmOutraAba(campo: string) {
    const retorno = `${campo}-${Date.now()}`;
    esperando.set(retorno, campo);
    window.open(`${location.pathname}#/itens/novo?${parametros({ retorno })}`, "_blank");
    modal = "";
  }

  const classes = $derived(classesDaVersao(estudio.versao));
  const opcoesDeClasse = $derived(classes.map((n, k) => [1 << k, n] as [number, string]));
  const resumoDeClasses = $derived.by(() => {
    const m = num("character_combo_id");
    const sim = classes.filter((_, k) => m & (1 << k));
    return sim.length === classes.length ? "Todas as classes" : sim.length ? sim.join(", ") : "Nenhuma classe";
  });
  const resumoDeRegras = $derived(REGRAS_DO_ITEM.filter(([b]) => num("proc_type") & b).map(([, n]) => n));
  const modelos = $derived([["Mão direita", "file_model_right"], ["Mão esquerda", "file_model_left"], ["No chão", "file_matter"]]
    .filter(([, c]) => existe(c)).map(([rotulo, c]) => ({ rotulo, caminho: String(valores[c] ?? "") })));
  const alterado = (n: string) => valores[n] !== originais[n];
</script>

{#snippet campo(n: string)}
  {#if existe(n)}
    {@const c = porNome[n]}
    <label title={n}>
      <span>{c.rotulo || n}</span>
      {#if n === "require_projectile"}
        <button class="campo escolha" type="button" class:alterado={alterado(n)} title="Escolher em PROJECTILE_TYPE"
          onclick={() => (modal = "municao")}>{nomeDaMunicao(num(n))}</button>
      {:else if n === "short_range_mode"}
        <select class="campo" class:alterado={alterado(n)} disabled={!editavel} value={String(valores[n])}
          onchange={(e) => (valores[n] = Number((e.target as HTMLSelectElement).value))}>
          {#each DISTANCIAS as d, k (k)}<option value={String(k)}>{d}</option>{/each}
        </select>
      {:else if c.tipo === "int32" || c.tipo === "float"}
        <input class="campo" class:alterado={alterado(n)} type="number" step={c.tipo === "float" ? "any" : "1"} disabled={!editavel}
          value={valores[n]} oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
      {:else}
        <input class="campo" class:alterado={alterado(n)} disabled={!editavel} value={valores[n]} oninput={(e) => (valores[n] = (e.target as HTMLInputElement).value)} />
      {/if}
      {#if n !== "require_projectile" && c.referencia?.nome && valores[n] === originais[n]}<span class="mini">{c.referencia.nome}</span>{/if}
    </label>
  {/if}
{/snippet}

{#if erro}
  <p class="caixa erro">{erro}</p>
{:else if !f}
  <p class="suave">Carregando…</p>
{:else}
  <div class="barra">
    <button class="botao" type="button" onclick={() => ir("")}><Icone nome="voltar" /> Voltar</button>
    <span class="suave">{sub?.nome ?? tabela} · id {f.id} · registro {indice}</span>
    <a class="botao fantasma" href={linkDosEditores(tabela, indice)} target="_blank" rel="noopener" title="Abre os Editores em outra aba: realm {estudio.realm}, {tabela}, id {f.id}"><Icone nome="editores" /> Abrir nos Editores</a>
    <span class="espaco"></span>
    {#if mudados.length || descricao !== descricaoOriginal}<span class="mini">{mudados.length} campo(s){descricao !== descricaoOriginal ? " + descrição" : ""} por gravar</span>{/if}
    <button class="botao" type="button" disabled={gravando} onclick={descartar}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || gravando} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
  </div>

  <div class="layout">
    <div class="coluna">
      <!-- 1. Principal -->
      <section class="caixa principal">
        <button class="icone-grande" type="button" title="Trocar o ícone" onclick={() => (modal = "icone")}>
          <IconeItem hex={iconeHex} nome={String(valores.Name)} tamanho={64} />
        </button>
        <div class="dados">
          <input class="campo nome" class:alterado={alterado("Name")} disabled={!editavel} value={valores.Name}
            oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} aria-label="Nome" maxlength="32" />
          <div class="linha">
            <button class="botao pequeno" type="button" onclick={() => (modal = "categoria")} title="Categoria e subcategoria">
              {nomes.id_major_type ?? `id ${valores.id_major_type}`} › {nomes.id_sub_type ?? `id ${valores.id_sub_type}`}
            </button>
            <span class="mini">{sub?.nome}</span>
          </div>
          <label class="desc">
            <span class="mini">Descrição (item_ext_desc.txt do cliente; os códigos ^RRGGBB mudam a cor)</span>
            <textarea class="campo" rows="3" class:alterado={descricao !== descricaoOriginal} disabled={!editavel} bind:value={descricao}></textarea>
          </label>
        </div>
      </section>

      <!-- 2. Requisitos -->
      <section class="caixa">
        <h3>{BOXES_DA_ARMA[0].titulo}</h3>
        <div class="linha classes">
          <button class="botao pequeno" type="button" class:alterado={alterado("character_combo_id")} onclick={() => (modal = "classes")}>Classes</button>
          <span class="suave">{resumoDeClasses}</span>
        </div>
        <div class="grade-campos">{#each BOXES_DA_ARMA[0].campos as n (n)}{@render campo(n)}{/each}</div>
      </section>

      <!-- 3. Combate e 4. Comércio -->
      <section class="caixa">
        <h3>{BOXES_DA_ARMA[1].titulo}</h3>
        <div class="grade-campos">{#each BOXES_DA_ARMA[1].campos as n (n)}{@render campo(n)}{/each}</div>
      </section>
      <section class="caixa">
        <h3>{BOXES_DA_ARMA[2].titulo}</h3>
        <div class="grade-campos">{#each BOXES_DA_ARMA[2].campos as n (n)}{@render campo(n)}{/each}</div>
        {#if existe("proc_type")}
          <div class="linha regras">
            <button class="botao pequeno" type="button" class:alterado={alterado("proc_type")} onclick={() => (modal = "regras")}>Regras do item</button>
            <span class="mini">{resumoDeRegras.length ? resumoDeRegras.join(" · ") : "nenhuma restrição"}</span>
          </div>
        {/if}
      </section>

      <!-- 5. Durabilidade e fabricação -->
      <section class="caixa">
        <h3>{BOXES_DA_ARMA[3].titulo}</h3>
        <div class="grade-campos">{#each BOXES_DA_ARMA[3].campos as n (n)}{@render campo(n)}{/each}</div>
        <div class="ligados">
          {#each ITENS_LIGADOS.filter((x) => existe(x.campo)) as l (l.campo)}
            <div class="ligado">
              <span class="mini">{l.titulo}</span>
              <button class="cartao-item" type="button" class:alterado={alterado(l.campo)} onclick={() => (modal = `item:${l.campo}`)}>
                <IconeItem hex={nomes[`icone:${l.campo}`] ?? porNome[l.campo].referencia?.icone} nome={nomes[l.campo]} />
                <span>{num(l.campo) ? (nomes[l.campo] ?? `id ${num(l.campo)}`) : "nenhum"}</span>
                {#if num(l.campo)}<span class="mini">id {num(l.campo)}</span>{/if}
              </button>
              {#if existe(l.quantidade)}
                <label class="mini">quantidade <input class="campo qtd" class:alterado={alterado(l.quantidade)} type="number" step="1" min="0" disabled={!editavel}
                  value={valores[l.quantidade]} oninput={(e) => (valores[l.quantidade] = Number((e.target as HTMLInputElement).value))} /></label>
              {/if}
            </div>
          {/each}
          {#if existe("levelup_addon")}
            <div class="ligado">
              <span class="mini">Efeito do refino (levelup_addon)</span>
              <button class="cartao-item" type="button" class:alterado={alterado("levelup_addon")} onclick={() => (modal = "refino")}>
                <Icone nome="refino" />
                <span>{num("levelup_addon") ? (efeitosTexto[num("levelup_addon")]?.texto || nomes.levelup_addon || `efeito ${num("levelup_addon")}`) : "nenhum"}</span>
                {#if num("levelup_addon") && !efeitosTexto[num("levelup_addon")]?.texto && efeitosTexto[num("levelup_addon")]?.args?.length}
                  <span class="mini">sem texto no cliente · parâmetros {efeitosTexto[num("levelup_addon")]?.args?.join(", ")}</span>
                {/if}
                {#if num("levelup_addon")}<span class="mini">id {num("levelup_addon")}</span>{/if}
              </button>
              {#if existe("material_need")}
                <label class="mini">pedras por nível <input class="campo qtd" class:alterado={alterado("material_need")} type="number" step="1" min="0" disabled={!editavel}
                  value={valores.material_need} oninput={(e) => (valores.material_need = Number((e.target as HTMLInputElement).value))} /></label>
              {/if}
            </div>
          {/if}
        </div>
        <div class="simples">
          {#each CHANCES_SIMPLES as s (s.prefixo)}
            {@const cs = simples(s.prefixo)}
            {#if cs.length}
              <ChancesSimples titulo={s.titulo} rotulos={cs.map((_, k) => s.rotulo(k))} pesos={cs.map(num)} {editavel}
                aoMudar={(p) => mudarSimples(cs, p)} />
            {/if}
          {/each}
        </div>
      </section>
    </div>

    <!-- 6. Visual 3D -->
    <aside class="lateral"><Visual3D {modelos} /></aside>
  </div>

  <!-- 7. Listas de efeitos -->
  <div class="listas">
    {#each listas as { d, lista } (d.prefixo)}
      <div>
        <ListaDeChances titulo={d.titulo} explica={d.explica} espaco="ADDON" {tabela} maximo={lista.ids.length}
          linhas={cartoes(lista)} {editavel} aoMudar={(l) => mudarLista(lista, l)} />
        {#if d.chanceGeral && existe(d.chanceGeral)}
          <label class="mini geral">Chance geral de efeito único (0–1)
            <input class="campo qtd" class:alterado={alterado(d.chanceGeral)} type="number" step="any" min="0" max="1" disabled={!editavel}
              value={valores[d.chanceGeral]} oninput={(e) => (valores[d.chanceGeral!] = Number((e.target as HTMLInputElement).value))} /></label>
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#if f && modal === "icone"}
  <ModalIcone caminho={String(valores.file_icon ?? "")} aoFechar={() => (modal = "")}
    aoEscolher={(novo) => { valores.file_icon = novo; const nome = novo.slice(novo.lastIndexOf("\\") + 1).toLowerCase(); buscarIconeHex(nome); }} />
{:else if f && modal === "categoria"}
  <ModalCategoria maior={sub?.maior} menor={sub?.menor} idMaior={num("id_major_type")} idMenor={num("id_sub_type")} aoFechar={() => (modal = "")}
    aoEscolher={(a, b) => { valores.id_major_type = a; valores.id_sub_type = b; resolverNome("id_major_type", a, sub?.maior); resolverNome("id_sub_type", b, sub?.menor); }} />
{:else if f && modal === "classes"}
  <ModalMascara titulo="Classes que podem equipar" opcoes={opcoesDeClasse} valor={num("character_combo_id")} aoFechar={() => (modal = "")}
    explica="Bit 1 << classe (item_astrolabe.cpp:221); o cliente {estudio.versao} tem {classes.length} classes."
    aoAplicar={(v) => (valores.character_combo_id = v)} />
{:else if f && modal === "regras"}
  <ModalMascara titulo="Regras do item" opcoes={REGRAS_DO_ITEM} valor={num("proc_type")} aoFechar={() => (modal = "")}
    explica="ITEM_PROC_TYPE_* do servidor 1.5.5 (cgame/gs/item.h:309-326)." aoAplicar={(v) => (valores.proc_type = v)} />
{:else if f && modal === "municao"}
  <Modal titulo="Tipo de munição exigido (PROJECTILE_TYPE)" aoFechar={() => (modal = "")}>
    <p class="mini">A arma só atira a munição cujo tipo é este (ExpTypes.h:185); 0 = não exige munição.</p>
    <ul class="escolhas">
      {#each [{ id: 0, nome: "nenhuma (não exige)" }, ...tiposDeMunicao] as t (t.id)}
        <li><button type="button" class:ativo={num("require_projectile") === t.id} disabled={!editavel}
          onclick={() => { valores.require_projectile = t.id; modal = ""; }}>{t.nome} <span class="mini">id {t.id}</span></button></li>
      {/each}
    </ul>
    {#if !editavel}<p class="mini">Escolha um rascunho no alto para mudar.</p>{/if}
  </Modal>
{:else if f && modal === "refino"}
  <SeletorDeEspaco titulo="Efeito do refino" espaco="ADDON" aoFechar={() => (modal = "")}
    aoEscolher={(r: Registro) => { valores.levelup_addon = r.id ?? 0; nomes.levelup_addon = r.nome; }} />
{:else if f && modal.startsWith("item:")}
  {@const c = modal.slice(5)}
  <SeletorDeEspaco titulo={ITENS_LIGADOS.find((x) => x.campo === c)?.titulo ?? "Item"} espaco="ESSENCE" aoFechar={() => (modal = "")}
    aoCriar={editavel ? () => criarEmOutraAba(c) : undefined}
    aoEscolher={(r: Registro) => { valores[c] = r.id ?? 0; nomes[c] = r.nome; nomes[`icone:${c}`] = r.icone ?? ""; }} />
{/if}

<style>
  .barra { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; }
  .espaco { flex: 1; }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(320px, 440px); gap: 14px; align-items: start; }
  .coluna { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .lateral { position: sticky; top: 70px; }
  .principal { display: flex; gap: 16px; align-items: flex-start; }
  .icone-grande { padding: 6px; border-radius: 12px; border: 1px solid var(--borda-forte); background: var(--fundo-2); cursor: pointer; }
  .icone-grande:hover { border-color: var(--ouro); }
  .dados { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .nome { font-size: 18px; font-weight: 600; height: 40px; }
  .desc { display: flex; flex-direction: column; gap: 4px; }
  .classes, .regras { margin-bottom: 10px; }
  .regras { margin: 10px 0 0; }
  .ligados { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; margin-top: 12px; }
  .ligado { display: flex; flex-direction: column; gap: 4px; }
  .cartao-item { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 10px; border: 1px solid var(--borda); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .cartao-item:hover { border-color: var(--ouro); }
  .qtd { width: 100px; height: 28px; }
  .escolha { text-align: left; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .escolhas { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 50vh; overflow: auto; }
  .escolhas button { width: 100%; text-align: left; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; }
  .escolhas button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .simples { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 12px; margin-top: 14px; }
  .listas { display: grid; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); gap: 14px; margin-top: 14px; }
  .geral { display: flex; align-items: center; gap: 8px; margin-top: 6px; }
  :global(.alterado) { border-color: var(--ouro) !important; background: var(--ouro-suave) !important; }
  @media (max-width: 960px) {
    .layout { grid-template-columns: 1fr; }
    .lateral { position: static; }
    .listas { grid-template-columns: 1fr; }
  }
</style>
