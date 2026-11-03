<script lang="ts">
  /* B392: texto com cor para falas e conversas. A cor vira `^RRGGBB` no texto (como o cliente lê, `lib/marcacao.ts`);
     o contador conta as unidades do arquivo, códigos inclusos — é isso que o limite do campo mede. */
  import { aplicarCor, ler, PALETA, tirarCores } from "../lib/marcacao";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { valor, limite, aoMudar, linhas = 3, desabilitado = false, rotulo = "Texto", previa = true, umaLinha = false }:
    { valor: string; limite: number; aoMudar: (v: string) => void; linhas?: number; desabilitado?: boolean; rotulo?: string;
      previa?: boolean; umaLinha?: boolean } = $props();

  let caixa = $state<HTMLTextAreaElement | HTMLInputElement | null>(null);
  let livre = $state("#ffcb4a");
  const leitura = $derived(ler(valor ?? ""));
  /** `limite` 0 = sem limite fixo no arquivo (texto de janela de conversa: tamanho variável, `conversas.rs`). */
  const estoura = $derived(limite > 0 && (valor ?? "").length > limite);

  function pintar(cor: string) {
    if (!caixa || desabilitado) return;
    const ini = caixa.selectionStart ?? valor.length, fim = caixa.selectionEnd ?? ini;
    const r = aplicarCor(valor ?? "", ini, fim, cor);
    aoMudar(r.texto);
    queueMicrotask(() => { caixa?.focus(); caixa?.setSelectionRange(r.cursor, r.cursor); });
  }
  function limpar() {
    if (!caixa || desabilitado) return;
    aoMudar(tirarCores(valor ?? "", caixa.selectionStart ?? 0, caixa.selectionEnd ?? 0));
  }
</script>

<div class="editor">
  <div class="ferramentas">
    <span class="mini">{rotulo}</span>
    <div class="paleta" role="group" aria-label="Cores">
      {#each PALETA as p (p.cor)}
        <button type="button" style:background="#{p.cor}" title="{p.nome} (^{p.cor})" aria-label={p.nome} disabled={desabilitado}
          onmousedown={(e) => e.preventDefault()} onclick={() => pintar(p.cor)}></button>
      {/each}
      <input type="color" bind:value={livre} aria-label="Outra cor" disabled={desabilitado} />
      <button class="botao pequeno" type="button" disabled={desabilitado} onmousedown={(e) => e.preventDefault()} onclick={() => pintar(livre)}>Usar</button>
      <button class="botao pequeno" type="button" disabled={desabilitado} onmousedown={(e) => e.preventDefault()} onclick={limpar} title="Tira as cores da seleção (ou do texto todo)">Tirar cor</button>
    </div>
    <span class="contador" class:estoura>{(valor ?? "").length}{limite > 0 ? `/${limite}` : " unidades"}</span>
  </div>
  {#if umaLinha}
    <input class="campo mono" bind:this={caixa} value={valor} disabled={desabilitado} aria-label={rotulo}
      oninput={(e) => aoMudar((e.target as HTMLInputElement).value)} />
  {:else}
    <textarea class="campo mono" bind:this={caixa} rows={linhas} value={valor} disabled={desabilitado} aria-label={rotulo}
      oninput={(e) => aoMudar((e.target as HTMLTextAreaElement).value)}></textarea>
  {/if}
  {#if estoura}<span class="erro mini">Passa do limite do arquivo ({limite} unidades, contando os códigos de cor): não grava.</span>{/if}
  {#if leitura.invalidos.length}<span class="erro mini">Há "^" seguido de algo que não é cor (RRGGBB): o cliente engole 6 caracteres depois dele.</span>{/if}
  {#if previa}
    <div class="previa"><TextoDoJogo texto={valor ?? ""} /></div>
  {/if}
</div>

<style>
  .editor { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .ferramentas { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .paleta { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; flex: 1; }
  .paleta > button:not(.botao) { width: 20px; height: 20px; border-radius: 5px; border: 1px solid var(--borda-forte); cursor: pointer; padding: 0; }
  .paleta input[type="color"] { width: 28px; height: 24px; padding: 0; border: 1px solid var(--borda-forte); border-radius: 5px; background: none; }
  .contador { font-variant-numeric: tabular-nums; font-size: 12px; color: var(--texto-3); }
  .contador.estoura { color: var(--vermelho); font-weight: 600; }
  .mono { font-family: "Cascadia Mono", Consolas, ui-monospace, monospace; font-size: 13px; }
  textarea.campo { min-height: 60px; line-height: 1.45; }
  .previa { background: #0d1117; border: 1px solid #5a4a2a; border-radius: 6px; padding: 8px 10px; font-size: 13px; line-height: 1.5; min-height: 34px; }
</style>
