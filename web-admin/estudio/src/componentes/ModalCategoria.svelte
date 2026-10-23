<script lang="ts">
  /* Categoria (`*_MAJOR_TYPE`) e subcategoria (`*_SUB_TYPE`) do item. Criar = clonar a escolhida no espaço `ESSENCE`
     (id novo = maior + 1, `ESTUDIO_DO_PAINEL.md` §3.1) e dar o nome. Na subcategoria de arma, o `action_type` do clone
     só deve ser um conjunto de ataque que os modelos de jogador já têm (`EC_Player.cpp:4600`, ESTUDIO §4.1). */
  import { clonar, listar, salvar, type Registro } from "../lib/api";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Modal from "./Modal.svelte";

  let { maior, menor, idMaior, idMenor, aoEscolher, aoFechar }:
    { maior?: string; menor?: string; idMaior: number; idMenor: number; aoEscolher: (maior: number, menor: number) => void; aoFechar: () => void } = $props();

  let listas = $state<Record<string, Registro[]>>({});
  let busca = $state<Record<string, string>>({});
  let selecao = $state<Record<string, number>>({});
  let nomeNovo = $state<Record<string, string>>({});

  const colunas = $derived([maior && { tabela: maior, titulo: "Categoria" }, menor && { tabela: menor, titulo: "Subcategoria" }]
    .filter(Boolean) as { tabela: string; titulo: string }[]);

  $effect(() => {
    if (maior && selecao[maior] === undefined) selecao[maior] = idMaior;
    if (menor && selecao[menor] === undefined) selecao[menor] = idMenor;
  });

  async function carregar(tabela: string) {
    try { listas[tabela] = (await listar(tabela, (busca[tabela] ?? "").trim(), 0, 200)).registros; }
    catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { for (const c of colunas) carregar(c.tabela); });

  async function criar(tabela: string) {
    const base = listas[tabela]?.find((r) => r.id === selecao[tabela]);
    const nome = (nomeNovo[tabela] ?? "").trim();
    if (!estudio.rascunho) { avisar("aviso", "Escolha um rascunho no alto."); return; }
    if (!base) { avisar("aviso", "Escolha na lista o registro que serve de base."); return; }
    if (!nome) { avisar("aviso", "Dê um nome."); return; }
    try {
      const c = await clonar(tabela, base.indice);
      await salvar(tabela, c.resultado.indice, { Name: nome });
      avisar("ok", `${tabela} id ${c.resultado.id} criado no rascunho (cópia de ${base.id}).`);
      nomeNovo[tabela] = "";
      await carregar(tabela);
      selecao[tabela] = c.resultado.id;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
</script>

<Modal titulo="Categoria e subcategoria" {aoFechar} largo>
  <div class="colunas">
    {#each colunas as c (c.tabela)}
      <section>
        <h4>{c.titulo} <span class="mini">{c.tabela}</span></h4>
        <input class="campo cheio" placeholder="id ou nome" bind:value={busca[c.tabela]} oninput={() => carregar(c.tabela)} aria-label="Buscar {c.titulo}" />
        <ul class="lista">
          {#each listas[c.tabela] ?? [] as r (r.indice)}
            <li><button type="button" class:ativo={selecao[c.tabela] === r.id} onclick={() => (selecao[c.tabela] = r.id ?? 0)}>{r.nome} <span class="mini">id {r.id}</span></button></li>
          {/each}
        </ul>
        <div class="linha">
          <input class="campo" placeholder="nome da nova (cópia da escolhida)" bind:value={nomeNovo[c.tabela]} aria-label="Nome da nova {c.titulo}" />
          <button class="botao pequeno" type="button" disabled={!estudio.rascunho} onclick={() => criar(c.tabela)}>Criar</button>
        </div>
      </section>
    {/each}
  </div>
  {#snippet rodape()}
    <button class="botao" type="button" onclick={aoFechar}>Cancelar</button>
    <button class="botao primario" type="button" onclick={() => { aoEscolher(maior ? selecao[maior] : idMaior, menor ? selecao[menor] : idMenor); aoFechar(); }}>Usar</button>
  {/snippet}
</Modal>

<style>
  .colunas { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 16px; }
  h4 { margin: 0 0 8px; }
  .cheio { width: 100%; }
  .lista { list-style: none; margin: 8px 0; padding: 0; max-height: 40vh; overflow: auto; display: flex; flex-direction: column; gap: 3px; }
  .lista button { width: 100%; text-align: left; padding: 5px 8px; border-radius: 6px; border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; }
  .lista button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
</style>
