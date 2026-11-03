<script lang="ts">
  /* Barra do Estúdio: realm e versão, e o rascunho onde toda edição vai (o mesmo dos Editores; publicar e reverter
     ficam em Recursos do cliente). */
  import { criarRascunho, rascunhos, type Rascunho } from "../lib/api";
  import { avisar, definirRascunho, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";

  let { pronto }: { pronto: boolean } = $props();
  let lista = $state<Rascunho[]>([]);
  let descricao = $state("");

  async function carregar() {
    try {
      lista = await rascunhos();
      if (estudio.rascunho && !lista.some((r) => r.id === estudio.rascunho)) definirRascunho(null);
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { if (pronto) carregar(); });

  async function criar(e: SubmitEvent) {
    e.preventDefault();
    if (!descricao.trim()) { avisar("aviso", "Descreva o rascunho."); return; }
    try {
      const id = await criarRascunho(descricao.trim());
      descricao = "";
      await carregar();
      definirRascunho(id);
      avisar("ok", `Rascunho #${id} criado.`);
    } catch (erro) { avisar("erro", (erro as Error).message); }
  }
</script>

<header class="topo">
  <a class="botao fantasma" href="/" title="Voltar ao painel"><Icone nome="voltar" /> Painel</a>
  <strong class="marca">Estúdio</strong>
  {#if estudio.realm}<span class="suave">Realm {estudio.realm} · cliente {estudio.versao}</span>{/if}
  {#if pronto}
    <div class="linha direita">
      <label class="suave" for="rascunho">Rascunho</label>
      <select id="rascunho" class="campo" value={estudio.rascunho ?? ""}
        onchange={(e) => definirRascunho(Number((e.target as HTMLSelectElement).value) || null)}>
        <option value="">— só ver o publicado —</option>
        {#each lista as r (r.id)}<option value={r.id}>#{r.id} {r.descricao ?? ""}</option>{/each}
      </select>
      <form class="linha" onsubmit={criar}>
        <input class="campo" maxlength="500" placeholder="Descrição de um rascunho novo" bind:value={descricao} aria-label="Descrição do rascunho novo" />
        <button class="botao" type="submit"><Icone nome="mais" /> Criar</button>
      </form>
    </div>
  {/if}
</header>
{#if pronto && !estudio.rascunho}
  <p class="faixa">Somente leitura: escolha ou crie um rascunho para editar. Publicar e reverter ficam em <a href="/">Recursos do cliente</a>.</p>
{/if}

<style>
  .topo { position: sticky; top: 0; z-index: 20; display: flex; align-items: center; gap: 12px; flex-wrap: wrap; padding: 10px 16px;
    background: var(--barra-topo); backdrop-filter: blur(8px); border-bottom: 1px solid var(--borda); }
  .marca { font-size: 16px; color: var(--ouro); }
  .direita { margin-left: auto; }
  .faixa { margin: 0; padding: 8px 16px; background: var(--ouro-suave); color: var(--texto); border-bottom: 1px solid var(--borda); }
</style>
