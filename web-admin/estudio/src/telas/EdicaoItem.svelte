<script lang="ts">
  /* Edição de Item (as tabelas de item que não são Equipáveis, e as Minas): uma tela só, com boxes por tipo
     (`lib/itens.ts`). O campo que a tabela ou a versão (v156 / v7) não tem não aparece. Gravar = `PUT tabelas/{T}/{i}`
     com os campos mudados, no rascunho do alto (as mesmas rotas dos Editores), mais a descrição (`item_ext_desc.txt`). */
  import { onDestroy } from "svelte";
  import { buscarIcones, buscarNoEspaco, definirTexto, descreverEfeitos, ficha as lerFicha, habilidadePorId, linkDosEditores, minaDoGs,
    novoTexto, salvar, textoDaChave, usosDoItem, type Campo, type Ficha, type MinaDoGs, type Registro, type UsosDoItem } from "../lib/api";
  import { ITENS_LIGADOS, REGRAS_DO_ITEM, camposDaBox, rotuloGerado } from "../lib/catalogo";
  import { BITS_DA_PEDRA, BOXES_DO_ITEM, CAMPOS_PRINCIPAIS, CATEGORIAS_DE_REMEDIO, ROTULOS_DE_ITEM, chancesDoRefino, ondeAPedraServe,
    regraDoAmuleto, regraDoRemedio, subDeItemPorTabela, type Talisma } from "../lib/itens";
  import { NOME_DO_CANAL, avisar, estudio, ir, parametros, type MensagemDeItem } from "../lib/estado.svelte";
  import BoxDeRecurso from "../componentes/BoxDeRecurso.svelte";
  import Icone from "../componentes/Icone.svelte";
  import IconeItem from "../componentes/IconeItem.svelte";
  import ModalCategoria from "../componentes/ModalCategoria.svelte";
  import ModalIcone from "../componentes/ModalIcone.svelte";
  import ModalMascara from "../componentes/ModalMascara.svelte";
  import ModeloDoEquipavel from "../componentes/ModeloDoEquipavel.svelte";
  import Origens from "../componentes/Origens.svelte";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";
  import Visual3D from "../componentes/Visual3D.svelte";

  let { tabela, indice }: { tabela: string; indice: number } = $props();
  const sub = subDeItemPorTabela(tabela);
  const ehMina = tabela === "MINE_ESSENCE";

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let nomes = $state<Record<string, string>>({});
  let iconeHex = $state<string | null>(null);
  let descricao = $state("");
  let descricaoOriginal = $state("");
  let descricaoExiste = false;
  let gravando = $state(false);
  let modal = $state("");
  let usos = $state<UsosDoItem | null>(null);
  let erroDosUsos = $state("");
  let mina = $state<MinaDoGs | null>(null);

  const editavel = $derived(!!estudio.rascunho);
  const existe = (n: string) => n in porNome;
  const num = (n: string) => Number(valores[n] ?? 0);
  const alterado = (n: string) => valores[n] !== originais[n];

  async function carregar() {
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      const n: Record<string, string> = {};
      for (const c of x.campos) if (c.referencia?.nome) { n[c.nome] = c.referencia.nome; n[`icone:${c.nome}`] = c.referencia.icone ?? ""; }
      nomes = n;
      iconeHex = x.icone;
      f = x;
      if (!ehMina) await carregarDescricao(x.id ?? 0);
      if (x.id) carregarUsos(x.id);
      if (ehMina) minaDoGs(indice).then((m) => (mina = m)).catch(() => (mina = null));
    } catch (e) { erro = (e as Error).message; }
  }
  function carregarUsos(id: number) {
    usos = null; erroDosUsos = "";
    usosDoItem(id).then((u) => (usos = u)).catch((e) => (erroDosUsos = (e as Error).message));
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

  /* ---- gravar (o mesmo da edição de Equipável) ---- */
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  function validar(): Record<string, number | string> | null {
    const campos: Record<string, number | string> = {};
    for (const n of mudados) {
      const c = porNome[n];
      const v = valores[n];
      if (c.tipo === "int32") {
        const x = Number(v);
        if (v === "" || !Number.isInteger(x) || x < -2147483648 || x > 2147483647) { avisar("aviso", `${rotulo(n)}: número inteiro inválido.`); return null; }
        campos[n] = x;
      } else if (c.tipo === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x)) { avisar("aviso", `${rotulo(n)}: número inválido.`); return null; }
        campos[n] = x;
      } else campos[n] = String(v);
    }
    return campos;
  }
  async function gravar() {
    if (!f) return;
    const campos = validar();
    if (!campos) return;
    const textoMudou = !ehMina && descricao !== descricaoOriginal;
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
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }
  function descartar() { valores = { ...originais }; descricao = descricaoOriginal; iconeHex = f?.icone ?? null; }

  async function buscarIconeHex(nome: string) {
    try { iconeHex = (await buscarIcones(nome, 20)).icones.find((i) => i.arquivo === nome)?.hex ?? null; } catch { iconeHex = null; }
  }
  async function resolverNome(campo: string, id: number, tabelaAlvo?: string) {
    if (!id) { delete nomes[campo]; return; }
    try {
      const r = (await buscarNoEspaco("ESSENCE", String(id), 5)).find((x) => x.id === id && (!tabelaAlvo || x.tabela === tabelaAlvo));
      nomes[campo] = r ? r.nome : `id ${id} (não existe)`;
      if (r) nomes[`icone:${campo}`] = r.icone ?? "";
    } catch { nomes[campo] = `id ${id}`; }
  }

  /* ---- item ligado criado em outra aba (BroadcastChannel, como nos Equipáveis) ---- */
  const canal = typeof BroadcastChannel !== "undefined" ? new BroadcastChannel(NOME_DO_CANAL) : null;
  const esperando = new Map<string, string>();
  if (canal) canal.onmessage = (e: MessageEvent<MensagemDeItem>) => {
    const m = e.data;
    if (m?.tipo !== "item-criado" || !esperando.has(m.retorno)) return;
    const campo = esperando.get(m.retorno)!;
    esperando.delete(m.retorno);
    valores[campo] = m.id;
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

  /* ---- rótulos e boxes ---- */
  const rotulo = (n: string) => ROTULOS_DE_ITEM[n] || porNome[n]?.rotulo || rotuloGerado(n) || n;
  const nomesDaFicha = $derived(Object.keys(porNome));
  /** Campos que uma box própria do tipo já mostra (não repetem nas genéricas). */
  const proprios = $derived.by(() => {
    const p = new Set<string>(CAMPOS_PRINCIPAIS);
    if (tabela === "MEDICINE_ESSENCE") ["require_level", "cool_time", "hp_add_total", "hp_add_time", "mp_add_total", "mp_add_time"].forEach((x) => p.add(x));
    if (tabela === "AUTOHP_ESSENCE" || tabela === "AUTOMP_ESSENCE") ["total_hp", "total_mp", "trigger_amount", "cool_time"].forEach((x) => p.add(x));
    if (tabela === "STONE_ESSENCE") ["level", "color", "install_price", "uninstall_price", "require_equip", "id_addon_damage", "id_addon_defence",
      "id_addon_decoration", "weapon_desc", "armor_desc", "decoration_desc"].forEach((x) => p.add(x));
    if (tabela === "REFINE_TICKET_ESSENCE") nomesDaFicha.filter((x) => /^(desc|ext_|fail_|binding_only|require_level_max)/.test(x)).forEach((x) => p.add(x));
    if (ehMina) nomesDaFicha.filter((x) => /^materials_\d+_/.test(x)).forEach((x) => p.add(x));
    if (existe("id_skill")) ["id_skill", "level_skill", "skill_level"].forEach((x) => p.add(x));
    return p;
  });
  const boxes = $derived.by(() => {
    const vistos = new Set(proprios);
    const r: { titulo: string; campos: string[]; explica?: string }[] = [];
    for (const b of BOXES_DO_ITEM) {
      const cs = camposDaBox(b, nomesDaFicha).filter((n) => !vistos.has(n) || (b.titulo === "Comércio" && ["price", "shop_price", "pile_num_max", "has_guid"].includes(n)));
      cs.forEach((n) => vistos.add(n));
      if (cs.length) r.push({ titulo: b.titulo, campos: cs, explica: b.explica });
    }
    const resto = nomesDaFicha.filter((n) => !vistos.has(n) && n !== "proc_type" && porNome[n].tipo !== "bytes");
    if (resto.length) r.push({ titulo: "Outros campos", campos: resto, explica: "Os demais campos da tabela nesta versão, com o rótulo do layout." });
    return r;
  });
  const resumoDeRegras = $derived(REGRAS_DO_ITEM.filter(([b]) => num("proc_type") & b).map(([, n]) => n));

  /* ---- remédio, amuleto ---- */
  const remedio = $derived(tabela === "MEDICINE_ESSENCE" ? regraDoRemedio({ id_major_type: num("id_major_type"), require_level: num("require_level"),
    cool_time: num("cool_time"), hp_add_total: num("hp_add_total"), hp_add_time: num("hp_add_time"), mp_add_total: num("mp_add_total"), mp_add_time: num("mp_add_time") }) : null);
  const amuleto = $derived(tabela === "AUTOHP_ESSENCE" || tabela === "AUTOMP_ESSENCE"
    ? regraDoAmuleto(num(tabela === "AUTOHP_ESSENCE" ? "total_hp" : "total_mp"), num("trigger_amount"), num("cool_time"), tabela === "AUTOHP_ESSENCE") : null);

  /* ---- pedra: efeitos com o texto do cliente (rota da edição de personagem, B209) ---- */
  const EFEITOS_DA_PEDRA = [
    { id: "id_addon_damage", texto: "weapon_desc", onde: "Na arma" },
    { id: "id_addon_defence", texto: "armor_desc", onde: "Na armadura" },
    { id: "id_addon_decoration", texto: "decoration_desc", onde: "No acessório" },
  ];
  let textosDosEfeitos = $state<Record<number, string>>({});
  $effect(() => {
    if (!f || tabela !== "STONE_ESSENCE") return;
    const ids = EFEITOS_DA_PEDRA.map((e) => num(e.id)).filter((id) => id && !(id in textosDosEfeitos));
    if (!ids.length) return;
    ids.forEach((id) => (textosDosEfeitos[id] = ""));
    descreverEfeitos(ids, f.id ?? 1).then((d) => { for (const e of d) textosDosEfeitos[e.id] = e.texto; }).catch(() => {});
  });
  const serve = $derived(ondeAPedraServe(num("require_equip")));

  /* ---- talismã de refino ---- */
  const talisma = $derived<Talisma | null>(tabela === "REFINE_TICKET_ESSENCE" ? {
    ext_succeed_prob: num("ext_succeed_prob"), ext_reserved_prob: num("ext_reserved_prob"), fail_reserve_level: num("fail_reserve_level"),
    fail_ext_succeed_prob: Array.from({ length: 12 }, (_, k) => num(`fail_ext_succeed_prob_${k + 1}`)),
  } : null);
  const pct = (x: number) => `${(x * 100).toLocaleString("pt-BR", { maximumFractionDigits: 1 })}%`;

  /* ---- habilidade citada (SKILLMATTER, INC_SKILL_ABILITY, TARGET_ITEM) ---- */
  let nomeDaHabilidade = $state("");
  $effect(() => {
    const id = num("id_skill");
    if (!existe("id_skill") || !id) { nomeDaHabilidade = ""; return; }
    habilidadePorId(id).then((h) => (nomeDaHabilidade = h ? h.nome : `habilidade ${id} não existe no catálogo do realm`));
  });

  /* ---- mina: materiais (até 16), soma exigida = 1 (npcgenerator.cpp:1323-1327) ---- */
  const posicoesDaMina = $derived(ehMina ? Array.from({ length: 32 }, (_, k) => k + 1).filter((k) => existe(`materials_${k}_id`)) : []);
  const linhasDaMina = $derived(posicoesDaMina.filter((k) => num(`materials_${k}_id`) || num(`materials_${k}_probability`)));
  const somaDaMina = $derived(posicoesDaMina.reduce((s, k) => s + (num(`materials_${k}_probability`) > 0 ? num(`materials_${k}_probability`) : 0), 0));
  function normalizarMina() {
    if (somaDaMina <= 0) return;
    for (const k of posicoesDaMina) { const p = num(`materials_${k}_probability`); if (p > 0) valores[`materials_${k}_probability`] = Math.round((p / somaDaMina) * 1e6) / 1e6; }
    const ks = posicoesDaMina.filter((k) => num(`materials_${k}_probability`) > 0);
    const resto = 1 - ks.reduce((s, k) => s + num(`materials_${k}_probability`), 0);
    if (ks.length) valores[`materials_${ks[0]}_probability`] = Math.round((num(`materials_${ks[0]}_probability`) + resto) * 1e6) / 1e6;
  }
  function adicionarNaMina() {
    const k = posicoesDaMina.find((x) => !num(`materials_${x}_id`) && !num(`materials_${x}_probability`));
    if (k == null) { avisar("aviso", "As 16 posições de material estão ocupadas."); return; }
    modal = `item:materials_${k}_id`;
  }
  function tirarDaMina(k: number) {
    valores[`materials_${k}_id`] = 0; valores[`materials_${k}_probability`] = 0;
    if (existe(`materials_${k}_life`)) valores[`materials_${k}_life`] = 0;
  }

  /* ---- 3D: o item no chão (`file_matter`) e o modelo da mina (`file_model`) montados como o CECMatter ---- */
  const camposNoChao = $derived(ehMina ? (existe("file_model") ? [{ rotulo: "Mina no mapa", campo: "file_model" }] : [])
    : existe("file_matter") ? [{ rotulo: "No chão", campo: "file_matter" }] : []);
  const caminhosNoChao = $derived(Object.fromEntries(camposNoChao.map((c) => [c.campo, String(originais[c.campo] ?? "")])));
  const outrosModelos = $derived(ehMina ? [] : nomesDaFicha.filter((n) => /^file_model\d*$/.test(n) && String(valores[n] ?? "").toLowerCase().endsWith(".ecm"))
    .map((n) => ({ rotulo: rotulo(n), caminho: String(valores[n]) })));
  async function trocarModelo(campo: string, ecm: string) {
    await salvar(tabela, indice, { [campo]: ecm });
    originais = { ...originais, [campo]: ecm };
    valores[campo] = ecm;
    avisar("ok", `${campo} agora cita ${ecm} (gravado no rascunho).`);
  }

  /** Para a box "Onde aparece": a própria mina ou as minas que dão o material. */
  const minasDaBox = $derived(ehMina ? (f?.id ? [{ id: f.id, nome: String(valores.Name ?? "") }] : [])
    : (usos?.minas ?? []).map((m) => ({ id: m.id ?? 0, nome: m.nome })).filter((m) => m.id));
  const limpo = (t: string | null | undefined) => (t ?? "").replace(/\^[0-9a-fA-F]{6}/g, "").trim();
</script>

{#snippet campo(n: string)}
  {#if existe(n)}
    {@const c = porNome[n]}
    <label title={n}>
      <span>{rotulo(n)}</span>
      {#if c.tipo === "int32" || c.tipo === "float"}
        <input class="campo" class:alterado={alterado(n)} type="number" step={c.tipo === "float" ? "any" : "1"} disabled={!editavel}
          value={valores[n]} oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
      {:else}
        <input class="campo" class:alterado={alterado(n)} disabled={!editavel} value={valores[n]} oninput={(e) => (valores[n] = (e.target as HTMLInputElement).value)} />
      {/if}
      {#if c.referencia?.nome && valores[n] === originais[n]}<span class="mini">{limpo(c.referencia.nome)}</span>{/if}
    </label>
  {/if}
{/snippet}

{#snippet cartao(campoId: string, titulo: string)}
  <button class="cartao-item" type="button" class:alterado={alterado(campoId)} onclick={() => (modal = `item:${campoId}`)} title={titulo}>
    <IconeItem hex={nomes[`icone:${campoId}`]} nome={nomes[campoId]} />
    <span>{num(campoId) ? (limpo(nomes[campoId]) || `id ${num(campoId)}`) : "nenhum"}</span>
    {#if num(campoId)}<span class="mini">id {num(campoId)}</span>{/if}
  </button>
{/snippet}

{#snippet ref(r: { tabela?: string; indice?: number; id: number | null; nome: string | null; icone?: string | null }, extra = "")}
  {#if r.tabela && r.indice != null}
    <a class="ref" href={r.tabela === "MINE_ESSENCE" || subDeItemPorTabela(r.tabela) ? `#/itens/${r.tabela}/${r.indice}?${parametros()}` : linkDosEditores(r.tabela, r.indice)}
      target="_blank" rel="noopener" title="Abrir em outra aba: {r.tabela} id {r.id}">
      <IconeItem hex={r.icone} nome={r.nome ?? ""} tamanho={24} discreto />
      <span class="nome">{limpo(r.nome) || "(sem nome)"}</span>
      <span class="mini">{extra ? `${extra} · ` : ""}id {r.id}</span>
    </a>
  {:else}
    <span class="mini">id {r.id} (não existe)</span>
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
    <a class="botao fantasma" href={linkDosEditores(tabela, indice)} target="_blank" rel="noopener"><Icone nome="editores" /> Abrir nos Editores</a>
    <span class="espaco"></span>
    {#if mudados.length || descricao !== descricaoOriginal}<span class="mini">{mudados.length} campo(s){descricao !== descricaoOriginal ? " + descrição" : ""} por gravar</span>{/if}
    <button class="botao" type="button" disabled={gravando} onclick={descartar}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || gravando} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
  </div>

  <div class="layout">
    <div class="coluna">
      <!-- Principal -->
      <section class="caixa principal">
        {#if existe("file_icon")}
          <button class="icone-grande" type="button" title="Trocar o ícone" onclick={() => (modal = "icone")}>
            <IconeItem hex={iconeHex} nome={String(valores.Name)} tamanho={64} />
          </button>
        {/if}
        <div class="dados">
          <input class="campo nome" class:alterado={alterado("Name")} disabled={!editavel} value={valores.Name}
            oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} aria-label="Nome" maxlength="32" />
          <div class="linha">
            {#if (sub?.maior && existe("id_major_type")) || (sub?.menor && (existe("id_sub_type") || existe("id_type")))}
              <button class="botao pequeno" type="button" onclick={() => (modal = "categoria")} title="Categoria e subcategoria">
                {#if sub?.maior}{limpo(nomes.id_major_type) || `id ${num("id_major_type")}`} › {/if}{limpo(nomes[ehMina ? "id_type" : "id_sub_type"]) || `id ${num(ehMina ? "id_type" : "id_sub_type")}`}
              </button>
            {/if}
            <span class="mini">{sub?.grupo} · {sub?.nome}</span>
          </div>
          {#if sub?.oQueE}<p class="mini">{sub.oQueE}</p>{/if}
          {#if existe("proc_type")}
            <div class="linha">
              <button class="botao pequeno" type="button" class:alterado={alterado("proc_type")} onclick={() => (modal = "regras")}>Regras do item</button>
              <span class="mini">{resumoDeRegras.length ? resumoDeRegras.join(" · ") : "nenhuma restrição"}</span>
            </div>
          {/if}
          {#if !ehMina}
            <label class="desc">
              <span class="mini">Descrição (item_ext_desc.txt do cliente; os códigos ^RRGGBB mudam a cor)</span>
              <textarea class="campo" rows="3" class:alterado={descricao !== descricaoOriginal} disabled={!editavel} bind:value={descricao}></textarea>
            </label>
          {/if}
        </div>
      </section>

      <!-- Remédio -->
      {#if remedio}
        <section class="caixa">
          <h3>O que cura {#if CATEGORIAS_DE_REMEDIO[num("id_major_type")]}<span class="mini">{CATEGORIAS_DE_REMEDIO[num("id_major_type")].nome}</span>{/if}</h3>
          <div class="grade-campos">{#each ["require_level", "cool_time", "hp_add_total", "hp_add_time", "mp_add_total", "mp_add_time"] as n (n)}{@render campo(n)}{/each}</div>
          <ul class="regras-lista">{#each remedio.linhas as l, k (k)}<li class="mini">{l}</li>{/each}{#each remedio.avisos as l, k (k)}<li class="mini erro">{l}</li>{/each}</ul>
          <p class="mini">A categoria (no alto) decide o efeito: generate_item_temp.h:913-1000 e item_potion.cpp.</p>
        </section>
      {/if}

      <!-- Amuleto -->
      {#if amuleto}
        <section class="caixa">
          <h3>Como dispara</h3>
          <div class="grade-campos">{#each ["total_hp", "total_mp", "trigger_amount", "cool_time"] as n (n)}{@render campo(n)}{/each}</div>
          <ul class="regras-lista">{#each amuleto as l, k (k)}<li class="mini">{l}</li>{/each}</ul>
        </section>
      {/if}

      <!-- Pedra -->
      {#if tabela === "STONE_ESSENCE"}
        <section class="caixa">
          <h3>Incrustar</h3>
          <div class="grade-campos">{#each ["level", "color", "install_price", "uninstall_price"] as n (n)}{@render campo(n)}{/each}</div>
          <p class="mini">Incrustar no NPC cobra o preço de incrustar (serviceprovider.cpp:1374-1385); remover cobra o de remover (item_list.cpp:205); trocar cobra os dois (player.cpp:23879).</p>
          {#if existe("require_equip")}
            <div class="linha">
              <button class="botao pequeno" type="button" class:alterado={alterado("require_equip")} onclick={() => (modal = "mascara-pedra")}>Onde serve</button>
              <span class="mini">arma {serve.arma ? "sim" : "não"} · armadura {serve.armadura ? "sim" : "não"} · acessório {serve.acessorio ? "sim" : "não"}
                (IsStoneFit, item.cpp:332-353)</span>
            </div>
          {:else}
            <p class="mini">Nesta versão a pedra só vai em arma e armadura (o layout v7 não tem efeito de acessório nem a máscara).</p>
          {/if}
          <div class="efeitos-pedra">
            {#each EFEITOS_DA_PEDRA.filter((e) => existe(e.id)) as e (e.id)}
              <div class="ligado">
                <span class="mini">{e.onde}</span>
                {@render cartao(e.id, rotulo(e.id))}
                <span class="mini">{num(e.id) ? (textosDosEfeitos[num(e.id)] || "carregando o texto do cliente…") : ""}</span>
                {#if existe(e.texto)}
                  <label class="mini">texto curto
                    <input class="campo" class:alterado={alterado(e.texto)} disabled={!editavel} value={valores[e.texto]} maxlength="15"
                      oninput={(x) => (valores[e.texto] = (x.target as HTMLInputElement).value)} /></label>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <!-- Talismã de refino -->
      {#if talisma}
        <section class="caixa">
          <h3>Chances do refino com este talismã</h3>
          <div class="grade-campos">{#each ["desc", "ext_succeed_prob", "ext_reserved_prob", "fail_reserve_level", "binding_only", "require_level_max"] as n (n)}{@render campo(n)}{/each}</div>
          {#if talisma.fail_reserve_level}
            <div class="grade-campos">{#each Array.from({ length: 12 }, (_, k) => `fail_ext_succeed_prob_${k + 1}`) as n, k (n)}
              {#if existe(n)}<label><span>Sucesso no +{k} → +{k + 1}</span>
                <input class="campo" class:alterado={alterado(n)} type="number" step="any" disabled={!editavel} value={valores[n]}
                  oninput={(e) => (valores[n] = Number((e.target as HTMLInputElement).value))} /></label>{/if}
            {/each}</div>
          {/if}
          <table class="niveis">
            <thead><tr><th>do nível</th><th>sucesso</th><th>nada muda</th><th>cai um</th><th>zera</th><th>sem talismã: sucesso</th></tr></thead>
            <tbody>
              {#each Array.from({ length: 12 }, (_, k) => k) as n (n)}
                {@const c = chancesDoRefino(n, talisma)}
                <tr><td>+{n} → +{n + 1}</td><td>{pct(c[0])}</td><td>{pct(c[1])}</td><td>{pct(c[2])}</td><td>{pct(c[3])}</td><td>{pct(chancesDoRefino(n)[0])}</td></tr>
              {/each}
            </tbody>
          </table>
          <p class="mini">Tabela do GS (equip_item.cpp:192-205) com o ajuste do talismã (player.cpp:11719-11747; equip_item.cpp:265-273): soma no sucesso e na
            chance de cair só um nível (presos a 0–1); "falhar não tira nível" troca o sucesso pelo valor do nível e o resto vira "nada muda". O sorteio
            (RandSelect, arandomgen.h:129-139) corta o que passa de 100%. {num("binding_only") ? "Só vale em peça vinculada. " : ""}{num("require_level_max") ? `Só em peça de grau até ${num("require_level_max")}.` : ""}</p>
        </section>
      {/if}

      <!-- Habilidade citada -->
      {#if existe("id_skill")}
        <section class="caixa">
          <h3>Habilidade</h3>
          <div class="grade-campos">{#each ["id_skill", "level_skill", "skill_level", "level_required", "inc_ratio"] as n (n)}{@render campo(n)}{/each}</div>
          <p class="mini">{nomeDaHabilidade || (num("id_skill") ? "…" : "nenhuma")} (catálogo de habilidades do realm)</p>
        </section>
      {/if}

      <!-- Livro de habilidade -->
      {#if tabela === "SKILLTOME_ESSENCE"}
        <section class="caixa">
          <h3>Habilidades que exigem este livro {#if usos}<span class="mini">{usos.habilidades.length}</span>{/if}</h3>
          {#if erroDosUsos}<p class="erro">{erroDosUsos}</p>{:else if !usos}<p class="suave">Carregando…</p>
          {:else if !usos.habilidades.length}<p class="suave">Nenhuma habilidade do catálogo do realm pede este livro.</p>
          {:else}
            <ul class="lista-ref">{#each usos.habilidades as h (h.id)}<li><span class="nome">{h.nome || "(sem nome)"}</span> <span class="mini">habilidade {h.id} · nível {h.niveis.join(", ")}</span></li>{/each}</ul>
          {/if}
          <p class="mini">O livro é gasto ao aprender o nível (item_exigido do catálogo; skill.cpp:79-84).</p>
        </section>
      {/if}

      <!-- Mina: materiais -->
      {#if ehMina}
        <section class="caixa">
          <h3>Materiais que dá <span class="mini">soma {somaDaMina.toLocaleString("pt-BR", { maximumFractionDigits: 6 })}</span></h3>
          {#if Math.abs(somaDaMina - 1) >= 1e-5}<p class="erro mini">As chances têm de somar 1, senão o GS recusa a mina (npcgenerator.cpp:1323-1327).
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={normalizarMina}>Ajustar para somar 1</button></p>{/if}
          <table class="materiais">
            <thead><tr><th>material</th><th>chance</th>{#if existe("materials_1_life")}<th>validade (s)</th>{/if}<th></th></tr></thead>
            <tbody>
              {#each linhasDaMina as k (k)}
                <tr>
                  <td>{@render cartao(`materials_${k}_id`, "Material")}</td>
                  <td><input class="campo qtd" class:alterado={alterado(`materials_${k}_probability`)} type="number" step="any" min="0" max="1" disabled={!editavel}
                    value={valores[`materials_${k}_probability`]} oninput={(e) => (valores[`materials_${k}_probability`] = Number((e.target as HTMLInputElement).value))} />
                    <span class="mini">{pct(num(`materials_${k}_probability`))}</span></td>
                  {#if existe(`materials_${k}_life`)}<td><input class="campo qtd" class:alterado={alterado(`materials_${k}_life`)} type="number" step="1" min="0" disabled={!editavel}
                    value={valores[`materials_${k}_life`]} oninput={(e) => (valores[`materials_${k}_life`] = Number((e.target as HTMLInputElement).value))} /></td>{/if}
                  <td><button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Tirar" onclick={() => tirarDaMina(k)}><Icone nome="lixeira" tamanho={16} /></button></td>
                </tr>
              {/each}
            </tbody>
          </table>
          <button class="botao pequeno" type="button" disabled={!editavel} onclick={adicionarNaMina}><Icone nome="mais" tamanho={16} /> Material</button>
          {#if mina}
            {#if !mina.valida}
              <p class="erro mini">O GS recusa esta mina (no rascunho gravado): {mina.motivo}</p>
            {:else if mina.mina}
              {@const m = mina.mina}
              <ul class="regras-lista">
                <li class="mini">Ao nascer, sorteia um material pelas chances e a quantidade: {m.quantidade}, ou {m.quantidade_bonus} com {pct(m.chance_bonus)} de chance (npcgenerator.cpp:5003-5016).</li>
                <li class="mini">Coleta de {m.tempo_minimo} a {m.tempo_maximo} s, nível {m.nivel}+, {m.coletores} coletor(es), a até {m.distancia} m; dá certo com {pct(m.chance_de_sucesso)} (matter.cpp:405-420).</li>
                <li class="mini">Ferramenta: {m.ferramenta ? `${limpo(m.ferramenta.nome) || `id ${m.ferramenta.id}`}${m.gasta_ferramenta ? " (gasta ao colher)" : ""}` : "nenhuma"}; {m.permanente ? "não some ao colher" : "some ao colher e renasce pela área"}; {m.ininterrupta ? "coleta não é interrompida" : "andar interrompe"}.</li>
                <li class="mini">Experiência {m.exp}, SP {m.sp}{m.missao_de_entrada ? `; mina de missão ${m.missao_de_entrada} → ${m.missao_de_saida}` : ""}.</li>
                {#each m.monstros_ao_colher as x, k (k)}
                  <li class="mini">Ao colher nasce(m) {x.quantidade} × {limpo(x.monstro.nome) || `id ${x.monstro.id}`} num raio de {x.raio} m, por {x.vida} s.</li>
                {/each}
              </ul>
            {/if}
          {/if}
        </section>
      {/if}

      <!-- Boxes genéricas -->
      {#each boxes as b (b.titulo)}
        <section class="caixa">
          <h3>{b.titulo}</h3>
          {#if b.explica}<p class="mini">{b.explica}</p>{/if}
          <div class="grade-campos">{#each b.campos as n (n)}{@render campo(n)}{/each}</div>
          {#if b.titulo === "Desmontar"}
            <div class="ligados">
              {#each ITENS_LIGADOS.filter((x) => existe(x.campo)) as l (l.campo)}
                <div class="ligado"><span class="mini">{l.titulo}</span>{@render cartao(l.campo, l.titulo)}</div>
              {/each}
            </div>
          {/if}
        </section>
      {/each}
    </div>

    <aside class="lateral">
      {#if camposNoChao.length}
        <ModeloDoEquipavel {tabela} campos={camposNoChao} caminhos={caminhosNoChao} idDoItem={f.id ?? 0} aoTrocarModelo={trocarModelo}
          como="matter" titulo={ehMina ? "Modelo 3D da mina" : "Modelo 3D no chão"} pastaPadrao={ehMina ? "models\\matters\\estudio\\minas" : "models\\matters\\estudio"} />
      {/if}
      {#if outrosModelos.length}<Visual3D modelos={outrosModelos} />{/if}
    </aside>
  </div>

  <!-- Para onde vai: receitas e minas -->
  {#if !ehMina}
    <div class="usos">
      <section class="caixa">
        <h3>Entra nas receitas {#if usos}<span class="mini">{usos.ingrediente_de.length}</span>{/if}</h3>
        {#if erroDosUsos}<p class="erro">{erroDosUsos}</p>{:else if !usos}<p class="suave">Carregando…</p>
        {:else if !usos.ingrediente_de.length}<p class="suave">Nenhuma receita gasta este item.</p>
        {:else}
          <ul class="lista-ref">{#each usos.ingrediente_de as r (r.indice)}<li>{@render ref(r, `${r.quantidade}× · faz ${r.faz.map((x) => limpo(x.nome) || `id ${x.id}`).join(", ") || "—"}`)}</li>{/each}</ul>
          <p class="mini">RECIPE_ESSENCE.materials_N (gastos na fabricação).</p>
        {/if}
      </section>
      <section class="caixa">
        <h3>Sai das minas {#if usos}<span class="mini">{usos.minas.length}</span>{/if}</h3>
        {#if erroDosUsos}<p class="erro">{erroDosUsos}</p>{:else if !usos}<p class="suave">Carregando…</p>
        {:else if !usos.minas.length}<p class="suave">Nenhuma mina dá este item.</p>
        {:else}
          <ul class="lista-ref">{#each usos.minas as m (m.indice)}<li>{@render ref(m, `${pct(m.chance)}${m.vida ? ` · validade ${m.vida} s` : ""}${m.valida ? "" : " · o GS recusa esta mina"}`)}
            {#if !m.valida}<span class="mini erro">{m.motivo}</span>{/if}</li>{/each}</ul>
          <p class="mini">MINE_ESSENCE.materials_N: a chance é a do material entre os da mina (sorteado quando a mina nasce).</p>
        {/if}
      </section>
    </div>
  {/if}

  <!-- Onde aparece: áreas de recurso das minas -->
  {#if ehMina || usos?.minas.length}
    <BoxDeRecurso minas={minasDaBox} />
  {/if}

  {#if f.id && !ehMina}<Origens id={f.id} />{/if}
{/if}

{#if f && modal === "icone"}
  <ModalIcone caminho={String(valores.file_icon ?? "")} aoFechar={() => (modal = "")}
    aoEscolher={(novo) => { valores.file_icon = novo; buscarIconeHex(novo.slice(novo.lastIndexOf("\\") + 1).toLowerCase()); }} />
{:else if f && modal === "categoria"}
  {@const campoMenor = ehMina ? "id_type" : "id_sub_type"}
  <ModalCategoria maior={sub?.maior} menor={sub?.menor} idMaior={num("id_major_type")} idMenor={num(campoMenor)} aoFechar={() => (modal = "")}
    aoEscolher={(a, b) => { if (sub?.maior) { valores.id_major_type = a; resolverNome("id_major_type", a, sub.maior); } valores[campoMenor] = b; resolverNome(campoMenor, b, sub?.menor); }} />
{:else if f && modal === "regras"}
  <ModalMascara titulo="Regras do item" opcoes={REGRAS_DO_ITEM} valor={num("proc_type")} aoFechar={() => (modal = "")}
    explica="ITEM_PROC_TYPE_* do servidor 1.5.5 (cgame/gs/item.h:309-326)." aoAplicar={(v) => (valores.proc_type = v)} />
{:else if f && modal === "mascara-pedra"}
  <ModalMascara titulo="Onde a pedra serve" opcoes={BITS_DA_PEDRA} valor={num("require_equip")} aoFechar={() => (modal = "")}
    explica="combined_switch da pedra (ExpTypes.h:2053-2065): arma e armadura servem salvo o bit; acessório só com colar ou cinto (IsStoneFit, item.cpp:332-353)."
    aoAplicar={(v) => (valores.require_equip = v)} />
{:else if f && modal.startsWith("item:")}
  {@const c = modal.slice(5)}
  {@const ehEfeito = c.startsWith("id_addon_")}
  <SeletorDeEspaco titulo={ehEfeito ? rotulo(c) : (ITENS_LIGADOS.find((x) => x.campo === c)?.titulo ?? rotulo(c))} espaco={ehEfeito ? "ADDON" : "ESSENCE"}
    aoFechar={() => (modal = "")} aoCriar={editavel && !ehEfeito ? () => criarEmOutraAba(c) : undefined}
    aoEscolher={(r: Registro) => {
      if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para mudar."); return; }
      valores[c] = r.id ?? 0; nomes[c] = r.nome; nomes[`icone:${c}`] = r.icone ?? "";
      const k = c.match(/^materials_(\d+)_id$/)?.[1];
      if (k && !num(`materials_${k}_probability`)) valores[`materials_${k}_probability`] = Math.max(0, Math.round((1 - somaDaMina) * 1e6) / 1e6);
    }} />
{/if}

<style>
  .barra { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; }
  .espaco { flex: 1; }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(320px, 440px); gap: 14px; align-items: start; }
  .coluna, .lateral { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .principal { display: flex; gap: 16px; align-items: flex-start; }
  .icone-grande { padding: 6px; border-radius: 12px; border: 1px solid var(--borda-forte); background: var(--fundo-2); cursor: pointer; }
  .icone-grande:hover { border-color: var(--ouro); }
  .dados { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .nome { font-size: 18px; font-weight: 600; height: 40px; }
  .desc { display: flex; flex-direction: column; gap: 4px; }
  .regras-lista { margin: 10px 0 0; padding-left: 18px; display: flex; flex-direction: column; gap: 4px; }
  .ligados, .efeitos-pedra { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 10px; margin-top: 12px; }
  .ligado { display: flex; flex-direction: column; gap: 4px; }
  .cartao-item { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 10px; border: 1px solid var(--borda); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .cartao-item:hover { border-color: var(--ouro); }
  .qtd { width: 100px; height: 28px; }
  .niveis, .materiais { border-collapse: collapse; font-size: 12px; width: 100%; margin-top: 10px; }
  .niveis th, .niveis td { border: 1px solid var(--borda); padding: 2px 4px; text-align: center; }
  .materiais td, .materiais th { padding: 4px; text-align: left; }
  .usos { display: grid; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); gap: 14px; margin-top: 14px; }
  .lista-ref { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 300px; overflow: auto; }
  .ref { display: flex; align-items: center; gap: 8px; color: var(--texto); text-decoration: none; }
  .ref:hover .nome { color: var(--ouro); }
  :global(.alterado) { border-color: var(--ouro) !important; background: var(--ouro-suave) !important; }
  @media (max-width: 960px) {
    .layout { grid-template-columns: 1fr; }
    .usos { grid-template-columns: 1fr; }
  }
</style>
