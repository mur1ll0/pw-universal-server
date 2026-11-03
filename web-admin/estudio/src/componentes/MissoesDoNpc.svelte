<script lang="ts">
  /* B392: aba Missões do NPC — atalho para as missões dele. As listas são dos serviços de missão ligados ao NPC
     (`NPC_TASK_OUT_SERVICE`: dá; `NPC_TASK_IN_SERVICE`: recebe; `NPC_TASK_MATTER_SERVICE`: itens de missão — lidos pelo
     cliente em `CheckTaskService`, `DlgNPC.cpp:1195-1210`). Acrescentar/tirar missão grava no serviço (registro próprio,
     que pode servir a outros NPCs). Embaixo, a conferência com o tasks.data: as missões que dizem que este NPC as entrega
     (`npc_que_entrega`) ou premia (`npc_que_premia`) e não estão nas listas dele. A missão em si abre aqui num painel ou
     na área Missões. */
  import { clonar, ficha as lerFicha, listarServicos, resumoDeMissoes, salvar, type MissaoResumida } from "../lib/api";
  import { avisar, estudio, parametros } from "../lib/estado.svelte";
  import type { Vinculado } from "../lib/npc";
  import { compactar, vagasEmLista } from "../lib/servicos";
  import FichaDaMissao from "./FichaDaMissao.svelte";
  import Icone from "./Icone.svelte";
  import Modal from "./Modal.svelte";
  import PainelDeServicos from "./PainelDeServicos.svelte";
  import SeletorDeMissao from "./SeletorDeMissao.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { nome, id, vinculados, aoVincular }:
    { nome: string; id: number; vinculados: Vinculado[]; aoVincular: (campo: string, tabela: string, id: number, nome: string, indice: number) => void } = $props();
  const editavel = $derived(!!estudio.rascunho);

  const TIPOS = [
    { campo: "id_task_out_service", tabela: "NPC_TASK_OUT_SERVICE", titulo: "Dá a missão", explica: "O jogador pega a missão com ele." },
    { campo: "id_task_in_service", tabela: "NPC_TASK_IN_SERVICE", titulo: "Recebe a missão", explica: "O jogador entrega a missão a ele." },
  ];
  interface Lista { campos: string[]; ids: number[]; usado: number; nomeDoServico: string; indice: number; tabela: string }
  let listas = $state<Record<string, Lista | null>>({});
  let nomes = $state<Record<number, MissaoResumida>>({});
  let doNpc = $state<{ entrega: MissaoResumida[]; premia: MissaoResumida[] } | null>(null);
  let abrindo = $state<number | null>(null);
  let escolhendo = $state<string | null>(null);
  let itensDeMissao = $state(false);

  async function carregar() {
    const l: Record<string, Lista | null> = {};
    for (const t of TIPOS) {
      const v = vinculados.find((x) => x.campo === t.campo);
      if (!v) { l[t.campo] = null; continue; }
      try {
        const f = await lerFicha(v.tabela, v.indice);
        const vl = vagasEmLista(f.campos.map((c) => c.nome));
        const por = Object.fromEntries(f.campos.map((c) => [c.nome, Number(c.valor ?? 0)]));
        l[t.campo] = { campos: vl?.campos ?? [], ids: (vl?.campos ?? []).map((c) => por[c]).filter((x) => x > 0), usado: f.usado_por.length, nomeDoServico: f.nome, indice: v.indice, tabela: v.tabela };
      } catch (e) { avisar("erro", (e as Error).message); l[t.campo] = null; }
    }
    listas = l;
    const ids = Object.values(l).flatMap((x) => x?.ids ?? []);
    try {
      const r = await resumoDeMissoes(ids, id);
      nomes = Object.fromEntries([...r.missoes, ...(r.do_npc?.entrega ?? []), ...(r.do_npc?.premia ?? [])].map((m) => [m.id, m]));
      doNpc = r.do_npc;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { void vinculados; void estudio.rascunho; if (id) carregar(); });

  async function gravarLista(campo: string, ids: number[]) {
    const l = listas[campo];
    if (!l) return;
    const v = compactar(ids, l.campos.length);
    try {
      await salvar(l.tabela, l.indice, Object.fromEntries(l.campos.map((c, k) => [c, v[k]])));
      avisar("ok", `"${l.nomeDoServico}": ${ids.length} missão(ões) no rascunho.`);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  function acrescentar(campo: string, m: number) {
    const l = listas[campo];
    if (!l) return;
    if (l.ids.includes(m)) { avisar("aviso", "Já está na lista."); return; }
    if (l.ids.length >= l.campos.length) { avisar("aviso", `Cabem ${l.campos.length}.`); return; }
    gravarLista(campo, [...l.ids, m]);
  }
  /** Serviço de missão novo, só deste NPC (clone vazio do primeiro da tabela). */
  async function criarServico(t: (typeof TIPOS)[number]) {
    try {
      const base = (await listarServicos(t.tabela, "", "", 0, 1)).registros[0];
      if (!base) throw new Error(`${t.tabela} não tem registro para servir de base.`);
      const c = await clonar(t.tabela, base.indice);
      const f = await lerFicha(t.tabela, c.resultado.indice);
      const zerar: Record<string, number | string> = { Name: `Missões de ${nome}`.slice(0, 31) };
      for (const x of f.campos) if (/\d/.test(x.nome) && x.nome !== "ID") zerar[x.nome] = x.tipo === "int32" || x.tipo === "float" ? 0 : "";
      await salvar(t.tabela, c.resultado.indice, zerar);
      aoVincular(t.campo, t.tabela, c.resultado.id, String(zerar.Name), c.resultado.indice);
      avisar("ok", `Serviço ${c.resultado.id} criado. Grave o NPC para ligar.`);
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  const nomeDe = (m: number) => nomes[m]?.nome ?? `missão ${m}`;
  const naoListadas = (lista: MissaoResumida[], campo: string) => lista.filter((m) => !(listas[campo]?.ids ?? []).includes(m.id));
  const itens = $derived(vinculados.find((x) => x.campo === "id_task_matter_service"));
</script>

<div class="cards">
  {#each TIPOS as t (t.campo)}
    {@const l = listas[t.campo]}
    <section class="caixa">
      <h3>{t.titulo} {#if l}<span class="mini">{l.ids.length} de {l.campos.length}</span>{/if}</h3>
      <p class="mini">{t.explica}</p>
      {#if l === undefined}<p class="suave">Carregando…</p>
      {:else if l === null}
        <p class="suave">Sem serviço.</p>
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => criarServico(t)}><Icone nome="mais" tamanho={16} /> Criar a lista para {nome}</button>
      {:else}
        <p class="mini">Serviço "{l.nomeDoServico}"{l.usado > 1 ? ` — compartilhado por ${l.usado} registros: mudar a lista muda em todos` : ""}.</p>
        <ul>
          {#each l.ids as m (m)}
            <li>
              <span class="nome"><TextoDoJogo texto={nomeDe(m)} padrao="var(--texto)" linha /></span>
              <span class="mini">{m}{nomes[m]?.nivel_min != null ? ` · nível ${nomes[m].nivel_min}–${nomes[m].nivel_max}` : ""}{nomes[m]?.ausente ? " · não existe no tasks.data" : ""}</span>
              <button class="botao pequeno" type="button" onclick={() => (abrindo = m)}>Editar aqui</button>
              <a class="botao pequeno fantasma" href="#/missoes/{m}?{parametros()}" target="_blank" rel="noopener" aria-label="Abrir em Missões"><Icone nome="abrir" tamanho={16} /></a>
              <button class="botao pequeno fantasma" type="button" disabled={!editavel} aria-label="Tirar da lista" onclick={() => gravarLista(t.campo, l.ids.filter((x) => x !== m))}><Icone nome="lixeira" tamanho={16} /></button>
            </li>
          {:else}
            <li class="mini">Lista vazia.</li>
          {/each}
        </ul>
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = t.campo)}><Icone nome="mais" tamanho={16} /> Missão</button>
      {/if}
    </section>
  {/each}
  <section class="caixa">
    <h3>Itens de missão</h3>
    <p class="mini">Itens que o NPC entrega para uma missão (NPC_TASK_MATTER_SERVICE).</p>
    {#if itens}
      <p>"{itens.nome}" <span class="mini">id {itens.id}</span></p>
      <button class="botao pequeno" type="button" onclick={() => (itensDeMissao = true)}><Icone nome="editar" tamanho={16} /> Editar os itens</button>
    {:else}
      <p class="suave">Sem serviço — vincule na aba Serviços (grupo Missões).</p>
    {/if}
  </section>
</div>

{#if doNpc}
  <section class="caixa">
    <h3>Conferência com o tasks.data</h3>
    <p class="mini">Cada missão guarda quem a entrega e quem a premia. As que apontam para este NPC e não estão nas listas dele aparecem aqui:
      o jogador não a verá com ele.</p>
    <div class="duas">
      {#each [["Diz que ele dá", doNpc.entrega, "id_task_out_service"], ["Diz que ele premia", doNpc.premia, "id_task_in_service"]] as [titulo, lista, campo] (titulo)}
        {@const faltam = naoListadas(lista as MissaoResumida[], campo as string)}
        <div>
          <h4>{titulo} <span class="mini">{(lista as MissaoResumida[]).length} missão(ões), {faltam.length} fora da lista</span></h4>
          <ul>
            {#each faltam.slice(0, 50) as m (m.id)}
              <li>
                <span class="nome"><TextoDoJogo texto={m.nome ?? `missão ${m.id}`} padrao="var(--texto)" linha /></span><span class="mini">{m.id}</span>
                <button class="botao pequeno" type="button" disabled={!editavel || !listas[campo as string]} onclick={() => acrescentar(campo as string, m.id)}>Pôr na lista</button>
              </li>
            {:else}
              <li class="mini">Tudo certo.</li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  </section>
{/if}

{#if escolhendo}
  {@const campo = escolhendo}
  <SeletorDeMissao aoFechar={() => (escolhendo = null)} aoEscolher={(m) => { nomes[m.id] = { id: m.id, nome: m.nome, nivel_min: m.nivel_min, nivel_max: m.nivel_max }; acrescentar(campo, m.id); }} />
{/if}
{#if abrindo != null}
  <Modal titulo="Missão {abrindo}" cheio aoFechar={() => { abrindo = null; carregar(); }}>
    <FichaDaMissao id={abrindo} aoAbrirOutra={(m) => (abrindo = m)} />
  </Modal>
{/if}
{#if itensDeMissao && itens}
  <Modal titulo="Itens de missão de {nome}" cheio aoFechar={() => (itensDeMissao = false)}>
    <PainelDeServicos tabelaInicial="NPC_TASK_MATTER_SERVICE" indiceInicial={itens.indice} />
  </Modal>
{/if}

<style>
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 14px; margin-bottom: 14px; align-items: start; }
  ul { list-style: none; margin: 6px 0 8px; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 360px; overflow: auto; }
  li { display: flex; gap: 6px; align-items: center; padding: 5px 8px; border: 1px solid var(--borda); border-radius: 8px; background: var(--fundo-2); }
  .nome { flex: 1; min-width: 0; }
  .duas { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  h4 { margin: 4px 0; font-size: 13px; color: var(--texto-2); }
  @media (max-width: 900px) { .duas { grid-template-columns: 1fr; } }
</style>
