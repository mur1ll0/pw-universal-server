<script lang="ts">
  /* B368: escolha de efeito para uma peça — todos os que servem na família dela (a regra do GS, `lib/efeitos.ts`),
     com o texto do cliente e o valor; filtro por texto, habilidade, nome interno ou id. */
  import { efeitosDaFamilia, filtrarEfeitos, type EfeitoDisponivel } from "../lib/efeitos";
  import Modal from "./Modal.svelte";

  let { titulo, tabela, aoEscolher, aoFechar }:
    { titulo: string; tabela: string; aoEscolher: (e: EfeitoDisponivel) => void; aoFechar: () => void } = $props();

  const LIMITE = 300;
  let todos = $state<EfeitoDisponivel[] | null>(null);
  let erro = $state("");
  let busca = $state("");
  let escolhido = $state<EfeitoDisponivel | null>(null);
  const achados = $derived(todos ? filtrarEfeitos(todos, busca) : []);

  efeitosDaFamilia(tabela).then((l) => (todos = l)).catch((e) => (erro = (e as Error).message));

  function usar(e: EfeitoDisponivel | null) {
    if (!e) return;
    aoEscolher(e);
    aoFechar();
  }
</script>

<Modal {titulo} {aoFechar} largo>
  <input class="campo cheio" placeholder="texto do efeito, habilidade, nome interno ou id (ex.: Mordaz, Crítico, 1286)" bind:value={busca} aria-label="Buscar efeito" />
  {#if erro}
    <p class="erro">{erro}</p>
  {:else if !todos}
    <p class="suave">Carregando os efeitos que servem em {tabela} (a primeira vez leva alguns segundos)…</p>
  {:else}
    <p class="mini">{achados.length} de {todos.length} efeitos que servem nesta família{achados.length > LIMITE ? `; mostrando ${LIMITE} — refine a busca` : ""}.</p>
    <ul class="lista">
      {#each achados.slice(0, LIMITE) as e (e.id)}
        <li>
          <button type="button" class:ativo={escolhido?.id === e.id} onclick={() => (escolhido = e)} ondblclick={() => usar(e)}>
            <span class="nome">{e.texto || e.nome || "(sem texto)"}</span>
            <span class="mini">{e.habilidade ? `habilidade ${e.habilidade} · ` : ""}{e.nome} · id {e.id}</span>
          </button>
        </li>
      {:else}
        <li class="suave">Nada encontrado.</li>
      {/each}
    </ul>
  {/if}
  {#snippet rodape()}
    <button class="botao primario" type="button" disabled={!escolhido} onclick={() => usar(escolhido)}>Escolher</button>
  {/snippet}
</Modal>

<style>
  .cheio { width: 100%; }
  .lista { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 55vh; overflow: auto; }
  .lista button { width: 100%; display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 6px 8px; border-radius: 8px;
    border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .lista button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
</style>
