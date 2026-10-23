<script lang="ts">
  /* Visual 3D do item: os modelos da ficha (`file_model_right`, `file_model_left`, `file_matter`) pelo GLB dos Editores.
     O brilho do refino não está no GLB: é um GFX que o cliente escolhe pela cor e pelo nível
     (`EC_Player.cpp:9299-9316`), e o painel não tem renderizador de GFX — o botão fica desligado (falta registrada). */
  import { onDestroy } from "svelte";
  import { rotaDoGlb } from "../lib/api";
  import { desenharGlb } from "../lib/glb";

  let { modelos }: { modelos: { rotulo: string; caminho: string }[] } = $props();
  const validos = $derived(modelos.filter((m) => m.caminho && m.caminho.trim()));
  let escolha = $state(0);
  let tela = $state<HTMLCanvasElement | null>(null);
  let erro = $state("");
  let carregando = $state(false);
  let parar: (() => void) | null = null;

  async function carregar(caminho: string) {
    parar?.(); parar = null; erro = ""; carregando = true;
    try {
      const r = await fetch(rotaDoGlb(caminho), { credentials: "same-origin" });
      if (!r.ok) throw new Error(`GLB: HTTP ${r.status}${r.status === 400 ? " — " + ((await r.json().catch(() => ({}))).detail || "") : ""}`);
      const buf = await r.arrayBuffer();
      if (tela) parar = desenharGlb(tela, buf);
    } catch (e) { erro = (e as Error).message; }
    finally { carregando = false; }
  }
  $effect(() => { const m = validos[escolha] ?? validos[0]; if (m && tela) carregar(m.caminho); });
  onDestroy(() => parar?.());
</script>

<section class="caixa visual">
  <h3>Visual 3D</h3>
  {#if validos.length}
    <div class="linha">
      <select class="campo" bind:value={escolha} aria-label="Modelo">
        {#each validos as m, i (m.rotulo)}<option value={i}>{m.rotulo}</option>{/each}
      </select>
      <label class="mini"><input type="checkbox" disabled /> efeito do refino</label>
    </div>
    <canvas bind:this={tela} width="560" height="420"></canvas>
    <p class="mini">{carregando ? "Carregando…" : erro || "Arraste para girar, roda para aproximar."} · {validos[escolha]?.caminho}</p>
    <p class="mini">Efeito do refino: falta — é um GFX do cliente (EC_Player.cpp:9299), não está no GLB.</p>
  {:else}
    <p class="suave">Sem modelo na ficha.</p>
  {/if}
  <button class="botao" type="button" disabled title="editor 3D: depois">Editar</button>
</section>

<style>
  .visual { display: flex; flex-direction: column; gap: 8px; }
  canvas { width: 100%; max-width: 560px; align-self: center; aspect-ratio: 4 / 3; background: #20242a; border-radius: 8px; cursor: grab; }
</style>
