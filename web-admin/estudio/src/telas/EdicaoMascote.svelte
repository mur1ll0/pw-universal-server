<script lang="ts">
  /* B394: edição de Mascote (`PET_ESSENCE`, `ExpTypes.h:2983-3060`) em cinco abas: Ficha (o que ele é, quem pode usar,
     combate, comida, planta e evolução, com o que o servidor faz com cada campo), Atributos por nível (as fórmulas do
     `GenerateBaseProp` com o valor e a curva em todos os níveis), Ovo (o `PET_EGG_ESSENCE` que o choca), Modelo 3D (o do
     monstro, com as ações por ambiente) e Simulador (contra outro mascote ou um monstro). Gravar = `PUT
     tabelas/PET_ESSENCE/{i}` com os campos mudados, no rascunho do alto. */
  import { buscarIcones, ficha as lerFicha, linkDosEditores, listar, salvar, type Campo, type Ficha } from "../lib/api";
  import { classesDaVersao } from "../lib/catalogo";
  import { avisar, estudio, ir, rota } from "../lib/estado.svelte";
  import { AMBIENTES, atributosNoNivel, COMIDAS, EXPLICACOES, FORMULAS, FORMULAS_DE_INVOCACAO, GRUPOS_DE_PLANTA, IMUNIDADES_DO_MASCOTE,
    POSTURAS, rotuloDoMascote, simulavel, TIPOS_DE_MASCOTE, tipoDoMascote } from "../lib/mascote";
  import Icone from "../componentes/Icone.svelte";
  import IconeItem from "../componentes/IconeItem.svelte";
  import ModalIcone from "../componentes/ModalIcone.svelte";
  import ModalMascara from "../componentes/ModalMascara.svelte";
  import ModeloDoMonstro from "../componentes/ModeloDoMonstro.svelte";
  import OvoDoMascote from "../componentes/OvoDoMascote.svelte";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";
  import SimuladorDeCombate from "../componentes/SimuladorDeCombate.svelte";

  let { tabela, indice }: { tabela: string; indice: number } = $props();

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let nomes = $state<Record<string, string>>({});
  let gravando = $state(false);
  let modal = $state("");
  let iconeHex = $state<string | null>(null);
  let aba = $state(rota.params.get("aba") || "ficha");
  let ovo = $state<{ id: number; nome: string; lealdade: number; nivel: number } | null>(null);
  let nivelDaCurva = $state(0);

  const editavel = $derived(!!estudio.rascunho);
  const existe = (n: string) => n in porNome;
  const num = (n: string) => Number(valores[n] ?? 0) || 0;
  const alterado = (n: string) => valores[n] !== originais[n];

  async function carregar() {
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      const n: Record<string, string> = {};
      for (const c of x.campos) if (c.referencia?.nome) n[c.nome] = c.referencia.nome;
      nomes = n;
      iconeHex = x.icone;
      f = x;
      if (!nivelDaCurva) nivelDaCurva = Math.min(Number(originais.level_max) || 150, Math.max(30, Number(originais.level_require) || 1));
    } catch (e) { erro = (e as Error).message; }
  }
  carregar();
  /* O ovo também fora da aba Ovo (selo do alto, lealdade inicial do simulador): o primeiro que choca este mascote. */
  $effect(() => {
    void estudio.rascunho;
    const id = f?.id;
    if (!id) return;
    listar("PET_EGG_ESSENCE", "", 0, 1, { colunas: ["honor_point", "level"], faixas: { id_pet: { min: id, max: id } } })
      .then((r) => { const o = r.registros[0]; ovo = o ? { id: o.id ?? 0, nome: o.nome, lealdade: Number(o.campos?.honor_point ?? 0), nivel: Number(o.campos?.level ?? 1) } : null; })
      .catch(() => {});
  });
  function trocarAba(a: string) {
    aba = a;
    const p = new URLSearchParams(rota.params);
    p.set("aba", a);
    history.replaceState(null, "", `#/${rota.caminho.join("/")}?${p}`);
  }

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
  const rotulo = (n: string) => rotuloDoMascote(n) || porNome[n]?.rotulo || n;

  /* ---- o que o mascote é ---- */
  const tipo = $derived(tipoDoMascote(num("id_type")));
  const tipos = $derived(TIPOS_DE_MASCOTE.filter((t) => !t.so || t.so === estudio.versao));
  const ambientes = $derived(estudio.versao.startsWith("1.2") ? AMBIENTES.slice(0, 3) : AMBIENTES);
  const classes = $derived(classesDaVersao(estudio.versao).map((n, k) => [1 << k, n] as [number, string]));
  const marcados = (opcoes: [number, string][], n: string) => opcoes.filter(([b]) => (num(n) >>> 0) & b).map(([, x]) => x).join(", ") || "nenhuma";
  /** Ações com sufixo de ambiente: invocado ou ambiente 6 (`EC_Pet.cpp:285-288`, `EC_NPCModel.cpp:588-590`; B281). */
  const habitatDoModelo = $derived(num("id_type") === 28752 || num("inhabit_type") === 6 ? num("inhabit_type") : null);

  /** O que faz o GS recusar o mascote ao carregar (`petdataman.cpp:120-133`; no 1.2.6 só o de combate é conferido). */
  const recusas = $derived.by(() => {
    const confere = estudio.versao.startsWith("1.2") ? num("id_type") === 8782 : !!tipo?.luta;
    if (!f || !confere) return [];
    const tiques = (s: number) => Math.trunc(Math.fround(Math.fround(Math.fround(s) * 20) + Math.fround(0.1)));
    const r: string[] = [];
    const atraso = tiques(num("damage_delay")), intervalo = tiques(num("attack_speed"));
    if (atraso < 2 || atraso > 200) r.push(`Atraso do dano ${num("damage_delay")} s = ${atraso} tiques: precisa ficar entre 2 e 200 (0,1 a 10 s).`);
    if (intervalo <= 2) r.push(`Intervalo do golpe ${num("attack_speed")} s = ${intervalo} tiques: precisa ser mais que 2 (≥ 0,15 s).`);
    if (num("sight_range") < 0.1) r.push("Visão abaixo de 0,1.");
    if (num("hp_a") <= 0) r.push("Coeficiente a da vida ≤ 0.");
    return r;
  });

  /* ---- curvas ---- */
  const maximo = $derived(Math.max(1, num("level_max") || 150));
  const agora = $derived(atributosNoNivel(valores, nivelDaCurva || 1));
  function curva(chave: string): string {
    const pts: number[] = [];
    const passo = Math.max(1, Math.floor(maximo / 40));
    for (let n = 1; n <= maximo; n += passo) pts.push(Number(atributosNoNivel(valores, n)[chave as keyof typeof agora]));
    const max = Math.max(1, ...pts.map(Math.abs));
    return pts.map((y, i) => `${((i / Math.max(1, pts.length - 1)) * 118 + 1).toFixed(1)},${(29 - (y / max) * 27).toFixed(1)}`).join(" ");
  }
  const fmt = (x: number, d = 0) => x.toLocaleString("pt-BR", { maximumFractionDigits: d });
  /** `float` do arquivo com os 7 algarismos que ele guarda (0.100000001 → 0.1); digitar 0.1 grava o mesmo float. */
  const curto = (v: number | string | undefined) => (typeof v === "number" && !Number.isInteger(v) ? Number(v.toPrecision(7)) : v);
  const letra = (k: number) => "abcd"[k];

  const ABAS = [["ficha", "Ficha"], ["atributos", "Atributos por nível"], ["ovo", "Ovo"], ["modelo", "Modelo 3D"], ["simulador", "Simulador de combate"]];
</script>

{#snippet campo(n: string, opcoes?: string[])}
  {#if existe(n)}
    {@const c = porNome[n]}
    <label title={n}>
      <span>{rotulo(n)}</span>
      {#if opcoes}
        <select class="campo" class:alterado={alterado(n)} disabled={!editavel} value={String(valores[n])}
          onchange={(e) => (valores[n] = Number((e.target as HTMLSelectElement).value))}>
          {#each opcoes as d, k (k)}<option value={String(k)}>{d}</option>{/each}
          {#if num(n) >= opcoes.length}<option value={String(num(n))}>{num(n)} (outro)</option>{/if}
        </select>
      {:else if c.tipo === "int32" || c.tipo === "float"}
        <input class="campo" class:alterado={alterado(n)} type="number" step={c.tipo === "float" ? "any" : "1"} disabled={!editavel}
          value={c.tipo === "float" ? curto(valores[n]) : valores[n]} oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
      {:else}
        <input class="campo" class:alterado={alterado(n)} disabled={!editavel} value={valores[n]} oninput={(e) => (valores[n] = (e.target as HTMLInputElement).value)} />
      {/if}
      {#if c.referencia?.nome && valores[n] === originais[n]}<span class="mini">{c.referencia.nome}</span>{/if}
      {#if EXPLICACOES[n]}<span class="mini explicacao">{EXPLICACOES[n]}</span>{/if}
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
    <span class="suave">Mascote · id {f.id} · registro {indice}</span>
    <a class="botao fantasma" href={linkDosEditores(tabela, indice)} target="_blank" rel="noopener"><Icone nome="editores" /> Abrir nos Editores</a>
    <span class="espaco"></span>
    {#if mudados.length}<span class="mini">{mudados.length} campo(s) por gravar</span>{/if}
    <button class="botao" type="button" disabled={gravando || !mudados.length} onclick={() => (valores = { ...originais })}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || gravando || !mudados.length} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
  </div>

  <header class="cabeca caixa">
    <button class="icone" type="button" disabled={!editavel} onclick={() => (modal = "icone")} title="Trocar o ícone"><IconeItem hex={iconeHex} nome="" tamanho={56} /></button>
    <div class="titulo">
      <input class="campo nome" class:alterado={alterado("Name")} disabled={!editavel} value={valores.Name} maxlength="31" aria-label="Nome do mascote"
        oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} />
      <div class="selos">
        <span class="selo ouro">{tipo?.nome ?? `tipo ${num("id_type")}`}</span>
        <span class="selo">{AMBIENTES[num("inhabit_type")] ?? `ambiente ${num("inhabit_type")}`}</span>
        <span class="selo">nível {num("level_require")}–{num("level_max")}</span>
        <span class="selo">{ovo ? `ovo: ${ovo.nome}` : "sem ovo"}</span>
        {#if recusas.length}<span class="selo erro">o GS recusa este mascote</span>{/if}
      </div>
    </div>
    {#if tipo?.luta}
      <div class="resumo">
        <span class="mini">no nível {nivelDaCurva}</span>
        <div><strong>{fmt(agora.vida)}</strong><span class="mini">vida</span></div>
        <div><strong>{fmt(agora.dano)}</strong><span class="mini">dano</span></div>
        <div><strong>{fmt(agora.defesa)}</strong><span class="mini">defesa</span></div>
        <div><strong>{fmt(num("attack_speed"), 2)} s</strong><span class="mini">golpe</span></div>
      </div>
    {/if}
  </header>

  <div class="abas" role="tablist">
    {#each ABAS as [k, n] (k)}
      <button type="button" role="tab" aria-selected={aba === k} class:on={aba === k} onclick={() => trocarAba(k)}>{n}</button>
    {/each}
  </div>

  {#if recusas.length}
    <div class="caixa erro mini">O servidor recusa este mascote ao carregar (petdataman.cpp:120-133) — ele some do jogo:
      <ul>{#each recusas as r (r)}<li>{r}</li>{/each}</ul></div>
  {/if}

  {#if aba === "ficha"}
    <div class="layout">
      <div class="coluna">
        <section class="caixa">
          <h3>O que ele é</h3>
          <div class="tipos" role="radiogroup" aria-label="Tipo">
            {#each tipos as t (t.idType)}
              <button type="button" role="radio" aria-checked={num("id_type") === t.idType} class:on={num("id_type") === t.idType}
                class:alterado={num("id_type") === t.idType && alterado("id_type")} disabled={!editavel} onclick={() => (valores.id_type = t.idType)}>
                <strong>{t.nome}</strong><span class="mini">{t.explica}</span></button>
            {/each}
          </div>
          {#if alterado("id_type")}<p class="mini aviso-texto">Trocar o tipo muda o que o mascote faz no jogo (montaria não luta; planta não anda). Confira as fórmulas.</p>{/if}
          <div class="grade-campos">
            {@render campo("inhabit_type", ambientes.map((a, k) => `${k} · ${a}`))}
            {@render campo("stand_mode", POSTURAS)}
            {@render campo("pet_snd_type")}
          </div>
          {#if estudio.versao.startsWith("1.2")}<p class="mini">No 1.2.6 o servidor só aceita chão, água e ar (outro valor vira chão).</p>{/if}
        </section>

        <section class="caixa">
          <h3>Quem pode ter</h3>
          <div class="grade-campos">{#each ["level_require", "level_max"] as n (n)}{@render campo(n)}{/each}</div>
          {#if existe("character_combo_id")}
            <div class="linha marcas">
              <button class="botao pequeno" type="button" class:alterado={alterado("character_combo_id")} disabled={!editavel} onclick={() => (modal = "classes")}>Classes</button>
              <span class="suave">{marcados(classes, "character_combo_id")}</span>
            </div>
            <p class="mini">{EXPLICACOES.character_combo_id}</p>
          {/if}
        </section>

        {#if tipo?.luta || !tipo}
          <section class="caixa">
            <h3>Como luta</h3>
            <div class="grade-campos">{#each ["attack_range", "size", "attack_speed", "damage_delay", "sight_range"] as n (n)}{@render campo(n)}{/each}</div>
            <p class="mini">Os atributos que crescem com o nível ficam na aba <button class="link" type="button" onclick={() => trocarAba("atributos")}>Atributos por nível</button>;
              para ver como ele se sai, a aba <button class="link" type="button" onclick={() => trocarAba("simulador")}>Simulador de combate</button>.</p>
            {#if existe("immune_type")}
              <div class="linha marcas"><button class="botao pequeno" type="button" class:alterado={alterado("immune_type")} disabled={!editavel} onclick={() => (modal = "imune")}>Imune a</button>
                <span class="suave">{marcados(IMUNIDADES_DO_MASCOTE, "immune_type")}</span></div>
            {/if}
          </section>
        {/if}

        {#if existe("food_mask")}
          <section class="caixa">
            <h3>O que come</h3>
            <div class="chips" role="group" aria-label="Comida">
              {#each COMIDAS as [bit, nome] (bit)}
                <button type="button" aria-pressed={!!(num("food_mask") & bit)} class:on={!!(num("food_mask") & bit)} class:alterado={alterado("food_mask")} disabled={!editavel}
                  onclick={() => (valores.food_mask = num("food_mask") ^ bit)}>{nome}</button>
              {/each}
            </div>
            <p class="mini">A comida (PET_FOOD_ESSENCE) só serve se o tipo dela estiver marcado aqui; comer dá lealdade e tira a fome (petman.cpp:1645-1666).</p>
          </section>
        {/if}

        {#if num("id_type") === 28913 && (existe("plant_group") || existe("group_limit"))}
          <section class="caixa">
            <h3>Planta</h3>
            <div class="grade-campos">
              {@render campo("plant_group", GRUPOS_DE_PLANTA)}
              {@render campo("group_limit")}
              {@render campo("player_gain_skill")}
            </div>
            <p class="mini">O dono pode ter até "limite do grupo" plantas do mesmo grupo ao mesmo tempo (plant_pet_manager::ActivePlant, petman.cpp:2303-2317).</p>
          </section>
        {/if}

        {#if existe("id_pet_egg_evolved")}
          <section class="caixa">
            <h3>Evolução</h3>
            <div class="grade-campos">
              {#if existe("id_pet_egg_evolved")}
                <label><span>{rotulo("id_pet_egg_evolved")}</span>
                  <button class="campo escolha" type="button" class:alterado={alterado("id_pet_egg_evolved")} disabled={!editavel} onclick={() => (modal = "evoluido")}>
                    {num("id_pet_egg_evolved") ? (nomes.id_pet_egg_evolved ?? `id ${num("id_pet_egg_evolved")}`) : "nenhum (não evolui)"}</button></label>
              {/if}
              {#each ["cost_index", "specific_skill", "id_evolved_skill_rand", "require_dye_count"] as n (n)}{@render campo(n)}{/each}
            </div>
            <h4>Quanto o evoluído herda (até %)</h4>
            <div class="grade-campos">{#each ["hp_inherit_max_rate", "attack_inherit_max_rate", "defence_inherit_max_rate", "attack_level_inherit_max_rate", "defence_level_inherit_max_rate"] as n (n)}{@render campo(n)}{/each}</div>
          </section>
        {/if}
      </div>

      <aside class="lateral">
        <section class="caixa">
          <h3>Arquivos</h3>
          <div class="grade-campos uma">{#each ["file_model", "file_icon", "file_gfx_short"] as n (n)}{@render campo(n)}{/each}</div>
          <p class="mini">O modelo e as animações se veem e trocam na aba <button class="link" type="button" onclick={() => trocarAba("modelo")}>Modelo 3D</button>. O efeito do golpe (.gfx) não tem prévia.</p>
        </section>
        <section class="caixa">
          <h3>Ovo</h3>
          {#if ovo}
            <p><strong>{ovo.nome}</strong> <span class="mini">id {ovo.id} · nasce no nível {ovo.nivel} · lealdade {ovo.lealdade}</span></p>
          {:else}<p class="mini">Nenhum ovo choca este mascote: o jogador não tem como conseguir.</p>{/if}
          <button class="botao pequeno" type="button" onclick={() => trocarAba("ovo")}><Icone nome="editar" tamanho={16} /> {ovo ? "Editar o ovo" : "Criar ou ligar um ovo"}</button>
        </section>
      </aside>
    </div>
  {:else if aba === "atributos"}
    <section class="caixa">
      <h3>Atributos por nível</h3>
      <p class="mini">Cada atributo é uma conta com o nível (GenerateBaseProp, petdataman.h:23-76). Mude os coeficientes e veja o valor e a curva na hora;
        o "nível exigido" ({num("level_require")}) também entra. O resultado é truncado como no servidor.</p>
      <label class="nivel"><span>Ver no nível <strong>{nivelDaCurva}</strong></span>
        <input type="range" min="1" max={maximo} value={nivelDaCurva} oninput={(e) => (nivelDaCurva = Number((e.target as HTMLInputElement).value))} /></label>
      <div class="formulas">
        {#each FORMULAS as fo (fo.chave)}
          {#if fo.campos.every(existe)}
            <div class="formula">
              <div class="cab"><strong>{fo.nome}</strong><span class="mini">{fo.texto}</span></div>
              <div class="coef">
                {#each fo.campos as c, k (c)}
                  <label title={c}><span>{letra(k)}</span>
                    <input class="campo" class:alterado={alterado(c)} type="number" step="any" disabled={!editavel} value={curto(valores[c])}
                      oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[c] = t === "" ? "" : Number(t); }} /></label>
                {/each}
              </div>
              <div class="valor">
                <strong>{fmt(Number(agora[fo.chave]), fo.chave === "correr" ? 2 : 0)}</strong>
                <svg viewBox="0 0 120 30" class="spark" aria-hidden="true"><polyline points={curva(fo.chave)} /></svg>
                <span class="mini">nível 1 → {maximo}</span>
              </div>
            </div>
          {/if}
        {/each}
      </div>
      {#if FORMULAS_DE_INVOCACAO.some((x) => existe(x.campo))}
        <h4>Só para invocação e planta (somam com parte dos atributos do dono)</h4>
        <div class="grade-campos">
          {#each FORMULAS_DE_INVOCACAO as x (x.campo)}
            {#if existe(x.campo)}
              <label><span>{x.nome} <span class="mini">({x.texto})</span></span>
                <input class="campo" class:alterado={alterado(x.campo)} type="number" step="any" disabled={!editavel} value={curto(valores[x.campo])}
                  oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[x.campo] = t === "" ? "" : Number(t); }} />
                <span class="mini">nível {nivelDaCurva}: {fmt(Math.trunc(Math.fround(num(x.campo) * (nivelDaCurva + 5))))}</span></label>
            {/if}
          {/each}
        </div>
      {/if}
    </section>
  {:else if aba === "ovo"}
    {#if f.id}<OvoDoMascote idDoMascote={f.id} nomeDoMascote={String(valores.Name ?? "")} aoEscolherOvo={(o) => (ovo = o)} />{/if}
  {:else if aba === "modelo"}
    {#if String(valores.file_model ?? "").trim()}
      <div class="modelo">
        <ModeloDoMonstro caminho={String(originais.file_model)} idDoMonstro={0} habitat={habitatDoModelo}
          aoTrocarModelo={async (ecm) => { await salvar(tabela, indice, { file_model: ecm }); avisar("ok", `O mascote agora usa ${ecm} (no rascunho).`); await carregar(); }} />
        <section class="caixa">
          <h3>Sobre as ações do mascote</h3>
          <p class="mini">O cliente pede ao mascote as mesmas ações do monstro (parado, andar, atacar, morrer…).
            {#if habitatDoModelo != null}Como este é {num("id_type") === 28752 ? "invocado" : "de chão, água e ar"}, cada ação vem com o ambiente no nome
              (<code>_地面</code> chão, <code>_水中</code> água, <code>_空中</code> ar — EC_NPCModel.cpp:588-620): a lista mostra uma linha por ambiente.
            {:else}Sem sufixo de ambiente: só o invocado e o de ambiente "chão, água e ar" pedem ações por ambiente (EC_Pet.cpp:285-288).{/if}</p>
          <p class="mini">Exportar/Importar GLB funciona como no monstro: "Só o modelo", "Só as animações" ou "Tudo" (o mascote passa a usar o .ecm novo).</p>
        </section>
      </div>
    {:else}
      <p class="caixa mini">Este mascote não tem modelo (file_model vazio).</p>
    {/if}
  {:else if aba === "simulador"}
    {#if tipo && !simulavel(num("id_type"))}
      <p class="caixa aviso">{tipo.nome}: {tipo.luta ? "os atributos dependem do dono (GenerateBaseProp2), que o Estúdio não tem — o simulador monta só mascote de combate e de evolução." : "este tipo não luta."}</p>
    {:else}
      <SimuladorDeCombate {valores} icone={iconeHex} lealdadeInicial={ovo?.lealdade ?? 500} />
    {/if}
  {/if}
{/if}

{#if f && modal === "classes"}
  <ModalMascara titulo="Classes que podem usar" opcoes={classes} valor={num("character_combo_id")} aoFechar={() => (modal = "")}
    explica="character_combo_id: bit k = classe k (ExpTypes.h:2991)." aoAplicar={(v) => (valores.character_combo_id = v)} />
{:else if f && modal === "imune"}
  <ModalMascara titulo="Imune a" opcoes={IMUNIDADES_DO_MASCOTE} valor={num("immune_type")} aoFechar={() => (modal = "")}
    explica="immune_type (ExpTypes.h:3044): bits de baixo para cima." aoAplicar={(v) => (valores.immune_type = v)} />
{:else if f && modal === "evoluido"}
  <SeletorDeEspaco titulo="Ovo do mascote evoluído" espaco="ESSENCE" categoriaInicial="PET_EGG_ESSENCE" tabela="PET_EGG_ESSENCE" aoFechar={() => (modal = "")}
    aoEscolher={(r) => { valores.id_pet_egg_evolved = r.id ?? 0; nomes.id_pet_egg_evolved = r.nome; }} />
{:else if f && modal === "icone"}
  <ModalIcone caminho={String(valores.file_icon ?? "")} aoFechar={() => (modal = "")}
    aoEscolher={async (novo) => { valores.file_icon = novo; const nome = novo.slice(novo.lastIndexOf("\\") + 1).toLowerCase();
      try { iconeHex = (await buscarIcones(nome, 20)).icones.find((i) => i.arquivo === nome)?.hex ?? null; } catch { iconeHex = null; } }} />
{/if}

<style>
  .barra { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 14px; }
  .espaco { flex: 1; }
  .cabeca { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; margin-bottom: 10px; }
  .icone { background: none; border: 1px solid var(--borda); border-radius: 12px; padding: 4px; cursor: pointer; }
  .icone:hover:not(:disabled) { border-color: var(--ouro); }
  .titulo { flex: 1; min-width: 240px; display: flex; flex-direction: column; gap: 6px; }
  .nome { font-size: 20px; font-weight: 600; height: 42px; }
  .selos { display: flex; gap: 6px; flex-wrap: wrap; }
  .selo { font-size: 12px; padding: 2px 10px; border-radius: 999px; border: 1px solid var(--borda-forte); color: var(--texto-2); }
  .selo.ouro { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .selo.erro { border-color: var(--vermelho); color: var(--texto-erro); }
  .resumo { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; }
  .resumo div { display: flex; flex-direction: column; align-items: center; }
  .resumo strong { font-size: 18px; font-variant-numeric: tabular-nums; }
  .abas { display: flex; gap: 2px; border-bottom: 1px solid var(--borda); margin-bottom: 14px; flex-wrap: wrap; }
  .abas button { padding: 10px 16px; border: 0; border-bottom: 2px solid transparent; background: none; color: var(--texto-2); font: inherit; font-size: 14px; cursor: pointer; }
  .abas button.on { color: var(--texto); border-color: var(--ouro); font-weight: 600; }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(300px, 400px); gap: 14px; align-items: start; }
  .coluna, .lateral { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .uma { grid-template-columns: 1fr; }
  h4 { margin: 14px 0 6px; font-size: 13px; color: var(--texto-2); }
  .tipos { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 6px; margin-bottom: 12px; }
  .tipos button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 8px 10px; border-radius: 10px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .tipos button.on { border-color: var(--ouro); background: var(--ouro-suave); }
  .aviso-texto { color: var(--ouro); }
  .marcas { margin-top: 10px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips button { padding: 5px 12px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto-2); font: inherit; font-size: 13px; cursor: pointer; }
  .chips button.on { border-color: var(--verde); color: var(--verde); background: var(--verde-suave); }
  .escolha { text-align: left; cursor: pointer; overflow: hidden; white-space: nowrap; }
  .explicacao { line-height: 1.3; }
  .link { background: none; border: 0; color: var(--ouro); font: inherit; cursor: pointer; padding: 0; text-decoration: underline; }
  .nivel { display: flex; flex-direction: column; gap: 4px; max-width: 520px; margin: 8px 0 14px; font-size: 13px; }
  .nivel input { accent-color: var(--ouro); }
  .formulas { display: flex; flex-direction: column; gap: 8px; }
  .formula { display: grid; grid-template-columns: minmax(170px, 1.1fr) minmax(0, 2fr) 170px; gap: 12px; align-items: center; padding: 8px 10px;
    border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .cab { display: flex; flex-direction: column; }
  .coef { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; }
  .coef label { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--texto-3); }
  .coef input { min-width: 0; }
  .coef .alterado { border-color: var(--ouro); background: var(--ouro-suave); }
  .valor { display: grid; grid-template-columns: auto 1fr; grid-template-rows: auto auto; column-gap: 8px; align-items: center; }
  .valor strong { font-size: 18px; font-variant-numeric: tabular-nums; }
  .valor .mini { grid-column: 1 / -1; }
  .spark { width: 100%; height: 30px; }
  .spark polyline { fill: none; stroke: var(--ouro); stroke-width: 1.5; }
  .modelo { display: grid; grid-template-columns: minmax(0, 1fr) minmax(260px, 360px); gap: 14px; align-items: start; }
  @media (max-width: 960px) { .layout, .modelo { grid-template-columns: 1fr; } .formula { grid-template-columns: 1fr; } }
</style>
