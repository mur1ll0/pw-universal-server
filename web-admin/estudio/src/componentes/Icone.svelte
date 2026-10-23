<script lang="ts">
  /* Ícones de traço 24×24 (os mesmos desenhos do `ICONES` do painel, `static/painel.js`). */
  const TRACOS: Record<string, string> = {
    espada: "M20 4l-9 9M20 4h-4M20 4v4M11 13l-3 3M5 14l5 5M7 17l-3 3",
    caixa: "M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10",
    pata: "M8 5a1.7 1.7 0 1 0 0 3.4a1.7 1.7 0 1 0 0-3.4M16 5a1.7 1.7 0 1 0 0 3.4a1.7 1.7 0 1 0 0-3.4M5 10a1.6 1.6 0 1 0 0 3.2a1.6 1.6 0 1 0 0-3.2M19 10a1.6 1.6 0 1 0 0 3.2a1.6 1.6 0 1 0 0-3.2M12 12c-3 0-5 3.5-5 5.5S9 20 12 20s5-.5 5-2.5S15 12 12 12z",
    npc: "M12 4a4 4 0 1 0 0 8a4 4 0 1 0 0-8M4 21c0-4 3.6-6 8-6s8 2 8 6M17 3l2 2 3-3",
    monstro: "M5 10l-2-6 5 3M19 10l2-6-5 3M5 10a7 7 0 0 0 14 0M9 13h.01M15 13h.01M9 17l1.5-1 1.5 1 1.5-1 1.5 1",
    mais: "M12 5v14M5 12h14",
    busca: "M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14M20 20l-4-4",
    lixeira: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    fechar: "M6 6l12 12M18 6L6 18",
    editar: "M4 20h4L19 9l-4-4L4 16zM14 6l4 4",
    voltar: "M15 18l-6-6 6-6",
    copiar: "M8 8h12v12H8zM4 16V4h12",
    abrir: "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
    editores: "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4",
    refino: "M12 19V5M5 12l7-7 7 7",
    check: "M5 12l5 5 9-10",
  };
  let { nome, tamanho = 20 }: { nome: string; tamanho?: number } = $props();
</script>

<svg class="icone" width={tamanho} height={tamanho} viewBox="0 0 24 24" aria-hidden="true">
  <path d={TRACOS[nome] ?? ""} />
</svg>

<style>
  .icone { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; flex: none; }
</style>
