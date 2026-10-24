<script lang="ts">
  /* B373: o modelo do monstro e as ações dele. O cliente pede ao monstro ações pelo nome (parado, andar, atacar, morrer…;
     a lista sai do `diagnostico_de_monstro`, B277) e cada uma é uma ação combinada do `.ecm` que toca uma animação-base
     do `.smd` (`BaseActName:`). Aqui se vê cada animação (GLB do servidor animado na CPU) e se troca a animação de cada
     ação (`POST modelos/base`, `ecm::trocar_base`); ação que falta se cria clonando outra (`modelos/acao`). Exportar =
     GLB da animação (Blender); importar = animações de um GLB viram ações do `.smd` (`acoes_glb`, B295). Efeitos (GFX)
     não têm prévia: o painel não tem renderizador de GFX. */
  import { onDestroy } from "svelte";
  import { acaoNova, fichaDoModelo, importarAcoesGlb, importarModeloGlb, rotaDoGlb, trocarBase, type FichaDoModelo } from "../lib/api";
  import { desenharGlb } from "../lib/glb";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";

  let { caminho, habilidades = [], politica = 0, idDoMonstro = 0, aoTrocarModelo }:
    { caminho: string; habilidades?: number[]; politica?: number; idDoMonstro?: number; aoTrocarModelo?: (ecm: string) => Promise<void> } = $props();
  let pastaNova = $state("models\\npcs\\estudio");
  let tituloNovo = $state("");
  const editavel = $derived(!!estudio.rascunho);

  let ficha = $state<FichaDoModelo | null>(null);
  let erro = $state("");
  let tocando = $state<string>("");   // animação-base em prévia ("" = pose parada)
  let tela = $state<HTMLCanvasElement | null>(null);
  let carregando = $state(false);
  let erroDoGlb = $state("");
  let ocupado = $state(false);
  let parar: (() => void) | null = null;

  /* Nomes do cliente → português (só para ler; o arquivo guarda o original). */
  const TRADUCOES: [string, string][] = [["站立", "Parado"], ["休闲", "Ocioso"], ["戒备", "Em guarda"], ["慢速移动", "Andando"], ["快速移动", "Correndo"],
    ["普攻", "Ataque "], ["魔攻", "Ataque mágico "], ["技能", "Habilidade "], ["施放", "lançar "], ["吟唱", "concentrar "], ["死亡", "Morrendo"],
    ["受伤", "Apanhando"], ["叠加", " (sobreposto)"], ["起", " — início"], ["落", " — golpe"], ["挂点", "Ponto de encaixe"], ["飞行", "Voando"],
    ["游泳", "Nadando"], ["跳", "Pulo"], ["眩晕", "Atordoado"], ["出生", "Nascendo"]];
  const traduzir = (n: string) => { let t = n; for (const [a, b] of TRADUCOES) t = t.split(a).join(b); return t.trim(); };

  async function carregar() {
    erro = "";
    try {
      const q: Record<string, string> = {};
      if (habilidades.length) q.habilidades = habilidades.join(",");
      if (politica) q.politica = String(politica);
      ficha = await fichaDoModelo(caminho, q);
    } catch (e) { erro = (e as Error).message; }
  }
  $effect(() => { void estudio.rascunho; if (caminho) carregar(); });

  async function tocar(base: string) {
    tocando = base;
    parar?.(); parar = null; erroDoGlb = ""; carregando = true;
    try {
      const r = await fetch(rotaDoGlb(caminho, base || undefined), { credentials: "same-origin" });
      if (!r.ok) throw new Error(`GLB: HTTP ${r.status}${r.status === 400 ? " — " + ((await r.json().catch(() => ({}))).detail || "") : ""}`);
      if (tela) parar = desenharGlb(tela, await r.arrayBuffer());
    } catch (e) { erroDoGlb = (e as Error).message; }
    finally { carregando = false; }
  }
  $effect(() => { if (tela && ficha && !parar && !carregando) tocar(""); });
  onDestroy(() => parar?.());

  const combinada = (nome: string) => ficha?.combinadas.find((c) => c.nome === nome);
  /** As ações que o cliente pede ao monstro, cada uma com a combinada que responde (a própria ou a alternativa). */
  const pedidos = $derived((ficha?.diagnostico_de_monstro ?? []).map((d) => {
    const c = combinada(d.pedido) ?? (d.alternativa ? combinada(d.alternativa) : undefined);
    return { ...d, combinada: c };
  }));
  const outras = $derived((ficha?.combinadas ?? []).filter((c) => !pedidos.some((p) => p.combinada?.nome === c.nome)));

  async function mudarBase(acao: string, base: string) {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para editar as animações."); return; }
    ocupado = true;
    try { await trocarBase(caminho, acao, base); avisar("ok", `${traduzir(acao)} agora usa a animação ${traduzir(base)} (${base}).`); await carregar(); tocar(base); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
  async function criarAcao(pedido: string, base: string) {
    const molde = ficha?.combinadas.find((c) => c.bases.length === 1);
    if (!molde) { avisar("erro", "O modelo não tem ação de uma só animação para servir de molde."); return; }
    ocupado = true;
    try { await acaoNova(caminho, molde.nome, pedido, base); avisar("ok", `Ação ${traduzir(pedido)} criada com a animação ${traduzir(base)}.`); await carregar(); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
  /** Modelo completo de um GLB: cria os arquivos no rascunho (o .ecm atual é o molde) e o monstro passa a usar o .ecm novo. */
  async function importarModelo(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    (e.target as HTMLInputElement).value = "";
    if (!f) return;
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para importar."); return; }
    const titulo = (tituloNovo.trim() || f.name.replace(/\.glb$/i, "")).trim();
    if (!pastaNova.trim().toLowerCase().startsWith("models\\")) { avisar("aviso", "A pasta tem de ficar dentro de models\\ (ex.: models\\npcs\\estudio)."); return; }
    ocupado = true;
    try {
      const glb = await f.arrayBuffer();
      let d;
      try { d = await importarModeloGlb(pastaNova.trim(), titulo, caminho, glb); }
      catch (er) {
        // O .ecm atual não serve de molde quando tem peles adicionais (regra do B299): o servidor escolhe um sozinho.
        if (!/molde/i.test((er as Error).message)) throw er;
        avisar("aviso", `O .ecm atual não serve de molde (${(er as Error).message}); usando o molde automático.`);
        d = await importarModeloGlb(pastaNova.trim(), titulo, "", glb);
      }
      const r = d?.resultado ?? {};
      if (!r.ecm) throw new Error("O servidor não devolveu o .ecm do modelo novo.");
      avisar("ok", `Modelo novo no rascunho: ${r.ossos ?? "?"} ossos, ${r.malhas ?? "?"} malhas, ${(r.acoes ?? []).length} animações (${r.ecm}).${(r.relatorio ?? []).length ? " Relatório: " + r.relatorio.join(" · ") : ""}`);
      if (aoTrocarModelo) await aoTrocarModelo(r.ecm);
    } catch (er) { avisar("erro", (er as Error).message); }
    finally { ocupado = false; }
  }

  async function importar(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    (e.target as HTMLInputElement).value = "";
    if (!f || !ficha) return;
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para importar."); return; }
    ocupado = true;
    try {
      const d = await importarAcoesGlb(ficha.smd, await f.arrayBuffer());
      const r = d?.resultado ?? {};
      avisar("ok", `Animações importadas: ${(r.acoes ?? []).map((a: any) => a.nome).join(", ") || "nenhuma"}. O esqueleto agora tem ${r.acoes_do_modelo ?? "?"}.`);
      await carregar();
    } catch (er) { avisar("erro", (er as Error).message); }
    finally { ocupado = false; }
  }
</script>

<div class="cards">
<section class="caixa visual">
  <h3>Modelo 3D</h3>
  <canvas bind:this={tela} width="560" height="420"></canvas>
  <p class="mini">{carregando ? "Carregando…" : erroDoGlb || (tocando ? `Tocando: ${traduzir(tocando)} (${tocando})` : "Pose parada — escolha uma animação")} · arraste para girar, roda para aproximar.</p>
  <div class="linha">
    <a class="botao pequeno" href={rotaDoGlb(caminho, tocando || undefined)} download><Icone nome="abrir" tamanho={16} /> Exportar GLB (Blender)</a>
    <label class="botao pequeno" class:desligado={!editavel || ocupado}>
      <Icone nome="mais" tamanho={16} /> Importar animações (GLB)
      <input type="file" accept=".glb,model/gltf-binary" hidden disabled={!editavel || ocupado} onchange={importar} />
    </label>
  </div>
  <p class="mini">Exportar leva o modelo com a animação em prévia; no Blender, cada ação do glTF que voltar pelo importar vira uma animação do esqueleto (substitui a de mesmo nome).</p>
  <button class="botao" type="button" disabled title="editor 3D: depois">Editar malha</button>
</section>


{#if erro}<section class="caixa"><p class="erro">{erro}</p></section>
{:else if !ficha}<section class="caixa"><p class="suave">Carregando o modelo…</p></section>
{:else}
  <section class="caixa">
    <h3>O que ele faz em cada ação</h3>
    <ul class="acoes">
      {#each pedidos as p (p.indice)}
        <li class:falta={!p.combinada}>
          <div class="cab">
            <strong>{traduzir(p.pedido)}</strong> <span class="mini">{p.pedido}{p.alternativa ? ` → usa ${p.alternativa}` : ""}</span>
          </div>
          {#if p.combinada && p.combinada.bases.length === 1}
            <div class="linha">
              <select class="campo" disabled={!editavel || ocupado} value={p.combinada.bases[0]} aria-label="Animação de {p.pedido}"
                onchange={(e) => mudarBase(p.combinada!.nome, (e.target as HTMLSelectElement).value)}>
                {#each ficha.bases_do_smd ?? [] as b (b)}<option value={b}>{traduzir(b)} · {b}</option>{/each}
              </select>
              <button class="botao pequeno" type="button" onclick={() => tocar(p.combinada!.bases[0])} aria-label="Ver"><Icone nome="olho" tamanho={16} /></button>
            </div>
            {#if p.combinada.bases_ausentes.length}<span class="erro mini">animação ausente no esqueleto: {p.combinada.bases_ausentes.join(", ")}</span>{/if}
          {:else if p.combinada}
            <span class="mini">Sequência de {p.combinada.bases.length} animações ({p.combinada.bases.map(traduzir).join(" → ")}): edite nos Editores → Modelos.</span>
          {:else}
            <div class="linha">
              <span class="erro mini">Falta: o cliente pede e o modelo não tem.</span>
              <select class="campo" disabled={!editavel || ocupado} aria-label="Criar {p.pedido}" onchange={(e) => criarAcao(p.pedido, (e.target as HTMLSelectElement).value)}>
                <option value="">criar com a animação…</option>
                {#each ficha.bases_do_smd ?? [] as b (b)}<option value={b}>{traduzir(b)} · {b}</option>{/each}
              </select>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if outras.length}
      <details><summary class="mini">Outras ações do modelo ({outras.length})</summary>
        <ul class="acoes">{#each outras as c (c.nome)}<li><strong>{traduzir(c.nome)}</strong> <span class="mini">{c.nome} · {c.bases.map(traduzir).join(" → ") || "sem animação"}</span></li>{/each}</ul>
      </details>
    {/if}
  </section>
  <section class="caixa">
    <h3>Animações do esqueleto <span class="mini">{ficha.bases_do_smd?.length ?? 0}</span></h3>
    <p class="mini">Clique para ver no Modelo 3D.</p>
    <div class="chips">
      {#each ficha.bases_do_smd ?? [] as b (b)}
        <button type="button" class:ativo={tocando === b} onclick={() => tocar(b)} title={b}>▶ {traduzir(b)}</button>
      {/each}
    </div>
  </section>
{/if}
<section class="caixa novo">
  <h3>Trocar o modelo inteiro</h3>
    <p class="mini">Por um GLB com malha, texturas, esqueleto e animações.</p>
    <p class="mini">Cria no rascunho <code>&lt;pasta&gt;\&lt;título&gt;</code>.smd, .bon, .ski, as texturas, as animações e o .ecm (o .ecm atual serve de molde:
      herda o cabeçalho e as ações), e o monstro passa a usar o .ecm novo. Recusa pasta fora de models\, nome fora do GBK e arquivo que já existe.
      Ao montar no Blender, nomeie as ações como as do cliente (站立, 普攻1起, 死亡…) para ligar sozinho.</p>
    <div class="linha">
      <input class="campo" bind:value={pastaNova} aria-label="Pasta do modelo novo" placeholder="models\npcs\estudio" />
      <input class="campo" bind:value={tituloNovo} aria-label="Título" placeholder={idDoMonstro ? `monstro_${idDoMonstro}` : "título (nome dos arquivos)"} />
      <label class="botao pequeno" class:desligado={!editavel || ocupado}>
        <Icone nome="mais" tamanho={16} /> Escolher GLB…
        <input type="file" accept=".glb,model/gltf-binary" hidden disabled={!editavel || ocupado} onchange={importarModelo} />
      </label>
    </div>
</section>
</div>

<style>
  .visual { display: flex; flex-direction: column; gap: 8px; }
  canvas { width: 100%; max-width: 560px; align-self: center; aspect-ratio: 4 / 3; background: #20242a; border-radius: 8px; cursor: grab; }
  .cards { display: flex; flex-direction: column; gap: 14px; }
  .acoes { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .acoes li { display: flex; flex-direction: column; gap: 4px; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .acoes li.falta { border-color: var(--vermelho); }
  .acoes select { flex: 1; min-width: 0; height: 30px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips button { padding: 4px 8px; border-radius: 999px; border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; font-size: 12px; cursor: pointer; }
  .chips button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .desligado { opacity: .45; pointer-events: none; }
  .novo .linha input { flex: 1; min-width: 120px; }
</style>
