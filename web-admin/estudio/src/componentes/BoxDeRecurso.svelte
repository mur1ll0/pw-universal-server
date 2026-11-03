<script lang="ts">
  /* Onde a mina aparece no mapa: as áreas de recurso do npcgen (`NPCGENFILERESAREA`) que citam a mina, editadas no npcgen
     do rascunho pelas rotas dos Editores (B243) com o tipo "recursos" — `PUT …/npcgen/recursos/{i}` com `campos` (vPos,
     fExtX/fExtZ) e `filhos` (as minas da área: idTemplate, dwNumber, dwRefreshTime, fHeiOff), `POST …/clonar` para uma
     área nova (clone de uma área de recurso do mesmo mapa) e `DELETE`. Como o GS lê a área: `npcgenerator.cpp:3886-3932`
     (`lib/itens.ts::CAMPOS_DA_AREA_DE_RECURSO`). No Material, `minas` são as minas que dão o material. */
  import { alturaDoChao, clonarArea, lerArea, mapasDoRealm, nascimentos, npcgenDoMapa, removerArea, salvarArea,
    type Area, type EscolhaNoMapa, type Local, type MapaDoRealm } from "../lib/api";
  import { CAMPOS_DA_AREA_DE_RECURSO, CAMPOS_DO_RECURSO, renascerDaMina } from "../lib/itens";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";
  import ModalMapa from "./ModalMapa.svelte";

  let { minas }: { minas: { id: number; nome: string }[] } = $props();
  const editavel = $derived(!!estudio.rascunho);

  type LocalDaMina = Local & { mina: number; nomeDaMina: string };
  let locais = $state<LocalDaMina[] | null>(null);
  let erro = $state("");
  let mapas = $state<MapaDoRealm[]>([]);
  let extensoes = $state<Record<string, Record<number, [number, number]>>>({}); // caminho → área → [fExtX, fExtZ]
  let detalhes = $state<Record<string, Area>>({}); // "caminho|área" → área lida (campos e minas)
  let ocupado = $state(false);
  let editando = $state<null | { mapa: MapaDoRealm; local: LocalDaMina | null; mina: number }>(null);
  let mapaNovo = $state("");
  let minaNova = $state(0);

  const chave = (l: Local) => `${l.caminho}|${l.area}`;

  async function carregar() {
    erro = "";
    try {
      const todos: LocalDaMina[] = [];
      for (const m of minas) {
        const r = await nascimentos(m.id);
        for (const l of r.locais) if (l.tipo === "recurso") todos.push({ ...l, mina: m.id, nomeDaMina: m.nome });
      }
      locais = todos;
      for (const c of new Set(todos.map((l) => l.caminho))) await extensoesDe(c);
      detalhes = {};
      for (const l of todos.slice(0, 40)) detalhes[chave(l)] = await lerArea(l.caminho, l.area, "recursos");
    } catch (e) { erro = (e as Error).message; }
  }
  async function extensoesDe(caminho: string, forcar = false) {
    if (extensoes[caminho] && !forcar) return;
    const n = await npcgenDoMapa(caminho);
    const cols: string[] = n.colunas;
    const [ii, ix, iz] = [cols.indexOf("indice"), cols.indexOf("ext_x"), cols.indexOf("ext_z")];
    const m: Record<number, [number, number]> = {};
    for (const a of n.recursos) m[a[ii]] = [Number(a[ix]), Number(a[iz])];
    extensoes[caminho] = m;
  }
  $effect(() => { void estudio.rascunho; void minas.map((m) => m.id).join(","); if (minas.length) carregar(); else locais = []; });
  $effect(() => { if (!minaNova && minas.length) minaNova = minas[0].id; });
  mapasDoRealm().then((m) => (mapas = m.filter((x) => x.npcgen))).catch(() => {});

  const porMapa = $derived.by(() => {
    const g = new Map<string, LocalDaMina[]>();
    for (const l of locais ?? []) g.set(l.caminho, [...(g.get(l.caminho) ?? []), l]);
    return [...g.entries()];
  });
  const mapaDe = (c: string) => mapas.find((m) => m.caminho === c);
  const ext = (l: Local) => extensoes[l.caminho]?.[l.area] ?? [0, 0];
  const filhoDa = (l: LocalDaMina) => detalhes[chave(l)]?.filhos.find((f) => Number(f.idTemplate) === l.mina);

  async function executar(acao: () => Promise<string>) {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para editar o npcgen."); return; }
    ocupado = true;
    try { avisar("ok", await acao()); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally {
      ocupado = false;
      extensoes = {};
      await carregar();
    }
  }

  /** Centro e retângulo: vPos (altura do chão pelo .hmap, B248) e fExtX/fExtZ (a área é centro ± ext/2, npcgenerator.cpp:3893). */
  async function camposDoLugar(caminho: string, e: EscolhaNoMapa) {
    const a = await alturaDoChao(caminho, e.x, e.z).catch(() => null);
    const c: Record<string, number> = { vPos_x: e.x, vPos_z: e.z, fExtX: e.largura, fExtZ: e.altura };
    if (a?.tem_terreno) c.vPos_y = Math.round(a.y * 100) / 100;
    return c;
  }

  function mudarLugar(l: LocalDaMina, e: EscolhaNoMapa) {
    executar(async () => {
      await salvarArea(l.caminho, l.area, await camposDoLugar(l.caminho, e), undefined, "recursos");
      return `Área de recurso ${l.area} (${l.caminho}) movida no rascunho.`;
    });
  }

  function mudarFilho(l: LocalDaMina, campo: "dwNumber" | "dwRefreshTime", n: number) {
    if (!Number.isInteger(n) || n < (campo === "dwNumber" ? 1 : 0)) { avisar("aviso", campo === "dwNumber" ? "Quantidade inteira, a partir de 1." : "Tempo inteiro, a partir de 0."); return; }
    executar(async () => {
      const a = await lerArea(l.caminho, l.area, "recursos");
      const filhos = a.filhos.map((f) => (Number(f.idTemplate) === l.mina ? { ...f, [campo]: n } : f));
      await salvarArea(l.caminho, l.area, {}, filhos, "recursos");
      return campo === "dwNumber" ? `Área ${l.area}: ${n} × ${l.nomeDaMina}.` : `Área ${l.area}: renasce em ${renascerDaMina(n)} s.`;
    });
  }

  function mudarCampoDaArea(l: LocalDaMina, campo: string, v: number | boolean) {
    executar(async () => {
      await salvarArea(l.caminho, l.area, { [campo]: v }, undefined, "recursos");
      return `Área ${l.area}: ${campo} = ${v}.`;
    });
  }

  function tirar(l: LocalDaMina) {
    executar(async () => {
      const a = await lerArea(l.caminho, l.area, "recursos");
      const resto = a.filhos.filter((f) => Number(f.idTemplate) !== l.mina);
      if (!resto.length) { await removerArea(l.caminho, l.area, "recursos"); return `Área de recurso ${l.area} (${l.caminho}) removida do npcgen.`; }
      await salvarArea(l.caminho, l.area, {}, resto, "recursos");
      return `${l.nomeDaMina} tirada da área ${l.area} (as outras minas dela ficam).`;
    });
  }

  /** Área nova: clona uma área desta mina no mapa; sem nenhuma, a primeira área de recurso do mapa. */
  function criar(caminho: string, mina: number, e: EscolhaNoMapa) {
    executar(async () => {
      const minhas = (locais ?? []).filter((l) => l.caminho === caminho && l.mina === mina);
      let base: number | null = minhas[0]?.area ?? null;
      let filhoBase: Record<string, number | boolean> | null = null;
      if (base == null) {
        const n = await npcgenDoMapa(caminho);
        const ii = (n.colunas as string[]).indexOf("indice");
        for (const a of (n.recursos as unknown[][]).slice(0, 60)) {
          const x = await lerArea(caminho, Number(a[ii]), "recursos");
          if (x.filhos.length) { base = x.indice; filhoBase = x.filhos[0]; break; }
        }
        if (base == null) throw new Error(`${caminho}: nenhuma área de recurso para servir de base (o npcgen não cria área do zero por aqui).`);
      } else {
        filhoBase = (await lerArea(caminho, base, "recursos")).filhos.find((f) => Number(f.idTemplate) === mina) ?? null;
      }
      const c = await clonarArea(caminho, base, "recursos");
      const novo = c.resultado?.indice ?? c.indice;
      if (novo == null) throw new Error("O clone não devolveu o índice da área nova.");
      const filho = { ...(filhoBase ?? {}), idTemplate: mina, dwNumber: Number(filhoBase?.dwNumber ?? 1) || 1 };
      await salvarArea(caminho, novo, { ...(await camposDoLugar(caminho, e)), bAutoRevive: true, idCtrl: 0 }, [filho], "recursos");
      return `Área de recurso nova ${novo} em ${caminho} (cópia da área ${base}).`;
    });
  }
</script>

<section class="caixa">
  <h3>Onde aparece {#if locais}<span class="mini">{locais.length} área(s) de recurso</span>{/if}</h3>
  <p class="mini">Cada área de recurso do npcgen sorteia as minas no retângulo centro ± largura/2 (npcgenerator.cpp:3893). Uma mina recusada pelo GS não nasce. Edita-se no rascunho; vale no jogo depois de publicar e reiniciar o mundo.</p>
  {#if erro}<p class="erro">{erro}</p>{:else if !locais}<p class="suave">Carregando…</p>
  {:else}
    {#each porMapa as [caminho, lista] (caminho)}
      <h4>{lista[0].mapa} <span class="mini">{caminho}</span></h4>
      <ul>
        {#each lista as l (`${l.area}-${l.mina}`)}
          {@const f = filhoDa(l)}
          {@const a = detalhes[chave(l)]}
          <li>
            <div class="linha-l">
              {#if minas.length > 1}<strong class="mini">{l.nomeDaMina}</strong>{/if}
              <span class="onde">({l.x.toFixed(1)}, {l.z.toFixed(1)}) · {ext(l)[0]}×{ext(l)[1]} m</span>
              <label class="mini">quantidade
                <input class="campo n" type="number" min="1" step="1" value={l.quantidade} disabled={!editavel || ocupado}
                  title={CAMPOS_DO_RECURSO.dwNumber} onchange={(e) => mudarFilho(l, "dwNumber", Number((e.target as HTMLInputElement).value))} /></label>
              {#if f}
                <label class="mini">renascer +s
                  <input class="campo n" type="number" min="0" step="1" value={Number(f.dwRefreshTime)} disabled={!editavel || ocupado}
                    title={CAMPOS_DO_RECURSO.dwRefreshTime} onchange={(e) => mudarFilho(l, "dwRefreshTime", Number((e.target as HTMLInputElement).value))} /></label>
                <span class="mini">= {renascerDaMina(Number(f.dwRefreshTime))} s</span>
              {/if}
              <button class="botao pequeno" type="button" disabled={!mapaDe(caminho) || ocupado} onclick={() => (editando = { mapa: mapaDe(caminho)!, local: l, mina: l.mina })}>
                <Icone nome="editar" tamanho={16} /> No mapa</button>
              <button class="botao fantasma pequeno" type="button" disabled={!editavel || ocupado} aria-label="Remover" onclick={() => tirar(l)}><Icone nome="lixeira" tamanho={16} /></button>
              <span class="mini">área {l.area}</span>
            </div>
            {#if a}
              <details>
                <summary class="mini">Campos da área ({a.filhos.length} mina(s) nela)</summary>
                <div class="campos">
                  {#each ["iMaxNum", "bAutoRevive", "idCtrl", "bInitGen", "bValidOnce"] as c (c)}
                    {#if c in a.campos}
                      <label class="mini" title={CAMPOS_DA_AREA_DE_RECURSO[c]}>
                        <span>{c}</span>
                        {#if typeof a.campos[c] === "boolean"}
                          <input type="checkbox" checked={!!a.campos[c]} disabled={!editavel || ocupado} onchange={(e) => mudarCampoDaArea(l, c, (e.target as HTMLInputElement).checked)} />
                        {:else}
                          <input class="campo n" type="number" step="1" value={Number(a.campos[c])} disabled={!editavel || ocupado}
                            onchange={(e) => mudarCampoDaArea(l, c, Number((e.target as HTMLInputElement).value))} />
                        {/if}
                        <span class="suave">{CAMPOS_DA_AREA_DE_RECURSO[c]}</span>
                      </label>
                    {/if}
                  {/each}
                  {#if f}
                    <p class="mini">fHeiOff {Number(f.fHeiOff)} — {CAMPOS_DO_RECURSO.fHeiOff}. iResType {Number(f.iResType)} — {CAMPOS_DO_RECURSO.iResType}.</p>
                  {/if}
                </div>
              </details>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="suave">{minas.length ? "Nenhuma área de recurso de nenhum mapa cita esta(s) mina(s)." : "Nenhuma mina dá este material."}</p>
    {/each}
    {#if minas.length}
      <div class="linha novo">
        {#if minas.length > 1}
          <select class="campo" bind:value={minaNova} aria-label="Mina da área nova">
            {#each minas as m (m.id)}<option value={m.id}>{m.nome} ({m.id})</option>{/each}
          </select>
        {/if}
        <select class="campo" bind:value={mapaNovo} aria-label="Mapa da área nova">
          <option value="">— mapa da área nova —</option>
          {#each mapas as m, k (k)}<option value={m.caminho}>{m.nome} ({m.caminho})</option>{/each}
        </select>
        <button class="botao pequeno" type="button" disabled={!editavel || !mapaNovo || ocupado}
          onclick={() => (editando = { mapa: mapaDe(mapaNovo)!, local: null, mina: minaNova })}><Icone nome="mais" tamanho={16} /> Criar área de recurso</button>
      </div>
    {/if}
  {/if}
</section>

{#if editando}
  {@const ed = editando}
  <ModalMapa mapa={ed.mapa} aoFechar={() => (editando = null)}
    inicial={ed.local ? { x: ed.local.x, z: ed.local.z, largura: ext(ed.local)[0], altura: ext(ed.local)[1] } : null}
    outros={(locais ?? []).filter((l) => l.caminho === ed.mapa.caminho && l !== ed.local).map((l) => ({ x: l.x, z: l.z, largura: ext(l)[0], altura: ext(l)[1], rotulo: `área ${l.area}` }))}
    aoAplicar={(e) => (ed.local ? mudarLugar(ed.local, e) : criar(ed.mapa.caminho, ed.mina, e))} />
{/if}

<style>
  h4 { margin: 10px 0 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; max-height: 420px; overflow: auto; }
  li { display: flex; flex-direction: column; gap: 4px; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .linha-l { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .onde { flex: 1; min-width: 180px; font-variant-numeric: tabular-nums; }
  .n { width: 80px; height: 28px; }
  .novo { margin-top: 10px; flex-wrap: wrap; }
  .campos { display: flex; flex-direction: column; gap: 4px; margin-top: 6px; }
  .campos label { display: grid; grid-template-columns: 90px auto 1fr; align-items: center; gap: 8px; }
</style>
