<script lang="ts">
  /* B392: editor de um serviço de NPC (`NPC_*_SERVICE`). O formato sai dos nomes do layout (`lib/servicos.ts`):
       abas com grade — loja (`pages_P_goods_K_id`, 8 × 32) e produção (`pages_P_id_goods_K`, receitas);
       lista de ids — pedras, missões, habilidades, gravações;
       teleporte — `targets_K_*` (destino do world_targets, custo, nível) e `num_targets`;
       itens de missão — `tasks_K_id_task` + `tasks_K_taks_matters_J_*`;
       o resto — campos simples com o nome do que citam.
     Gravar = `PUT tabelas/{T}/{i}` com os campos mudados, no rascunho. Serviço usado por vários NPCs: o aviso fica no
     alto (quem chama oferece "cópia só para este NPC"). */
  import { ficha as lerFicha, habilidadePorId, mapasDoRealm, resumoDeMissoes, salvar, teleportesDoMapa, type Campo, type Ficha, type MapaDoRealm, type Produto, type Registro } from "../lib/api";
  import { abrirItemEmNovaAba, esconderDica, mostrarDica } from "../lib/dica.svelte";
  import { avisar, estudio, parametros } from "../lib/estado.svelte";
  import { categoriaDaTabela, compactar, espacoDaLista, vagasEmAbas, vagasEmLista, NOME_DA_TABELA } from "../lib/servicos";
  import CamposSimples from "./CamposSimples.svelte";
  import EditorDeReceita from "./EditorDeReceita.svelte";
  import EditorDeTexto from "./EditorDeTexto.svelte";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";
  import SeletorDeHabilidade from "./SeletorDeHabilidade.svelte";
  import SeletorDeMissao from "./SeletorDeMissao.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { tabela, indice, aoGravado, cabecalho }: { tabela: string; indice: number; aoGravado?: (id: number) => void; cabecalho?: import("svelte").Snippet<[Ficha]> } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let porNome = $state.raw<Record<string, Campo>>({});
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  /** nome e ícone dos ids (itens, receitas) — os da ficha e os escolhidos na tela */
  const nomes = $state<Record<string, { nome: string; icone: string | null; produto?: Produto | null }>>({});
  let gravando = $state(false);

  async function carregar() {
    erro = "";
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      for (const c of x.campos) if (c.referencia?.nome && typeof c.valor === "number") nomes[`${c.cita}:${c.valor}`] = { nome: c.referencia.nome, icone: c.referencia.icone ?? null, produto: c.referencia.produto };
      f = x;
    } catch (e) { erro = (e as Error).message; }
  }
  $effect(() => { void tabela; void indice; void estudio.rascunho; carregar(); });

  const todos = $derived(Object.keys(porNome));
  const num = (n: string) => Number(valores[n] ?? 0);
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  const categoria = $derived(categoriaDaTabela(tabela));
  const abas = $derived(vagasEmAbas(todos));
  const lista = $derived(abas ? null : vagasEmLista(todos));
  const eTeleporte = $derived(todos.some((n) => /^targets_\d+_idTarget$/.test(n)));
  const eItensDeMissao = $derived(todos.some((n) => /^tasks_\d+_id_task$/.test(n)));
  const espacoDasAbas = $derived(todos.some((n) => /^pages_\d+_id_goods_\d+$/.test(n)) ? "RECIPE" : "ESSENCE");
  const ehLoja = $derived(tabela === "NPC_SELL_SERVICE");

  /** Campos que os editores próprios mostram; o resto vai em "Outros campos". */
  const usados = $derived.by(() => {
    const u = new Set<string>(["ID", "Name"]);
    if (abas) for (const n of todos) if (n.startsWith("pages_")) u.add(n);
    if (lista) lista.campos.forEach((n) => u.add(n));
    if (eTeleporte) for (const n of todos) if (n.startsWith("targets_") || n === "num_targets") u.add(n);
    if (eItensDeMissao) for (const n of todos) if (n.startsWith("tasks_")) u.add(n);
    if (tabela === "NPC_MAKE_SERVICE") u.add("id_make_skill");
    return u;
  });
  const outros = $derived(todos.filter((n) => !usados.has(n)));
  const nomeDe = (espaco: string, id: number) => nomes[`${espaco}:${id}`]?.nome ?? (id ? `id ${id}` : "");
  const iconeDe = (espaco: string, id: number) => nomes[`${espaco}:${id}`]?.icone ?? null;
  /** B393: o item que a vaga mostra — o próprio item, ou o produto da receita (`EC_IvtrMaterial.cpp:165,200-218`). */
  const itemDaVaga = (espaco: string, id: number): Produto | null =>
    espaco === "RECIPE" ? nomes[`RECIPE:${id}`]?.produto ?? null : { id, nome: nomeDe(espaco, id), icone: iconeDe(espaco, id) };

  /* ---- abas com grade ---- */
  let aba = $state(0);
  let vagaSel = $state<string | null>(null);
  let escolhendo = $state<null | { campo: string; espaco: string; categoria?: string }>(null);
  let arrastada: string | null = null;
  const vagasDaAba = $derived(abas ? abas.vagas.filter((v) => v.pagina === aba).sort((a, b) => a.posicao - b.posicao) : []);
  const tituloDaAba = (p: number) => `pages_${p + 1}_page_title`;
  const ocupadas = (p: number) => (abas?.vagas ?? []).filter((v) => v.pagina === p && num(v.campo) > 0).length;
  function soltarEm(destino: string) {
    if (!arrastada || arrastada === destino) return;
    const a = arrastada, extra = (c: string) => (ehLoja ? [c.replace(/_id$/, "_contrib_cost"), c.replace(/_id$/, "_force_contribution_cost")] : []);
    // troca as duas vagas (com os custos da loja junto)
    for (const [x, y] of [[a, destino], ...extra(a).map((e, i) => [e, extra(destino)[i]])]) {
      if (!(x in valores) || !(y in valores)) continue;
      const t = valores[x]; valores[x] = valores[y]; valores[y] = t;
    }
    arrastada = null;
    vagaSel = destino;
  }
  /** índice da receita escolhida na tela (para abrir o editor antes de gravar) */
  const indicesDeReceita = $state<Record<number, number>>({});
  function escolherItem(campo: string, espaco: string, r: Registro) {
    valores[campo] = r.id ?? 0;
    if (espaco === "RECIPE" && r.id) indicesDeReceita[r.id] = r.indice;
    if (r.id) nomes[`${espaco}:${r.id}`] = { nome: r.nome, icone: r.icone ?? null, produto: r.produto };
    vagaSel = campo;
  }
  function limparVaga(campo: string) {
    valores[campo] = 0;
    if (ehLoja) for (const c of [campo.replace(/_id$/, "_contrib_cost"), campo.replace(/_id$/, "_force_contribution_cost")]) if (c in valores) valores[c] = 0;
  }
  const receitaSel = $derived(espacoDasAbas === "RECIPE" && vagaSel && num(vagaSel)
    ? indicesDeReceita[num(vagaSel)] ?? (valores[vagaSel] === originais[vagaSel] ? porNome[vagaSel]?.referencia?.indice ?? null : null) : null);

  /* ---- lista de ids ---- */
  let escolhendoMissao = $state(false);
  let escolhendoHabilidade = $state(false);
  const espacoLista = $derived(lista ? espacoDaLista(lista.prefixo, tabela) : "");
  const idsDaLista = $derived(lista ? lista.campos.map(num).filter((x) => x > 0) : []);
  let nomesDeMissao = $state<Record<number, string | null>>({});
  let nomesDeHabilidade = $state<Record<number, string>>({});
  $effect(() => {
    const ids = idsDaLista.filter((x) => !(x in (espacoLista === "MISSAO" ? nomesDeMissao : nomesDeHabilidade)));
    if (!ids.length) return;
    if (espacoLista === "MISSAO") resumoDeMissoes(ids).then((r) => { for (const m of r.missoes) nomesDeMissao[m.id] = m.nome; }).catch(() => {});
    if (espacoLista === "HABILIDADE") for (const i of ids) habilidadePorId(i).then((h) => (nomesDeHabilidade[i] = h?.nome ?? "(não existe)"));
  });
  function gravarLista(ids: number[]) {
    if (!lista) return;
    const v = compactar(ids, lista.campos.length);
    lista.campos.forEach((c, k) => (valores[c] = v[k]));
  }
  function acrescentar(id: number) {
    if (!lista) return;
    if (idsDaLista.includes(id)) { avisar("aviso", "Já está na lista."); return; }
    if (idsDaLista.length >= lista.campos.length) { avisar("aviso", `A lista cabe ${lista.campos.length}.`); return; }
    gravarLista([...idsDaLista, id]);
  }
  function nomeNaLista(id: number) {
    if (espacoLista === "MISSAO") return nomesDeMissao[id] ?? "…";
    if (espacoLista === "HABILIDADE") return nomesDeHabilidade[id] ?? "…";
    return nomeDe(espacoLista, id);
  }

  /* ---- teleporte ---- */
  let mapas = $state<MapaDoRealm[]>([]);
  let mapaDoTeleporte = $state("");
  let pontosDoMapa = $state<{ id: number; nome: string | null }[]>([]);
  const nomesDeDestino = $state<Record<number, string>>({});
  const destinos = $derived(eTeleporte ? Array.from({ length: 32 }, (_, k) => k + 1).filter((k) => `targets_${k}_idTarget` in porNome) : []);
  const usadosNoTeleporte = $derived(destinos.filter((k) => num(`targets_${k}_idTarget`) > 0));
  $effect(() => {
    if (!eTeleporte || mapas.length) return;
    mapasDoRealm().then(async (m) => {
      mapas = m.filter((x) => x.npcgen);
      // nomes de todos os destinos (world_targets é um arquivo só; a rota filtra por mapa)
      for (const x of mapas.slice(0, 60)) {
        try { for (const p of (await teleportesDoMapa(x.caminho)).pontos) nomesDeDestino[p.id] = `${p.nome ?? `destino ${p.id}`} · ${x.nome}`; } catch { /* mapa sem teleporte */ }
      }
    }).catch(() => {});
  });
  $effect(() => { if (mapaDoTeleporte) teleportesDoMapa(mapaDoTeleporte).then((r) => (pontosDoMapa = r.pontos.map((p) => ({ id: p.id, nome: p.nome })))).catch((e) => avisar("erro", (e as Error).message)); });
  function gravarDestinos(l: { id: number; fee: number; nivel: number }[]) {
    destinos.forEach((k, i) => {
      valores[`targets_${k}_idTarget`] = l[i]?.id ?? 0;
      valores[`targets_${k}_fee`] = l[i]?.fee ?? 0;
      valores[`targets_${k}_required_level`] = l[i]?.nivel ?? 0;
    });
    if ("num_targets" in valores) valores.num_targets = l.length;
  }
  const linhasDeDestino = () => usadosNoTeleporte.map((k) => ({ id: num(`targets_${k}_idTarget`), fee: num(`targets_${k}_fee`), nivel: num(`targets_${k}_required_level`) }));

  /* ---- itens de missão ---- */
  const missoesDeItem = $derived(eItensDeMissao ? Array.from({ length: 16 }, (_, k) => k + 1).filter((k) => `tasks_${k}_id_task` in porNome) : []);
  let escolhendoMissaoDeItem = $state<number | null>(null);
  $effect(() => {
    const ids = missoesDeItem.map((k) => num(`tasks_${k}_id_task`)).filter((x) => x > 0 && !(x in nomesDeMissao));
    if (ids.length) resumoDeMissoes(ids).then((r) => { for (const m of r.missoes) nomesDeMissao[m.id] = m.nome; }).catch(() => {});
  });

  /* ---- produção: habilidade ---- */
  let nomeDaHabilidadeDeProducao = $state("");
  let escolhendoHabProd = $state(false);
  $effect(() => { const s = num("id_make_skill"); if (s > 0) habilidadePorId(s).then((h) => (nomeDaHabilidadeDeProducao = h?.nome ?? `habilidade ${s}`)); else nomeDaHabilidadeDeProducao = "nenhuma"; });

  async function gravar() {
    const c: Record<string, number | string> = {};
    for (const n of mudados) {
      const t = porNome[n].tipo, v = valores[n];
      if (t === "int32" || t === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x) || (t === "int32" && !Number.isInteger(x))) { avisar("aviso", `${n}: número inválido.`); return; }
        c[n] = x;
      } else c[n] = String(v);
    }
    if (!Object.keys(c).length) { avisar("aviso", "Nada mudou."); return; }
    gravando = true;
    try {
      const r = await salvar(tabela, indice, c);
      avisar("ok", r.igual_ao_publicado ? "Igual ao publicado." : `Serviço ${f?.id}: ${Object.keys(c).length} campo(s) gravados no rascunho.`);
      await carregar();
      aoGravado?.(f?.id ?? 0);
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }
  const npcs = $derived((f?.usado_por ?? []) as { tabela: string; indice: number; id: number; nome: string; campo?: string }[]);
</script>

{#if erro}<p class="erro">{erro}</p>
{:else if !f}<p class="suave">Carregando o serviço…</p>
{:else}
  <div class="servico">
    {#if cabecalho}{@render cabecalho(f)}{/if}
    <div class="topo">
      <div class="nome">
        <span class="mini">{categoria?.nome ?? tabela}{NOME_DA_TABELA[tabela] ? ` · ${NOME_DA_TABELA[tabela]}` : ""} · id {f.id} · registro {indice}</span>
        <EditorDeTexto rotulo="Texto do botão no jogo (Name)" valor={String(valores.Name ?? "")} limite={31} umaLinha previa={false} desabilitado={!editavel}
          aoMudar={(v) => (valores.Name = v)} />
      </div>
      <div class="acoes">
        {#if mudados.length}<span class="mini">{mudados.length} campo(s) por gravar</span>{/if}
        <button class="botao" type="button" disabled={!mudados.length || gravando} onclick={() => (valores = { ...originais })}>Descartar</button>
        <button class="botao primario" type="button" disabled={!editavel || !mudados.length || gravando} onclick={gravar}><Icone nome="check" /> Gravar serviço</button>
      </div>
    </div>
    {#if categoria?.explica}<p class="mini">{categoria.explica}</p>{/if}
    <div class="usado">
      <span class="mini">Usado por {npcs.length} {npcs.length === 1 ? "registro" : "registros"}:</span>
      {#each npcs.slice(0, 12) as n (n.tabela + n.indice)}
        {#if n.tabela === "NPC_ESSENCE"}<a class="chip" href="#/npcs/NPC_ESSENCE/{n.indice}?{parametros()}" target="_blank" rel="noopener">{n.nome || `NPC ${n.id}`}</a>
        {:else}<span class="chip">{n.tabela} {n.id}</span>{/if}
      {/each}
      {#if npcs.length > 12}<span class="mini">e mais {npcs.length - 12}</span>{/if}
    </div>

    {#if abas}
      <!-- Loja e produção: abas com grade de 32 vagas (arrastar troca as vagas) -->
      <div class="abas-servico" role="tablist">
        {#each abas.titulos as t, p (t)}
          <button type="button" role="tab" aria-selected={aba === p} class:on={aba === p} onclick={() => { aba = p; vagaSel = null; }}>
            <TextoDoJogo texto={String(valores[t] ?? "") || `Aba ${p + 1}`} padrao="currentColor" linha /> <span class="mini">{ocupadas(p)}</span>
          </button>
        {/each}
      </div>
      <div class="grade-e-lado">
        <div>
          <div class="linha">
            <label class="mini titulo-aba">Título da aba <input class="campo" value={String(valores[tituloDaAba(aba)] ?? "")} disabled={!editavel}
              oninput={(e) => (valores[tituloDaAba(aba)] = (e.target as HTMLInputElement).value)} /></label>
            {#if ehLoja}
              {#each ["require_contrib", "require_force", "require_force_reputation"] as r (r)}
                {#if `pages_${aba + 1}_${r}` in porNome}
                  <label class="mini titulo-aba">{r === "require_contrib" ? "Exige contribuição" : r === "require_force" ? "Exige força" : "Exige reputação da força"}
                    <input class="campo n" type="number" disabled={!editavel} value={valores[`pages_${aba + 1}_${r}`]}
                      oninput={(e) => (valores[`pages_${aba + 1}_${r}`] = Number((e.target as HTMLInputElement).value))} /></label>
                {/if}
              {/each}
            {/if}
          </div>
          <div class="slots" style:grid-template-columns="repeat(8, 44px)">
            {#each vagasDaAba as v (v.campo)}
              {@const id = num(v.campo)}
              <button type="button" class="slot" class:vazio={!id} class:sel={vagaSel === v.campo} class:mudou={valores[v.campo] !== originais[v.campo]}
                draggable={editavel && !!id} ondragstart={() => (arrastada = v.campo)} ondragover={(e) => e.preventDefault()} ondrop={() => soltarEm(v.campo)}
                onclick={() => { vagaSel = v.campo; if (!id && editavel) escolhendo = { campo: v.campo, espaco: espacoDasAbas }; }}
                onmouseenter={(e) => { const it = id ? itemDaVaga(espacoDasAbas, id) : null; if (it) mostrarDica(it.id, it.nome, e.currentTarget as HTMLElement); }} onmouseleave={esconderDica}
                aria-label={id ? (itemDaVaga(espacoDasAbas, id)?.nome ?? nomeDe(espacoDasAbas, id)) : "vaga livre — clique para pôr"}>
                {#if id}
                  {@const it = itemDaVaga(espacoDasAbas, id)}
                  {#if it}<IconeItem hex={it.icone} nome={it.nome} tamanho={34} />
                  {:else}<span class="receita-mini" title="receita sem produto: o jogo mostra Unknown.dds">{nomeDe("RECIPE", id).slice(0, 10)}</span>{/if}
                {:else}<Icone nome="mais" tamanho={14} />{/if}
              </button>
            {/each}
          </div>
          <p class="mini">Clique numa vaga vazia para pôr {espacoDasAbas === "RECIPE" ? "uma receita" : "um item"}; arraste para trocar de lugar; passe o mouse para ver o item.</p>
        </div>
        <aside class="lado">
          {#if vagaSel && num(vagaSel)}
            {@const id = num(vagaSel)}
            {@const prod = espacoDasAbas === "RECIPE" ? itemDaVaga("RECIPE", id) : null}
            <h4>{prod ? prod.nome : nomeDe(espacoDasAbas, id)}</h4>
            <span class="mini">{#if prod}receita {nomeDe("RECIPE", id)} · {/if}id {id} · {vagaSel}</span>
            {#if ehLoja}
              {#each [["_contrib_cost", "Custo em contribuição"], ["_force_contribution_cost", "Custo em contribuição de força"]] as [suf, rot] (suf)}
                {@const c = vagaSel.replace(/_id$/, suf)}
                {#if c in porNome}<label class="mini">{rot} <input class="campo" type="number" disabled={!editavel} value={valores[c]}
                  oninput={(e) => (valores[c] = Number((e.target as HTMLInputElement).value))} /></label>{/if}
              {/each}
              <p class="mini">O preço em moedas é o do item (price e shop_price, na ficha do item).</p>
            {/if}
            <div class="linha">
              <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { campo: vagaSel!, espaco: espacoDasAbas })}>Trocar</button>
              <button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => limparVaga(vagaSel!)}><Icone nome="lixeira" tamanho={16} /> Tirar</button>
              {#if espacoDasAbas === "ESSENCE" || prod}<button class="botao pequeno fantasma" type="button" onclick={() => abrirItemEmNovaAba(prod ? prod.id : id)} aria-label="Abrir o item em outra aba"><Icone nome="abrir" tamanho={16} /></button>{/if}
            </div>
          {:else}
            <p class="mini">Escolha uma vaga para ver {espacoDasAbas === "RECIPE" ? "a receita" : "os custos"}.</p>
          {/if}
          {#if tabela === "NPC_MAKE_SERVICE"}
            <label class="mini">Habilidade de produção exigida
              <button class="campo escolha" type="button" disabled={!editavel} onclick={() => (escolhendoHabProd = true)}>{nomeDaHabilidadeDeProducao}</button></label>
          {/if}
        </aside>
      </div>
      {#if receitaSel != null}
        {#key receitaSel}<EditorDeReceita indice={receitaSel} aoFechar={() => (vagaSel = null)} />{/key}
      {/if}
    {:else if lista}
      <h4>{espacoLista === "MISSAO" ? "Missões" : espacoLista === "HABILIDADE" ? "Habilidades" : "Itens"} <span class="mini">{idsDaLista.length} de {lista.campos.length}</span></h4>
      <ol class="lista">
        {#each idsDaLista as id, k (id)}
          <li>
            <span class="n">{k + 1}</span>
            {#if espacoLista === "ESSENCE"}<IconeItem hex={iconeDe("ESSENCE", id)} nome={nomeDe("ESSENCE", id)} tamanho={24} />{/if}
            <span class="nome-lista"><TextoDoJogo texto={nomeNaLista(id) ?? `id ${id}`} padrao="var(--texto)" linha /></span>
            <span class="mini">id {id}</span>
            {#if espacoLista === "MISSAO"}<a class="botao pequeno fantasma" href="#/missoes/{id}?{parametros()}" target="_blank" rel="noopener" aria-label="Abrir a missão"><Icone nome="abrir" tamanho={16} /></a>{/if}
            <button class="botao pequeno fantasma" type="button" disabled={!editavel || k === 0} onclick={() => { const l = [...idsDaLista]; [l[k - 1], l[k]] = [l[k], l[k - 1]]; gravarLista(l); }} aria-label="Subir"><Icone nome="cima" tamanho={16} /></button>
            <button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => gravarLista(idsDaLista.filter((x) => x !== id))} aria-label="Tirar"><Icone nome="lixeira" tamanho={16} /></button>
          </li>
        {/each}
      </ol>
      <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => {
        if (espacoLista === "MISSAO") escolhendoMissao = true;
        else if (espacoLista === "HABILIDADE") escolhendoHabilidade = true;
        else escolhendo = { campo: "", espaco: espacoLista, categoria: tabela.includes("INSTALL") ? "STONE_ESSENCE" : "" };
      }}><Icone nome="mais" tamanho={16} /> Acrescentar</button>
    {:else if eTeleporte}
      <h4>Destinos <span class="mini">{usadosNoTeleporte.length} de {destinos.length}</span></h4>
      <table class="tabela">
        <thead><tr><th>Destino (world_targets)</th><th>Custo</th><th>Nível mínimo</th><th></th></tr></thead>
        <tbody>
          {#each linhasDeDestino() as l, i (i)}
            <tr>
              <td>{nomesDeDestino[l.id] ?? `destino ${l.id}`} <span class="mini">id {l.id}</span></td>
              <td><input class="campo n" type="number" min="0" disabled={!editavel} value={l.fee} oninput={(e) => { const x = linhasDeDestino(); x[i].fee = Number((e.target as HTMLInputElement).value); gravarDestinos(x); }} /></td>
              <td><input class="campo n" type="number" min="0" disabled={!editavel} value={l.nivel} oninput={(e) => { const x = linhasDeDestino(); x[i].nivel = Number((e.target as HTMLInputElement).value); gravarDestinos(x); }} /></td>
              <td><button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => gravarDestinos(linhasDeDestino().filter((_, k) => k !== i))} aria-label="Tirar destino"><Icone nome="lixeira" tamanho={16} /></button></td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div class="linha">
        <select class="campo" bind:value={mapaDoTeleporte} aria-label="Mapa dos destinos">
          <option value="">— escolher destino num mapa —</option>
          {#each mapas as m, k (k)}<option value={m.caminho}>{m.nome} ({m.caminho})</option>{/each}
        </select>
        {#if mapaDoTeleporte}
          <select class="campo" aria-label="Destino" disabled={!editavel} onchange={(e) => {
            const id = Number((e.target as HTMLSelectElement).value); (e.target as HTMLSelectElement).value = "";
            if (!id) return;
            const l = linhasDeDestino();
            if (l.some((x) => x.id === id)) { avisar("aviso", "Esse destino já está na lista."); return; }
            if (l.length >= destinos.length) { avisar("aviso", `Cabem ${destinos.length} destinos.`); return; }
            gravarDestinos([...l, { id, fee: 0, nivel: 0 }]);
          }}>
            <option value="">+ destino…</option>
            {#each pontosDoMapa as p (p.id)}<option value={p.id}>{p.nome ?? `destino ${p.id}`} (id {p.id})</option>{/each}
          </select>
        {/if}
      </div>
      <p class="mini">O jogador escolhe pela posição na lista; o servidor confere nível e dinheiro e cobra (B68: GP_NPCSEV_TRANSMIT).</p>
    {:else if eItensDeMissao}
      <h4>Itens que o NPC entrega para cada missão</h4>
      <ol class="lista">
        {#each missoesDeItem.filter((k) => num(`tasks_${k}_id_task`) > 0) as k (k)}
          {@const idm = num(`tasks_${k}_id_task`)}
          <li class="bloco">
            <div class="linha"><strong><TextoDoJogo texto={nomesDeMissao[idm] ?? `missão ${idm}`} padrao="var(--texto)" /></strong> <span class="mini">id {idm}</span>
              <button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => { valores[`tasks_${k}_id_task`] = 0; for (let j = 1; j <= 4; j++) { if (`tasks_${k}_taks_matters_${j}_id_matter` in valores) { valores[`tasks_${k}_taks_matters_${j}_id_matter`] = 0; valores[`tasks_${k}_taks_matters_${j}_num_matter`] = 0; } } }} aria-label="Tirar a missão"><Icone nome="lixeira" tamanho={16} /></button></div>
            <div class="itens-missao">
              {#each [1, 2, 3, 4].filter((j) => `tasks_${k}_taks_matters_${j}_id_matter` in porNome) as j (j)}
                {@const ci = `tasks_${k}_taks_matters_${j}_id_matter`}
                <div class="item-missao">
                  <button class="slot" type="button" disabled={!editavel} onclick={() => (escolhendo = { campo: ci, espaco: "ESSENCE", categoria: "TASKMATTER_ESSENCE" })}>
                    {#if num(ci)}<IconeItem hex={iconeDe("ESSENCE", num(ci))} nome={nomeDe("ESSENCE", num(ci))} tamanho={28} />{:else}<Icone nome="mais" tamanho={14} />{/if}</button>
                  <input class="campo n" type="number" min="0" disabled={!editavel} aria-label="Quantidade" value={valores[`tasks_${k}_taks_matters_${j}_num_matter`]}
                    oninput={(e) => (valores[`tasks_${k}_taks_matters_${j}_num_matter`] = Number((e.target as HTMLInputElement).value))} />
                </div>
              {/each}
            </div>
          </li>
        {/each}
      </ol>
      <button class="botao pequeno" type="button" disabled={!editavel || !missoesDeItem.some((k) => !num(`tasks_${k}_id_task`))}
        onclick={() => (escolhendoMissaoDeItem = missoesDeItem.find((k) => !num(`tasks_${k}_id_task`)) ?? null)}><Icone nome="mais" tamanho={16} /> Missão</button>
    {/if}

    {#if outros.length}
      <h4>Outros campos</h4>
      <CamposSimples nomes={outros} {porNome} bind:valores {originais} {editavel}
        rotulos={{ id_dialog: "Conversa antes de abrir (id_dialog)", fee: "Taxa", id_decompose_skill: "Habilidade de decompor", id_object_need: "Item exigido", price: "Preço" }}
        explicacoes={{ id_dialog: "Mostrada antes do serviço abrir (DlgNPC.cpp:2854-3023). Edite o texto na aba Conversa do NPC.",
          fee: tabela === "NPC_IDENTIFY_SERVICE" ? "Negativa = o NPC não avalia (B379)." : "" }} />
    {/if}
  </div>
{/if}

{#if escolhendo}
  {@const e = escolhendo}
  <SeletorDeEspaco titulo={e.espaco === "RECIPE" ? "Escolher receita" : "Escolher item"} espaco={e.espaco} categoriaInicial={e.categoria ?? ""} soItens={e.espaco === "ESSENCE"}
    aoFechar={() => (escolhendo = null)} aoEscolher={(r) => {
      if (e.campo) escolherItem(e.campo, e.espaco, r);
      else { if (r.id) nomes[`${e.espaco}:${r.id}`] = { nome: r.nome, icone: r.icone ?? null }; acrescentar(r.id ?? 0); }
    }} />
{/if}
{#if escolhendoMissao}<SeletorDeMissao aoFechar={() => (escolhendoMissao = false)} aoEscolher={(m) => { nomesDeMissao[m.id] = m.nome; acrescentar(m.id); }} />{/if}
{#if escolhendoHabilidade}<SeletorDeHabilidade aoFechar={() => (escolhendoHabilidade = false)} aoEscolher={(h) => { nomesDeHabilidade[h.id] = h.nome; acrescentar(h.id); }} />{/if}
{#if escolhendoHabProd}<SeletorDeHabilidade titulo="Habilidade de produção" aoFechar={() => (escolhendoHabProd = false)} aoEscolher={(h) => (valores.id_make_skill = h.id)} />{/if}
{#if escolhendoMissaoDeItem != null}
  {@const k = escolhendoMissaoDeItem}
  <SeletorDeMissao aoFechar={() => (escolhendoMissaoDeItem = null)} aoEscolher={(m) => { nomesDeMissao[m.id] = m.nome; valores[`tasks_${k}_id_task`] = m.id; }} />
{/if}

<style>
  .servico { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .topo { display: flex; gap: 12px; align-items: flex-end; flex-wrap: wrap; }
  .nome { flex: 1; min-width: 260px; display: flex; flex-direction: column; gap: 4px; }
  .acoes { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .usado { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
  .chip { font-size: 12px; padding: 2px 8px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto); text-decoration: none; }
  a.chip:hover { border-color: var(--ouro); }
  h4 { margin: 6px 0 4px; font-size: 13px; color: var(--texto-2); }
  .abas-servico { display: flex; gap: 4px; flex-wrap: wrap; }
  .abas-servico button { display: flex; gap: 6px; align-items: center; max-width: 180px; padding: 4px 10px; border-radius: 8px; border: 1px solid var(--borda-forte);
    background: var(--fundo-2); color: var(--texto-2); font: inherit; font-size: 12px; cursor: pointer; }
  .abas-servico button.on { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .grade-e-lado { display: grid; grid-template-columns: auto minmax(200px, 1fr); gap: 14px; align-items: start; }
  .titulo-aba { display: flex; flex-direction: column; gap: 3px; }
  .titulo-aba input { height: 30px; }
  .slots { display: grid; gap: 4px; margin-top: 8px; }
  .slot { width: 44px; height: 44px; border-radius: 6px; border: 1px solid #3a3122; background: #12161c; display: grid; place-items: center; cursor: pointer;
    padding: 0; color: #7d8896; position: relative; }
  .slot.vazio { border-style: dashed; border-color: #2f3d4c; }
  .slot.sel { outline: 2px solid var(--ouro); outline-offset: 1px; }
  .slot.mudou::after { content: ""; position: absolute; top: 2px; right: 2px; width: 6px; height: 6px; border-radius: 50%; background: var(--ouro); }
  .receita-mini { font-size: 9px; line-height: 1.1; color: #f1e6c8; padding: 2px; text-align: center; overflow: hidden; }
  .lado { display: flex; flex-direction: column; gap: 6px; border: 1px solid var(--borda); border-radius: 10px; padding: 10px; background: var(--fundo-2); }
  .lado h4 { margin: 0; color: var(--texto); }
  .lado label { display: flex; flex-direction: column; gap: 3px; }
  .escolha { text-align: left; cursor: pointer; }
  .lista { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 420px; overflow: auto; }
  .lista li { display: flex; align-items: center; gap: 8px; padding: 4px 8px; border: 1px solid var(--borda); border-radius: 8px; background: var(--fundo-2); }
  .lista li.bloco { flex-direction: column; align-items: stretch; }
  .n { width: 24px; color: var(--texto-3); font-size: 12px; text-align: right; }
  .nome-lista { flex: 1; min-width: 0; }
  .itens-missao { display: flex; gap: 10px; flex-wrap: wrap; }
  .item-missao { display: flex; gap: 4px; align-items: center; }
  .item-missao .slot { width: 36px; height: 36px; }
  .campo.n { width: 90px; height: 30px; }
  .tabela { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabela th { text-align: left; font-size: 11px; color: var(--texto-3); text-transform: uppercase; padding: 4px 6px; border-bottom: 1px solid var(--borda-forte); }
  .tabela td { padding: 4px 6px; border-bottom: 1px solid var(--borda); }
  @media (max-width: 900px) { .grade-e-lado { grid-template-columns: 1fr; } }
</style>
