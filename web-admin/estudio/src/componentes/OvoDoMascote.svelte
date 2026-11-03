<script lang="ts">
  /* B394: o ovo que choca o mascote. O mascote não aponta para o ovo: é o ovo (`PET_EGG_ESSENCE.id_pet`) que diz qual
     mascote nasce (`pw_data_loader::pet::carregar_ovos`; chocar = serviço 28 da Gerente de Mascotes,
     `hatch_pet_service_executor`, `serviceprovider.cpp:3160`). Aqui: os ovos deste mascote (faixa `id_pet` sobre a
     tabela, B376), a edição do escolhido, trocar por outro ovo existente (grava o `id_pet` dele) e criar um ovo novo
     (cópia de um ovo, já apontando para este mascote). Tudo no rascunho do alto. */
  import { clonar, ficha as lerFicha, linkDosEditores, listar, salvar, type Campo, type Ficha, type Registro } from "../lib/api";
  import { acharLista, type LinhaDeHabilidade } from "../lib/chances";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import { EXPLICACOES, FAIXAS_DE_LEALDADE, ROTULOS } from "../lib/mascote";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import ListaDeHabilidades from "./ListaDeHabilidades.svelte";
  import Origens from "./Origens.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";

  let { idDoMascote, nomeDoMascote, aoEscolherOvo }:
    { idDoMascote: number; nomeDoMascote: string; aoEscolherOvo?: (o: { id: number; nome: string; lealdade: number; nivel: number } | null) => void } = $props();

  const editavel = $derived(!!estudio.rascunho);
  let ovos = $state<Registro[]>([]);
  let carregando = $state(true);
  let escolhido = $state<number | null>(null); // índice do registro
  let f = $state<Ficha | null>(null);
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let gravando = $state(false);
  let seletor = $state(false);
  let capturado = $state<Registro[]>([]);

  async function carregarOvos(manter = true) {
    carregando = true;
    try {
      const r = await listar("PET_EGG_ESSENCE", "", 0, 50, { colunas: ["id_pet", "level", "money_hatched", "honor_point"], faixas: { id_pet: { min: idDoMascote, max: idDoMascote } } });
      ovos = r.registros;
      if (!manter || escolhido === null || !ovos.some((o) => o.indice === escolhido)) escolhido = ovos[0]?.indice ?? null;
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { carregando = false; }
  }
  $effect(() => { void estudio.rascunho; if (idDoMascote) carregarOvos(); });

  async function carregarOvo(i: number) {
    try {
      const x = await lerFicha("PET_EGG_ESSENCE", i);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      f = x;
      aoEscolherOvo?.({ id: x.id ?? 0, nome: x.nome, lealdade: Number(originais.honor_point ?? 0), nivel: Number(originais.level ?? 1) });
      // Monstros que, capturados (Domesticar Animal), dão este ovo (`id_pet_egg_captured`).
      const m = await listar("MONSTER_ESSENCE", "", 0, 30, { colunas: ["level"], faixas: { id_pet_egg_captured: { min: x.id ?? 0, max: x.id ?? 0 } } }).catch(() => null);
      capturado = m?.registros ?? [];
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { if (escolhido === null) { f = null; aoEscolherOvo?.(null); } else carregarOvo(escolhido); });

  const num = (n: string) => Number(valores[n] ?? 0);
  const alterado = (n: string) => valores[n] !== originais[n];
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  const existe = (n: string) => n in porNome;
  const faixa = (p: number) => FAIXAS_DE_LEALDADE.find((x) => p <= x.ate) ?? FAIXAS_DE_LEALDADE[3];

  async function gravar() {
    if (!f || escolhido === null) return;
    const c: Record<string, number | string> = {};
    for (const n of mudados) {
      const t = porNome[n].tipo, v = valores[n];
      if (t === "int32" || t === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x) || (t === "int32" && !Number.isInteger(x))) { avisar("aviso", `${ROTULOS[n] ?? n}: número inválido.`); return; }
        c[n] = x;
      } else c[n] = String(v);
    }
    if (!Object.keys(c).length) return;
    gravando = true;
    try {
      await salvar("PET_EGG_ESSENCE", escolhido, c);
      avisar("ok", `Ovo gravado no rascunho (${Object.keys(c).length} campo(s)).`);
      await carregarOvos();
      await carregarOvo(escolhido);
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }

  /* habilidades iniciais (`skills_N_id_skill`/`_level`, 32 pares; o mascote nasce sabendo) */
  const lista = $derived(f ? acharLista(Object.keys(porNome), "skills", "id_skill", "level") : null);
  const habilidades = $derived(lista ? lista.ids.map((c, k) => ({ id: num(c), nivel: num(lista.chances[k]) })).filter((x) => x.id > 0) : []);
  function gravarHabilidades(l: LinhaDeHabilidade[]) {
    if (!lista) return;
    lista.ids.forEach((c, k) => { valores[c] = l[k]?.id ?? 0; valores[lista.chances[k]] = l[k]?.nivel ?? 0; });
  }

  async function usarOvo(r: Registro) {
    seletor = false;
    if (r.indice === undefined) return;
    try {
      const x = await lerFicha("PET_EGG_ESSENCE", r.indice);
      const antes = Number(x.campos.find((c) => c.nome === "id_pet")?.valor ?? 0);
      if (antes === idDoMascote) { avisar("aviso", "Esse ovo já choca este mascote."); escolhido = r.indice; return; }
      const outro = x.campos.find((c) => c.nome === "id_pet")?.referencia?.nome;
      if (antes && !confirm(`O ovo "${r.nome}" hoje choca ${outro ?? `o mascote ${antes}`}. Passar a chocar ${nomeDoMascote}?`)) return;
      await salvar("PET_EGG_ESSENCE", r.indice, { id_pet: idDoMascote });
      avisar("ok", `"${r.nome}" agora choca ${nomeDoMascote} (no rascunho).`);
      escolhido = r.indice;
      await carregarOvos();
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  async function criarOvo() {
    if (!editavel) { avisar("aviso", "Escolha ou crie um rascunho no alto para criar."); return; }
    try {
      // Base: o ovo escolhido; sem nenhum, o ovo de menor id do arquivo.
      let base = escolhido;
      if (base === null) base = (await listar("PET_EGG_ESSENCE", "", 0, 1, { ordem: "id" })).registros[0]?.indice ?? null;
      if (base === null) throw new Error("Sem ovo para servir de base.");
      const c = await clonar("PET_EGG_ESSENCE", base);
      await salvar("PET_EGG_ESSENCE", c.resultado.indice, { id_pet: idDoMascote, Name: `Ovo de ${nomeDoMascote}`.slice(0, 31) });
      avisar("ok", `Ovo novo id ${c.resultado.id} no rascunho. Ponha-o numa loja ou no drop para o jogador conseguir.`);
      escolhido = c.resultado.indice;
      await carregarOvos();
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  const fmt = (x: number) => x.toLocaleString("pt-BR");
</script>

{#snippet campo(n: string, passo = "1")}
  {#if existe(n)}
    <label title={n}>
      <span>{ROTULOS[n] ?? n}</span>
      <input class="campo" class:alterado={alterado(n)} type="number" step={passo} disabled={!editavel} value={valores[n]}
        oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
      {#if EXPLICACOES[n]}<span class="mini explicacao">{EXPLICACOES[n]}</span>{/if}
    </label>
  {/if}
{/snippet}

<section class="caixa">
  <div class="topo">
    <h3>Ovos que chocam {nomeDoMascote || "este mascote"}</h3>
    <span class="espaco"></span>
    <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (seletor = true)}><Icone nome="busca" tamanho={16} /> Usar um ovo que já existe</button>
    <button class="botao pequeno primario" type="button" disabled={!editavel} onclick={criarOvo}><Icone nome="mais" tamanho={16} /> Criar ovo novo</button>
  </div>
  <p class="mini">O jogador leva o ovo à Gerente de Mascotes, paga as moedas e o mascote nasce com o nível, a lealdade e as habilidades do ovo.</p>
  {#if carregando}
    <p class="suave">Procurando ovos…</p>
  {:else if !ovos.length}
    <div class="vazio">
      <strong>Nenhum ovo choca este mascote.</strong>
      <span class="mini">Sem ovo o jogador não tem como conseguir o mascote. Crie um ovo novo ou ligue um ovo que já existe.</span>
    </div>
  {:else}
    <div class="ovos" role="radiogroup" aria-label="Ovos">
      {#each ovos as o (o.indice)}
        <button type="button" role="radio" aria-checked={escolhido === o.indice} class:on={escolhido === o.indice} onclick={() => (escolhido = o.indice)}>
          <IconeItem hex={o.icone} nome="" tamanho={36} />
          <span class="txt"><strong>{o.nome}</strong>
            <span class="mini">id {o.id} · nasce no nível {o.campos?.level ?? "?"} · chocar {fmt(Number(o.campos?.money_hatched ?? 0))} moedas</span></span>
        </button>
      {/each}
    </div>
  {/if}
</section>

{#if f}
  <div class="duas">
    <section class="caixa">
      <div class="topo">
        <h3>Ovo · {f.nome}</h3>
        <span class="espaco"></span>
        <a class="botao pequeno fantasma" href={`#/itens/PET_EGG_ESSENCE/${escolhido}?realm=${encodeURIComponent(estudio.realm)}${estudio.rascunho ? `&rascunho=${estudio.rascunho}` : ""}`}
          target="_blank" rel="noopener" title="Abrir o ovo na tela de Itens"><Icone nome="abrir" tamanho={16} /> Abrir como item</a>
        <a class="botao pequeno fantasma" href={linkDosEditores("PET_EGG_ESSENCE", escolhido ?? 0)} target="_blank" rel="noopener"><Icone nome="editores" tamanho={16} /></a>
      </div>
      {#if existe("Name")}
        <label><span>Nome do ovo</span>
          <input class="campo" class:alterado={alterado("Name")} disabled={!editavel} maxlength="31" value={valores.Name}
            oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} /></label>
      {/if}
      <h4>Como o mascote nasce</h4>
      <div class="grade-campos">{#each ["level", "exp", "skill_point"] as n (n)}{@render campo(n)}{/each}</div>
      {#if existe("honor_point")}
        <h4>Lealdade inicial</h4>
        <div class="lealdade">
          <input type="range" min="0" max="999" disabled={!editavel} value={num("honor_point")} aria-label="Lealdade"
            oninput={(e) => (valores.honor_point = Number((e.target as HTMLInputElement).value))} />
          <input class="campo curto" class:alterado={alterado("honor_point")} type="number" min="0" max="999" disabled={!editavel} value={valores.honor_point}
            oninput={(e) => (valores.honor_point = Number((e.target as HTMLInputElement).value))} />
          <span class="selo">{faixa(num("honor_point")).nome} · dano {faixa(num("honor_point")).ajuste >= 0 ? "+" : ""}{faixa(num("honor_point")).ajuste}%</span>
        </div>
        <p class="mini">{EXPLICACOES.honor_point}</p>
      {/if}
      <h4>Custos e preço</h4>
      <div class="grade-campos">{#each ["money_hatched", "money_restored", "price", "shop_price", "pile_num_max"] as n (n)}{@render campo(n)}{/each}</div>
      <div class="acoes">
        {#if mudados.length}<span class="mini">{mudados.length} campo(s) do ovo por gravar</span>{/if}
        <button class="botao" type="button" disabled={!mudados.length || gravando} onclick={() => (valores = { ...originais })}>Descartar</button>
        <button class="botao primario" type="button" disabled={!editavel || !mudados.length || gravando} onclick={gravar}><Icone nome="check" /> Gravar o ovo</button>
      </div>
    </section>
    <div class="coluna">
      {#if lista}
        <section class="caixa">
          <ListaDeHabilidades titulo="Habilidades com que nasce" maximo={lista.ids.length} linhas={habilidades} {editavel} deMonstro={false}
            explica="O mascote já nasce sabendo estas, no nível indicado (até 32). Gravar com o botão do ovo." aoMudar={gravarHabilidades} />
        </section>
      {/if}
      <section class="caixa">
        <h3>Capturado de</h3>
        {#if capturado.length}
          <ul class="capt">{#each capturado as m (m.indice)}
            <li><button class="link" type="button" onclick={() => ir(`monstros/MONSTER_ESSENCE/${m.indice}`)}>{m.nome}</button>
              <span class="mini">id {m.id} · nível {m.campos?.level ?? "?"}</span></li>{/each}</ul>
          <p class="mini">Domesticar Animal neste monstro dá o ovo (id_pet_egg_captured do monstro).</p>
        {:else}<p class="mini">Nenhum monstro dá este ovo ao ser capturado.</p>{/if}
      </section>
    </div>
  </div>
  {#if f.id}<Origens id={f.id} />{/if}
{/if}

{#if seletor}
  <SeletorDeEspaco titulo="Escolher o ovo que choca {nomeDoMascote}" espaco="ESSENCE" categoriaInicial="PET_EGG_ESSENCE" tabela="PET_EGG_ESSENCE"
    aoFechar={() => (seletor = false)} aoEscolher={usarOvo} />
{/if}

<style>
  .topo { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .topo h3 { margin: 0; }
  .espaco { flex: 1; }
  h4 { margin: 14px 0 6px; font-size: 13px; color: var(--texto-2); }
  .vazio { display: flex; flex-direction: column; gap: 4px; padding: 14px; border: 1px dashed var(--ouro); border-radius: 10px; background: var(--ouro-suave); margin-top: 8px; }
  .ovos { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; margin-top: 10px; }
  .ovos button { display: flex; gap: 10px; align-items: center; padding: 8px 10px; border-radius: 10px; border: 1px solid var(--borda); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .ovos button.on { border-color: var(--ouro); background: var(--ouro-suave); }
  .txt { display: flex; flex-direction: column; min-width: 0; }
  .duas { display: grid; grid-template-columns: minmax(0, 1fr) minmax(300px, 420px); gap: 14px; align-items: start; margin: 14px 0; }
  .coluna { display: flex; flex-direction: column; gap: 14px; }
  .lealdade { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .lealdade input[type="range"] { flex: 1; min-width: 160px; accent-color: var(--ouro); }
  .curto { width: 90px; }
  .selo { font-size: 12px; padding: 2px 10px; border-radius: 999px; border: 1px solid var(--borda-forte); color: var(--texto-2); }
  .acoes { display: flex; justify-content: flex-end; align-items: center; gap: 8px; margin-top: 14px; flex-wrap: wrap; }
  .explicacao { line-height: 1.3; }
  .capt { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .link { background: none; border: 0; color: var(--texto); font: inherit; font-weight: 600; cursor: pointer; padding: 0; }
  .link:hover { color: var(--ouro); }
  @media (max-width: 960px) { .duas { grid-template-columns: 1fr; } }
</style>
