<script lang="ts">
  /* B371: edição de Monstro (`MONSTER_ESSENCE`, `ExpTypes.h:2781-2960`). Os campos confusos (comportamento, ódio,
     habilidades) vêm explicados pelo que o servidor faz com eles (`lib/monstro.ts`); o spawn edita o npcgen
     (`BoxDeSpawn`). Gravar = `PUT tabelas/MONSTER_ESSENCE/{i}` com os campos mudados, no rascunho do alto; o resto dos
     campos fica nos Editores. */
  import { ficha as lerFicha, linkDosEditores, salvar, type Campo, type Ficha } from "../lib/api";
  import { camposDaBox, rotuloGerado } from "../lib/catalogo";
  import { acharLista, gravarLista, lerLista, paraGravar, porcentagens, ajustar, type Cartao, type LinhaDeHabilidade, type Lista } from "../lib/chances";
  import { APOS_A_MORTE, ATITUDES, bitsLigados, CAMPOS_DO_MONSTRO, GRUPOS, LADOS, CHAVES_DO_MONSTRO, DISTANCIAS_DO_MONSTRO, ESTILOS_DE_LUTA, HABITATS, IMUNIDADES,
    PATRULHAS, POLITICAS_DE_ODIO, rotuloDoMonstro, BOOLEANOS, EXPLICACOES, PAPEIS_NA_GUERRA } from "../lib/monstro";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";
  import IconeItem from "../componentes/IconeItem.svelte";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import BoxDeSpawn from "../componentes/BoxDeSpawn.svelte";
  import ChancesSimples from "../componentes/ChancesSimples.svelte";
  import Icone from "../componentes/Icone.svelte";
  import ListaDeChances from "../componentes/ListaDeChances.svelte";
  import ListaDeHabilidades from "../componentes/ListaDeHabilidades.svelte";
  import ModalCategoria from "../componentes/ModalCategoria.svelte";
  import ModalMascara from "../componentes/ModalMascara.svelte";
  import ModeloDoMonstro from "../componentes/ModeloDoMonstro.svelte";

  let { tabela, indice }: { tabela: string; indice: number } = $props();

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let nomes = $state<Record<string, string>>({});
  let gravando = $state(false);
  let icones = $state<Record<number, string | null>>({}); // ícone do item escolhido na tela (antes de gravar)
  let ovoIcone = $state<string | null>(null);
  let modal = $state("");

  const editavel = $derived(!!estudio.rascunho);
  const existe = (n: string) => n in porNome;
  const num = (n: string) => Number(valores[n] ?? 0);
  const alterado = (n: string) => valores[n] !== originais[n];
  const nomesDaFicha = $derived(Object.keys(porNome));
  const campos = (lista: string[]) => camposDaBox({ titulo: "", campos: lista }, nomesDaFicha);

  async function carregar() {
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      const n: Record<string, string> = {};
      for (const c of x.campos) if (c.referencia?.nome) n[c.nome] = c.referencia.nome;
      nomes = n;
      f = x;
    } catch (e) { erro = (e as Error).message; }
  }
  carregar();

  /* ---- gravar ---- */
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  async function gravar() {
    const c: Record<string, number | string> = {};
    for (const n of mudados) {
      const t = porNome[n].tipo, v = valores[n];
      if (t === "int32" || t === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x) || (t === "int32" && !Number.isInteger(x))) { avisar("aviso", `${rotulo(n)}: número inválido.`); return; }
        c[n] = x;
      } else c[n] = String(v);
    }
    if (!Object.keys(c).length) { avisar("aviso", "Nada mudou."); return; }
    gravando = true;
    try {
      const r = await salvar(tabela, indice, c);
      avisar("ok", r.igual_ao_publicado ? "Igual ao publicado: o elements saiu do rascunho." : `${r.resultado?.campos_mudados ?? Object.keys(c).length} campo(s) gravados no rascunho.`);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }
  const rotulo = (n: string) => rotuloDoMonstro(n) || porNome[n]?.rotulo || rotuloGerado(n) || n;

  /* ---- ódio (aggro_strategy_N): até 4 políticas sorteadas pelo peso ao nascer ---- */
  const odio = $derived(f ? acharLista(nomesDaFicha, "aggro_strategy", "id", "probability") : null);
  const linhasDeOdio = $derived.by(() => {
    if (!odio) return [];
    // Linha usada = peso > 0 (o servidor pula as de peso < 1e-7, `npcgenerator.cpp:303`); o id 0 é a política Normal.
    return odio.ids.map((c, k) => ({ k, id: num(c), peso: num(odio.chances[k]) })).filter((l) => l.peso > 1e-7);
  });
  function gravarOdio(l: { id: number; peso: number }[]) {
    if (!odio) return;
    const p = paraGravar(l.map((x) => x.peso));
    odio.ids.forEach((c, k) => { valores[c] = k < l.length ? l[k].id : 0; valores[odio.chances[k]] = k < l.length ? p[k] : 0; });
  }

  /* ---- habilidades ---- */
  function lista(prefixo: string, id: string, extra: string): Lista | null { return f ? acharLista(nomesDaFicha, prefixo, id, extra) : null; }
  function lerHabilidades(prefixo: string, comChance: boolean): LinhaDeHabilidade[] {
    const l = lista(prefixo, "id_skill", "level");
    if (!l) return [];
    return l.ids.map((c, k) => ({ id: num(c), nivel: num(l.chances[k]), peso: comChance ? num(c.replace("_id_skill", "_probability")) : undefined }))
      .filter((x) => x.id > 0);
  }
  function gravarHabilidades(prefixo: string, comChance: boolean, linhas: LinhaDeHabilidade[]) {
    const l = lista(prefixo, "id_skill", "level");
    if (!l) return;
    const p = comChance ? paraGravar(linhas.map((x) => x.peso ?? 0)) : [];
    l.ids.forEach((c, k) => {
      const x = linhas[k];
      valores[c] = x ? x.id : 0;
      valores[l.chances[k]] = x ? x.nivel : 0;
      if (comChance) valores[c.replace("_id_skill", "_probability")] = x ? p[k] : 0;
    });
  }
  const FAIXAS = [["skill_hp75", "Vida abaixo de 75%"], ["skill_hp50", "Vida abaixo de 50%"], ["skill_hp25", "Vida abaixo de 25%"]] as const;

  /* ---- drops (drop_matters_N): com mais de uma rodada a posição conta ---- */
  const drops = $derived(f ? acharLista(nomesDaFicha, "drop_matters", "id", "probability") : null);
  const varias = $derived(num("drop_times") > 1);
  function cartoesDeDrop(): (Cartao & { posicao: number })[] {
    if (!drops) return [];
    return drops.ids.map((c, k) => ({ k, c })).filter(({ c }) => num(c) > 0).map(({ c, k }) => ({
      id: num(c), peso: num(drops.chances[k]), posicao: k, nome: nomes[c] ?? nomes[`item:${num(c)}`] ?? "",
      icone: valores[c] === originais[c] ? porNome[c]?.referencia?.icone ?? null : icones[num(c)] ?? null,
      extra: varias ? `linha ${k + 1}${k >= 16 ? " · só na 1ª rodada" : ""}` : "",
    }));
  }
  function gravarDrops(l: (Cartao & { posicao?: number })[]) {
    if (!drops) return;
    for (const x of l) { if (x.nome) nomes[`item:${x.id}`] = x.nome; if (x.icone !== undefined) icones[x.id] = x.icone ?? null; }
    if (!varias) { Object.assign(valores, gravarLista(drops, l)); return; }
    // Várias rodadas: sem compactar (`elementdataman.cpp`, MONSTER_ESSENCE: "不能进行去零操作"); item novo na 1ª linha livre
    // de 1–16 (vale em todas as rodadas), depois de 17–32.
    const ocupadas = new Set(l.filter((x) => x.posicao != null).map((x) => x.posicao!));
    const livre = () => { for (let k = 0; k < drops.ids.length; k++) if (!ocupadas.has(k)) { ocupadas.add(k); return k; } return -1; };
    const comPos = l.map((x) => ({ ...x, posicao: x.posicao ?? livre() })).filter((x) => x.posicao >= 0);
    const p = paraGravar(comPos.map((x) => x.peso));
    drops.ids.forEach((c, k) => { valores[c] = 0; valores[drops.chances[k]] = 0; });
    comPos.forEach((x, i) => { valores[drops.ids[x.posicao]] = x.id; valores[drops.chances[x.posicao]] = p[i]; });
  }
  const simples = (prefixo: string) => { const r: string[] = []; for (let k = 0; existe(`${prefixo}${k}`); k++) r.push(`${prefixo}${k}`); return r; };

  const marcados = (opcoes: [number, string][], n: string) => opcoes.filter(([b]) => num(n) & b).map(([, x]) => x).join(", ") || "nenhum";
  const fmt = (x: number) => x.toLocaleString("pt-BR", { maximumFractionDigits: 1 });
</script>

{#snippet campo(n: string)}
  {#if existe(n)}
    {@const c = porNome[n]}
    <label title={n}>
      <span>{rotulo(n)}</span>
      {#if n === "id_pet_egg_captured"}
        <button class="campo escolha" type="button" class:alterado={alterado(n)} disabled={!editavel} onclick={() => (modal = "ovo")} title="Escolher o ovo (PET_EGG_ESSENCE)">
          {#if num(n)}<IconeItem hex={ovoIcone ?? c.referencia?.icone} nome="" tamanho={18} discreto />{/if}
          {num(n) ? (nomes[n] ?? c.referencia?.nome ?? `id ${num(n)}`) : "nenhum (não captura)"}</button>
      {:else if BOOLEANOS.has(n) || n === "role_in_war"}
        <select class="campo" class:alterado={alterado(n)} disabled={!editavel} value={String(valores[n])}
          onchange={(e) => (valores[n] = Number((e.target as HTMLSelectElement).value))}>
          {#each n === "role_in_war" ? PAPEIS_NA_GUERRA : ["0 · não", "1 · sim"] as d, k (k)}<option value={String(k)}>{d}</option>{/each}
          {#if num(n) > (n === "role_in_war" ? 1 : 1)}<option value={String(num(n))}>{num(n)} (outro)</option>{/if}
        </select>
      {:else if n === "short_range_mode" || n === "inhabit_type"}
        <select class="campo" class:alterado={alterado(n)} disabled={!editavel} value={String(valores[n])}
          onchange={(e) => (valores[n] = Number((e.target as HTMLSelectElement).value))}>
          {#each n === "inhabit_type" ? HABITATS.map((h, k) => `${k} · ${h}`) : DISTANCIAS_DO_MONSTRO as d, k (k)}<option value={String(k)}>{d}</option>{/each}
        </select>
      {:else if c.tipo === "int32" || c.tipo === "float"}
        <input class="campo" class:alterado={alterado(n)} type="number" step={c.tipo === "float" ? "any" : "1"} disabled={!editavel}
          value={valores[n]} oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
      {:else}
        <input class="campo" class:alterado={alterado(n)} disabled={!editavel} value={valores[n]} oninput={(e) => (valores[n] = (e.target as HTMLInputElement).value)} />
      {/if}
      {#if n !== "id_pet_egg_captured" && c.referencia?.nome && valores[n] === originais[n]}<span class="mini">{c.referencia.nome}</span>{/if}
      {#if EXPLICACOES[n]}<span class="mini explicacao">{EXPLICACOES[n]}</span>{/if}
    </label>
  {/if}
{/snippet}

{#snippet escolhaGrande(n: string, opcoes: { valor: number; nome: string; explica: string }[])}
  <div class="escolhas" role="radiogroup" aria-label={rotulo(n)}>
    {#each opcoes as o (o.valor)}
      <button type="button" role="radio" aria-checked={num(n) === o.valor} class:ativo={num(n) === o.valor} class:alterado={num(n) === o.valor && alterado(n)}
        disabled={!editavel} onclick={() => (valores[n] = o.valor)} title={o.explica}>
        <strong>{o.nome}</strong><span class="mini">{o.explica}</span>
      </button>
    {/each}
  </div>
{/snippet}

{#if erro}
  <p class="caixa erro">{erro}</p>
{:else if !f}
  <p class="suave">Carregando…</p>
{:else}
  <div class="barra">
    <button class="botao" type="button" onclick={() => ir("")}><Icone nome="voltar" /> Voltar</button>
    <span class="suave">Monstro · id {f.id} · registro {indice}</span>
    <a class="botao fantasma" href={linkDosEditores(tabela, indice)} target="_blank" rel="noopener"><Icone nome="editores" /> Abrir nos Editores</a>
    <span class="espaco"></span>
    {#if mudados.length}<span class="mini">{mudados.length} campo(s) por gravar</span>{/if}
    <button class="botao" type="button" disabled={gravando} onclick={() => (valores = { ...originais })}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || gravando} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
  </div>

  <div class="layout">
    <div class="coluna">
      <section class="caixa principal">
        <div class="dados">
          <input class="campo nome" class:alterado={alterado("Name")} disabled={!editavel} value={valores.Name}
            oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} aria-label="Nome" maxlength="32" />
          <div class="linha">
            <button class="botao pequeno" type="button" onclick={() => (modal = "categoria")}>Tipo: {nomes.id_type ?? `id ${valores.id_type}`}</button>
          </div>
          <div class="grade-campos">{#each campos(["level", "show_level", "prop", "desc", "size", "inhabit_type", "short_range_mode"]) as n (n)}{@render campo(n)}{/each}</div>
        </div>
      </section>

      <section class="caixa">
        <h3>Vida e defesa</h3>
        <div class="grade-campos">{#each campos([...CAMPOS_DO_MONSTRO.vida, ...CAMPOS_DO_MONSTRO.defesaExtra]) as n (n)}{@render campo(n)}{/each}</div>
        {#if existe("immune_type")}
          <div class="linha marcas"><button class="botao pequeno" type="button" class:alterado={alterado("immune_type")} onclick={() => (modal = "imune")}>Imune a</button>
            <span class="suave">{marcados(IMUNIDADES, "immune_type")}</span></div>
        {/if}
      </section>

      <section class="caixa">
        <h3>Ataque</h3>
        <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.ataque) as n (n)}{@render campo(n)}{/each}</div>
      </section>

      <!-- Comportamento: os campos confusos, por pergunta -->
      <section class="caixa">
        <h3>Comportamento</h3>
        {#if existe("aggressive_mode")}<h4>Quando ataca?</h4>{@render escolhaGrande("aggressive_mode", ATITUDES)}{/if}
        {#if existe("id_strategy")}
          <h4>Como luta?</h4>{@render escolhaGrande("id_strategy", ESTILOS_DE_LUTA)}
          {#if (num("id_strategy") === 2 || num("id_strategy") === 3) && !lerHabilidades("skills", false).length}
            <p class="erro mini">Estilo mágico sem nenhuma habilidade: o servidor avisa e o monstro fica sem o que usar (npcgenerator.cpp:288).</p>
          {/if}
        {/if}
        {#if existe("patroll_mode")}<h4>Quando está ocioso?</h4>{@render escolhaGrande("patroll_mode", PATRULHAS.map((p) => ({ ...p, valor: p.valor })))}{/if}
        <h4>O que ele enxerga e até onde persegue</h4>
        <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.visao) as n (n)}{@render campo(n)}{/each}</div>
        <p class="mini">Visão = de onde nota o jogador; distância de ódio = de onde o agressivo ataca; tempo de ódio = segundos que guarda a raiva (≤ 0 vira 1);
          perseguição máxima: 0 = sem limite, abaixo de 20 vira 20 (npcgenerator.cpp:230, 314-317).</p>
        {#if campos(CAMPOS_DO_MONSTRO.invisibilidade).length}
          <h4>Invisibilidade</h4>
          <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.invisibilidade) as n (n)}{@render campo(n)}{/each}</div>
          <p class="mini">O GS soma o nível do monstro aos dois quando não são 0; detecção negativa vira 0 (npcgenerator.cpp:322-326).
            Um jogador só vê o invisível se a detecção dele alcançar o nível de invisibilidade.</p>
        {/if}
        <h4>Movimento</h4>
        <div class="grade-campos">{#each campos([...CAMPOS_DO_MONSTRO.movimento, ...CAMPOS_DO_MONSTRO.movimentoExtra]) as n (n)}{@render campo(n)}{/each}</div>

        {#if odio}
          <h4>Em quem ele foca (estratégia de ódio)</h4>
          <p class="mini">Ao nascer, o monstro sorteia uma das políticas pelos pesos (npcgenerator.cpp:2562). Se nenhuma tiver peso, fica a Normal.</p>
          <details class="legenda"><summary class="mini">O que cada política faz</summary>
            <ul>{#each POLITICAS_DE_ODIO as p (p.valor)}<li><strong>{p.valor} · {p.nome}</strong> — <span class="mini">{p.explica}</span></li>{/each}</ul>
          </details>
          <ul class="odio">
            {#each linhasDeOdio as l, i (l.k)}
              <li>
                <select class="campo" disabled={!editavel} value={String(l.id)} aria-label="Política"
                  onchange={(e) => gravarOdio(linhasDeOdio.map((x, k) => (k === i ? { ...x, id: Number((e.target as HTMLSelectElement).value) } : x)))}>
                  {#each POLITICAS_DE_ODIO as p (p.valor)}<option value={String(p.valor)}>{p.nome}</option>{/each}
                </select>
                <span class="mini explica">{POLITICAS_DE_ODIO.find((p) => p.valor === l.id)?.explica}</span>
                <input type="range" min="0" max="1000" disabled={!editavel || linhasDeOdio.length < 2} aria-label="Peso"
                  value={Math.round(porcentagens(linhasDeOdio.map((x) => x.peso))[i] * 10)}
                  oninput={(e) => { const p = ajustar(linhasDeOdio.map((x) => x.peso), i, Number((e.target as HTMLInputElement).value) / 1000); gravarOdio(linhasDeOdio.map((x, k) => ({ ...x, peso: p[k] }))); }} />
                <span class="pct">{fmt(porcentagens(linhasDeOdio.map((x) => x.peso))[i])}%</span>
                <button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Remover"
                  onclick={() => gravarOdio(linhasDeOdio.filter((_, k) => k !== i))}><Icone nome="lixeira" tamanho={16} /></button>
              </li>
            {/each}
          </ul>
          <button class="botao pequeno" type="button" disabled={!editavel || linhasDeOdio.length >= odio.ids.length}
            onclick={() => gravarOdio([...linhasDeOdio, { id: 0, peso: linhasDeOdio.length ? 1 / (linhasDeOdio.length + 1) : 1 }])}><Icone nome="mais" tamanho={16} /> Adicionar política</button>
        {/if}

        <h4>Lado e ajuda entre monstros</h4>
        <div class="ajuda">
          {#if existe("faction")}
            <div><button class="botao pequeno" type="button" class:alterado={alterado("faction")} onclick={() => (modal = "lado")}>Lado</button>
              <span class="suave">{LADOS.filter(([b]) => (num("faction") >>> 0) & b).map(([, n]) => n).join(", ") || "nenhum"}</span>
              <p class="mini">Decide quem é inimigo dele (tabela ENEMY_FACTION_CONFIG, nos Editores). Monstro comum costuma ser só "30 · Monstro comum".</p></div>
          {/if}
          {#if existe("monster_faction")}
            <div><button class="botao pequeno" type="button" class:alterado={alterado("monster_faction")} onclick={() => (modal = "grupo")}>Grupo do monstro</button>
              <span class="suave">{bitsLigados(num("monster_faction")).map((k) => `grupo ${k}`).join(", ") || "nenhum"}</span></div>
          {/if}
          {#if existe("monster_faction_ask_help")}
            <div><button class="botao pequeno" type="button" class:alterado={alterado("monster_faction_ask_help")} onclick={() => (modal = "pede")}>Quando atacado, pede ajuda a</button>
              <span class="suave">{bitsLigados(num("monster_faction_ask_help")).map((k) => `grupo ${k}`).join(", ") || "ninguém (não pede)"}</span></div>
          {/if}
          {#if existe("monster_faction_can_help")}
            <div><button class="botao pequeno" type="button" class:alterado={alterado("monster_faction_can_help")} onclick={() => (modal = "atende")}>Atende pedido de</button>
              <span class="suave">{bitsLigados(num("monster_faction_can_help")).map((k) => `grupo ${k}`).join(", ") || "ninguém (não ajuda)"}</span></div>
          {/if}
          <p class="mini">Como funciona: ao ganhar ódio de alguém, o monstro que pede ajuda grita (no máximo a cada 4 s) para quem está até a
            distância da <strong>visão</strong> dele (fator 1,0: config.h:67; npc.cpp:2175-2190). Um vizinho entra na luta se as três coisas valem:
            o <em>grupo do vizinho</em> está entre os que o atacado pede, o <em>"atende pedido de"</em> do vizinho inclui o grupo do atacado, e o vizinho
            está com no máximo 1 inimigo — então soma 2 de ódio no atacante (npc.cpp:669-680; ainpc.h:867-876). Cada spawner do npcgen pode trocar
            esses grupos (npcgenerator.cpp:3821-3826).</p>
        </div>

        <h4>Ao morrer e outras chaves</h4>
        <div class="linha marcas">
          {#if existe("after_death")}<button class="botao pequeno" type="button" class:alterado={alterado("after_death")} onclick={() => (modal = "morte")}>Ao morrer</button>
            <span class="suave">{marcados(APOS_A_MORTE, "after_death")}</span>{/if}
          {#if existe("combined_switch")}<button class="botao pequeno" type="button" class:alterado={alterado("combined_switch")} onclick={() => (modal = "chaves")}>Chaves</button>
            <span class="suave">{marcados(CHAVES_DO_MONSTRO, "combined_switch")}</span>{/if}
        </div>
        <h4>Inteligência (IA)</h4>
        <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.ia) as n (n)}{@render campo(n)}{/each}</div>
        <p class="mini">As variáveis locais são o valor inicial que o roteiro de IA lê e muda (npc.cpp:1996; npcgenerator.cpp:2520). O roteiro em si se edita nos Editores → Comportamento.</p>
      </section>

      <section class="caixa">
        <h3>Habilidades</h3>
        <ListaDeHabilidades titulo="Habilidades de combate" maximo={lista("skills", "id_skill", "level")?.ids.length ?? 0}
          explica="O servidor separa por tipo: ataque, bênção e maldição (npcgenerator.cpp:250-275); a IA escolhe entre elas na luta."
          linhas={lerHabilidades("skills", false)} {editavel} aoMudar={(l) => gravarHabilidades("skills", false, l)} />
        {#each FAIXAS as [prefixo, titulo] (prefixo)}
          {#if lista(prefixo, "id_skill", "level")}
            <ListaDeHabilidades {titulo} maximo={lista(prefixo, "id_skill", "level")?.ids.length ?? 0} comChance
              explica="Quando a vida cai abaixo da faixa, sorteia uma destas pelos pesos (normalizados ao carregar)."
              linhas={lerHabilidades(prefixo, true)} {editavel} aoMudar={(l) => gravarHabilidades(prefixo, true, l)} />
          {/if}
        {/each}
        {#if campos(CAMPOS_DO_MONSTRO.legado).length}
          <h4>Campos antigos</h4>
          <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.legado) as n (n)}{@render campo(n)}{/each}</div>
        {/if}
      </section>

      <section class="caixa">
        <h3>Recompensa e drops</h3>
        <div class="grade-campos">{#each campos([...CAMPOS_DO_MONSTRO.recompensa, "id_pet_egg_captured", "drop_times", "drop_protected", "drop_for_no_profit_time"]) as n (n)}{@render campo(n)}{/each}</div>
        {#if simples("probability_drop_num").length}
          <ChancesSimples titulo="Quantos itens caem por rodada" rotulos={simples("probability_drop_num").map((_, k) => `${k} item(ns)`)}
            pesos={simples("probability_drop_num").map(num)} {editavel}
            aoMudar={(p) => { const g = paraGravar(p); simples("probability_drop_num").forEach((c, k) => (valores[c] = g[k])); }} />
        {/if}
        {#if varias}<p class="mini">Com {num("drop_times")} rodadas, a posição conta: nas rodadas depois da 1ª, cair numa das linhas 17–32 cancela o drop (elementdataman.cpp, MONSTER_ESSENCE).</p>{/if}
      </section>
      {#if drops}
        <ListaDeChances titulo="Itens que caem" espaco="ESSENCE" maximo={drops.ids.length} linhas={cartoesDeDrop()} {editavel}
          explica="Peso de cada item na lista; o servidor normaliza a soma (NormalizeRandom)." aoMudar={gravarDrops} />
      {/if}

      {#if f.id}<BoxDeSpawn id={f.id} nome={String(valores.Name ?? "")} />{/if}

      {#if campos(CAMPOS_DO_MONSTRO.eventos).length}
        <section class="caixa">
          <h3>Eventos, guerras e território</h3>
          <p class="mini">Só importam em batalha, base de clã, território ou mapa com variáveis globais; num monstro comum ficam em 0.</p>
          <div class="grade-campos">{#each campos(CAMPOS_DO_MONSTRO.eventos) as n (n)}{@render campo(n)}{/each}</div>
        </section>
      {/if}
    </div>
    <aside class="lateral">
      {#if existe("file_model") && String(valores.file_model ?? "").trim()}
        <ModeloDoMonstro caminho={String(originais.file_model)} politica={num("common_strategy")} idDoMonstro={f.id ?? 0}
          aoTrocarModelo={async (ecm) => { await salvar(tabela, indice, { file_model: ecm }); avisar("ok", `O monstro agora usa ${ecm} (no rascunho).`); await carregar(); }}
          habilidades={[...lerHabilidades("skills", false), ...FAIXAS.flatMap(([p]) => lerHabilidades(p, true))].map((h) => h.id)} />
      {/if}
      {#if campos(["file_model", "file_gfx_short", "file_gfx_short_hit"]).length}
        <section class="caixa">
          <h3>Arquivos do modelo e efeitos</h3>
          <div class="grade-campos uma">{#each campos(["file_model", "file_gfx_short", "file_gfx_short_hit"]) as n (n)}{@render campo(n)}{/each}</div>
          <p class="mini">Efeito do golpe e do acerto: arquivos .gfx do cliente (sem prévia aqui — o painel não desenha GFX). O modelo muda ao gravar.</p>
        </section>
      {/if}
    </aside>
  </div>
{/if}

{#if f && modal === "categoria"}
  <ModalCategoria maior="MONSTER_TYPE" idMaior={num("id_type")} idMenor={0} aoFechar={() => (modal = "")}
    aoEscolher={(a) => { valores.id_type = a; nomes.id_type = `id ${a}`; }} />
{:else if f && modal === "lado"}
  <ModalMascara titulo="Lado do monstro (faction)" opcoes={LADOS} valor={num("faction")} aoFechar={() => (modal = "")}
    explica="Bit k = lado k do ENEMY_FACTION_CONFIG (nomes do ExpTypes.h; faction.h do GS)." aoAplicar={(v) => (valores.faction = v)} />
{:else if f && (modal === "grupo" || modal === "pede" || modal === "atende")}
  {@const c = modal === "grupo" ? "monster_faction" : modal === "pede" ? "monster_faction_ask_help" : "monster_faction_can_help"}
  <ModalMascara titulo={modal === "grupo" ? "Grupo do monstro" : modal === "pede" ? "Quando atacado, pede ajuda a" : "Atende pedido de"}
    opcoes={GRUPOS} valor={num(c)} aoFechar={() => (modal = "")}
    explica="Grupos livres (não têm nome em nenhum arquivo): o mesmo número em dois monstros quer dizer o mesmo grupo." aoAplicar={(v) => (valores[c] = v)} />
{:else if f && modal === "ovo"}
  <SeletorDeEspaco titulo="Ovo ao ser capturado" espaco="ESSENCE" categoriaInicial="PET_EGG_ESSENCE" aoFechar={() => (modal = "")}
    aoEscolher={(r) => { valores.id_pet_egg_captured = r.id ?? 0; nomes.id_pet_egg_captured = r.nome; ovoIcone = r.icone ?? null; }} />
{:else if f && modal === "imune"}
  <ModalMascara titulo="Imune a" opcoes={IMUNIDADES} valor={num("immune_type")} aoFechar={() => (modal = "")}
    explica="immune_type (ExpTypes.h): bits de baixo para cima." aoAplicar={(v) => (valores.immune_type = v)} />
{:else if f && modal === "morte"}
  <ModalMascara titulo="Ao morrer" opcoes={APOS_A_MORTE} valor={num("after_death")} aoFechar={() => (modal = "")}
    explica="after_death (ExpTypes.h): 0x1 explode, 0x2 renasce." aoAplicar={(v) => (valores.after_death = v)} />
{:else if f && modal === "chaves"}
  <ModalMascara titulo="Chaves do monstro" opcoes={CHAVES_DO_MONSTRO} valor={num("combined_switch")} aoFechar={() => (modal = "")}
    explica="MONSTER_COMBINED_SWITCH (cgame/gs/template/exptypes.h:2962-2968)." aoAplicar={(v) => (valores.combined_switch = v)} />
{/if}

<style>
  .barra { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; }
  .espaco { flex: 1; }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(320px, 440px); gap: 14px; align-items: start; }
  .coluna { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .lateral { position: sticky; top: 70px; display: flex; flex-direction: column; gap: 14px; max-height: calc(100vh - 80px); overflow: auto; }
  .uma { grid-template-columns: 1fr; }
  .dados { display: flex; flex-direction: column; gap: 8px; }
  .nome { font-size: 18px; font-weight: 600; height: 40px; }
  h4 { margin: 14px 0 6px; font-size: 13px; color: var(--texto-2); }
  .escolhas { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 6px; }
  .escolhas button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 8px 10px; border-radius: 10px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .escolhas button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .marcas { margin-top: 10px; }
  .ajuda { display: flex; flex-direction: column; gap: 8px; }
  .explicacao { line-height: 1.3; }
  .escolha { display: flex; align-items: center; gap: 6px; text-align: left; cursor: pointer; overflow: hidden; white-space: nowrap; }
  .legenda ul { margin: 6px 0; padding-left: 18px; display: flex; flex-direction: column; gap: 3px; }
  .odio { list-style: none; margin: 0 0 8px; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .odio li { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .explica { flex: 1; min-width: 160px; }
  .pct { width: 52px; text-align: right; font-variant-numeric: tabular-nums; }
  input[type="range"] { width: 120px; accent-color: var(--ouro); }
  details summary { cursor: pointer; color: var(--texto-2); }
  @media (max-width: 960px) { .layout { grid-template-columns: 1fr; } .lateral { position: static; } }
</style>
