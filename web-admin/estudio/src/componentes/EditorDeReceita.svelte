<script lang="ts">
  /* B392: uma receita (`RECIPE_ESSENCE`) da produção. A regra do GS: `recipe_manager::LoadTemplate`
     (`cgame/gs/npcgenerator.cpp:1380-1430`) — tempo = duração × 20 tiques (1 a 10000), 32 materiais, 4 resultados; ao
     produzir (`player.cpp:16419-16423`) com chance `fail_probability` não sai nada (os materiais vão), senão sorteia um dos
     4 resultados pelas chances (`abase::RandSelect`). Gravar = `PUT tabelas/RECIPE_ESSENCE/{i}` no rascunho. */
  import { ficha as lerFicha, habilidadePorId, salvar, type Campo, type Registro } from "../lib/api";
  import { esconderDica, mostrarDica } from "../lib/dica.svelte";
  import { avisar, estudio } from "../lib/estado.svelte";
  import { regraDaReceita, tiquesDaReceita } from "../lib/servicos";
  import CamposSimples from "./CamposSimples.svelte";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";
  import SeletorDeHabilidade from "./SeletorDeHabilidade.svelte";

  let { indice, aoFechar }: { indice: number; aoFechar?: () => void } = $props();
  const editavel = $derived(!!estudio.rascunho);
  let porNome = $state.raw<Record<string, Campo>>({});
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let id = $state<number | null>(null);
  let erro = $state("");
  let escolhendo = $state<string | null>(null);
  let escolhendoHabilidade = $state(false);
  let nomeDaHabilidade = $state("");
  const cache = $state<Record<number, { nome: string; icone: string | null }>>({});

  async function carregar() {
    try {
      const f = await lerFicha("RECIPE_ESSENCE", indice);
      porNome = Object.fromEntries(f.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(f.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      id = f.id;
      for (const c of f.campos) if (c.referencia?.nome && typeof c.valor === "number") cache[c.valor] = { nome: c.referencia.nome, icone: c.referencia.icone ?? null };
    } catch (e) { erro = (e as Error).message; }
  }
  carregar();
  $effect(() => { const s = Number(valores.id_skill ?? 0); if (s > 0) habilidadePorId(s).then((h) => (nomeDaHabilidade = h?.nome ?? `habilidade ${s}`)); else nomeDaHabilidade = "nenhuma"; });

  const num = (n: string) => Number(valores[n] ?? 0);
  const alvos = $derived([1, 2, 3, 4].filter((k) => `targets_${k}_id_to_make` in porNome));
  const materiais = $derived(Array.from({ length: 32 }, (_, k) => k + 1).filter((k) => `materials_${k}_id` in porNome));
  const regra = $derived(regraDaReceita(num("fail_probability"), alvos.map((k) => num(`targets_${k}_probability`))));
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  const usados = new Set(["ID", "Name", "id_skill", "fail_probability", "duration", ...[1, 2, 3, 4].flatMap((k) => [`targets_${k}_id_to_make`, `targets_${k}_probability`]),
    ...Array.from({ length: 32 }, (_, k) => [`materials_${k + 1}_id`, `materials_${k + 1}_num`]).flat()]);
  const outros = $derived(Object.keys(porNome).filter((n) => !usados.has(n)));
  const pct = (x: number) => `${(x * 100).toLocaleString("pt-BR", { maximumFractionDigits: 2 })}%`;
  const nomeDe = (c: string) => { const v = num(c); return v ? (cache[v]?.nome ?? `item ${v}`) : ""; };
  const iconeDe = (c: string) => { const v = num(c); return v ? cache[v]?.icone ?? null : null; };

  /** Materiais compactados: os com id na frente (o GS pula os vazios, `npcgenerator.cpp:1422-1428`). */
  function tirarMaterial(k: number) {
    const lista = materiais.map((m) => ({ id: num(`materials_${m}_id`), n: num(`materials_${m}_num`) })).filter((_, i) => materiais[i] !== k && true);
    const cheios = lista.filter((x) => x.id > 0);
    materiais.forEach((m, i) => { valores[`materials_${m}_id`] = cheios[i]?.id ?? 0; valores[`materials_${m}_num`] = cheios[i]?.n ?? 0; });
  }
  function escolher(campo: string, r: Registro) {
    valores[campo] = r.id ?? 0;
    if (r.id) cache[r.id] = { nome: r.nome, icone: r.icone ?? null };
    if (campo.startsWith("materials_") && !num(campo.replace("_id", "_num"))) valores[campo.replace("_id", "_num")] = 1;
  }
  const livre = $derived(materiais.find((m) => !num(`materials_${m}_id`)));

  async function gravar() {
    const c: Record<string, number | string> = {};
    for (const n of mudados) {
      const t = porNome[n].tipo, v = valores[n];
      if (t === "int32" || t === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x)) { avisar("aviso", `${n}: número inválido.`); return; }
        c[n] = x;
      } else c[n] = String(v);
    }
    try { await salvar("RECIPE_ESSENCE", indice, c); avisar("ok", `Receita ${id}: ${mudados.length} campo(s) gravados no rascunho.`); await carregar(); }
    catch (e) { avisar("erro", (e as Error).message); }
  }
</script>

<section class="receita">
  {#if erro}<p class="erro">{erro}</p>
  {:else if id === null}<p class="suave">Carregando a receita…</p>
  {:else}
    <div class="cab">
      <h4>Receita {id} · <input class="campo nome" value={valores.Name} disabled={!editavel} aria-label="Nome da receita" oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} /></h4>
      <span class="espaco"></span>
      {#if mudados.length}<span class="mini">{mudados.length} por gravar</span>{/if}
      <button class="botao pequeno" type="button" disabled={!editavel || !mudados.length} onclick={gravar}><Icone nome="check" tamanho={16} /> Gravar receita</button>
      {#if aoFechar}<button class="botao pequeno fantasma" type="button" onclick={aoFechar} aria-label="Fechar receita"><Icone nome="fechar" tamanho={16} /></button>{/if}
    </div>

    <div class="duas">
      <div>
        <h5>O que sai</h5>
        <div class="barra-chance" title="falha e resultados">
          <span style:width={pct(regra.falha)} class="falha"></span>
          {#each regra.porResultado as p, k (k)}<span style:width={pct(p)} class="r{k}"></span>{/each}
        </div>
        <ul class="linhas">
          {#each alvos as k, i (k)}
            {@const campo = `targets_${k}_id_to_make`}
            <li>
              <button class="slot" type="button" disabled={!editavel} onclick={() => (escolhendo = campo)} title="Trocar o item"
                onmouseenter={(e) => num(campo) && mostrarDica(num(campo), nomeDe(campo), e.currentTarget as HTMLElement)} onmouseleave={esconderDica}>
                {#if num(campo)}<IconeItem hex={iconeDe(campo)} nome={nomeDe(campo)} tamanho={28} />{:else}<Icone nome="mais" tamanho={16} />{/if}
              </button>
              <span class="nome">{num(campo) ? nomeDe(campo) : "— vazio —"}</span>
              <label class="mini">chance <input class="campo n" type="number" step="0.01" min="0" max="1" disabled={!editavel}
                value={valores[`targets_${k}_probability`]} oninput={(e) => (valores[`targets_${k}_probability`] = Number((e.target as HTMLInputElement).value))} /></label>
              <span class="pct r{i}">{pct(regra.porResultado[i] ?? 0)}</span>
            </li>
          {/each}
          <li>
            <span class="slot vazio">✕</span><span class="nome">Falha (nada sai, os materiais vão)</span>
            <label class="mini">chance <input class="campo n" type="number" step="0.01" min="0" max="1" disabled={!editavel}
              value={valores.fail_probability} oninput={(e) => (valores.fail_probability = Number((e.target as HTMLInputElement).value))} /></label>
            <span class="pct falha-t">{pct(regra.falha)}</span>
          </li>
        </ul>
        <p class="mini">Primeiro sorteia a falha; se não falhar, escolhe um resultado pelas chances (player.cpp:16419-16423).
          {#if Math.abs(regra.somaDosResultados - 1) > 0.001 && regra.somaDosResultados > 0}As chances dos resultados somam {regra.somaDosResultados.toFixed(3)}: a porcentagem mostrada é a proporção.{/if}</p>
        <div class="grade-campos">
          <label><span>Duração (s)</span><input class="campo" type="number" step="0.05" disabled={!editavel} value={valores.duration}
            oninput={(e) => (valores.duration = Number((e.target as HTMLInputElement).value))} />
            <span class="mini">{tiquesDaReceita(num("duration"))} tiques = {(tiquesDaReceita(num("duration")) / 20).toLocaleString("pt-BR")} s (npcgenerator.cpp:1415-1418)</span></label>
          <label><span>Habilidade exigida</span><button class="campo escolha" type="button" disabled={!editavel} onclick={() => (escolhendoHabilidade = true)}>{nomeDaHabilidade}</button></label>
        </div>
      </div>
      <div>
        <h5>Materiais <span class="mini">{materiais.filter((m) => num(`materials_${m}_id`)).length} de {materiais.length}</span></h5>
        <ul class="linhas">
          {#each materiais.filter((m) => num(`materials_${m}_id`)) as m (m)}
            {@const campo = `materials_${m}_id`}
            <li>
              <button class="slot" type="button" disabled={!editavel} onclick={() => (escolhendo = campo)}
                onmouseenter={(e) => mostrarDica(num(campo), nomeDe(campo), e.currentTarget as HTMLElement)} onmouseleave={esconderDica}>
                <IconeItem hex={iconeDe(campo)} nome={nomeDe(campo)} tamanho={28} /></button>
              <span class="nome">{nomeDe(campo)}</span>
              <label class="mini">× <input class="campo n" type="number" step="1" min="1" disabled={!editavel} value={valores[`materials_${m}_num`]}
                oninput={(e) => (valores[`materials_${m}_num`] = Number((e.target as HTMLInputElement).value))} /></label>
              <button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => tirarMaterial(m)} aria-label="Tirar material"><Icone nome="lixeira" tamanho={16} /></button>
            </li>
          {/each}
        </ul>
        <button class="botao pequeno" type="button" disabled={!editavel || !livre} onclick={() => (escolhendo = `materials_${livre}_id`)}><Icone nome="mais" tamanho={16} /> Material</button>
      </div>
    </div>
    <details><summary class="mini">Outros campos da receita</summary>
      <CamposSimples nomes={outros} {porNome} bind:valores {originais} {editavel} />
    </details>
  {/if}
</section>

{#if escolhendo}
  {@const campo = escolhendo}
  <SeletorDeEspaco titulo={campo.startsWith("targets") ? "Item que a receita faz" : "Material"} espaco="ESSENCE" aoFechar={() => (escolhendo = null)}
    aoEscolher={(r) => escolher(campo, r)} />
{/if}
{#if escolhendoHabilidade}
  <SeletorDeHabilidade titulo="Habilidade de produção exigida" aoFechar={() => (escolhendoHabilidade = false)} aoEscolher={(h) => (valores.id_skill = h.id)} />
{/if}

<style>
  .receita { display: flex; flex-direction: column; gap: 10px; border: 1px solid var(--ouro); border-radius: 12px; padding: 12px; background: var(--fundo-2); }
  .cab { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  h4 { margin: 0; display: flex; align-items: center; gap: 6px; font-size: 14px; }
  h5 { margin: 0 0 6px; font-size: 13px; color: var(--texto-2); }
  .nome { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  input.nome { height: 30px; width: 240px; }
  .espaco { flex: 1; }
  .duas { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .linhas { list-style: none; margin: 0 0 8px; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .linhas li { display: flex; align-items: center; gap: 8px; }
  .slot { width: 36px; height: 36px; border-radius: 6px; border: 1px solid #3a3122; background: #12161c; display: grid; place-items: center; cursor: pointer; padding: 0; color: var(--texto-2); flex: none; }
  .slot.vazio { color: var(--vermelho); cursor: default; }
  .n { width: 80px; height: 28px; }
  .pct { width: 64px; text-align: right; font-variant-numeric: tabular-nums; font-size: 12px; }
  .barra-chance { height: 10px; border-radius: 999px; background: var(--borda); overflow: hidden; display: flex; margin-bottom: 8px; }
  .barra-chance span { height: 100%; }
  .falha { background: var(--vermelho); } .falha-t { color: var(--vermelho); }
  .r0 { background: var(--verde); } .r1 { background: var(--ouro); } .r2 { background: var(--azul); } .r3 { background: #b07dd1; }
  .pct.r0, .pct.r1, .pct.r2, .pct.r3 { background: none; }
  .escolha { text-align: left; cursor: pointer; }
  @media (max-width: 900px) { .duas { grid-template-columns: 1fr; } }
</style>
