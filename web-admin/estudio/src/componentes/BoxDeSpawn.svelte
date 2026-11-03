<script lang="ts">
  /* B371: onde o monstro nasce — os spawners (áreas do npcgen) que o citam, editados direto no npcgen do rascunho pelas
     rotas dos Editores (B243): `PUT …/npcgen/areas/{i}` com `campos` (vPos, vExts) e `filhos` (monstros e quantidade),
     `POST …/clonar` para um spawner novo (clona uma área de monstro do mesmo mapa) e `DELETE` para tirar.
     Uma área pode ter vários monstros (`filhos`): tirar este monstro só apaga a área se ele for o único. */
  import { alturaDoChao, clonarArea, lerArea, mapasDoRealm, nascimentos, npcgenDoMapa, removerArea, salvarArea,
    type EscolhaNoMapa, type Local, type MapaDoRealm } from "../lib/api";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";
  import ModalMapa from "./ModalMapa.svelte";

  let { id, nome }: { id: number; nome: string } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let locais = $state<Local[] | null>(null);
  let erro = $state("");
  let mapas = $state<MapaDoRealm[]>([]);
  let extensoes = $state<Record<string, Record<number, [number, number]>>>({}); // caminho → área → [ext_x, ext_z]
  let ocupado = $state(false);
  let editando = $state<null | { mapa: MapaDoRealm; local: Local | null }>(null);
  let mapaNovo = $state("");

  async function carregar() {
    erro = "";
    try {
      locais = (await nascimentos(id)).locais.filter((l) => l.tipo === "monstro" || l.tipo === "criatura" || true);
      for (const c of new Set(locais.map((l) => l.caminho))) await extensoesDe(c);
    } catch (e) { erro = (e as Error).message; }
  }
  async function extensoesDe(caminho: string, forcar = false) {
    if (extensoes[caminho] && !forcar) return;
    const n = await npcgenDoMapa(caminho);
    const cols: string[] = n.colunas;
    const [ii, ix, iz] = [cols.indexOf("indice"), cols.indexOf("ext_x"), cols.indexOf("ext_z")];
    const m: Record<number, [number, number]> = {};
    for (const a of n.areas) m[a[ii]] = [Number(a[ix]), Number(a[iz])];
    extensoes[caminho] = m;
  }
  $effect(() => { void estudio.rascunho; if (id) carregar(); });
  mapasDoRealm().then((m) => (mapas = m.filter((x) => x.npcgen))).catch(() => {});

  const porMapa = $derived.by(() => {
    const g = new Map<string, Local[]>();
    for (const l of locais ?? []) g.set(l.caminho, [...(g.get(l.caminho) ?? []), l]);
    return [...g.entries()];
  });
  const mapaDe = (c: string) => mapas.find((m) => m.caminho === c);
  const ext = (l: Local) => extensoes[l.caminho]?.[l.area] ?? [0, 0];

  async function executar(acao: () => Promise<string>) {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para editar o spawn."); return; }
    ocupado = true;
    try { avisar("ok", await acao()); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally {
      ocupado = false;
      extensoes = {};
      await carregar();
    }
  }

  /** Posição e área: vPos (com a altura do chão do .hmap, B248) e vExts. */
  async function camposDoLugar(caminho: string, e: EscolhaNoMapa) {
    const a = await alturaDoChao(caminho, e.x, e.z).catch(() => null);
    const c: Record<string, number> = { vPos_x: e.x, vPos_z: e.z, vExts_x: e.largura, vExts_z: e.altura };
    if (a?.tem_terreno) c.vPos_y = Math.round(a.y * 100) / 100;
    return c;
  }

  function mudarLugar(l: Local, e: EscolhaNoMapa) {
    executar(async () => {
      await salvarArea(l.caminho, l.area, await camposDoLugar(l.caminho, e));
      return `Spawner ${l.area} (${l.caminho}) movido no rascunho.`;
    });
  }

  function mudarQuantidade(l: Local, n: number) {
    if (!Number.isInteger(n) || n < 1) { avisar("aviso", "Quantidade inteira, a partir de 1."); return; }
    executar(async () => {
      const a = await lerArea(l.caminho, l.area);
      const filhos = a.filhos.map((f) => (Number(f.dwID) === id ? { ...f, dwNum: n } : f));
      await salvarArea(l.caminho, l.area, {}, filhos);
      return `Spawner ${l.area}: ${n} × ${nome}.`;
    });
  }

  function tirar(l: Local) {
    executar(async () => {
      const a = await lerArea(l.caminho, l.area);
      const resto = a.filhos.filter((f) => Number(f.dwID) !== id);
      if (!resto.length) { await removerArea(l.caminho, l.area); return `Spawner ${l.area} (${l.caminho}) removido do npcgen.`; }
      await salvarArea(l.caminho, l.area, {}, resto);
      return `${nome} tirado do spawner ${l.area} (os outros monstros dele ficam).`;
    });
  }

  /** Spawner novo: clona uma área deste monstro no mapa; sem nenhuma, a primeira área de monstro (iNPCType 0) do mapa. */
  function criar(caminho: string, e: EscolhaNoMapa) {
    executar(async () => {
      const meus = (locais ?? []).filter((l) => l.caminho === caminho);
      let base: number | null = meus[0]?.area ?? null;
      let filhoBase: Record<string, number | boolean> | null = null;
      if (base == null) {
        const n = await npcgenDoMapa(caminho);
        const ii = (n.colunas as string[]).indexOf("indice");
        for (const a of (n.areas as unknown[][]).slice(0, 60)) {
          const x = await lerArea(caminho, Number(a[ii]));
          if (Number(x.campos.iNPCType) === 0 && x.filhos.length) { base = x.indice; filhoBase = x.filhos[0]; break; }
        }
        if (base == null) throw new Error(`${caminho}: nenhuma área de monstro para servir de base (o npcgen não tem como criar área do zero por aqui).`);
      } else {
        filhoBase = (await lerArea(caminho, base)).filhos.find((f) => Number(f.dwID) === id) ?? null;
      }
      const c = await clonarArea(caminho, base);
      const novo = c.resultado?.indice ?? c.indice;
      if (novo == null) throw new Error("O clone não devolveu o índice da área nova.");
      const filho = { ...(filhoBase ?? {}), dwID: id, dwNum: Number(filhoBase?.dwNum ?? 1) || 1 };
      await salvarArea(caminho, novo, { ...(await camposDoLugar(caminho, e)), bInitGen: true, bAutoRevive: true, idCtrl: 0 }, [filho]);
      return `Spawner novo ${novo} em ${caminho} (cópia da área ${base}).`;
    });
  }
</script>

<section class="caixa">
  <h3>Onde nasce {#if locais}<span class="mini">{locais.length} spawner(s)</span>{/if}</h3>
  <p class="mini">Cada spawner é uma área do npcgen do mapa: o ponto central e o retângulo onde o servidor sorteia o nascimento. Edita-se no rascunho; vale no jogo depois de publicar.</p>
  {#if erro}<p class="erro">{erro}</p>{:else if !locais}<p class="suave">Carregando…</p>
  {:else}
    {#each porMapa as [caminho, lista] (caminho)}
      <h4>{lista[0].mapa} <span class="mini">{caminho}</span></h4>
      <ul>
        {#each lista as l (l.area)}
          <li>
            <span class="onde">({l.x.toFixed(1)}, {l.z.toFixed(1)}) · área {ext(l)[0]}×{ext(l)[1]}</span>
            <label class="mini">quantidade
              <input class="campo n" type="number" min="1" step="1" value={l.quantidade} disabled={!editavel || ocupado}
                onchange={(e) => mudarQuantidade(l, Number((e.target as HTMLInputElement).value))} /></label>
            <button class="botao pequeno" type="button" disabled={!mapaDe(caminho) || ocupado} onclick={() => (editando = { mapa: mapaDe(caminho)!, local: l })}>
              <Icone nome="editar" tamanho={16} /> No mapa</button>
            <button class="botao fantasma pequeno" type="button" disabled={!editavel || ocupado} aria-label="Remover" onclick={() => tirar(l)}><Icone nome="lixeira" tamanho={16} /></button>
            <span class="mini">spawner {l.area}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="suave">Este monstro não nasce em nenhum npcgen.</p>
    {/each}
    <div class="linha novo">
      <select class="campo" bind:value={mapaNovo} aria-label="Mapa do spawner novo">
        <option value="">— mapa do spawner novo —</option>
        {#each mapas as m, k (k)}<option value={m.caminho}>{m.nome} ({m.caminho})</option>{/each}
      </select>
      <button class="botao pequeno" type="button" disabled={!editavel || !mapaNovo || ocupado}
        onclick={() => (editando = { mapa: mapaDe(mapaNovo)!, local: null })}><Icone nome="mais" tamanho={16} /> Adicionar spawner</button>
    </div>
  {/if}
</section>

{#if editando}
  {@const ed = editando}
  <ModalMapa mapa={ed.mapa} aoFechar={() => (editando = null)}
    inicial={ed.local ? { x: ed.local.x, z: ed.local.z, largura: ext(ed.local)[0], altura: ext(ed.local)[1] } : null}
    outros={(locais ?? []).filter((l) => l.caminho === ed.mapa.caminho && l !== ed.local).map((l) => ({ x: l.x, z: l.z, largura: ext(l)[0], altura: ext(l)[1], rotulo: `spawner ${l.area}` }))}
    aoAplicar={(e) => (ed.local ? mudarLugar(ed.local, e) : criar(ed.mapa.caminho, e))} />
{/if}

<style>
  h4 { margin: 10px 0 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; max-height: 320px; overflow: auto; }
  li { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .onde { flex: 1; min-width: 200px; font-variant-numeric: tabular-nums; }
  .n { width: 80px; height: 28px; }
  .novo { margin-top: 10px; }
</style>
