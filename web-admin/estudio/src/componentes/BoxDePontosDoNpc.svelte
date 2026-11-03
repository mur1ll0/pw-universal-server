<script lang="ts">
  /* B392: onde o NPC fica — as áreas de NPC do npcgen (`iNPCType` = 1, `npcgendata.h:313-317`) que o citam, cada uma um
     ponto com direção. O GS usa a direção do `vDir` só quando a área é um ponto (extensão ~0); com tamanho ele sorteia
     (`base_spawner::SetRegion`/`GenDir`, `npcgenerator.cpp:4348-4367`; o nosso `pw-gs::entity::direcao_do_gerador`, B59).
     Por isso gravar o lugar zera `vExts` e grava `vPos` (com o chão do .hmap, B248) e `vDir`.
     Uma área com mais de um NPC: mover este o separa numa área própria (clone com só ele), para não arrastar os outros.
     No mapa principal (id 1) o cliente também usa o rastreador de missões (`task_npc.data`, `DlgNameLink.cpp:240`): a
     posição nova vai para lá junto, se marcado. Tudo no rascunho, pelas rotas dos Editores (B243/B248/B297). */
  import { alturaDoChao, clonarArea, lerArea, mapasDoRealm, nascimentos, npcgenDoMapa, posicaoNoRastreador, removerArea, salvarArea,
    type EscolhaNoMapa, type MapaDoRealm } from "../lib/api";
  import { avisar, estudio } from "../lib/estado.svelte";
  import { grausDoVetor, vetorDosGraus } from "../lib/npc";
  import Icone from "./Icone.svelte";
  import ModalMapa from "./ModalMapa.svelte";

  let { id, nome }: { id: number; nome: string } = $props();
  const editavel = $derived(!!estudio.rascunho);

  interface Ponto { caminho: string; mapa: string; area: number; x: number; z: number; direcao: number; ponto: boolean; outros: number }
  let pontos = $state<Ponto[] | null>(null);
  let erro = $state("");
  let mapas = $state<MapaDoRealm[]>([]);
  let ocupado = $state(false);
  let editando = $state<null | { mapa: MapaDoRealm; ponto: Ponto | null }>(null);
  let vizinhos = $state<{ x: number; z: number; largura: number; altura: number; rotulo: string }[]>([]);
  let mapaNovo = $state("");
  let rastreador = $state(true);

  async function carregar() {
    erro = "";
    try {
      const locais = (await nascimentos(id)).locais.filter((l) => l.tipo === "npc");
      const r: Ponto[] = [];
      for (const l of locais) {
        const a = await lerArea(l.caminho, l.area);
        const c = a.campos as Record<string, number>;
        const ext = Math.abs(c.vExts_x ?? 0) + Math.abs(c.vExts_y ?? 0) + Math.abs(c.vExts_z ?? 0);
        r.push({ caminho: l.caminho, mapa: l.mapa, area: l.area, x: l.x, z: l.z, direcao: grausDoVetor(Number(c.vDir_x ?? 0), Number(c.vDir_z ?? 0)),
          ponto: ext < 0.05, outros: a.filhos.filter((f) => Number(f.dwID) !== id).length });
      }
      pontos = r;
    } catch (e) { erro = (e as Error).message; }
  }
  $effect(() => { void estudio.rascunho; if (id) carregar(); });
  mapasDoRealm().then((m) => (mapas = m.filter((x) => x.npcgen))).catch(() => {});
  const mapaDe = (c: string) => mapas.find((m) => m.caminho === c);

  async function abrirNoMapa(mapa: MapaDoRealm, ponto: Ponto | null) {
    vizinhos = [];
    try {
      const n = await npcgenDoMapa(mapa.caminho);
      vizinhos = (n.areas as unknown[][]).filter((a) => a[1] === "npc" && !(ponto && a[0] === ponto.area))
        .slice(0, 3000).map((a) => ({ x: Number(a[2]), z: Number(a[3]), largura: 0, altura: 0, rotulo: `${n.nomes?.[String(a[6])] ?? "NPC"} (área ${a[0]})` }));
    } catch { /* sem vizinhos, o mapa abre igual */ }
    editando = { mapa, ponto };
  }

  async function executar(acao: () => Promise<string>) {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para editar onde o NPC fica."); return; }
    ocupado = true;
    try { avisar("ok", await acao()); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; await carregar(); }
  }

  async function camposDoLugar(caminho: string, e: EscolhaNoMapa) {
    const a = await alturaDoChao(caminho, e.x, e.z).catch(() => null);
    const d = vetorDosGraus(e.direcao ?? 0);
    const c: Record<string, number> = { vPos_x: e.x, vPos_z: e.z, vExts_x: 0, vExts_y: 0, vExts_z: 0, vDir_x: d.x, vDir_y: 0, vDir_z: d.z };
    if (a?.tem_terreno) c.vPos_y = Math.round(a.y * 100) / 100;
    return c;
  }
  async function noRastreador(caminho: string, c: Record<string, number>) {
    if (!rastreador || mapaDe(caminho)?.id !== 1) return "";
    await posicaoNoRastreador(id, [c.vPos_x, c.vPos_y ?? 0, c.vPos_z]);
    return " Rastreador de missões atualizado.";
  }

  function mover(p: Ponto, e: EscolhaNoMapa) {
    executar(async () => {
      const c = await camposDoLugar(p.caminho, e);
      const a = await lerArea(p.caminho, p.area);
      const meu = a.filhos.find((f) => Number(f.dwID) === id);
      if (a.filhos.length > 1 && meu) {
        // Separar: clone com só este NPC no lugar novo; a área antiga fica com os outros.
        const r = await clonarArea(p.caminho, p.area);
        const novo = r.resultado?.indice ?? r.indice;
        if (novo == null) throw new Error("O clone não devolveu o índice da área nova.");
        await salvarArea(p.caminho, novo, c, [meu]);
        await salvarArea(p.caminho, p.area, {}, a.filhos.filter((f) => Number(f.dwID) !== id));
        return `${nome} separado na área ${novo} (a área ${p.area} ficou com os outros ${a.filhos.length - 1}).${await noRastreador(p.caminho, c)}`;
      }
      await salvarArea(p.caminho, p.area, c);
      return `${nome} no lugar novo (área ${p.area}, direção ${e.direcao ?? 0}°).${await noRastreador(p.caminho, c)}`;
    });
  }

  function tirar(p: Ponto) {
    executar(async () => {
      const a = await lerArea(p.caminho, p.area);
      const resto = a.filhos.filter((f) => Number(f.dwID) !== id);
      if (!resto.length) { await removerArea(p.caminho, p.area); return `Área ${p.area} (${p.caminho}) removida do npcgen.`; }
      await salvarArea(p.caminho, p.area, {}, resto);
      return `${nome} tirado da área ${p.area} (os outros NPCs dela ficam).`;
    });
  }

  /** Ponto novo: clone de uma área deste NPC no mapa; sem nenhuma, da primeira área de NPC (iNPCType 1) do mapa. */
  function criar(caminho: string, e: EscolhaNoMapa) {
    executar(async () => {
      let base: number | null = (pontos ?? []).find((p) => p.caminho === caminho)?.area ?? null;
      let filho: Record<string, number | boolean> | null = null;
      if (base != null) filho = (await lerArea(caminho, base)).filhos.find((f) => Number(f.dwID) === id) ?? null;
      else {
        const n = await npcgenDoMapa(caminho);
        const linha = (n.areas as unknown[][]).find((a) => a[1] === "npc");
        if (!linha) throw new Error(`${caminho}: o npcgen não tem área de NPC para servir de base.`);
        base = Number(linha[0]);
        filho = (await lerArea(caminho, base)).filhos[0] ?? null;
      }
      const r = await clonarArea(caminho, base);
      const novo = r.resultado?.indice ?? r.indice;
      if (novo == null) throw new Error("O clone não devolveu o índice da área nova.");
      const c = await camposDoLugar(caminho, e);
      await salvarArea(caminho, novo, { ...c, bInitGen: true, bAutoRevive: true }, [{ ...(filho ?? {}), dwID: id, dwNum: 1 }]);
      return `Ponto novo: área ${novo} em ${caminho} (cópia da área ${base}).${await noRastreador(caminho, c)}`;
    });
  }
</script>

<section class="caixa">
  <h3>Onde fica {#if pontos}<span class="mini">{pontos.length} ponto(s)</span>{/if}</h3>
  <p class="mini">Cada ponto é uma área de NPC do npcgen. Com a área num ponto, o servidor vira o NPC para a direção da seta; com tamanho, ele sorteia
    (npcgenerator.cpp:4348-4367). Grava no rascunho; vale no jogo depois de publicar.</p>
  {#if erro}<p class="erro">{erro}</p>
  {:else if !pontos}<p class="suave">Carregando…</p>
  {:else}
    <ul>
      {#each pontos as p (p.caminho + p.area)}
        <li>
          <span class="seta" style:transform="rotate({-p.direcao}deg)" title="{p.direcao}°">➜</span>
          <span class="onde"><strong>{p.mapa}</strong> <span class="mini">{p.caminho}</span><br />
            ({p.x.toFixed(1)}, {p.z.toFixed(1)}) · {p.direcao}°
            {#if !p.ponto}<span class="aviso-mini">área com tamanho: a direção é sorteada</span>{/if}
            {#if p.outros}<span class="aviso-mini">a área tem mais {p.outros} NPC(s): mover separa este</span>{/if}</span>
          <button class="botao pequeno" type="button" disabled={!mapaDe(p.caminho) || ocupado} onclick={() => abrirNoMapa(mapaDe(p.caminho)!, p)}>
            <Icone nome="mapa" tamanho={16} /> No mapa</button>
          <button class="botao fantasma pequeno" type="button" disabled={!editavel || ocupado} aria-label="Remover este ponto" onclick={() => tirar(p)}><Icone nome="lixeira" tamanho={16} /></button>
        </li>
      {:else}
        <li class="vazio">Este NPC não está em nenhum npcgen: não nasce em mapa nenhum.</li>
      {/each}
    </ul>
    <div class="linha novo">
      <select class="campo" bind:value={mapaNovo} aria-label="Mapa do ponto novo">
        <option value="">— mapa do ponto novo —</option>
        {#each mapas as m, k (k)}<option value={m.caminho}>{m.nome} ({m.caminho})</option>{/each}
      </select>
      <button class="botao pequeno" type="button" disabled={!editavel || !mapaNovo || ocupado} onclick={() => abrirNoMapa(mapaDe(mapaNovo)!, null)}>
        <Icone nome="mais" tamanho={16} /> Pôr em outro lugar</button>
    </div>
    <label class="mini rastreador"><input type="checkbox" bind:checked={rastreador} /> No mapa principal, levar a posição também ao rastreador de missões (task_npc.data)</label>
  {/if}
</section>

{#if editando}
  {@const ed = editando}
  <ModalMapa mapa={ed.mapa} comDirecao titulo="Onde fica {nome}" aoFechar={() => (editando = null)}
    inicial={ed.ponto ? { x: ed.ponto.x, z: ed.ponto.z, largura: 0, altura: 0, direcao: ed.ponto.direcao } : null}
    outros={vizinhos}
    aoAplicar={(e) => (ed.ponto ? mover(ed.ponto, e) : criar(ed.mapa.caminho, e))} />
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; max-height: 340px; overflow: auto; }
  li { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  li.vazio { color: var(--texto-2); }
  .onde { flex: 1; min-width: 200px; font-variant-numeric: tabular-nums; line-height: 1.4; }
  .seta { display: inline-grid; place-items: center; width: 26px; height: 26px; border-radius: 50%; border: 1px solid var(--ouro); color: var(--ouro); font-size: 14px; }
  .aviso-mini { display: inline-block; margin-left: 6px; font-size: 11px; color: var(--ouro); }
  .novo { margin-top: 10px; }
  .rastreador { display: flex; gap: 6px; align-items: center; margin-top: 8px; }
</style>
