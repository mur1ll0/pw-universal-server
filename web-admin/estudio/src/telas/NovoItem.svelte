<script lang="ts">
  /* Item novo pedido por outra aba (ex.: "Item gerado ao desmontar" → Criar), B366. Itens ainda não têm tela própria:
     aqui só se escolhe o item que serve de base, clona (no rascunho), dá nome e preço, e o id volta para a aba de
     origem pelo BroadcastChannel. O resto do item se edita nos Editores. */
  import { buscarNoEspaco, clonar, ficha, salvar, type Registro } from "../lib/api";
  import { NOME_DO_CANAL, avisar, estudio, rota, type MensagemDeItem } from "../lib/estado.svelte";
  import IconeItem from "../componentes/IconeItem.svelte";

  const retorno = rota.params.get("retorno") ?? "";
  let busca = $state("");
  let achados = $state<Registro[]>([]);
  let base = $state<Registro | null>(null);
  let nome = $state("");
  let preco = $state<number | "">("");
  let criado = $state<{ id: number; tabela: string; indice: number } | null>(null);
  let temPreco = $state(false);
  let ocupado = $state(false);
  let espera: ReturnType<typeof setTimeout> | undefined;

  async function buscar() {
    try { achados = (await buscarNoEspaco("ESSENCE", busca.trim(), 40)).filter((r) => !/_(TYPE|SERVICE)$/.test(r.tabela ?? "")); }
    catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { buscar(); });
  function digitar() { clearTimeout(espera); espera = setTimeout(buscar, 250); }

  async function criar() {
    if (!base?.tabela || !nome.trim()) { avisar("aviso", "Escolha a base e dê um nome."); return; }
    ocupado = true;
    try {
      const c = await clonar(base.tabela, base.indice);
      const f = await ficha(base.tabela, c.resultado.indice);
      temPreco = f.campos.some((x) => x.nome === "price");
      const campos: Record<string, string | number> = { Name: nome.trim() };
      if (temPreco && preco !== "") campos.price = Number(preco);
      await salvar(base.tabela, c.resultado.indice, campos);
      criado = { id: c.resultado.id, tabela: base.tabela, indice: c.resultado.indice };
      avisar("ok", `${base.tabela} id ${c.resultado.id} criado no rascunho #${estudio.rascunho}.`);
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }

  function devolver() {
    if (!criado) return;
    const m: MensagemDeItem = { tipo: "item-criado", retorno, id: criado.id, nome: nome.trim() };
    const canal = new BroadcastChannel(NOME_DO_CANAL);
    canal.postMessage(m);
    canal.close();
    window.close();
  }
</script>

<section class="caixa novo">
  <h3>Item novo</h3>
  {#if !estudio.rascunho}
    <p class="erro">Esta aba abriu sem rascunho: escolha o mesmo rascunho da aba de origem no alto.</p>
  {/if}
  {#if !criado}
    <p class="suave">1. Escolha o item que serve de base (a cópia herda tudo dele):</p>
    <input class="campo cheio" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar item base" />
    <ul class="lista">
      {#each achados as r (`${r.tabela}/${r.indice}`)}
        <li><button type="button" class:ativo={base === r} onclick={() => { base = r; if (!nome) nome = `${r.nome} (novo)`; }}>
          <IconeItem hex={r.icone} nome={r.nome} /><span class="nome">{r.nome}</span><span class="mini">{r.tabela} · id {r.id}</span></button></li>
      {/each}
    </ul>
    <p class="suave">2. Nome e preço de venda:</p>
    <div class="linha">
      <input class="campo" maxlength="32" placeholder="Nome" bind:value={nome} aria-label="Nome do item novo" />
      <input class="campo" type="number" min="0" step="1" placeholder="preço (opcional)" bind:value={preco} aria-label="Preço" />
      <button class="botao primario" type="button" disabled={!estudio.rascunho || !base || ocupado} onclick={criar}>Criar no rascunho</button>
    </div>
  {:else}
    <p>Criado: <strong>{nome}</strong> · {criado.tabela} id {criado.id}. O resto do item se edita nos <a href="/">Editores</a>.</p>
    <div class="linha">
      <button class="botao primario" type="button" disabled={!retorno} onclick={devolver}>Usar na aba de origem e fechar</button>
    </div>
    {#if !retorno}<p class="mini">Esta aba não veio de outra: não há para onde devolver.</p>{/if}
  {/if}
</section>

<style>
  .novo { max-width: 760px; }
  .cheio { width: 100%; }
  .lista { list-style: none; margin: 8px 0 12px; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 40vh; overflow: auto; }
  .lista button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .lista button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .nome { flex: 1; }
</style>
