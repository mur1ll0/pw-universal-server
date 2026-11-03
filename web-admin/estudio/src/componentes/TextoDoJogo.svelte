<script lang="ts">
  /* B392: um texto com a marcação do cliente (`^RRGGBB`…, `lib/marcacao.ts`) desenhado como o jogo: sem os códigos,
     com as cores. Fundo escuro de janela do jogo (as janelas do cliente são escuras nos dois temas do painel). */
  import { ler } from "../lib/marcacao";

  let { texto, padrao = "#f1e6c8", linha = false }: { texto: string; padrao?: string; linha?: boolean } = $props();
  const leitura = $derived(ler(texto ?? ""));
</script>

<span class="jogo" class:linha style:--padrao={padrao}>{#each leitura.trechos as t, k (k)}<span style:color={t.cor ? `#${t.cor}` : null}
  class:sub={t.sublinhado}>{t.texto}</span>{/each}</span>

<style>
  .jogo { color: var(--padrao); white-space: pre-wrap; word-break: break-word; }
  .jogo.linha { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { text-decoration: underline; }
</style>
