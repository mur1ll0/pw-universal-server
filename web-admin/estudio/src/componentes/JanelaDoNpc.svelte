<script lang="ts">
  /* B392: a janela do NPC como o cliente a monta (`CDlgNPC::PopupNPCDialog`, `DlgNPC.cpp:339-1135`): nome, fala inicial e
     as linhas — serviços simples, serviços com tabela (o texto é o `Name` do serviço), missões e "Sair". Clicar numa linha
     chama `aoClicar` (a tela do NPC abre a conversa ou o serviço). */
  import type { LinhaDoMenu } from "../lib/npc";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { nome, fala, linhas, aoClicar }: { nome: string; fala: string; linhas: LinhaDoMenu[]; aoClicar?: (l: LinhaDoMenu) => void } = $props();
</script>

<div class="jogo">
  <div class="cab"><span><TextoDoJogo texto={nome} padrao="#f6dc9a" /></span><span aria-hidden="true">✕</span></div>
  <div class="texto"><TextoDoJogo texto={fala || ""} /></div>
  <ul>
    {#each linhas as l, k (k)}
      <li class={l.tipo}>
        <button type="button" disabled={!aoClicar} onclick={() => aoClicar?.(l)} title={l.nota ?? ""}>
          <TextoDoJogo texto={l.texto} padrao={l.tipo === "missao" ? "#9fe39f" : "#f6dc9a"} linha />
        </button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .jogo { background: #0d1117; border: 2px solid #5a4a2a; border-radius: 6px; color: #f1e6c8; font-size: 13px; overflow: hidden; }
  .cab { background: linear-gradient(#3a2f1a, #22190b); padding: 6px 10px; font-weight: 600; display: flex; justify-content: space-between; color: #f6dc9a; }
  .texto { padding: 10px 12px; min-height: 70px; line-height: 1.5; }
  ul { list-style: none; margin: 0; padding: 4px 8px 10px; display: flex; flex-direction: column; gap: 1px; }
  button { width: 100%; text-align: left; background: none; border: 0; padding: 3px 6px; border-radius: 4px; font: inherit; cursor: pointer; display: flex; min-width: 0; }
  button:hover:not(:disabled) { background: rgba(246, 220, 154, .12); }
  button:disabled { cursor: default; }
  li.simples button::before, li.servico button::before { content: "◆ "; color: #b8923d; font-size: 10px; margin-right: 4px; flex: none; }
  li.missao button::before { content: "! "; color: #9fe39f; font-weight: 700; margin-right: 4px; flex: none; }
  li.sair button { color: #c9b98f; }
</style>
