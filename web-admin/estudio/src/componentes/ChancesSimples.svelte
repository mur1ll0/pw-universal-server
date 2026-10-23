<script lang="ts">
  /* Chances de posição fixa (furos 0–2, quantos efeitos 0–5): um `float` por posição, normalizado pelo servidor ao
     carregar (`NormalizeRandom`, `elementdataman.cpp:626-628`). */
  import { ajustar, porcentagens } from "../lib/chances";

  let { titulo, rotulos, pesos, editavel, aoMudar }:
    { titulo: string; rotulos: string[]; pesos: number[]; editavel: boolean; aoMudar: (p: number[]) => void } = $props();
  const pct = $derived(porcentagens(pesos));
  const fmt = (x: number) => x.toLocaleString("pt-BR", { maximumFractionDigits: 2 });
</script>

<div class="bloco">
  <h4>{titulo}</h4>
  {#each pesos as _, i (i)}
    <label class="linha-chance">
      <span>{rotulos[i]}</span>
      <input type="range" min="0" max="1000" step="1" value={Math.round(pct[i] * 10)} disabled={!editavel || pesos.length < 2}
        oninput={(e) => aoMudar(ajustar(pesos, i, Number((e.target as HTMLInputElement).value) / 1000))} />
      <span class="pct">{fmt(pct[i])}%</span>
    </label>
  {/each}
</div>

<style>
  .bloco { display: flex; flex-direction: column; gap: 4px; }
  h4 { margin: 4px 0; font-size: 13px; color: var(--texto-2); }
  .linha-chance { display: grid; grid-template-columns: 80px 1fr 64px; align-items: center; gap: 8px; font-size: 13px; }
  .pct { text-align: right; font-variant-numeric: tabular-nums; }
  input[type="range"] { width: 100%; accent-color: var(--ouro); }
</style>
