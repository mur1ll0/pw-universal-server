<script lang="ts">
  /* B383: o modelo próprio do equipamento (arma, asa, voo, arma de moda) ↔ Blender, como o do Monstro (B373/B380), mas
     montado como o cliente monta o equipamento: o `.ecm` com as peles do próprio `.smd` (`CECModel::Load(…,
     LSF_UNIQUESKIN)`, `EC_Player.cpp:4547-4583`) — `como=equipamento` no GLB e na importação —, pendurado no personagem
     pelo gancho do modelo (`CC_weapon`/`CC_qiu`, `CC_chibang`, `CC_feijian`; `lib/equipavel3d.ts`). Importar:
     - só o modelo: malha e texturas na 1ª pele do .smd (mesmo esqueleto, os ganchos ficam);
     - só as animações: as ações do GLB no .smd (mesmo esqueleto);
     - tudo: modelo novo em `<pasta>\<título>` (o GLB tem de trazer o gancho), o campo passa a citá-lo e o antigo que nada
       mais usa sai no rascunho.
     "Conferir" roda a mesma importação sem gravar e mostra o relatório (pesos corrigidos, ossos, ganchos). */
  import { onDestroy } from "svelte";
  import { fichaDoModelo, importarGlbNoModelo, importarModeloGlb, removerModeloSemUso, rotaDoGlb, type FichaDoModelo } from "../lib/api";
  import { ganchosExigidos, pastaSugerida, type CampoDeModelo } from "../lib/equipavel3d";
  import { desenharGlb } from "../lib/glb";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";

  /* `como`: "equipamento" (arma, asa, voo — peles do .smd, pendura num gancho) ou "matter" (item no chão e mina:
     `CECMatter::LoadMatterModel`, `EC_Matter.cpp:71-107` — o `CECModel::Load` comum, as peles do .smd e, sem nenhuma, a
     `<.smd>.ski`; não pendura em gancho). */
  let { tabela, campos, caminhos, modoDeVoo = 0, idDoItem = 0, aoTrocarModelo, como = "equipamento", titulo: tituloDoCard = "Modelo 3D no personagem", pastaPadrao = "" }:
    { tabela: string; campos: CampoDeModelo[]; caminhos: Record<string, string>; modoDeVoo?: number; idDoItem?: number;
      aoTrocarModelo: (campo: string, ecm: string) => Promise<void>; como?: "equipamento" | "matter"; titulo?: string; pastaPadrao?: string } = $props();

  const comModelo = $derived(campos.filter((c) => (caminhos[c.campo] ?? "").trim()));
  let escolhido = $state("");
  const campo = $derived(comModelo.find((c) => c.campo === escolhido) ?? comModelo[0]);
  const caminho = $derived(campo ? caminhos[campo.campo] : "");
  const ganchos = $derived(campo ? ganchosExigidos(tabela, campo.campo, modoDeVoo) : []);
  const editavel = $derived(!!estudio.rascunho);
  const arquivoDoGlb = $derived(`${(caminho.split("\\").pop() ?? "modelo").replace(/\.[^.]*$/, "")}.glb`);

  let ficha = $state<FichaDoModelo | null>(null);
  /** B384: o nome legível que a ficha manda (o mesmo do GLB exportado: "Voando parado · 飞行悬停"). */
  const traduzir = (n: string) => ficha?.nomes_legiveis?.[n] ?? n;
  let tela = $state<HTMLCanvasElement | null>(null);
  let tocando = $state("");
  let erroDoGlb = $state("");
  let carregando = $state(false);
  let ocupado = $state(false);
  let parar: (() => void) | null = null;
  let modo = $state<"modelo" | "animacoes" | "tudo">("modelo");
  let pasta = $state("");
  let titulo = $state("");
  let relatorio = $state<{ titulo: string; linhas: string[]; ok: boolean } | null>(null);
  let arquivo: File | null = $state(null);

  $effect(() => { if (!pasta) pasta = pastaPadrao || pastaSugerida(tabela); });

  async function carregarFicha() {
    ficha = null;
    try { ficha = await fichaDoModelo(caminho); } catch { ficha = null; }
  }
  async function tocar(base: string) {
    tocando = base;
    parar?.(); parar = null; erroDoGlb = ""; carregando = true;
    try {
      const r = await fetch(rotaDoGlb(caminho, base || undefined, false, { como }), { credentials: "same-origin", cache: "no-store" });
      if (!r.ok) throw new Error(`GLB: HTTP ${r.status}${r.status === 400 ? " — " + ((await r.json().catch(() => ({}))).detail || "") : ""}`);
      if (tela) parar = desenharGlb(tela, await r.arrayBuffer());
    } catch (e) { erroDoGlb = (e as Error).message; }
    finally { carregando = false; }
  }
  let mostrado = "";
  $effect(() => {
    const chave = `${caminho}|${estudio.rascunho ?? ""}`;
    if (caminho && tela && chave !== mostrado) { mostrado = chave; carregarFicha(); tocar(""); }
  });
  onDestroy(() => parar?.());

  const linhasDe = (r: any): string[] => {
    const x = r?.resultado ?? r ?? {};
    const l: string[] = [];
    if (x.malhas != null) l.push(`${x.malhas} malha(s), ${x.vertices ?? "?"} vértices, ${x.ossos ?? "?"} osso(s)${x.ganchos ? ` · ganchos: ${(x.ganchos as string[]).join(", ") || "nenhum"}` : ""}`);
    if (x.acoes) l.push(`animações: ${(x.acoes as any[]).map((a) => (typeof a === "string" ? a : a.nome)).map((n) => (traduzir(n) !== n ? `${traduzir(n)} (${n})` : n)).join(", ") || "nenhuma"}`);
    if (x.pele) l.push(`pele: ${x.pele}`);
    if (x.ecm) l.push(`modelo novo: ${x.ecm}`);
    if (x.peles_tiradas_do_smd) l.push(`peles tiradas do .smd (as malhas foram para a 1ª): ${(x.peles_tiradas_do_smd as string[]).join(", ")}`);
    for (const t of x.texturas_gravadas ?? []) l.push(`textura: ${t.caminho ?? t.nome ?? JSON.stringify(t)} ${t.acao ?? ""}`);
    for (const t of x.relatorio ?? []) l.push(t);
    if (!(x.relatorio ?? []).length) l.push("nada a corrigir: pesos, ossos e pose de repouso como o cliente lê.");
    return l;
  };

  async function executar(validar: boolean) {
    if (!arquivo) { avisar("aviso", "Escolha o GLB primeiro."); return; }
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto."); return; }
    ocupado = true; relatorio = null;
    try {
      const glb = await arquivo.arrayBuffer();
      if (modo === "tudo") {
        const t = (titulo.trim() || arquivo.name.replace(/\.glb$/i, "")).trim();
        if (!pasta.trim().toLowerCase().startsWith("models\\")) throw new Error("A pasta tem de ficar dentro de models\\.");
        let d;
        try { d = await importarModeloGlb(pasta.trim(), t, caminho, glb, { ganchos: ganchos.join(","), validar }); }
        catch (er) {
          if (!/molde/i.test((er as Error).message)) throw er;
          d = await importarModeloGlb(pasta.trim(), t, "", glb, { ganchos: ganchos.join(","), validar });
        }
        relatorio = { titulo: validar ? "Conferência (nada gravado)" : "Modelo novo no rascunho", linhas: linhasDe(d), ok: true };
        if (!validar) {
          const ecm = d?.resultado?.ecm;
          if (!ecm) throw new Error("O servidor não devolveu o .ecm do modelo novo.");
          const antigo = caminho;
          await aoTrocarModelo(campo!.campo, ecm);
          if (antigo.toLowerCase() !== String(ecm).toLowerCase()) {
            const s = await removerModeloSemUso(antigo);
            relatorio.linhas.push(s.removidos.length ? `modelo antigo: ${s.removidos.length} arquivo(s) marcados para remoção` : `modelo antigo fica: ${s.motivo ?? "ainda usado"}`);
          }
        }
      } else {
        const d = await importarGlbNoModelo(caminho, modo, glb, { como, validar });
        relatorio = { titulo: validar ? "Conferência (nada gravado)" : "Importado no rascunho", linhas: linhasDe(d), ok: true };
        if (!validar) { mostrado = ""; await carregarFicha(); await tocar(""); }
      }
      if (!validar) avisar("ok", "GLB importado no rascunho.");
    } catch (e) {
      relatorio = { titulo: "Recusado", linhas: [(e as Error).message], ok: false };
    } finally { ocupado = false; }
  }
</script>

{#if comModelo.length}
<section class="caixa visual">
  <h3>{tituloDoCard}</h3>
  {#if comModelo.length > 1}
    <select class="campo" bind:value={escolhido} aria-label="Modelo">
      {#each comModelo as c (c.campo)}<option value={c.campo}>{c.rotulo}</option>{/each}
    </select>
  {/if}
  <canvas bind:this={tela} width="560" height="420"></canvas>
  <p class="mini">{carregando ? "Carregando…" : erroDoGlb || (tocando ? `Tocando: ${traduzir(tocando)} (${tocando})` : "Pose parada")} · {caminho}</p>
  {#if como === "matter"}
    <p class="mini">Como o cliente monta no chão: o .ecm com as peles do .smd; se o .smd não tem pele, a &lt;.smd&gt;.ski (EC_Matter.cpp:71-107). Não pendura em gancho.</p>
  {:else}
    <p class="mini">Pendura no personagem pelo gancho {ganchos.length ? ganchos.join(" ou ") : "— (não pendura)"}; as peles são as do .smd, como o cliente carrega.</p>
  {/if}
  <div class="linha">
    <a class="botao pequeno" href={rotaDoGlb(caminho, undefined, true, { como })} download={arquivoDoGlb}><Icone nome="abrir" tamanho={16} /> Exportar GLB (Blender)</a>
  </div>

  <fieldset class="importar" disabled={!editavel || ocupado}>
    <legend class="mini">Importar GLB{editavel ? "" : " — escolha um rascunho no alto"}</legend>
    <label class="mini"><input type="radio" bind:group={modo} value="modelo" /> Só o modelo — malha e texturas na pele do .smd{como === "matter" ? " (ou na <.smd>.ski)" : ""}; esqueleto, ganchos e animações ficam</label>
    <label class="mini"><input type="radio" bind:group={modo} value="animacoes" /> Só as animações — substitui as de mesmo nome, acrescenta as novas</label>
    <label class="mini"><input type="radio" bind:group={modo} value="tudo" /> Tudo — modelo, texturas, esqueleto e animações (arquivos novos{ganchos.length ? "; o GLB tem de ter o gancho" : ""})</label>
    {#if modo === "tudo"}
      <div class="linha">
        <input class="campo" bind:value={pasta} aria-label="Pasta do modelo novo" />
        <input class="campo" bind:value={titulo} aria-label="Título" placeholder={idDoItem ? `item_${idDoItem}` : "título"} />
      </div>
    {/if}
    <div class="linha">
      <label class="botao pequeno">
        <Icone nome="mais" tamanho={16} /> {arquivo ? arquivo.name : "Escolher GLB…"}
        <input type="file" accept=".glb,model/gltf-binary" hidden onchange={(e) => { arquivo = (e.target as HTMLInputElement).files?.[0] ?? null; relatorio = null; }} />
      </label>
      <button class="botao pequeno" type="button" disabled={!arquivo} onclick={() => executar(true)}>Conferir sem gravar</button>
      <button class="botao pequeno primario" type="button" disabled={!arquivo} onclick={() => executar(false)}>Importar</button>
    </div>
  </fieldset>
  {#if relatorio}
    <div class="relatorio" class:recusado={!relatorio.ok}>
      <strong class="mini">{relatorio.titulo}</strong>
      <ul>{#each relatorio.linhas as l, k (k)}<li class="mini">{l}</li>{/each}</ul>
    </div>
  {/if}
</section>

<section class="caixa">
  <h3>Animações do modelo <span class="mini">{ficha?.bases_do_smd?.length ?? 0}</span></h3>
  {#if ficha?.bases_do_smd?.length}
    <p class="mini">Clique para ver acima (passe o mouse para o nome do cliente). Exportar GLB leva todas com o nome em português e o do
      cliente depois de " · " ("Voando parado · 飞行悬停"); no Blender pode renomear a parte em português — "Só as animações" usa o nome do
      cliente, então a animação volta para a mesma ação.</p>
    <div class="chips">
      <button type="button" class:ativo={!tocando} onclick={() => tocar("")}>■ parado</button>
      {#each ficha.bases_do_smd as b (b)}<button type="button" class:ativo={tocando === b} onclick={() => tocar(b)} title={b}>▶ {traduzir(b)}</button>{/each}
    </div>
  {:else}
    <p class="mini">O modelo não tem animação (o .smd não cita ação). Para criar, anime no Blender e importe "Só as animações".</p>
  {/if}
</section>
{/if}

<style>
  .visual { display: flex; flex-direction: column; gap: 8px; }
  canvas { width: 100%; max-width: 560px; align-self: center; aspect-ratio: 4 / 3; background: #20242a; border-radius: 8px; cursor: grab; }
  .importar { display: flex; flex-direction: column; gap: 4px; border: 1px solid var(--borda); border-radius: 10px; padding: 8px 10px; margin: 0; }
  .importar label.mini { display: flex; gap: 6px; align-items: baseline; }
  .importar .linha input { flex: 1; min-width: 120px; }
  .relatorio { border: 1px solid var(--ouro); border-radius: 10px; padding: 6px 10px; background: var(--fundo-2); }
  .relatorio.recusado { border-color: var(--vermelho); }
  .relatorio ul { margin: 4px 0 0; padding-left: 18px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips button { padding: 4px 8px; border-radius: 999px; border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; font-size: 12px; cursor: pointer; }
  .chips button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
</style>
