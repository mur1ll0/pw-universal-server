<script lang="ts">
  /* B392: aba Serviços do NPC — as vagas `id_*_service` do `NPC_ESSENCE` agrupadas pelo que fazem (`lib/npc.ts`), cada
     uma com o serviço ligado, se o nosso servidor o atende (despacho do pw-gs) e os atalhos: Editar (o serviço, no painel
     de serviços), Trocar, Tirar. Vincular abre o mesmo painel já na categoria da vaga. Mais os serviços simples (bits de
     `combined_services`/`combined_services2`, `ExpTypes.h:3763-3775`). Mudanças nas vagas e nos bits ficam no NPC até
     gravar (botão do alto); o que se edita dentro de um serviço grava no serviço. */
  import { tabelas } from "../lib/api";
  import { estudio, parametros } from "../lib/estado.svelte";
  import { comBit, GRUPOS_DE_SERVICO, ligado, SERVICOS_DO_NPC, SIMPLES, SIMPLES2, type ServicoDoNpc, type Vinculado } from "../lib/npc";
  import { categoriaDaTabela } from "../lib/servicos";
  import Icone from "./Icone.svelte";
  import Modal from "./Modal.svelte";
  import PainelDeServicos from "./PainelDeServicos.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { nome, valores = $bindable(), originais, vinculados, aoVincular }:
    { nome: string; valores: Record<string, number | string>; originais: Record<string, number | string>; vinculados: Vinculado[];
      aoVincular: (campo: string, tabela: string, id: number, nome: string, indice: number) => void } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let existentes = $state<Set<string> | null>(null);
  tabelas().then((t) => (existentes = new Set(t.tabelas.map((x) => x.nome)))).catch(() => {});
  /** Vaga que existe no NPC desta versão e cuja tabela existe (o v7 tem `id_cross_server_service` sem a tabela). */
  const vagas = $derived(SERVICOS_DO_NPC.filter((s) => s.campo in valores && (!existentes || existentes.has(s.tabela))));
  const doCampo = (c: string) => vinculados.find((v) => v.campo === c);

  let modal = $state<null | { vaga: ServicoDoNpc; editar: boolean }>(null);
  let filtro = $state("");
  const simples = $derived([
    ...SIMPLES.map((s) => ({ ...s, campo: "combined_services" as const })),
    ...("combined_services2" in valores ? SIMPLES2.map((s) => ({ ...s, campo: "combined_services2" as const })) : []),
  ].filter((s) => !filtro.trim() || s.nome.toLowerCase().includes(filtro.trim().toLowerCase())));
  const ligados = $derived(simples.filter((s) => ligado(Number(valores[s.campo] ?? 0), s.bit)).length);
  const tem = $derived(vagas.filter((s) => Number(valores[s.campo] ?? 0) !== 0));
  const livres = $derived(vagas.filter((s) => Number(valores[s.campo] ?? 0) === 0));
  /** Os simples ligados, sem o filtro da caixa de baixo. */
  const simplesLigados = $derived([
    ...SIMPLES.map((s) => ({ ...s, campo: "combined_services" as const })),
    ...("combined_services2" in valores ? SIMPLES2.map((s) => ({ ...s, campo: "combined_services2" as const })) : []),
  ].filter((s) => ligado(Number(valores[s.campo] ?? 0), s.bit)));
</script>

<!-- B393: no alto, tudo o que o NPC já tem (vagas ligadas e serviços simples ligados), em cartões do mesmo tamanho;
     embaixo, o que ainda dá para vincular, por categoria, em caixas de altura fixa com rolagem. -->
<section class="caixa">
  <h3>Serviços deste NPC <span class="mini">{tem.length + simplesLigados.length}</span></h3>
  {#if tem.length || simplesLigados.length}
    <div class="tem">
      {#each tem as s (s.campo)}
        {@const v = doCampo(s.campo)}
        {@const id = Number(valores[s.campo] ?? 0)}
        <article class="cartao" class:mudou={valores[s.campo] !== originais[s.campo]}>
          <span class="tp">{s.grupo} · {s.nome}{@render selo(s.atende)}</span>
          <strong class="nome"><TextoDoJogo texto={v?.nome || `serviço ${id}`} padrao="var(--texto)" linha /></strong>
          <span class="mini">id {id}{s.menu === "na_loja" ? " · fica na janela da loja" : s.menu === "condicional" ? " · só em certa condição" : ""}</span>
          <div class="acoes">
            <button class="botao pequeno" type="button" disabled={!v} onclick={() => (modal = { vaga: s, editar: true })}><Icone nome="editar" tamanho={16} /> Editar</button>
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (modal = { vaga: s, editar: false })}>Trocar</button>
            <button class="botao pequeno fantasma" type="button" disabled={!editavel} aria-label="Tirar o serviço do NPC" onclick={() => (valores[s.campo] = 0)}><Icone nome="lixeira" tamanho={16} /></button>
          </div>
        </article>
      {/each}
      {#each simplesLigados as s (s.campo + s.bit)}
        <article class="cartao simples">
          <span class="tp">Serviço simples · bit {s.campo === "combined_services2" ? "2:" : ""}{s.bit}{@render selo(s.atende)}</span>
          <strong class="nome">{s.nome}</strong>
          <span class="mini">sem tabela: linha fixa na janela do NPC</span>
          <div class="acoes">
            <button class="botao pequeno fantasma" type="button" disabled={!editavel}
              onclick={() => (valores[s.campo] = comBit(Number(valores[s.campo] ?? 0), s.bit, false))}><Icone nome="lixeira" tamanho={16} /> Desligar</button>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <p class="mini">Nenhum serviço ligado: a janela do NPC só terá a conversa e o Sair.</p>
  {/if}
</section>

<h3 class="sub">Vincular</h3>
<div class="grupos">
  {#each GRUPOS_DE_SERVICO as g (g)}
    {@const lista = livres.filter((s) => s.grupo === g)}
    {#if lista.length}
      <section class="caixa grupo">
        <h4>{g} <span class="mini">{lista.length}</span></h4>
        <ul>
          {#each lista as s (s.campo)}
            <li class:mudou={valores[s.campo] !== originais[s.campo]} title={s.explica || "sem serviço"}>
              <div class="info">
                <span class="nome-vaga">{s.nome}{@render selo(s.atende)}</span>
                <span class="mini corta">{s.explica || "sem serviço"}</span>
              </div>
              <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (modal = { vaga: s, editar: false })}><Icone nome="mais" tamanho={16} /> Vincular</button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/each}
</div>

<section class="caixa">
  <h3>Serviços simples <span class="mini">{ligados} ligado(s)</span></h3>
  <p class="mini">Sem tabela: cada um é um bit do NPC e vira uma linha fixa na janela dele (DlgNPC.cpp:448-850). Nomes do cliente 1.5.5; no 1.2.6
    só existe o primeiro campo. "Servidor atende" = o pw-gs tem o tratador.</p>
  <input class="campo filtro" placeholder="filtrar (ovo, clã, Daimon…)" bind:value={filtro} aria-label="Filtrar serviços simples" />
  <div class="bits">
    {#each simples as s (s.campo + s.bit)}
      {@const on = ligado(Number(valores[s.campo] ?? 0), s.bit)}
      <label class="bit" class:on>
        <input type="checkbox" checked={on} disabled={!editavel} onchange={(e) => (valores[s.campo] = comBit(Number(valores[s.campo] ?? 0), s.bit, (e.target as HTMLInputElement).checked))} />
        <span>{s.nome}</span>
        <span class="mini">{s.campo === "combined_services2" ? "2:" : ""}{s.bit}</span>
        {#if s.atende === "sim"}<span class="selo ok">atende</span>{/if}
      </label>
    {/each}
  </div>
</section>

{#snippet selo(atende: string | null | undefined)}
  {#if atende === "sim"}<span class="selo ok" title="O despacho de serviço do pw-gs atende">servidor atende</span>
  {:else if atende === "nao"}<span class="selo nao" title="O pw-gs ainda não trata este serviço">servidor não atende</span>{/if}
{/snippet}

<p class="mini rodape">Serviços servem a vários NPCs: para editar sem passar por um NPC, use a <a href="#/servicos?{parametros()}" target="_blank" rel="noopener">área Serviços</a>.</p>

{#if modal}
  {@const m = modal}
  {@const v = doCampo(m.vaga.campo)}
  <Modal titulo={m.editar ? `Serviço: ${v?.nome ?? m.vaga.nome}` : `Vincular a ${nome}: ${m.vaga.nome}`} cheio aoFechar={() => (modal = null)}>
    <PainelDeServicos tabelaInicial={m.vaga.tabela} categoriaInicial={categoriaDaTabela(m.vaga.tabela)?.chave}
      indiceInicial={m.editar && v ? v.indice : null}
      vinculo={{
        npc: nome, campo: m.vaga.campo,
        aoVincular: (tabela, id, n) => {
          const destino = SERVICOS_DO_NPC.find((x) => x.tabela === tabela && x.campo === m.vaga.campo) ?? SERVICOS_DO_NPC.find((x) => x.tabela === tabela);
          if (destino) aoVincular(destino.campo, tabela, id, n, -1);
          modal = null;
        },
        aoLigarBit: (campo, bit) => { valores[campo] = comBit(Number(valores[campo] ?? 0), bit, true); modal = null; },
      }} />
  </Modal>
{/if}

<style>
  .tem { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px; }
  .cartao { display: flex; flex-direction: column; gap: 4px; height: 118px; padding: 10px 12px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); min-width: 0; }
  .cartao.simples { background: var(--ouro-suave); }
  .cartao.mudou, li.mudou { border-color: var(--ouro); }
  .cartao .nome { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cartao .acoes { margin-top: auto; }
  .sub { margin: 16px 0 8px; }
  .grupos { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 12px; margin-bottom: 14px; }
  .grupo { display: flex; flex-direction: column; height: 300px; min-width: 0; }
  .grupo h4 { margin: 0 0 8px; font-size: 13px; text-transform: uppercase; letter-spacing: .04em; color: var(--texto-2); }
  .grupo ul { overflow-y: auto; flex: 1; padding-right: 4px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  li { display: flex; gap: 8px; align-items: center; justify-content: space-between; height: 52px; flex: none; padding: 6px 10px; border: 1px dashed var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .info { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .nome-vaga { font-size: 13px; display: flex; gap: 6px; align-items: center; min-width: 0; }
  .corta { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tp { font-size: 11px; color: var(--texto-3); text-transform: uppercase; letter-spacing: .04em; display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .acoes { display: flex; gap: 4px; flex-wrap: wrap; }
  .selo { text-transform: none; letter-spacing: 0; font-size: 10px; padding: 1px 6px; border-radius: 999px; border: 1px solid; white-space: nowrap; }
  .selo.ok { color: var(--verde); border-color: var(--verde); background: var(--verde-suave); }
  .selo.nao { color: var(--vermelho); border-color: var(--vermelho); background: var(--vermelho-suave); }
  .filtro { width: min(360px, 100%); margin: 6px 0 10px; }
  .bits { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 4px 12px; max-height: 320px; overflow-y: auto; }
  .bit { display: flex; gap: 8px; align-items: center; padding: 4px 6px; border-radius: 6px; font-size: 13px; cursor: pointer; }
  .bit.on { background: var(--ouro-suave); }
  .bit span:nth-child(2) { flex: 1; }
  .rodape { margin-top: 10px; }
</style>
