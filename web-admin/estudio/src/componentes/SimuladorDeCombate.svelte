<script lang="ts">
  /* B394: simulador de combate do mascote. Monta o mascote no nível escolhido (`corpo_do_mascote` com os valores da
     tela, inclusive os ainda não gravados) e o põe contra outro mascote do mesmo nível ou contra um monstro
     (`MonsterEntity::do_template`), com o golpe do GS (`lib/mascote.ts`, porte de `pw_gs::combat::resolver`). Mostra o
     esperado (média, sem sorte), o resultado de 400 lutas e uma luta de exemplo. O ranking põe todos os mascotes que
     lutam contra o adversário escolhido — a pergunta "qual mascote aguenta o Lobo-rei de Hades?". */
  import { ficha as lerFicha, listar, type Registro } from "../lib/api";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import { esperado, FAIXAS_DE_LEALDADE, FORMULAS, mascoteNoNivel, monstroDaFicha, simular, simulavel, tipoDoMascote,
    type Combatente, type Esperado, type Resumo } from "../lib/mascote";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";

  let { valores, icone = null, lealdadeInicial = 500 }:
    { valores: Record<string, number | string>; icone?: string | null; lealdadeInicial?: number } = $props();

  const num = (n: string) => Number(valores[n] ?? 0) || 0;
  const minimo = $derived(1);
  const maximo = $derived(Math.max(1, num("level_max") || 150));
  let nivel = $state(0);
  $effect(() => { if (!nivel) nivel = Math.min(maximo, Math.max(minimo, num("level_require") || 1, 30)); });
  // B394: a lealdade começa na do ovo (até o Murillo mexer); 999 = Fiel.
  let lealdade = $state<number | null>(null);
  const lealdadeUsada = $derived(lealdade ?? lealdadeInicial);

  const a = $derived<Combatente>({ ...mascoteNoNivel(valores, nivel || 1, lealdadeUsada), icone });

  /* ---- adversário ---- */
  let modo = $state<"monstro" | "mascote">("monstro");
  let escolhendo = $state(false);
  let adversario = $state<{ tipo: "monstro" | "mascote"; valores: Record<string, number | string>; nome: string; icone: string | null; indice: number; id: number } | null>(null);
  async function escolher(r: Registro) {
    escolhendo = false;
    if (r.indice === undefined) return;
    try {
      const t = modo === "monstro" ? "MONSTER_ESSENCE" : "PET_ESSENCE";
      const f = await lerFicha(t, r.indice);
      const v: Record<string, number | string> = {};
      for (const c of f.campos) v[c.nome] = c.valor ?? 0;
      if (modo === "mascote" && !simulavel(Number(v.id_type))) {
        avisar("aviso", `${f.nome} é ${tipoDoMascote(Number(v.id_type))?.nome ?? "de outro tipo"}: o simulador só monta mascote de combate ou evolução.`);
        return;
      }
      adversario = { tipo: modo, valores: v, nome: f.nome, icone: f.icone, indice: r.indice, id: f.id ?? 0 };
      resumo = null;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  /** Mascote contra mascote: o outro no mesmo nível (limitado ao nível máximo dele). */
  const nivelDoOutro = $derived(adversario?.tipo === "mascote" ? Math.min(nivel || 1, Math.max(1, Number(adversario.valores.level_max) || 150)) : 0);
  const b = $derived<Combatente | null>(!adversario ? null
    : adversario.tipo === "monstro" ? { ...monstroDaFicha(adversario.valores, adversario.nome), icone: adversario.icone }
    : { ...mascoteNoNivel(adversario.valores, nivelDoOutro, lealdadeUsada, adversario.nome), icone: adversario.icone });
  const ida = $derived<Esperado | null>(b ? esperado(a, b) : null);
  const volta = $derived<Esperado | null>(b ? esperado(b, a) : null);

  /* ---- lutas ---- */
  let resumo = $state<Resumo | null>(null);
  let rodando = $state(false);
  async function rodar() {
    if (!b) return;
    rodando = true;
    await new Promise((r) => setTimeout(r, 10));
    try { resumo = simular(a, b, 400); } finally { rodando = false; }
  }
  // Mudou o nível, a lealdade, um campo ou o adversário: o resultado anterior não vale mais.
  $effect(() => { void a; void b; resumo = null; });

  /* ---- ranking: todos os mascotes que lutam contra o adversário ---- */
  const CAMPOS = ["id_type", "level_require", "level_max", "attack_speed", "damage_delay", "attack_range", "size", "immune_type",
    ...FORMULAS.flatMap((f) => f.campos)];
  /** `derruba`/`cai`: segundos das lutas sorteadas (o vencedor de cada lado); sem nenhuma, a conta pela média. */
  interface Linha { r: Registro; c: Combatente; ida: Esperado; volta: Esperado; vitorias: number; derruba: number; cai: number }
  let ranking = $state<Linha[] | null>(null);
  let rankeando = $state(false);
  let soAlcancam = $state(true);
  async function rankear() {
    if (!b) return;
    rankeando = true; ranking = null;
    try {
      const todos: Registro[] = [];
      for (const t of [8782, 37698]) {
        if (t === 37698 && estudio.versao.startsWith("1.2")) continue;
        // A listagem devolve até 12 colunas por pedido (`FiltroDeListagem::da_consulta`, `estudio.rs:208`): os campos vão
        // em grupos e se juntam pelo registro (mesma faixa, mesma ordem do arquivo, mesmas páginas).
        const grupos: string[][] = [];
        for (let k = 0; k < CAMPOS.length; k += 12) grupos.push(CAMPOS.slice(k, k + 12));
        for (let p = 0; p < 20; p++) {
          const partes = await Promise.all(grupos.map((g) => listar("PET_ESSENCE", "", p, 200, { colunas: g, faixas: { id_type: { min: t, max: t } } })));
          const base = partes[0];
          for (const r of base.registros) {
            const campos: Record<string, number | null> = {};
            for (const x of partes) Object.assign(campos, x.registros.find((y) => y.indice === r.indice)?.campos ?? {});
            todos.push({ ...r, campos });
          }
          if ((p + 1) * 200 >= base.total) break;
        }
      }
      const linhas: Linha[] = [];
      for (const r of todos) {
        const v: Record<string, number | string> = { ...(r.campos as Record<string, number>), Name: r.nome };
        const max = Number(v.level_max) || 150;
        if (soAlcancam && max < nivel) continue;
        const c = mascoteNoNivel(v, Math.min(nivel, max), lealdadeUsada, r.nome);
        const contra = adversario?.tipo === "mascote" ? mascoteNoNivel(adversario.valores, nivelDoOutro, lealdadeUsada, adversario.nome) : b;
        const s = simular(c, contra, 40);
        const ida = esperado(c, contra), volta = esperado(contra, c);
        linhas.push({ r, c, ida, volta, vitorias: s.vitorias[0] / s.lutas,
          derruba: Number.isNaN(s.segundosAteVencer[0]) ? ida.tempo : s.segundosAteVencer[0],
          cai: Number.isNaN(s.segundosAteVencer[1]) ? volta.tempo : s.segundosAteVencer[1] });
      }
      // Quem vence mais; empatados, quem tira mais vida do adversário antes de cair.
      const fracao = (l: Linha) => Math.min(1, (l.ida.liquido > 0 ? l.ida.liquido : 0) * (Number.isFinite(l.cai) ? l.cai : 1e9) / Math.max(1, b.vida));
      linhas.sort((x, y) => y.vitorias - x.vitorias || fracao(y) - fracao(x));
      ranking = linhas.slice(0, 40);
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { rankeando = false; }
  }

  /* ---- formatação ---- */
  const fmt = (x: number, d = 0) => (Number.isFinite(x) ? x.toLocaleString("pt-BR", { maximumFractionDigits: d, minimumFractionDigits: 0 }) : "—");
  const pct = (x: number) => `${fmt(x * 100, 1)}%`;
  const tempo = (s: number) => (!Number.isFinite(s) ? "nunca" : s < 60 ? `${fmt(s, 1)} s` : `${Math.floor(s / 60)} min ${fmt(s % 60, 0)} s`);
  const faixa = (p: number) => FAIXAS_DE_LEALDADE.find((x) => p <= x.ate) ?? FAIXAS_DE_LEALDADE[3];

  /* ---- gráfico da luta de exemplo (vida em % ao longo do tempo) ---- */
  const L = 640, H = 170, M = 28;
  const linha = (k: "a" | "b") => {
    if (!resumo?.exemplo.curva || !b) return "";
    const c = resumo.exemplo.curva, fim = Math.max(1, c[c.length - 1].t), max = k === "a" ? a.vida : b.vida;
    return c.map((p) => `${(M + (p.t / fim) * (L - M - 8)).toFixed(1)},${(8 + (1 - p[k] / max) * (H - 30)).toFixed(1)}`).join(" ");
  };
  const veredito = $derived.by(() => {
    if (!resumo || !b) return "";
    const [va, vb] = resumo.vitorias;
    if (va === 0 && vb === 0) return `Ninguém derruba ninguém em 10 minutos: a regeneração cobre o dano.`;
    const quem = va >= vb ? a.nome : b.nome, quanto = Math.max(va, vb) / resumo.lutas;
    return `${quem || "?"} vence ${pct(quanto)} das lutas, em média em ${tempo(resumo.segundosMedio)}.`;
  });
</script>

{#snippet ficha(c: Combatente, cor: string, rotulo: string)}
  <div class="ficha" style="--cor: var({cor})">
    <div class="quem">
      <IconeItem hex={c.icone} nome="" tamanho={40} />
      <div><span class="mini">{rotulo}</span><strong>{c.nome || "(sem nome)"}</strong><span class="mini">nível {c.nivel}</span></div>
    </div>
    <dl>
      <dt>Vida</dt><dd>{fmt(c.vida)}</dd>
      <dt>Regenera</dt><dd>{fmt(c.regeneracao)}/s</dd>
      <dt>Dano</dt><dd>{c.danoMin === c.danoMax ? fmt(c.danoMin) : `${fmt(c.danoMin)}–${fmt(c.danoMax)}`}{c.danoMagico.some(([x]) => x > 0) ? " + mágico" : ""}</dd>
      <dt>Golpe a cada</dt><dd>{fmt(c.intervalo / 20, 2)} s</dd>
      <dt>Acerto</dt><dd>{fmt(c.acerto)}</dd>
      <dt>Esquiva</dt><dd>{fmt(c.esquiva)}</dd>
      <dt>Defesa física</dt><dd>{fmt(c.defesa)}</dd>
      <dt>Defesa mágica</dt><dd>{c.resistencias.every((x) => x === c.resistencias[0]) ? fmt(c.resistencias[0]) : c.resistencias.map((x) => fmt(x)).join(" / ")}</dd>
      {#if c.grauDeAtaque || c.grauDeDefesa}<dt>Graus atq/def</dt><dd>{c.grauDeAtaque} / {c.grauDeDefesa}</dd>{/if}
      <dt>Alcance</dt><dd>{fmt(c.alcance, 1)} m</dd>
    </dl>
  </div>
{/snippet}

{#snippet lado(e: Esperado, x: Combatente, y: Combatente)}
  <div class="lado">
    <h4>{x.nome || "?"} → {y.nome || "?"}</h4>
    {#if e.imune}<p class="erro mini">{y.nome} é imune ao golpe de {x.nome}: nenhum dano passa.</p>{/if}
    <dl>
      <dt>Chance de acertar</dt><dd>{pct(e.chance)}</dd>
      <dt>Dano por golpe</dt><dd>{e.danoMin === e.danoMax ? fmt(e.danoMin) : `${fmt(e.danoMin)}–${fmt(e.danoMax)}`} <span class="mini">(média {fmt(e.danoMedio)})</span></dd>
      <dt>Dano por segundo</dt><dd>{fmt(e.dps, 1)}</dd>
      <dt>Menos a regeneração</dt><dd class:ruim={e.liquido <= 0}>{fmt(e.liquido, 1)}/s</dd>
      <dt>Golpes certos para derrubar</dt><dd>{Number.isFinite(e.golpesParaMatar) ? fmt(e.golpesParaMatar) : "—"}</dd>
      <dt>Tempo para derrubar</dt><dd class:ruim={!Number.isFinite(e.tempo) && !e.umGolpe}><strong>{tempo(e.tempo)}</strong></dd>
    </dl>
    {#if e.umGolpe}<p class="mini aviso-texto">O golpe mais forte ({fmt(e.danoMax)}) derruba {y.nome} de vida cheia ({fmt(y.vida)}): a média não conta isso — veja o tempo nas lutas.</p>{/if}
  </div>
{/snippet}

<section class="caixa">
  <h3>Simulador de combate</h3>
  <p class="mini">Ponha o mascote num nível e compare com outro mascote do mesmo nível ou com um monstro. As contas são as do servidor
    (acerto, defesa pelo nível de quem bate, graus, lealdade e regeneração em combate); <strong>só o golpe normal</strong> — habilidades,
    adicionais do dono e o "addon" aleatório do tipo de monstro não entram. Os valores da tela valem antes de gravar.</p>

  <div class="controles">
    <label class="nivel"><span>Nível do mascote: <strong>{nivel}</strong></span>
      <input type="range" min={minimo} max={maximo} value={nivel} oninput={(e) => (nivel = Number((e.target as HTMLInputElement).value))} aria-label="Nível" />
      <span class="mini">1 – {maximo} (nível exigido {num("level_require")}: entra nas fórmulas)</span>
    </label>
    <div class="lealdades" role="radiogroup" aria-label="Lealdade">
      <span class="mini">Lealdade</span>
      {#each FAIXAS_DE_LEALDADE as f (f.ate)}
        <button type="button" role="radio" aria-checked={faixa(lealdadeUsada).ate === f.ate} class:on={faixa(lealdadeUsada).ate === f.ate}
          onclick={() => (lealdade = f.ate)} title="dano {f.ajuste >= 0 ? '+' : ''}{f.ajuste}%">{f.nome.split(" ")[0]} <span class="mini">{f.ajuste >= 0 ? "+" : ""}{f.ajuste}%</span></button>
      {/each}
    </div>
  </div>

  <div class="arena">
    {@render ficha(a, "--azul", "Este mascote")}
    <div class="versus">
      <span>contra</span>
      <div class="modo" role="radiogroup" aria-label="Adversário">
        <button type="button" role="radio" aria-checked={modo === "monstro"} class:on={modo === "monstro"} onclick={() => (modo = "monstro")}><Icone nome="monstro" tamanho={16} /> Monstro</button>
        <button type="button" role="radio" aria-checked={modo === "mascote"} class:on={modo === "mascote"} onclick={() => (modo = "mascote")}><Icone nome="pata" tamanho={16} /> Mascote</button>
      </div>
      <button class="botao" type="button" onclick={() => (escolhendo = true)}><Icone nome="busca" tamanho={16} /> Escolher {modo === "monstro" ? "o monstro" : "o mascote"}</button>
    </div>
    {#if b}
      {@render ficha(b, "--vermelho", adversario?.tipo === "monstro" ? "Monstro" : `Mascote (mesmo nível${nivelDoOutro < nivel ? `: o máximo dele é ${nivelDoOutro}` : ""})`)}
    {:else}
      <div class="ficha vazia"><Icone nome={modo === "monstro" ? "monstro" : "pata"} tamanho={40} />
        <span class="mini">Escolha {modo === "monstro" ? "um monstro" : "outro mascote"} para comparar.</span></div>
    {/if}
  </div>

  {#if b && ida && volta}
    <h4 class="sub">O que se espera (média, sem sorte)</h4>
    <div class="lados">{@render lado(ida, a, b)}{@render lado(volta, b, a)}</div>

    <div class="rodar">
      <button class="botao primario" type="button" disabled={rodando} onclick={rodar}><Icone nome="seta" /> {rodando ? "Lutando…" : "Simular 400 lutas"}</button>
      <button class="botao" type="button" disabled={rankeando} onclick={rankear} title="Todos os mascotes de combate neste nível contra este adversário">
        <Icone nome="editores" /> {rankeando ? "Calculando…" : "Qual mascote vai melhor contra este?"}</button>
      <label class="mini"><input type="checkbox" bind:checked={soAlcancam} /> só os que chegam ao nível {nivel}</label>
    </div>

    {#if resumo}
      <div class="resultado">
        <p class="veredito">{veredito}</p>
        <div class="barra-vitoria" aria-label="Vitórias">
          {#if resumo.vitorias[0]}<span class="va" style="flex: {resumo.vitorias[0]}">{a.nome} {pct(resumo.vitorias[0] / resumo.lutas)}</span>{/if}
          {#if resumo.empates}<span class="ve" style="flex: {resumo.empates}">empate {pct(resumo.empates / resumo.lutas)}</span>{/if}
          {#if resumo.vitorias[1]}<span class="vb" style="flex: {resumo.vitorias[1]}">{b.nome} {pct(resumo.vitorias[1] / resumo.lutas)}</span>{/if}
        </div>
        <p class="mini">{resumo.lutas} lutas · duração média {tempo(resumo.segundosMedio)}
          {#if resumo.vitorias[0]} · quando {a.nome} vence, fica com {pct(resumo.vidaRestante[0])} da vida{/if}
          {#if resumo.vitorias[1]} · quando {b.nome} vence, fica com {pct(resumo.vidaRestante[1])} da vida{/if}
          · luta de mais de 10 min conta como empate.</p>
        <h4 class="sub">Uma luta de exemplo</h4>
        <svg viewBox="0 0 {L} {H}" class="grafico" role="img" aria-label="Vida dos dois durante a luta">
          {#each [0, 0.5, 1] as g (g)}
            <line x1={M} x2={L - 8} y1={8 + (1 - g) * (H - 30)} y2={8 + (1 - g) * (H - 30)} class="grade" />
            <text x={M - 4} y={12 + (1 - g) * (H - 30)} text-anchor="end">{g * 100}%</text>
          {/each}
          <polyline points={linha("a")} class="la" />
          <polyline points={linha("b")} class="lb" />
          <text x={L - 8} y={H - 4} text-anchor="end">{tempo(resumo.exemplo.segundos)}</text>
          <text x={M} y={H - 4}>0 s</text>
        </svg>
        <div class="legenda"><span class="ca"></span>{a.nome} <span class="cb"></span>{b.nome}
          <span class="mini">· golpes {resumo.exemplo.golpes[0]} × {resumo.exemplo.golpes[1]} · acertos {resumo.exemplo.acertos[0]} × {resumo.exemplo.acertos[1]}
            · dano {fmt(resumo.exemplo.dano[0])} × {fmt(resumo.exemplo.dano[1])}</span></div>
        {#if resumo.exemplo.relato?.length}
          <details><summary class="mini">Os primeiros golpes</summary><ol class="relato">{#each resumo.exemplo.relato as x, i (i)}<li class="mini">{x}</li>{/each}</ol></details>
        {/if}
      </div>
    {/if}

    {#if ranking}
      <h4 class="sub">Mascotes no nível {nivel} contra {b.nome} <span class="mini">(lealdade {faixa(lealdadeUsada).nome}; 40 lutas cada)</span></h4>
      <p class="mini">{ranking.some((l) => l.vitorias > 0)
        ? "Ordem: quem vence mais lutas; empatados, quem tira mais vida do adversário antes de cair."
        : `Nenhum mascote vence ${b.nome} sozinho neste nível: a ordem é de quem tira mais vida dele antes de cair.`}
        "Derruba em" e "Cai em" vêm das lutas sorteadas; sem nenhuma vitória do lado, da conta pela média.</p>
      <div class="rolagem">
        <table class="tabela">
          <thead><tr><th></th><th>Mascote</th><th>Alcance</th><th>Vida</th><th>Dano/s</th><th>Derruba em</th><th>Cai em</th><th>Vence</th></tr></thead>
          <tbody>
            {#each ranking as l, i (l.r.indice)}
              <tr class:este={l.r.id === num("ID")}>
                <td class="num">{i + 1}</td>
                <td><span class="nome"><IconeItem hex={l.r.icone} nome="" tamanho={22} discreto />
                  <button class="link" type="button" onclick={() => ir(`mascotes/PET_ESSENCE/${l.r.indice}`, { aba: "simulador" })}>{l.r.nome}</button></span>
                  <span class="mini">id {l.r.id}{Number(l.r.campos?.level_max) < nivel ? ` · máx. ${l.r.campos?.level_max}` : ""}</span></td>
                <td class="num">{fmt(l.c.alcance, 1)} m</td>
                <td class="num">{fmt(l.c.vida)}</td>
                <td class="num">{fmt(l.ida.dps, 0)}</td>
                <td class="num">{tempo(l.derruba)}</td>
                <td class="num">{tempo(l.cai)}</td>
                <td class="num"><strong>{pct(l.vitorias)}</strong></td>
              </tr>
            {:else}<tr><td colspan="8" class="suave">Nenhum mascote de combate chega a este nível.</td></tr>{/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</section>

{#if escolhendo}
  <SeletorDeEspaco titulo={modo === "monstro" ? "Escolher o monstro" : "Escolher o mascote"} espaco="ESSENCE" soItens={false}
    categoriaInicial={modo === "monstro" ? "MONSTER_ESSENCE" : "PET_ESSENCE"} tabela={modo === "monstro" ? "MONSTER_ESSENCE" : "PET_ESSENCE"}
    aoFechar={() => (escolhendo = false)} aoEscolher={escolher} />
{/if}

<style>
  h4 { margin: 14px 0 6px; font-size: 13px; color: var(--texto-2); }
  .sub { margin-top: 18px; }
  .controles { display: flex; gap: 18px; flex-wrap: wrap; align-items: flex-end; margin: 10px 0 14px; }
  .nivel { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 260px; }
  .nivel input { accent-color: var(--ouro); }
  .lealdades { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .lealdades button, .modo button { padding: 5px 10px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto-2);
    font: inherit; font-size: 12px; cursor: pointer; display: inline-flex; align-items: center; gap: 4px; }
  .lealdades button.on, .modo button.on { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .arena { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); gap: 12px; align-items: stretch; }
  .ficha { border: 1px solid var(--borda); border-top: 3px solid var(--cor, var(--borda)); border-radius: 12px; padding: 12px; background: var(--fundo-2); }
  .ficha.vazia { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--texto-3); border-style: dashed; }
  .quem { display: flex; gap: 10px; align-items: center; margin-bottom: 8px; }
  .quem div { display: flex; flex-direction: column; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 3px 12px; margin: 0; font-size: 13px; }
  dt { color: var(--texto-3); }
  dd { margin: 0; text-align: right; font-variant-numeric: tabular-nums; }
  dd.ruim { color: var(--texto-erro); }
  .versus { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--texto-3); font-size: 13px; }
  .modo { display: flex; gap: 4px; }
  .lados { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .lado { border: 1px solid var(--borda); border-radius: 10px; padding: 10px 12px; }
  .lado h4 { margin-top: 0; }
  .aviso-texto { color: var(--ouro); margin: 6px 0 0; }
  .rodar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin: 16px 0 6px; }
  .resultado { margin-top: 10px; }
  .veredito { font-size: 16px; font-weight: 600; margin: 6px 0 10px; }
  .barra-vitoria { display: flex; height: 28px; border-radius: 8px; overflow: hidden; border: 1px solid var(--borda); font-size: 12px; }
  .barra-vitoria span { display: flex; align-items: center; justify-content: center; white-space: nowrap; overflow: hidden; padding: 0 6px; min-width: 0; }
  .va { background: var(--azul-suave); color: var(--azul); }
  .vb { background: var(--vermelho-suave); color: var(--vermelho); }
  .ve { background: var(--fundo-2); color: var(--texto-3); }
  .grafico { width: 100%; height: auto; max-height: 220px; display: block; }
  .grafico text { font-size: 10px; fill: var(--texto-3); }
  .grade { stroke: var(--borda); stroke-dasharray: 3 3; }
  .la, .lb { fill: none; stroke-width: 2; }
  .la { stroke: var(--azul); }
  .lb { stroke: var(--vermelho); }
  .legenda { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; font-size: 13px; }
  .legenda .ca, .legenda .cb { width: 14px; height: 3px; border-radius: 2px; display: inline-block; }
  .ca { background: var(--azul); } .cb { background: var(--vermelho); margin-left: 8px; }
  .relato { margin: 6px 0; padding-left: 20px; }
  .rolagem { overflow-x: auto; }
  .tabela { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabela th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--texto-3); padding: 6px; border-bottom: 1px solid var(--borda-forte); }
  .tabela td { padding: 4px 6px; border-bottom: 1px solid var(--borda); }
  .tabela tr.este td { background: var(--ouro-suave); }
  .num { font-variant-numeric: tabular-nums; text-align: right; }
  .nome { display: inline-flex; gap: 6px; align-items: center; }
  .link { background: none; border: 0; color: var(--texto); font: inherit; font-weight: 600; cursor: pointer; padding: 0; text-align: left; }
  .link:hover { color: var(--ouro); }
  @media (max-width: 860px) { .arena, .lados { grid-template-columns: 1fr; } }
</style>
