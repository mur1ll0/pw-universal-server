<script lang="ts">
  /* B383: armadura e moda vestidas no corpo. A peça não tem modelo: o cliente monta
     `Models\Players\装备\{男|女}\<realname>\<grupo><realname>{一级,二级,三级}.ski` (`ChangeArmor`, tabela do binário 1.5.5 BR
     em 0xdd83f0 — `Estudio::GRUPOS_DE_PELE`) e liga a pele ao esqueleto do corpo da classe
     (`Models\Players\形象\<classe>\躯干\<classe>.ecm`). A prévia veste a peça (nível 一级) no esqueleto do corpo escolhido,
     com o corpo e a cabeça da classe se pedido. Exportar = a peça no esqueleto do corpo (o Blender recebe o esqueleto
     certo); importar = os três `.ski` do grupo (a mesma malha nos três níveis), com os ossos conferidos em todas as
     classes do grupo e os pesos corrigidos como o motor lê. */
  import { onDestroy } from "svelte";
  import { importarPeleDoEquipamento, peleDoEquipamento, rotaDoGlb, type PeleDoEquipamento } from "../lib/api";
  import { caminhosDaPele, NOME_DO_GRUPO, pelesDoCorpo } from "../lib/equipavel3d";
  import { classesDaVersao } from "../lib/catalogo";
  import { desenharGlb } from "../lib/glb";
  import { avisar, estudio } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";

  let { realname, generoFixo = null, local = 0 }: { realname: string; generoFixo?: number | null; local?: number } = $props();

  let genero = $state(0);
  $effect(() => { if (generoFixo != null) genero = generoFixo; });
  let info = $state<PeleDoEquipamento | null>(null);
  let erro = $state("");
  let grupo = $state("");
  let classe = $state<number | null>(null);
  let comCorpo = $state(true);
  let tela = $state<HTMLCanvasElement | null>(null);
  let erroDoGlb = $state("");
  let carregando = $state(false);
  let parar: (() => void) | null = null;
  let ocupado = $state(false);
  let arquivo: File | null = $state(null);
  let relatorio = $state<{ titulo: string; linhas: string[]; ok: boolean } | null>(null);
  const editavel = $derived(!!estudio.rascunho);
  const classes = $derived(classesDaVersao(estudio.versao));

  async function carregarInfo() {
    erro = ""; info = null;
    if (!realname.trim()) { erro = "Sem realname: a peça não tem pele."; return; }
    try {
      info = await peleDoEquipamento(realname.trim(), genero);
      const g = info.grupos.find((x) => x.arquivos.every((a) => a.existe)) ?? info.grupos[0];
      grupo = g?.grupo ?? "";
      classe = null;
    } catch (e) { erro = (e as Error).message; }
  }
  $effect(() => { void genero; void estudio.rascunho; void realname; carregarInfo(); });

  const doGrupo = $derived(info?.grupos.find((g) => g.grupo === grupo) ?? null);
  /** Só as classes que o cliente desta versão tem. */
  const corpos = $derived((doGrupo?.corpos ?? []).filter((c) => c.classe < classes.length));
  const corpo = $derived(corpos.find((c) => c.classe === classe) ?? corpos[0] ?? null);
  const peca = $derived(grupo ? caminhosDaPele(realname.trim(), genero, grupo) : []);
  const existe = $derived(doGrupo ? doGrupo.arquivos.map((a) => a.existe) : []);

  function rotaDaPrevia(comOCorpo: boolean) {
    if (!corpo || !peca.length) return "";
    const peles = [peca[0], ...(comOCorpo ? pelesDoCorpo(corpo.nome) : [])].join("|");
    return rotaDoGlb(corpo.ecm, undefined, false, { peles });
  }
  async function desenhar() {
    parar?.(); parar = null; erroDoGlb = ""; carregando = true;
    try {
      const url = rotaDaPrevia(comCorpo);
      if (!url) return;
      const r = await fetch(url, { credentials: "same-origin", cache: "no-store" });
      if (!r.ok) throw new Error(`GLB: HTTP ${r.status}${r.status === 400 ? " — " + ((await r.json().catch(() => ({}))).detail || "") : ""}`);
      if (tela) parar = desenharGlb(tela, await r.arrayBuffer());
    } catch (e) { erroDoGlb = (e as Error).message; }
    finally { carregando = false; }
  }
  $effect(() => { void corpo; void comCorpo; void peca; if (tela && corpo) desenhar(); });
  onDestroy(() => parar?.());

  const linhasDe = (d: any): string[] => {
    const x = d?.resultado ?? {};
    const l = [`${x.malhas ?? "?"} malha(s), ${x.vertices ?? "?"} vértices, ${x.ossos ?? "?"} osso(s); esqueleto de ${x.esqueleto ?? "?"}; ossos conferidos em ${x.corpos_conferidos ?? "?"} classe(s) do grupo`];
    for (const a of x.arquivos ?? []) l.push(`pele: ${a}`);
    if (x.niveis) l.push(x.niveis);
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
      const d = await importarPeleDoEquipamento(realname.trim(), genero, grupo, corpo?.classe ?? null, validar, await arquivo.arrayBuffer());
      relatorio = { titulo: validar ? "Conferência (nada gravado)" : "Importado no rascunho", linhas: linhasDe(d), ok: true };
      if (!validar) { avisar("ok", "Pele importada no rascunho (os três níveis)."); await carregarInfo(); await desenhar(); }
    } catch (e) { relatorio = { titulo: "Recusado", linhas: [(e as Error).message], ok: false }; }
    finally { ocupado = false; }
  }
</script>

<section class="caixa visual">
  <h3>Vestido no personagem</h3>
  {#if erro}<p class="erro mini">{erro}</p>{/if}
  <div class="linha">
    {#if generoFixo == null}
      <select class="campo" bind:value={genero} aria-label="Gênero"><option value={0}>Masculino</option><option value={1}>Feminino</option></select>
    {:else}<span class="mini">{generoFixo === 0 ? "Masculino" : "Feminino"} (gênero da peça)</span>{/if}
    <select class="campo" bind:value={grupo} aria-label="Grupo de pele">
      {#each info?.grupos ?? [] as g (g.grupo)}<option value={g.grupo}>{NOME_DO_GRUPO[g.grupo] ?? g.grupo}{g.arquivos.every((a) => a.existe) ? "" : " — sem pele"}</option>{/each}
    </select>
    <select class="campo" value={corpo?.classe ?? ""} onchange={(e) => (classe = Number((e.target as HTMLSelectElement).value))} aria-label="Corpo">
      {#each corpos as c (c.classe)}<option value={c.classe}>{classes[c.classe] ?? c.nome} · {c.nome}</option>{/each}
    </select>
    <label class="mini"><input type="checkbox" bind:checked={comCorpo} /> com o corpo</label>
  </div>
  <canvas bind:this={tela} width="560" height="420"></canvas>
  <p class="mini">{carregando ? "Carregando…" : erroDoGlb || "Arraste para girar, roda para aproximar."} · local {local} · {peca[0] ?? ""}</p>
  {#if existe.length && !existe.every(Boolean)}
    <p class="erro mini">Faltam níveis desta pele ({peca.filter((_, k) => !existe[k]).map((c) => c.split("\\").pop()).join(", ")}): o cliente não veste a peça e usa a roupa padrão.</p>
  {/if}
  {#if info?.usado_por?.length && info.usado_por.length > 1}
    <p class="mini aviso">Esta pele (realname "{realname}") é usada por {info.usado_por.length} registros — trocar a pele muda todos: {info.usado_por.slice(0, 6).map((u) => `${u.nome} (${u.id})`).join(", ")}{info.usado_por.length > 6 ? "…" : ""}. Para mudar só este, troque o realname antes.</p>
  {/if}
  <div class="linha">
    <a class="botao pequeno" class:desligado={!corpo} href={rotaDaPrevia(false)} download={`${realname}.glb`}><Icone nome="abrir" tamanho={16} /> Exportar GLB (Blender)</a>
  </div>
  <p class="mini">Vai a peça com o esqueleto do corpo escolhido. No Blender, mantenha os nomes dos ossos e a pose de repouso; até 4 ossos por vértice.</p>
  <fieldset class="importar" disabled={!editavel || ocupado || !grupo}>
    <legend class="mini">Importar GLB como {NOME_DO_GRUPO[grupo] ?? grupo}{editavel ? "" : " — escolha um rascunho no alto"}</legend>
    <div class="linha">
      <label class="botao pequeno">
        <Icone nome="mais" tamanho={16} /> {arquivo ? arquivo.name : "Escolher GLB…"}
        <input type="file" accept=".glb,model/gltf-binary" hidden onchange={(e) => { arquivo = (e.target as HTMLInputElement).files?.[0] ?? null; relatorio = null; }} />
      </label>
      <button class="botao pequeno" type="button" disabled={!arquivo} onclick={() => executar(true)}>Conferir sem gravar</button>
      <button class="botao pequeno primario" type="button" disabled={!arquivo} onclick={() => executar(false)}>Importar</button>
    </div>
    <p class="mini">Grava os três níveis (一级, 二级, 三级) com a mesma malha, em Models\Players\装备\{genero === 0 ? "男" : "女"}\{realname}\.</p>
  </fieldset>
  {#if relatorio}
    <div class="relatorio" class:recusado={!relatorio.ok}>
      <strong class="mini">{relatorio.titulo}</strong>
      <ul>{#each relatorio.linhas as l, k (k)}<li class="mini">{l}</li>{/each}</ul>
    </div>
  {/if}
</section>

<style>
  .visual { display: flex; flex-direction: column; gap: 8px; }
  canvas { width: 100%; max-width: 560px; align-self: center; aspect-ratio: 4 / 3; background: #20242a; border-radius: 8px; cursor: grab; }
  .linha select { min-width: 0; flex: 1; }
  .importar { display: flex; flex-direction: column; gap: 4px; border: 1px solid var(--borda); border-radius: 10px; padding: 8px 10px; margin: 0; }
  .relatorio { border: 1px solid var(--ouro); border-radius: 10px; padding: 6px 10px; background: var(--fundo-2); }
  .relatorio.recusado { border-color: var(--vermelho); }
  .relatorio ul { margin: 4px 0 0; padding-left: 18px; }
  .aviso { color: var(--ouro); }
  .desligado { opacity: .45; pointer-events: none; }
</style>
