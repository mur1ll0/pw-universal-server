<script lang="ts">
  /* B392: o NPC de serviço (`NPC_ESSENCE`, `ExpTypes.h` do cliente / `template/exptypes.h` do GS) no Estúdio, em cinco
     abas: Ficha (campos explicados pelo que o GS faz, `lib/npc.ts`, e o modelo 3D com Parado/Ocioso), Onde fica (pontos do
     npcgen com direção), Conversa (janela do NPC, textos dos botões e conversas), Serviços (vagas e serviços simples) e
     Missões (atalho). Os campos do NPC ficam na tela até "Gravar no rascunho" (`PUT tabelas/NPC_ESSENCE/{i}`); serviços,
     conversas, missões e npcgen gravam no registro deles, cada um pelo seu botão. */
  import { ficha as lerFicha, linkDosEditores, listar, salvar, type Campo, type Ficha } from "../lib/api";
  import { avisar, estudio, ir, rota } from "../lib/estado.svelte";
  import { EXPLICACOES_DO_NPC, ROTULOS_DO_NPC, SERVICOS_DO_NPC, type Vinculado } from "../lib/npc";
  import BoxDePontosDoNpc from "../componentes/BoxDePontosDoNpc.svelte";
  import CamposSimples from "../componentes/CamposSimples.svelte";
  import ConversasDoNpc from "../componentes/ConversasDoNpc.svelte";
  import Icone from "../componentes/Icone.svelte";
  import MissoesDoNpc from "../componentes/MissoesDoNpc.svelte";
  import ModeloDoMonstro from "../componentes/ModeloDoMonstro.svelte";
  import SeletorDeEspaco from "../componentes/SeletorDeEspaco.svelte";
  import ServicosDoNpc from "../componentes/ServicosDoNpc.svelte";
  import TextoDoJogo from "../componentes/TextoDoJogo.svelte";

  let { tabela, indice }: { tabela: string; indice: number } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let f = $state<Ficha | null>(null);
  let erro = $state("");
  let valores = $state<Record<string, number | string>>({});
  let originais = $state.raw<Record<string, number | string>>({});
  let porNome = $state.raw<Record<string, Campo>>({});
  let gravando = $state(false);
  let aba = $state(rota.params.get("aba") || "ficha");
  /** serviços ligados nesta tela (antes de gravar), por vaga */
  const locais = $state<Record<string, Vinculado>>({});
  const nomes = $state<Record<string, string>>({});
  let escolhendo = $state<null | { campo: string; categoria: string; titulo: string }>(null);
  let tipos = $state<{ id: number; nome: string }[]>([]);

  async function carregar() {
    try {
      const x = await lerFicha(tabela, indice);
      porNome = Object.fromEntries(x.campos.map((c) => [c.nome, c]));
      originais = Object.fromEntries(x.campos.map((c) => [c.nome, c.valor ?? (c.tipo === "int32" || c.tipo === "float" ? 0 : "")]));
      valores = { ...originais };
      f = x;
    } catch (e) { erro = (e as Error).message; }
  }
  carregar();
  listar("NPC_TYPE", "", 0, 200).then((r) => (tipos = r.registros.map((x) => ({ id: x.id ?? 0, nome: x.nome })))).catch(() => {});
  function trocarAba(a: string) {
    aba = a;
    const p = new URLSearchParams(rota.params);
    p.set("aba", a);
    history.replaceState(null, "", `#/${rota.caminho.join("/")}?${p}`);
  }

  const num = (n: string) => Number(valores[n] ?? 0);
  const mudados = $derived(Object.keys(valores).filter((n) => valores[n] !== originais[n]));
  const vinculados = $derived.by((): Vinculado[] => SERVICOS_DO_NPC.filter((s) => s.campo in valores && num(s.campo) > 0).flatMap((s) => {
    const id = num(s.campo);
    const l = locais[s.campo];
    if (l && l.id === id) return [l];
    const r = porNome[s.campo]?.referencia;
    if (r && r.id === id && r.tabela && r.indice != null) return [{ campo: s.campo, tabela: r.tabela, indice: r.indice, id, nome: r.nome ?? "" }];
    return [];
  }));
  /* A vaga diz a tabela; quando o catálogo de referências não resolve o id (no 1.2.6 as vagas de missão vêm sem `cita`),
     acha o serviço pelo id na tabela da vaga. */
  $effect(() => {
    for (const s of SERVICOS_DO_NPC) {
      const id = Number(originais[s.campo] ?? 0);
      const r = porNome[s.campo]?.referencia;
      if (id <= 0 || (r && r.id === id && r.tabela && r.indice != null) || locais[s.campo]?.id === id) continue;
      listar(s.tabela, String(id), 0, 20).then((x) => {
        const achado = x.registros.find((y) => y.id === id);
        if (achado) locais[s.campo] = { campo: s.campo, tabela: s.tabela, indice: achado.indice, id, nome: achado.nome };
      }).catch(() => {});
    }
  });
  async function aoVincular(campo: string, t: string, id: number, nome: string, ind: number) {
    let i = ind;
    if (i < 0) {
      const r = await listar(t, String(id), 0, 20).catch(() => null);
      i = r?.registros.find((x) => x.id === id)?.indice ?? -1;
    }
    if (i < 0) { avisar("erro", `Serviço ${id} não encontrado em ${t}.`); return; }
    locais[campo] = { campo, tabela: t, indice: i, id, nome };
    valores[campo] = id;
    avisar("ok", `"${nome}" ligado à vaga — grave o NPC para valer.`);
  }

  async function gravar() {
    const c: Record<string, number | string> = {};
    for (const n of mudados) {
      const t = porNome[n].tipo, v = valores[n];
      if (t === "int32" || t === "float") {
        const x = Number(v);
        if (v === "" || !Number.isFinite(x) || (t === "int32" && !Number.isInteger(x))) { avisar("aviso", `${n}: número inválido.`); return; }
        c[n] = x;
      } else c[n] = String(v);
    }
    if (!Object.keys(c).length) { avisar("aviso", "Nada mudou."); return; }
    if (typeof c.hello_msg === "string" && c.hello_msg.length > 255) { avisar("aviso", "A fala inicial passa de 255 caracteres."); return; }
    gravando = true;
    try {
      const r = await salvar(tabela, indice, c);
      avisar("ok", r.igual_ao_publicado ? "Igual ao publicado." : `${Object.keys(c).length} campo(s) do NPC gravados no rascunho.`);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { gravando = false; }
  }

  /* ---- ficha ---- */
  const monstro = $derived(porNome.id_src_monster?.referencia);
  const semMonstro = $derived(!num("id_src_monster") || (valores.id_src_monster === originais.id_src_monster && !!monstro?.ausente));
  const camposDaFicha = ["refresh_time", "attack_rule", "tax_rate", "id_to_discover", "domain_related", "no_collision"];
  const servicos = new Set(SERVICOS_DO_NPC.map((s) => s.campo));
  const jaMostrados = new Set(["ID", "Name", "id_type", "hello_msg", "id_src_monster", "file_model", "id_mine", "distance_service", "combined_services", "combined_services2", ...camposDaFicha]);
  const outros = $derived(Object.keys(porNome).filter((n) => !jaMostrados.has(n) && !servicos.has(n)));
  const nomeDe = (campo: string) => nomes[campo] ?? (valores[campo] === originais[campo] ? porNome[campo]?.referencia?.nome : null) ?? (num(campo) ? `id ${num(campo)}` : "nenhum");
  const ABAS = $derived([
    ["ficha", "Ficha", ""], ["onde", "Onde fica", ""], ["conversa", "Conversa", ""],
    ["servicos", "Serviços", String(vinculados.length)], ["missoes", "Missões", String(vinculados.filter((v) => v.campo.startsWith("id_task_")).length)],
  ]);
</script>

{#if erro}<p class="caixa erro">{erro}</p>
{:else if !f}<p class="suave">Carregando…</p>
{:else}
  <div class="barra">
    <button class="botao" type="button" onclick={() => ir("")}><Icone nome="voltar" /> Voltar</button>
    <span class="suave">NPC · id {f.id} · registro {indice}</span>
    <a class="botao fantasma" href={linkDosEditores(tabela, indice)} target="_blank" rel="noopener"><Icone nome="editores" /> Abrir nos Editores</a>
    <span class="espaco"></span>
    {#if mudados.length}<span class="mini">{mudados.length} campo(s) do NPC por gravar</span>{/if}
    <button class="botao" type="button" disabled={gravando || !mudados.length} onclick={() => (valores = { ...originais })}>Descartar</button>
    <button class="botao primario" type="button" disabled={!editavel || gravando || !mudados.length} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
  </div>

  <header class="cabeca">
    <input class="campo nome" class:alterado={valores.Name !== originais.Name} disabled={!editavel} value={valores.Name} maxlength="31" aria-label="Nome do NPC"
      oninput={(e) => (valores.Name = (e.target as HTMLInputElement).value)} />
    {#if semMonstro}<span class="selo erro">sem monstro de base: o servidor não cria este NPC</span>{/if}
  </header>

  <div class="abas" role="tablist">
    {#each ABAS as [k, n, c] (k)}
      <button type="button" role="tab" aria-selected={aba === k} class:on={aba === k} onclick={() => trocarAba(k)}>{n}{#if c && c !== "0"}<span class="cont">{c}</span>{/if}</button>
    {/each}
  </div>

  {#if aba === "ficha"}
    <div class="layout">
      <div class="coluna">
        <section class="caixa">
          <h3>Principal</h3>
          <div class="grade-campos">
            <label><span>Tipo (NPC_TYPE)</span>
              <select class="campo" class:alterado={valores.id_type !== originais.id_type} disabled={!editavel} value={String(valores.id_type)}
                onchange={(e) => (valores.id_type = Number((e.target as HTMLSelectElement).value))}>
                {#each tipos as t (t.id)}<option value={String(t.id)}>{t.nome || `tipo ${t.id}`}</option>{/each}
                {#if !tipos.some((t) => t.id === num("id_type"))}<option value={String(valores.id_type)}>id {valores.id_type}</option>{/if}
              </select></label>
          </div>
          <div class="fala">
            <span class="mini">Fala inicial</span>
            <div class="previa"><TextoDoJogo texto={String(valores.hello_msg ?? "")} /></div>
            <button class="botao pequeno" type="button" onclick={() => trocarAba("conversa")}><Icone nome="conversa" tamanho={16} /> Editar na aba Conversa</button>
          </div>
        </section>

        <section class="caixa">
          <h3>Corpo e luta</h3>
          <div class="monstro" class:erro-borda={semMonstro}>
            <span class="mini">{ROTULOS_DO_NPC.id_src_monster}</span>
            <strong>{nomeDe("id_src_monster")}</strong>
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { campo: "id_src_monster", categoria: "MONSTER_ESSENCE", titulo: "Monstro de base" })}>Trocar</button>
            {#if num("id_src_monster") && monstro?.indice != null && valores.id_src_monster === originais.id_src_monster}
              <a class="botao pequeno fantasma" href="#/monstros/MONSTER_ESSENCE/{monstro.indice}?realm={estudio.realm}{estudio.rascunho ? `&rascunho=${estudio.rascunho}` : ''}" target="_blank" rel="noopener">Abrir o monstro</a>{/if}
            <p class="mini">{EXPLICACOES_DO_NPC.id_src_monster}</p>
          </div>
          <CamposSimples nomes={camposDaFicha} {porNome} bind:valores {originais} {editavel} rotulos={ROTULOS_DO_NPC} explicacoes={EXPLICACOES_DO_NPC} />
          <div class="linha extra">
            {#if "distance_service" in porNome}
              <label class="mini"><input type="checkbox" checked={(num("distance_service") & 1) === 1} disabled={!editavel}
                onchange={(e) => (valores.distance_service = (num("distance_service") & ~1) | ((e.target as HTMLInputElement).checked ? 1 : 0))} />
                Atende a qualquer distância</label>
              <span class="mini">{EXPLICACOES_DO_NPC.distance_service}</span>
            {/if}
          </div>
          {#if "id_mine" in porNome}
            <div class="monstro">
              <span class="mini">Mina anexa</span><strong>{nomeDe("id_mine")}</strong>
              <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { campo: "id_mine", categoria: "MINE_ESSENCE", titulo: "Mina anexa" })}>Trocar</button>
              {#if num("id_mine")}<button class="botao pequeno fantasma" type="button" disabled={!editavel} onclick={() => (valores.id_mine = 0)}>Tirar</button>{/if}
            </div>
          {/if}
        </section>

        {#if outros.length}
          <section class="caixa">
            <h3>Outros campos</h3>
            <CamposSimples nomes={outros} {porNome} bind:valores {originais} {editavel} rotulos={ROTULOS_DO_NPC} explicacoes={EXPLICACOES_DO_NPC} />
          </section>
        {/if}
      </div>
      <aside class="lateral">
        {#if String(valores.file_model ?? "").trim()}
          <ModeloDoMonstro npc caminho={String(originais.file_model)} idDoMonstro={f.id ?? 0}
            aoTrocarModelo={async (ecm) => { await salvar(tabela, indice, { file_model: ecm }); avisar("ok", `O NPC agora usa ${ecm} (no rascunho).`); await carregar(); }} />
        {:else}
          <section class="caixa"><p class="suave">Sem modelo (file_model vazio).</p></section>
        {/if}
        <section class="caixa">
          <h3>Arquivo do modelo</h3>
          <CamposSimples nomes={["file_model"]} {porNome} bind:valores {originais} {editavel} rotulos={ROTULOS_DO_NPC} />
          <p class="mini">Trocar por um .ecm existente: escreva o caminho e grave; trocar por um GLB: "Importar GLB → Tudo" no card do modelo.</p>
        </section>
      </aside>
    </div>
  {:else if aba === "onde"}
    {#if f.id}<BoxDePontosDoNpc id={f.id} nome={String(valores.Name ?? "")} />{/if}
  {:else if aba === "conversa"}
    <ConversasDoNpc nome={String(valores.Name ?? "")} bind:valores {vinculados} {aoVincular} />
  {:else if aba === "servicos"}
    <ServicosDoNpc nome={String(valores.Name ?? "")} bind:valores {originais} {vinculados} {aoVincular} />
  {:else if aba === "missoes"}
    {#if f.id}<MissoesDoNpc nome={String(valores.Name ?? "")} id={f.id} {vinculados} {aoVincular} />{/if}
  {/if}
{/if}

{#if escolhendo}
  {@const e = escolhendo}
  <SeletorDeEspaco titulo={e.titulo} espaco="ESSENCE" categoriaInicial={e.categoria} soItens={false} aoFechar={() => (escolhendo = null)}
    aoEscolher={(r) => { valores[e.campo] = r.id ?? 0; nomes[e.campo] = r.nome; }} />
{/if}

<style>
  .barra { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .espaco { flex: 1; }
  .cabeca { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 6px; }
  .nome { font-size: 20px; font-weight: 600; height: 42px; min-width: 280px; }
  .selo { font-size: 12px; padding: 3px 10px; border-radius: 999px; border: 1px solid var(--vermelho); background: var(--vermelho-suave); }
  .abas { display: flex; gap: 2px; border-bottom: 1px solid var(--borda); margin-bottom: 14px; flex-wrap: wrap; }
  .abas button { padding: 10px 16px; border: 0; border-bottom: 2px solid transparent; background: none; color: var(--texto-2); font: inherit; font-size: 14px; cursor: pointer; }
  .abas button.on { color: var(--texto); border-color: var(--ouro); font-weight: 600; }
  .cont { margin-left: 6px; font-size: 11px; color: var(--texto-3); }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(320px, 460px); gap: 14px; align-items: start; }
  .coluna, .lateral { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .fala { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; }
  .previa { background: #0d1117; border: 1px solid #5a4a2a; border-radius: 6px; padding: 8px 10px; font-size: 13px; line-height: 1.5; min-height: 34px; }
  .fala .botao { align-self: flex-start; }
  .monstro { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 8px 10px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); margin-bottom: 10px; }
  .monstro p { flex-basis: 100%; margin: 0; }
  .erro-borda { border-color: var(--vermelho); }
  .extra { margin-top: 10px; }
  @media (max-width: 960px) { .layout { grid-template-columns: 1fr; } }
</style>
