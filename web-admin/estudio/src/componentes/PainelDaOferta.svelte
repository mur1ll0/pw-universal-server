<script lang="ts">
  /* Painel da oferta da G-Shop: os campos do `GSHOP_ITEM` do layout da versão (`gshop_editavel::campos_do_formato`).
     Campo que o layout não tem aparece desligado com a explicação. Gravar = `PUT loja/{arquivo}/{i}` só com o que
     mudou; antes, a conferência (`POST …/conferir`, as regras de carga do GS) roda a cada mudança e os erros impedem
     gravar. Unidades: preço em centavos de gold, validade em segundos, datas em time_t GMT (ver `lib/gshop.ts`). */
  import { untrack } from "svelte";
  import type { Registro } from "../lib/api";
  import { partes } from "../lib/dica.svelte";
  import { avisar, estudio } from "../lib/estado.svelte";
  import {
    DIAS_DA_SEMANA, SELOS, bitsParaDias, cabeEmWchar, campoParaData, camposDaVersao, conferirOferta, dataParaCampo,
    diasParaBits, goldEmCentavos, gravarOferta, lerOferta, linhaParaOferta, novaOferta, precoEmGold, removerOferta,
    trocarImagemDaOferta, urlDaImagem, validadeEmTexto, type Conferencia, type Oferta,
  } from "../lib/gshop";
  import CartaoDaOferta from "./CartaoDaOferta.svelte";
  import Icone from "./Icone.svelte";
  import Modal from "./Modal.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";

  let { arquivo, formato, indice, total, linha, categorias, aoMudar }: {
    arquivo: string; formato: string; indice: number; total: number; linha: Oferta;
    categorias: { nome: string; subcategorias: string[] }[];
    aoMudar: (selecionar?: number | null) => Promise<void> | void;
  } = $props();

  type Valor = number | string;
  const v = $derived(camposDaVersao(formato));
  const editavel = $derived(!!estudio.rascunho);
  let original = $state<Record<string, Valor>>({});
  let edit = $state<Record<string, Valor>>({});
  let itemDaOferta = $state<{ nome: string; tabela: string; indice: number } | null>(null);
  let conferencia = $state<Conferencia>({ erros: [], avisos: [] });
  let opcao = $state(0);
  let escolhendo = $state<"item" | "brinde" | null>(null);
  let removendo = $state(false);
  let gravando = $state(false);
  let versaoDaImagem = $state(0);
  let carregando = $state(true);

  async function carregar() {
    carregando = true;
    try {
      const d = await lerOferta(arquivo, indice);
      original = { ...d.campos };
      edit = { ...d.campos };
      itemDaOferta = d.item;
    } catch (e) { avisar("erro", (e as Error).message); }
    carregando = false;
  }
  $effect(() => { indice; arquivo; estudio.rascunho; untrack(carregar); });

  const tem = (k: string) => k in original;
  const num = (k: string) => Number(edit[k] ?? 0);
  const mudados = $derived(Object.fromEntries(Object.entries(edit).filter(([k, x]) => original[k] !== x)) as Record<string, Valor>);
  const nMudados = $derived(Object.keys(mudados).length);

  /* Conferência ao vivo (com espera), contra o elements do rascunho. */
  let relogio: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const c = { ...mudados };
    clearTimeout(relogio);
    relogio = setTimeout(async () => {
      try { conferencia = await conferirOferta(arquivo, indice, c); } catch (e) { conferencia = { erros: [(e as Error).message], avisos: [] }; }
    }, 350);
  });

  /* Erros que o próprio painel acha antes de mandar: texto que não cabe no wchar. */
  const errosLocais = $derived([
    ...(typeof edit.szName === "string" && !cabeEmWchar(edit.szName, 32) ? ["Nome: no máximo 31 caracteres (wchar[32])"] : []),
    ...(typeof edit.desc === "string" && !cabeEmWchar(edit.desc, 512) ? ["Descrição: no máximo 511 caracteres (wchar[512])"] : []),
  ]);
  const podeGravar = $derived(editavel && nMudados > 0 && !errosLocais.length && !gravando);

  async function gravar() {
    gravando = true;
    try {
      const r = await gravarOferta(arquivo, indice, mudados);
      avisar("ok", r.igual_ao_publicado ? "Igual ao publicado: a loja saiu do rascunho." : `Oferta #${indice} gravada no rascunho.`);
      await aoMudar(indice);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); }
    gravando = false;
  }

  async function clonar() {
    try {
      const r = await novaOferta(arquivo, indice, {});
      avisar("ok", `Cópia criada no fim: #${r.resultado.indice} (nenhuma oferta mudou de índice).`);
      await aoMudar(r.resultado.indice);
    } catch (e) { avisar("erro", (e as Error).message); }
  }

  async function remover() {
    removendo = false;
    try {
      const r = await removerOferta(arquivo, indice);
      avisar("ok", r.igual_ao_publicado ? "Igual ao publicado: a loja saiu do rascunho." : `Oferta #${indice} removida no rascunho.`);
      await aoMudar(null);
    } catch (e) { avisar("erro", (e as Error).message); }
  }

  function desativar() {
    for (let k = 0; k < 4; k++) edit[`buy_${k}_price`] = 0;
    removendo = false;
    avisar("aviso", "Preços zerados no formulário: grave para desativar a oferta sem mudar nenhum índice.");
  }

  async function trocarImagem(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    (e.target as HTMLInputElement).value = "";
    if (!f) return;
    try {
      await trocarImagemDaOferta(arquivo, indice, f);
      avisar("ok", "Imagem da oferta gravada no rascunho (DDS novo em surfaces\\百宝阁\\).");
      versaoDaImagem = Date.now();
      await aoMudar(indice);
      await carregar();
    } catch (erro) { avisar("erro", (erro as Error).message); }
  }

  function escolher(r: Registro) {
    if (escolhendo === "item") {
      const nomeAntigo = itemDaOferta?.nome ?? "";
      edit.id = r.id ?? 0;
      if (!edit.szName || edit.szName === nomeAntigo) edit.szName = [...(r.nome ?? "")].slice(0, 31).join("");
      itemDaOferta = { nome: r.nome ?? "", tabela: r.tabela ?? "", indice: r.indice ?? 0 };
    } else if (escolhendo === "brinde") {
      edit.idGift = r.id ?? 0;
      if (!num("iGiftNum")) edit.iGiftNum = 1;
    }
    escolhendo = null;
  }

  /* Prévia: a linha da vitrine com os valores do formulário. */
  const previa = $derived({
    ...linha, nome: String(edit.szName ?? ""), id: num("id"), quantidade: num("num"), preco: num("buy_0_price"),
    selo: v.v126 ? num("status") : num("buy_0_status"), vip: v.vip ? num("buy_0_min_vip_level") : null,
    brinde: v.brinde ? num("idGift") : null,
    opcoes: [0, 1, 2, 3].findIndex((k) => !num(`buy_${k}_price`)) === -1 ? 4 : [0, 1, 2, 3].findIndex((k) => !num(`buy_${k}_price`)),
    item: itemDaOferta?.nome ?? linha.item,
    mostrado: num(`buy_${Math.max(0, [0, 1, 2, 3].findIndex((k) => num(`buy_${k}_price`) > 0))}_price`),
  } satisfies Oferta);

  const subs = $derived(categorias[num("main_type")]?.subcategorias ?? []);
  const ofertasDepois = $derived(total - indice - 1);

  /* Campos de opção. */
  const k = $derived(opcao);
  const chave = (x: string) => `buy_${k}_${x}`;
  function precoTexto(c: string) { return precoEmGold(num(c)); }
  function definirPreco(c: string, t: string) {
    const x = goldEmCentavos(t);
    if (x === null) avisar("aviso", "Preço: número em gold, com até 2 casas (100 = 1 gold no arquivo).");
    else edit[c] = x;
  }
  function validade(c: string) {
    const s = num(c);
    return { dias: Math.floor(s / 86400), horas: Math.floor((s % 86400) / 3600) };
  }
  function definirValidade(c: string, dias: number, horas: number) { edit[c] = Math.max(0, Math.round(dias)) * 86400 + Math.max(0, Math.round(horas)) * 3600; }
  function alternarDia(c: string, d: number) {
    const mensal = num(chave("type")) === 2;
    const atuais = bitsParaDias(num(c), mensal);
    edit[c] = diasParaBits(atuais.includes(d) ? atuais.filter((x) => x !== d) : [...atuais, d]);
  }
  const textoComCor = (t: string) => partes(String(t ?? "").replace(/\\r/g, "").replace(/\\n|\r?\n/g, " ⏎ "));
</script>

{#if carregando && !Object.keys(original).length}
  <p class="suave">Carregando a oferta…</p>
{:else}
  <div class="painel">
    <header class="linha cabeca">
      <h2>Oferta #{indice}</h2>
      <span class="mini">goods_index {indice} · {formato}</span>
      <span class="direita linha">
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={clonar} title="Cópia no fim da lista"><Icone nome="copiar" /> Clonar</button>
        <button class="botao pequeno perigo" type="button" disabled={!editavel} onclick={() => (removendo = true)}><Icone nome="lixeira" /> Remover</button>
      </span>
    </header>

    <!-- Prévia como no jogo -->
    <section class="previa">
      <CartaoDaOferta oferta={previa} grande />
      <div class="imagem">
        {#if edit.icon}
          <img src={urlDaImagem(String(edit.icon), versaoDaImagem)} alt="Imagem da oferta" width="128" height="128" />
        {:else}<span class="vazia">sem imagem</span>{/if}
        <label class="botao pequeno" class:desligado={!editavel}>
          <Icone nome="editar" /> Trocar (PNG 128×128)
          <input type="file" accept="image/png" disabled={!editavel} onchange={trocarImagem} hidden />
        </label>
      </div>
      <p class="mini nota">O cartão da vitrine usa o ícone do <b>item</b> (`DlgQShopItem.cpp:380`); a imagem de 128×128 é a da
        <b>oferta</b> (campo `icon`), que o cliente mostra quando a oferta é escolhida e na janela de compra
        (`DlgQShop.cpp:989`, `DlgQShopBuy.cpp:137`). Caminho: <code>{edit.icon}</code></p>
      {#if edit.desc}<p class="desc">{#each textoComCor(String(edit.desc)) as p}<span style:color="#{p.cor}">{p.texto}</span>{/each}</p>{/if}
    </section>

    {#if conferencia.erros.length || errosLocais.length || conferencia.avisos.length}
      <section class="conferencia">
        {#each [...errosLocais, ...conferencia.erros] as e}<p class="erro">⚠ {e}</p>{/each}
        {#each conferencia.avisos as a}<p class="aviso-t">! {a}</p>{/each}
      </section>
    {/if}

    <!-- Item, aba e texto -->
    <section class="caixa">
      <h3>Item e aba</h3>
      <div class="grade">
        <label>Item <span class="linha">
          <input class="campo curto" type="number" min="1" value={num("id")} disabled={!editavel} onchange={(e) => (edit.id = Number((e.target as HTMLInputElement).value) || 0)} />
          <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = "item")}><Icone nome="busca" /> Escolher</button>
        </span><span class="mini">{itemDaOferta?.nome ?? "não está no elements"}</span></label>
        <label>Quantidade <input class="campo" type="number" min="1" value={num("num")} disabled={!editavel} onchange={(e) => (edit.num = Number((e.target as HTMLInputElement).value) || 0)} /></label>
        <label>Aba
          <select class="campo" value={num("main_type")} disabled={!editavel} onchange={(e) => { edit.main_type = Number((e.target as HTMLSelectElement).value); edit.sub_type = 0; }}>
            {#each categorias as c, i}<option value={i}>{i} · {c.nome}</option>{/each}
          </select></label>
        <label>Subaba
          <select class="campo" value={num("sub_type")} disabled={!editavel || !subs.length} onchange={(e) => (edit.sub_type = Number((e.target as HTMLSelectElement).value))}>
            {#each subs as s, i}<option value={i}>{i} · {s}</option>{/each}
            {#if !subs.length}<option value={num("sub_type")}>(a aba não tem subaba)</option>{/if}
          </select></label>
      </div>
      <label class="largo">Nome na loja (szName, até 31) <input class="campo" maxlength="31" value={String(edit.szName ?? "")} disabled={!editavel} oninput={(e) => (edit.szName = (e.target as HTMLInputElement).value)} /></label>
      {#if tem("desc")}
        <label class="largo">Descrição (desc, até 511; aceita ^RRGGBB de cor) <textarea class="campo area" maxlength="511" disabled={!editavel} oninput={(e) => (edit.desc = (e.target as HTMLTextAreaElement).value)}>{String(edit.desc ?? "")}</textarea></label>
      {/if}
      {#if v.v126 && tem("status")}
        <label>Selo da oferta (status@196)
          <select class="campo" value={num("status")} disabled={!editavel} onchange={(e) => (edit.status = Number((e.target as HTMLSelectElement).value))}>
            {#each SELOS.slice(0, 14) as s}<option value={s.valor}>{s.valor} · {s.nome}</option>{/each}
          </select></label>
      {/if}
    </section>

    <!-- 4 opções de compra -->
    <section class="caixa">
      <h3>Opções de compra</h3>
      <div class="abas-opcao" role="tablist">
        {#each [0, 1, 2, 3] as i}
          <button type="button" role="tab" class="aba-opcao" class:ativa={opcao === i} aria-selected={opcao === i} onclick={() => (opcao = i)}>
            {i + 1}ª {#if num(`buy_${i}_price`)}<span class="mini">{precoTexto(`buy_${i}_price`)}</span>{:else}<span class="mini">—</span>{/if}
          </button>
        {/each}
      </div>
      <p class="mini">O GS vende da 1ª até a primeira opção sem preço (`playermall.cpp:198`). Sem nenhum preço a oferta fica
        desativada: o cliente não a mostra (`EC_Shop.cpp:172-197`).</p>
      <div class="grade">
        <label>Preço (gold; arquivo em centavos)
          <input class="campo" value={precoTexto(chave("price"))} disabled={!editavel} onchange={(e) => definirPreco(chave("price"), (e.target as HTMLInputElement).value)} />
          <span class="mini">{num(chave("price"))} no arquivo</span></label>
        <label>Validade do item comprado
          <span class="linha">
            <input class="campo micro" type="number" min="0" value={validade(chave("time")).dias} disabled={!editavel}
              onchange={(e) => definirValidade(chave("time"), Number((e.target as HTMLInputElement).value), validade(chave("time")).horas)} /> d
            <input class="campo micro" type="number" min="0" max="23" value={validade(chave("time")).horas} disabled={!editavel}
              onchange={(e) => definirValidade(chave("time"), validade(chave("time")).dias, Number((e.target as HTMLInputElement).value))} /> h
          </span>
          <span class="mini">{validadeEmTexto(num(chave("time")))} ({num(chave("time"))} s)</span></label>
        {#if v.seloPorOpcao}
          <label>Selo
            <select class="campo" value={num(chave("status"))} disabled={!editavel} onchange={(e) => (edit[chave("status")] = Number((e.target as HTMLSelectElement).value))}>
              {#each SELOS as s}<option value={s.valor}>{s.valor} · {s.nome}</option>{/each}
            </select></label>
          <label>Grupo (flag)
            <input class="campo" type="number" min="0" value={num(chave("flag"))} disabled={!editavel} onchange={(e) => (edit[chave("flag")] = Number((e.target as HTMLInputElement).value) || 0)} />
            <span class="mini">opções do mesmo grupo não podem se sobrepor</span></label>
        {/if}
        <label>VIP mínimo
          {#if v.vip}
            <select class="campo" value={num(chave("min_vip_level"))} disabled={!editavel} onchange={(e) => (edit[chave("min_vip_level")] = Number((e.target as HTMLSelectElement).value))}>
              {#each [0, 1, 2, 3, 4, 5, 6] as n}<option value={n}>{n === 0 ? "nenhum" : `VIP ${n}`}</option>{/each}
            </select>
          {:else}<input class="campo" disabled value="—" title="Este layout não tem min_vip_level" /><span class="mini">o layout desta versão não tem VIP</span>{/if}
        </label>
      </div>
      {#if v.periodo}
        <div class="grade">
          <label>Período de venda
            <select class="campo" value={num(chave("type"))} disabled={!editavel} onchange={(e) => (edit[chave("type")] = Number((e.target as HTMLSelectElement).value))}>
              <option value={-1}>sempre (-1)</option><option value={0}>entre datas (0)</option>
              <option value={1}>dias da semana (1)</option><option value={2}>dias do mês (2)</option>
            </select></label>
          {#if num(chave("type")) === 0}
            <label>Início (UTC) <input class="campo" type="datetime-local" value={dataParaCampo(num(chave("start_time")))} disabled={!editavel}
              onchange={(e) => (edit[chave("start_time")] = campoParaData((e.target as HTMLInputElement).value))} /></label>
            <label>Fim (UTC) <input class="campo" type="datetime-local" value={dataParaCampo(num(chave("end_time")))} disabled={!editavel}
              onchange={(e) => (edit[chave("end_time")] = campoParaData((e.target as HTMLInputElement).value))} /></label>
          {/if}
        </div>
        {#if num(chave("type")) === 1 || num(chave("type")) === 2}
          {@const mensal = num(chave("type")) === 2}
          <div class="dias">
            {#each Array.from({ length: mensal ? 31 : 7 }, (_, d) => d) as d}
              <label class="dia"><input type="checkbox" checked={bitsParaDias(num(chave("day")), mensal).includes(d)} disabled={!editavel}
                onchange={() => alternarDia(chave("day"), d)} />{mensal ? d + 1 : DIAS_DA_SEMANA[d]}</label>
            {/each}
          </div>
        {/if}
        <p class="mini">Datas em time_t GMT (o GS ajusta ao fuso dele, `playermall.cpp:195`). Opção com validade só pode ter
          período "sempre" e só serve para item que não empilha (`playermall.cpp:252-261`). O nosso GS ainda recusa a compra
          de opção com período ou grupo (`pw-gs`, `comprar_na_loja_gold`).</p>
      {:else}
        <div class="grade">
          <label>Data (end_time, time_t) <input class="campo" type="datetime-local" value={dataParaCampo(num(chave("end_time")))} disabled={!editavel}
            onchange={(e) => (edit[chave("end_time")] = campoParaData((e.target as HTMLInputElement).value))} /></label>
        </div>
        <p class="mini desligado-t">{v.explicacao126}</p>
      {/if}
    </section>

    <!-- Brinde, donos, limite -->
    <section class="caixa">
      <h3>Brinde, NPCs e limite</h3>
      {#if v.brinde}
        <div class="grade">
          <label>Brinde (item) <span class="linha">
            <input class="campo curto" type="number" min="0" value={num("idGift")} disabled={!editavel} onchange={(e) => (edit.idGift = Number((e.target as HTMLInputElement).value) || 0)} />
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = "brinde")}><Icone nome="busca" /></button>
          </span></label>
          <label>Quantidade do brinde <input class="campo" type="number" min="0" value={num("iGiftNum")} disabled={!editavel} onchange={(e) => (edit.iGiftNum = Number((e.target as HTMLInputElement).value) || 0)} /></label>
          <label>Validade do brinde (s) <input class="campo" type="number" min="0" value={num("iGiftTime")} disabled={!editavel} onchange={(e) => (edit.iGiftTime = Number((e.target as HTMLInputElement).value) || 0)} /><span class="mini">{validadeEmTexto(num("iGiftTime"))}</span></label>
          <label>Preço de registro (log) <input class="campo" type="number" min="0" value={num("iLogPrice")} disabled={!editavel} onchange={(e) => (edit.iLogPrice = Number((e.target as HTMLInputElement).value) || 0)} /></label>
        </div>
        <p class="mini">NPCs donos: com algum, a oferta só aparece na loja aberta por esse NPC (`owner_npcs`, `AdjustMainTypeWithOwnerNPC`, `DlgQShop.cpp:577-603`); 0 = qualquer.</p>
        <div class="grade">
          {#each [0, 1, 2, 3, 4, 5, 6, 7] as n}
            <label>NPC {n + 1} <input class="campo" type="number" min="0" value={num(`owner_npcs_${n}`)} disabled={!editavel} onchange={(e) => (edit[`owner_npcs_${n}`] = Number((e.target as HTMLInputElement).value) || 0)} /></label>
          {/each}
        </div>
      {:else}<p class="mini desligado-t">{v.explicacao126}</p>{/if}
      {#if v.limite}
        <div class="grade">
          <label>Limite de compras <input class="campo" type="number" min="0" value={num("buy_times_limit")} disabled={!editavel} onchange={(e) => (edit.buy_times_limit = Number((e.target as HTMLInputElement).value) || 0)} /></label>
          <label>Modo do limite
            <select class="campo" value={num("buy_times_limit_mode")} disabled={!editavel} onchange={(e) => (edit.buy_times_limit_mode = Number((e.target as HTMLSelectElement).value))}>
              <option value={0}>sem limite</option><option value={1}>por dia</option><option value={2}>por semana</option><option value={3}>por mês</option><option value={4}>por ano</option>
            </select><span class="mini">`template/globaldataman.cpp:79`</span></label>
        </div>
      {:else if !v.v126}<p class="mini desligado-t">O layout sem VIP (1.412 B) não tem limite de compras.</p>{/if}
    </section>

    <footer class="linha rodape">
      <span class="mini">{nMudados ? `${nMudados} campo(s) mudados` : "Nada mudou"}{#if !editavel} · escolha um rascunho no alto para editar{/if}</span>
      <button class="botao" type="button" disabled={!nMudados} onclick={() => (edit = { ...original })}>Desfazer</button>
      <button class="botao primario" type="button" disabled={!podeGravar || conferencia.erros.length > 0} onclick={gravar}><Icone nome="check" /> Gravar no rascunho</button>
    </footer>
  </div>
{/if}

{#if escolhendo}
  <SeletorDeEspaco titulo={escolhendo === "item" ? "Item da oferta" : "Item de brinde"} espaco="ESSENCE" aoEscolher={escolher} aoFechar={() => (escolhendo = null)} />
{/if}

{#if removendo}
  <Modal titulo={`Remover a oferta #${indice}?`} aoFechar={() => (removendo = false)}>
    <p>A posição na lista é o <b>goods_index</b> da compra.
      {#if ofertasDepois > 0}Removendo a #{indice}, as {ofertasDepois} ofertas seguintes <b>sobem um índice</b> (#{indice + 1} vira
        #{indice}…).{:else}É a última da lista: nenhuma outra muda de índice.{/if} Cliente e servidor recebem o arquivo novo juntos na
      publicação (item <code>ambos</code>), então continuam casando — mas só depois que o launcher atualizar o cliente.</p>
    <p class="suave">Para tirar a oferta da loja sem mexer em índice nenhum, <b>desative</b>: zere os preços (o cliente não
      mostra oferta sem preço, `EC_Shop.cpp:172-197`).</p>
    {#snippet rodape()}
      <button class="botao" type="button" onclick={desativar}>Desativar (zerar preços)</button>
      <button class="botao perigo" type="button" onclick={remover}><Icone nome="lixeira" /> Remover mesmo</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .painel { display: flex; flex-direction: column; gap: 12px; }
  .cabeca h2 { margin: 0; font-size: 16px; }
  .direita { margin-left: auto; }
  .previa { display: grid; grid-template-columns: 1fr auto; gap: 10px; align-items: start; }
  .imagem { display: flex; flex-direction: column; gap: 6px; align-items: center; }
  .imagem img, .vazia { width: 128px; height: 128px; border-radius: 8px; background: var(--fundo-2); border: 1px solid var(--borda); }
  .vazia { display: grid; place-items: center; color: var(--texto-3); font-size: 12px; }
  .nota { grid-column: 1 / -1; margin: 0; }
  .nota code { word-break: break-all; }
  .desc { grid-column: 1 / -1; margin: 0; padding: 8px 10px; background: #0d1116; border-radius: 8px; font-size: 13px; color: #fff; }
  .conferencia { border-radius: var(--raio); padding: 8px 12px; background: var(--superficie); border: 1px solid var(--borda); }
  .conferencia p { margin: 2px 0; font-size: 13px; }
  .aviso-t { color: var(--ouro); }
  .grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 10px 12px; margin: 8px 0; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 13px; color: var(--texto-2); }
  .largo { margin: 8px 0; }
  .curto { width: 110px; }
  .micro { width: 70px; }
  .area { height: 70px; padding: 6px 10px; resize: vertical; }
  .abas-opcao { display: flex; gap: 6px; flex-wrap: wrap; }
  .aba-opcao { padding: 6px 12px; border-radius: 8px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; }
  .aba-opcao.ativa { border-color: var(--ouro); color: var(--ouro); }
  .dias { display: flex; flex-wrap: wrap; gap: 6px; }
  .dia { flex-direction: row; align-items: center; gap: 3px; padding: 2px 6px; border: 1px solid var(--borda); border-radius: 6px; }
  .desligado-t { opacity: .8; border-left: 3px solid var(--borda-forte); padding-left: 8px; }
  .desligado { opacity: .5; pointer-events: none; }
  .rodape { position: sticky; bottom: 0; background: var(--fundo); padding: 8px 0; justify-content: flex-end; }
  .rodape .mini { margin-right: auto; }
</style>
