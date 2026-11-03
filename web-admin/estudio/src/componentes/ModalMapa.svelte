<script lang="ts">
  /* B371: escolher no mapa o ponto e a área de um spawner do npcgen.
     Geometria do minimapa (a mesma dos Editores, `static/mapa.js:93-151`): o mapa tem `colunas × linhas` blocos de 1024
     unidades; a imagem tem 512 px por bloco, em peças de 256 px (`pecas/LLCC.png`); px = (x + colunas·512)/2,
     py = (linhas·512 − z)/2.
     Área: o servidor sorteia o nascimento no retângulo `vPos ± vExts/2` (`npcgenerator.cpp:4348-4367`) — o formato só
     tem retângulo alinhado aos eixos, então é isso que se desenha (não há polígono livre no npcgen).
     B392: `comDirecao` (NPC) — só o ponto (área sem tamanho) e uma seta: com a área num ponto o GS usa a direção do
     `vDir`, `atan2(z, x) × 128/π` (`npcgenerator.cpp:4348-4367`); com tamanho sorteia. A seta se arrasta pela bolinha. */
  import { onMount } from "svelte";
  import { urlDaPeca, type EscolhaNoMapa as Escolha, type MapaDoRealm } from "../lib/api";
  import Modal from "./Modal.svelte";

  let { mapa, inicial, outros = [], aoAplicar, aoFechar, comDirecao = false, titulo = "" }:
    { mapa: MapaDoRealm; inicial: Escolha | null; outros?: { x: number; z: number; largura: number; altura: number; rotulo: string }[];
      aoAplicar: (e: Escolha) => void; aoFechar: () => void; comDirecao?: boolean; titulo?: string } = $props();

  const LARGURA = mapa.colunas * 512, ALTURA = mapa.linhas * 512; // px da imagem inteira
  let caixa = $state<HTMLDivElement | null>(null);
  let w = $state(800), h = $state(560);
  let escala = $state(1), ox = $state(0), oy = $state(0);
  let ponto = $state<Escolha | null>(inicial ? { ...inicial, ...(comDirecao ? { largura: 0, altura: 0, direcao: inicial.direcao ?? 0 } : {}) } : null);
  const RAIO_DA_SETA = 46; // px na tela

  const pxDe = (x: number) => (x + mapa.colunas * 512) / 2;
  const pyDe = (z: number) => (mapa.linhas * 512 - z) / 2;
  const xDe = (px: number) => 2 * px - mapa.colunas * 512;
  const zDe = (py: number) => mapa.linhas * 512 - 2 * py;
  const tela = (px: number, py: number) => [ox + px * escala, oy + py * escala];

  function enquadrar() {
    if (!caixa) return;
    w = caixa.clientWidth; h = caixa.clientHeight;
    if (ponto) {
      // Começa perto do ponto, com a área ocupando uma boa parte da tela.
      escala = Math.min(4, Math.max(0.25, Math.min(w, h) / Math.max(200, (comDirecao ? 60 : ponto.largura) * 3)));
      ox = w / 2 - pxDe(ponto.x) * escala; oy = h / 2 - pyDe(ponto.z) * escala;
    } else {
      escala = Math.min(w / LARGURA, h / ALTURA); ox = (w - LARGURA * escala) / 2; oy = (h - ALTURA * escala) / 2;
    }
  }
  // A caixa só tem tamanho depois que o modal aparece: enquadra na primeira medida e acompanha o redimensionamento.
  onMount(() => {
    let primeira = true;
    const ro = new ResizeObserver(() => {
      if (!caixa || !caixa.clientWidth) return;
      if (primeira) { primeira = false; enquadrar(); } else { w = caixa.clientWidth; h = caixa.clientHeight; }
    });
    if (caixa) ro.observe(caixa);
    return () => ro.disconnect();
  });

  // Peças visíveis.
  const pecas = $derived.by(() => {
    const r: { nome: string; l: number; c: number }[] = [];
    const l0 = Math.max(0, Math.floor(-oy / escala / 256)), l1 = Math.min(mapa.linhas * 2 - 1, Math.floor((h - oy) / escala / 256));
    const c0 = Math.max(0, Math.floor(-ox / escala / 256)), c1 = Math.min(mapa.colunas * 2 - 1, Math.floor((w - ox) / escala / 256));
    for (let l = l0; l <= l1; l++) for (let c = c0; c <= c1; c++) r.push({ nome: `${String(l).padStart(2, "0")}${String(c).padStart(2, "0")}`, l, c });
    return r;
  });

  // Arrastar (mover o mapa), alça (tamanho da área) e clique (ponto).
  let arrasto: null | { tipo: "mapa" | "alca" | "seta"; x: number; y: number; moveu: boolean } = null;
  function baixar(e: PointerEvent, tipo: "mapa" | "alca" | "seta") {
    e.preventDefault(); e.stopPropagation();
    arrasto = { tipo, x: e.clientX, y: e.clientY, moveu: false };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }
  function mover(e: PointerEvent) {
    if (!arrasto) return;
    const dx = e.clientX - arrasto.x, dy = e.clientY - arrasto.y;
    if (Math.abs(dx) + Math.abs(dy) > 3) arrasto.moveu = true;
    if (arrasto.tipo === "mapa") { ox += dx; oy += dy; }
    else if (arrasto.tipo === "seta" && ponto) {
      // Ângulo na tela (y para baixo = −z) → graus do mundo: 0 = +x, 90 = +z.
      const r = caixa!.getBoundingClientRect();
      const [cx, cy] = tela(pxDe(ponto.x), pyDe(ponto.z));
      const g = (Math.atan2(-(e.clientY - r.top - cy), e.clientX - r.left - cx) * 180) / Math.PI;
      ponto.direcao = Math.round(((g % 360) + 360) % 360);
    }
    else if (ponto) {
      // A alça fica no canto inferior direito: a meia-largura é a distância do centro, em unidades do mundo (2 por px).
      const r = caixa!.getBoundingClientRect();
      const px = (e.clientX - r.left - ox) / escala, py = (e.clientY - r.top - oy) / escala;
      ponto.largura = Math.max(1, Math.round(Math.abs(xDe(px) - ponto.x) * 2));
      ponto.altura = Math.max(1, Math.round(Math.abs(zDe(py) - ponto.z) * 2));
    }
    arrasto.x = e.clientX; arrasto.y = e.clientY;
  }
  function soltar(e: PointerEvent) {
    if (arrasto?.tipo === "mapa" && !arrasto.moveu) {
      const r = caixa!.getBoundingClientRect();
      const px = (e.clientX - r.left - ox) / escala, py = (e.clientY - r.top - oy) / escala;
      const x = Math.round(xDe(px) * 10) / 10, z = Math.round(zDe(py) * 10) / 10;
      ponto = comDirecao ? { x, z, largura: 0, altura: 0, direcao: ponto?.direcao ?? 0 } : { x, z, largura: ponto?.largura ?? 20, altura: ponto?.altura ?? 20 };
    }
    arrasto = null;
  }
  function roda(e: WheelEvent) {
    e.preventDefault();
    const r = caixa!.getBoundingClientRect();
    const mx = e.clientX - r.left, my = e.clientY - r.top;
    const f = e.deltaY > 0 ? 0.8 : 1.25;
    const nova = Math.min(16, Math.max(0.05, escala * f));
    ox = mx - ((mx - ox) * nova) / escala; oy = my - ((my - oy) * nova) / escala; escala = nova;
  }
  const ret = (p: { x: number; z: number; largura: number; altura: number }) => {
    const [x1, y1] = tela(pxDe(p.x - p.largura / 2), pyDe(p.z + p.altura / 2));
    const [x2, y2] = tela(pxDe(p.x + p.largura / 2), pyDe(p.z - p.altura / 2));
    return { x: x1, y: y1, w: Math.max(2, x2 - x1), h: Math.max(2, y2 - y1) };
  };
</script>

<Modal titulo="{titulo || 'Onde nasce'} — {mapa.nome} ({mapa.caminho})" {aoFechar} largo>
  {#if comDirecao}
    <p class="mini">Clique para pôr o NPC; arraste a bolinha da seta para girá-lo; arraste o mapa para mover e use a roda para aproximar.
      A área fica num ponto, para o servidor usar esta direção (com tamanho ele sorteia).</p>
  {:else}
  <p class="mini">Clique para pôr o centro; arraste a alça (canto) para o tamanho da área; arraste o mapa para mover e use a roda para aproximar.
    O servidor sorteia cada nascimento dentro do retângulo.</p>
  {/if}
  <div class="mapa" bind:this={caixa} role="application" aria-label="Mapa"
    onpointerdown={(e) => baixar(e, "mapa")} onpointermove={mover} onpointerup={soltar} onwheel={roda}>
    <div class="camada" style:transform="translate({ox}px, {oy}px) scale({escala})" style:width="{LARGURA}px" style:height="{ALTURA}px">
      {#each pecas as p (p.nome)}
        <img src={urlDaPeca(mapa.caminho, p.nome)} alt="" style:left="{p.c * 256}px" style:top="{p.l * 256}px" draggable="false"
          onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")} />
      {/each}
    </div>
    <svg class="sobre" width={w} height={h}>
      {#each outros as o, k (k)}
        {@const r = ret(o)}
        <rect x={r.x} y={r.y} width={r.w} height={r.h} class="outro"><title>{o.rotulo}</title></rect>
      {/each}
      {#if ponto}
        {@const r = ret(ponto)}
        {@const [cx, cy] = tela(pxDe(ponto.x), pyDe(ponto.z))}
        {#if comDirecao}
          {@const rad = ((ponto.direcao ?? 0) * Math.PI) / 180}
          {@const sx = cx + Math.cos(rad) * RAIO_DA_SETA}
          {@const sy = cy - Math.sin(rad) * RAIO_DA_SETA}
          <circle cx={cx} cy={cy} r={RAIO_DA_SETA} class="giro" />
          <line x1={cx} y1={cy} x2={sx} y2={sy} class="seta" />
          <circle cx={cx} cy={cy} r="7" class="centro" />
          <circle cx={sx} cy={sy} r="8" class="alca" role="slider" aria-label="Direção do NPC" aria-valuenow={ponto.direcao ?? 0}
            tabindex="0" onpointerdown={(e) => baixar(e, "seta")} />
        {:else}
        <rect x={r.x} y={r.y} width={r.w} height={r.h} class="area" />
        <circle cx={cx} cy={cy} r="5" class="centro" />
        <rect x={r.x + r.w - 6} y={r.y + r.h - 6} width="12" height="12" class="alca" role="slider" aria-label="Tamanho da área" aria-valuenow={ponto.largura}
          tabindex="0" onpointerdown={(e) => baixar(e, "alca")} />
        {/if}
      {/if}
    </svg>
  </div>
  {#if ponto}
    <div class="linha valores">
      <label class="mini">x <input class="campo n" type="number" step="0.1" bind:value={ponto.x} /></label>
      <label class="mini">z <input class="campo n" type="number" step="0.1" bind:value={ponto.z} /></label>
      {#if comDirecao}
        <label class="mini">direção (graus; 0 = leste, 90 = norte) <input class="campo n" type="number" step="1" min="0" max="359" bind:value={ponto.direcao} /></label>
      {:else}
      <label class="mini">largura (x) <input class="campo n" type="number" step="1" min="0" bind:value={ponto.largura} /></label>
      <label class="mini">altura (z) <input class="campo n" type="number" step="1" min="0" bind:value={ponto.altura} /></label>
      {/if}
      <button class="botao pequeno" type="button" onclick={enquadrar}>Centralizar</button>
    </div>
  {:else}
    <p class="suave">Clique no mapa para escolher o ponto.</p>
  {/if}
  {#snippet rodape()}
    <button class="botao" type="button" onclick={aoFechar}>Cancelar</button>
    <button class="botao primario" type="button" disabled={!ponto} onclick={() => { if (ponto) { aoAplicar({ ...ponto }); aoFechar(); } }}>Usar este lugar</button>
  {/snippet}
</Modal>

<style>
  .mapa { position: relative; height: min(60vh, 560px); overflow: hidden; background: #0d1117; border-radius: 10px; cursor: crosshair; touch-action: none; user-select: none; }
  .camada { position: absolute; left: 0; top: 0; transform-origin: 0 0; }
  .camada img { position: absolute; width: 256px; height: 256px; pointer-events: none; }
  .sobre { position: absolute; left: 0; top: 0; pointer-events: none; }
  .area { fill: rgba(227, 182, 91, 0.18); stroke: var(--ouro); stroke-width: 2; }
  .outro { fill: rgba(106, 167, 255, 0.12); stroke: var(--azul); stroke-width: 1; stroke-dasharray: 4 3; }
  .centro { fill: var(--ouro); stroke: #000; }
  .alca { fill: var(--ouro); stroke: #000; cursor: nwse-resize; pointer-events: all; }
  circle.alca { cursor: grab; }
  .giro { fill: rgba(227, 182, 91, 0.08); stroke: rgba(227, 182, 91, 0.6); stroke-dasharray: 4 4; }
  .seta { stroke: var(--ouro); stroke-width: 3; }
  .valores { margin-top: 8px; }
  .n { width: 110px; height: 28px; }
</style>
