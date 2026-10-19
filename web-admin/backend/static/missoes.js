/* Estúdio, fase S7 parte 2 (B255): navegador de missões (`tasks.data` do realm, só leitura). Os campos são os do
   `TaskTemplate` do leitor do GS (`pw-data-loader/src/tasks.rs`); NPCs, itens e monstros citados vêm com o nome do
   elements e link para a ficha. Usa `api`, `criar`, `elemento` do painel.js e `estudio`, `mostrarVistaDoEstudio`,
   `abrirRegistroDoEstudio`. */

const missoes = { pagina: 0, por_pagina: 100, total: 0, aberta: null };

function rotaDeMissoes(resto, q = {}) {
  if (estudio.rascunho) q = { ...q, rascunho: estudio.rascunho };
  const p = new URLSearchParams(q).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/missoes${resto}${p ? `?${p}` : ""}`;
}

async function carregarMissoes() {
  if (!estudio.realm) return;
  const busca = elemento("missoes-busca").value.trim();
  elemento("missoes-situacao").textContent = "Carregando (a primeira vez lê o tasks.data inteiro)…";
  let r;
  try {
    r = await api(rotaDeMissoes("", { busca, topo: elemento("missoes-topo").checked ? "true" : "false", pagina: missoes.pagina, por_pagina: missoes.por_pagina }));
  } catch (erro) { elemento("missoes-situacao").textContent = erro.message; return; }
  missoes.total = r.total;
  elemento("missoes-situacao").textContent = `${r.missoes} missões (${r.de_topo} de topo) · versão ${r.versao_do_arquivo} · ${r.total} na lista · página ${r.pagina + 1} de ${Math.max(1, Math.ceil(r.total / missoes.por_pagina))}`;
  const area = elemento("missoes-lista");
  area.replaceChildren();
  for (const [id, nome, tipo, nmin, nmax, mae, filhas] of r.linhas) {
    const b = criar("button", "", `${id} · ${nome || "(sem nome)"} · nível ${nmin}–${nmax}${filhas ? ` · ${filhas} filhas` : ""}${mae ? ` · mãe ${mae}` : ""}`);
    b.type = "button";
    b.setAttribute("aria-current", missoes.aberta === id ? "true" : "false");
    b.addEventListener("click", () => abrirMissao(id));
    area.appendChild(b);
  }
  if (!r.linhas.length) area.appendChild(criar("div", "texto-suave", "Nenhuma missão."));
}

function linkDeMissao(m) {
  const a = criar("a", "", `${m.id} ${m.nome || "(não existe)"}`);
  a.href = "#";
  a.addEventListener("click", (e) => { e.preventDefault(); abrirMissao(m.id); });
  return a;
}

function valorComNome(v, nomes) {
  const n = nomes[String(v)];
  if (!n) return criar("span", "", String(v));
  const a = criar("a", "", `${v} (${n.nome})`);
  a.href = "#";
  a.title = `${n.tabela} #${n.indice}`;
  a.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio(n.tabela, n.indice); });
  return a;
}

/* Desenha um valor da ficha: objetos como lista de chave/valor, listas de objetos como tabela, ids com nome. */
function desenharValor(v, chave, nomes) {
  if (v === null || v === undefined) return criar("span", "texto-suave", "—");
  if (Array.isArray(v)) {
    if (!v.length) return criar("span", "texto-suave", "(vazio)");
    if (typeof v[0] === "object" && v[0] !== null && !Array.isArray(v[0])) {
      const t = criar("table", "tabela-simples");
      const cab = criar("tr");
      Object.keys(v[0]).forEach((k) => cab.appendChild(criar("th", "", k)));
      t.appendChild(cab);
      for (const linha of v) {
        const tr = criar("tr");
        Object.entries(linha).forEach(([k, x]) => { const td = criar("td"); td.appendChild(desenharValor(x, k, nomes)); tr.appendChild(td); });
        t.appendChild(tr);
      }
      return t;
    }
    const s = criar("span");
    v.forEach((x, i) => { if (i) s.append(", "); s.appendChild(desenharValor(x, chave, nomes)); });
    return s;
  }
  if (typeof v === "object") {
    const d = criar("div", "missao-objeto");
    for (const [k, x] of Object.entries(v)) {
      const linha = criar("div");
      linha.append(criar("strong", "", `${k}: `), desenharValor(x, k, nomes));
      d.appendChild(linha);
    }
    return d;
  }
  const k = (chave || "").toLowerCase();
  if (typeof v === "number" && v > 0 && (k === "id" || /npc|item|monstro|brinde/.test(k)) && !/quantidade/.test(k)) return valorComNome(v, nomes);
  return criar("span", "", typeof v === "boolean" ? (v ? "sim" : "não") : String(v));
}

async function abrirMissao(id) {
  missoes.aberta = id;
  const area = elemento("missoes-ficha");
  area.textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeMissoes(`/${id}`)); } catch (erro) { area.textContent = erro.message; return; }
  area.replaceChildren();
  const f = r.ficha;
  area.appendChild(criar("h3", "", `${f.id} · ${f.name || "(sem nome)"}`));
  const rel = criar("div", "texto-suave");
  if (r.mae) { rel.append("Mãe: "); rel.appendChild(linkDeMissao(r.mae)); rel.append(" · "); }
  if (r.filhas.length) { rel.append("Filhas: "); r.filhas.forEach((m, i) => { if (i) rel.append(", "); rel.appendChild(linkDeMissao(m)); }); rel.append(" · "); }
  if (r.pre_missoes.length) { rel.append("Pré-missões: "); r.pre_missoes.forEach((m, i) => { if (i) rel.append(", "); rel.appendChild(linkDeMissao(m)); }); rel.append(" · "); }
  if (r.nova_missao) { rel.append("Ao concluir, dá: "); rel.appendChild(linkDeMissao(r.nova_missao)); }
  area.appendChild(rel);
  if (f.descricao) area.appendChild(criar("p", "", f.descricao));
  const ordem = [
    ["Quem dá e quem premia", ["npc_que_entrega", "npc_que_premia", "entrega_automatica", "tipo", "metodo", "tipo_de_conclusao"]],
    ["Requisitos", ["min_level", "max_level", "req_classes", "genero", "pre_tasks", "missoes_exclusivas", "itens_exigidos", "dinheiro_pedido", "frequencia", "limite_por_periodo", "limite_de_tempo", "espera", "max_receptores"]],
    ["Objetivo", ["monster_kills", "item_collections", "nivel_a_alcancar", "mundo_a_alcancar"]],
    ["Ao receber", ["itens_entregues"]],
    ["Prêmio", ["rewards"]],
    ["Prêmio de falha", ["premio_de_falha"]],
  ];
  const usados = new Set(["id", "name", "descricao", "parent", "sub_tasks"]);
  for (const [titulo, chaves] of ordem) {
    const fs = criar("fieldset");
    fs.appendChild(criar("legend", "", titulo));
    for (const k of chaves) {
      if (!(k in f)) continue;
      usados.add(k);
      const linha = criar("div");
      linha.append(criar("strong", "", `${k}: `), desenharValor(f[k], k, r.nomes));
      fs.appendChild(linha);
    }
    area.appendChild(fs);
  }
  const resto = criar("details");
  resto.appendChild(criar("summary", "", "Demais campos"));
  for (const [k, v] of Object.entries(f)) {
    if (usados.has(k)) continue;
    const linha = criar("div");
    linha.append(criar("strong", "", `${k}: `), desenharValor(v, k, r.nomes));
    resto.appendChild(linha);
  }
  area.appendChild(resto);
  // Clonar (B267): só missão de topo sem submissões; a nova só aparece em jogo depois de entrar no serviço de um NPC.
  if (estudio.rascunho && f.parent == null && !(f.sub_tasks || []).length) {
    const linha = criar("div", "recursos-linha");
    const novo = criar("input", "campo");
    novo.type = "number"; novo.min = "1"; novo.max = "65535"; novo.placeholder = "id novo (vazio = maior + 1)"; novo.style.maxWidth = "200px";
    const b = criar("button", "botao botao-pequeno botao-secundario", "Clonar missão");
    b.type = "button";
    b.addEventListener("click", async () => {
      try {
        const corpo = novo.value ? { novo: Math.trunc(Number(novo.value)) } : {};
        const res = await api(rotaDeMissoes(`/${f.id}/clonar`), { method: "POST", body: JSON.stringify(corpo) });
        const id = res.resultado?.id;
        alerta("ok", "Estúdio", `Missão ${id} criada no rascunho. Para ser oferecida, inclua-a no serviço de missões de um NPC (Tabelas → NPC_TASK_OUT_SERVICE).`);
        if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
        if (id) await abrirMissao(id);
      } catch (erro) { alerta("erro", "Estúdio", erro.message); }
    });
    linha.append(novo, b);
    area.appendChild(linha);
  }
  area.appendChild(formularioDaMissao(f, r.editaveis || [], r.textos || {}, r.listas || [], r.vetores || [], r.grupos_do_premio, r.dialogos || []));
  document.querySelectorAll("#missoes-lista button").forEach((b) => b.setAttribute("aria-current", b.textContent.startsWith(`${id} `) ? "true" : "false"));
}

/* Edição no lugar (B256): só os campos de tamanho fixo (`tasks_editavel`); o resto da ficha é só leitura. */
const ROTULOS_DA_MISSAO = {
  name: "Nome", min_level: "Nível mínimo", max_level: "Nível máximo", npc_que_entrega: "NPC que entrega",
  npc_que_premia: "NPC que premia", limite_de_tempo: "Prazo (s)", frequencia: "Frequência", espera: "Espera (s)",
  pode_desistir: "Pode desistir", pode_repetir: "Pode repetir", entrega_automatica: "Entrega automática",
  money: "Prêmio: dinheiro", exp: "Prêmio: experiência", sp: "Prêmio: SP", reputation: "Prêmio: reputação", nova_missao: "Prêmio: missão seguinte",
};

function valorDaFicha(f, nome) {
  if (["money", "exp", "sp", "reputation", "nova_missao"].includes(nome)) return f.rewards?.[nome];
  return f[nome];
}

const ROTULOS_DOS_TEXTOS = { descricao: "Descrição", ok: "Texto ao aceitar", nao: "Texto ao recusar", tributo: "Tributo" };

const ROTULOS_DAS_LISTAS = { pre_tasks: "Pré-missões (ids)", missoes_exclusivas: "Missões exclusivas (ids)", req_classes: "Classes exigidas (ids)" };

/* Vetores variáveis (B264): uma linha por registro com os campos principais; os outros campos do registro ficam como
   estão (ou com o padrão, numa linha nova). Teto do cliente: 10 itens, 3 monstros (TaskInterface.h). */
const VETORES_DA_MISSAO = {
  itens_exigidos: { rotulo: "Itens exigidos", campos: [["id", "Item"], ["quantidade", "Qtd."]], novo: { id: 0, comum: true, quantidade: 1, probabilidade: 1, validade: 0 } },
  itens_entregues: { rotulo: "Itens entregues ao aceitar", campos: [["id", "Item"], ["quantidade", "Qtd."]], novo: { id: 0, comum: true, quantidade: 1, probabilidade: 1, validade: 0 } },
  monster_kills: { rotulo: "Monstros a matar", campos: [["monstro", "Monstro"], ["quantidade", "Mortes"], ["item_que_cai", "Item que cai"], ["quantidade_do_item", "Qtd. do item"]],
    novo: { monstro: 0, quantidade: 1, item_que_cai: 0, quantidade_do_item: 0, item_comum: false, chance_do_item: 0, nivel_do_matador: false, dps: 0, dph: 0 } },
};

function editorDeVetor(nome, teto, registros) {
  const def = VETORES_DA_MISSAO[nome];
  const caixa = criar("div", "missao-objeto");
  const linhas = registros.map((x) => ({ ...x }));
  const desenhar = () => {
    caixa.replaceChildren(criar("strong", "", `${def ? def.rotulo : nome} (até ${teto})`));
    linhas.forEach((x, k) => {
      const l = criar("div", "recursos-linha");
      for (const [campo, rot] of def.campos) {
        const i = criar("input", "campo");
        i.type = "number"; i.min = "0"; i.step = "1"; i.value = x[campo] ?? 0; i.style.maxWidth = "110px";
        i.disabled = !estudio.rascunho;
        i.addEventListener("input", () => { x[campo] = Math.trunc(Number(i.value)); });
        l.append(` ${rot} `, i);
      }
      if (estudio.rascunho) {
        const b = criar("button", "botao botao-pequeno botao-secundario", "Remover");
        b.type = "button";
        b.addEventListener("click", () => { linhas.splice(k, 1); desenhar(); });
        l.appendChild(b);
      }
      caixa.appendChild(l);
    });
    if (estudio.rascunho && linhas.length < teto) {
      const b = criar("button", "botao botao-pequeno botao-secundario", "+ Linha");
      b.type = "button";
      b.addEventListener("click", () => { linhas.push({ ...def.novo }); desenhar(); });
      caixa.appendChild(b);
    }
  };
  desenhar();
  return { caixa, valor: () => linhas };
}

/* Grupos de itens de prêmio (B265): cada grupo dá todos os itens ou sorteia um pelo peso. Tetos: 12 grupos, 64 itens. */
function editorDeGrupos(rotulo, grupos, tetos) {
  const caixa = criar("div", "missao-objeto");
  const gs = grupos.map((g) => ({ sorteia_um: g.sorteia_um, itens: g.itens.map((i) => ({ ...i })) }));
  const numero = (x, campo, passo) => {
    const i = criar("input", "campo");
    i.type = "number"; i.min = "0"; i.step = passo; i.value = x[campo] ?? 0; i.style.maxWidth = "100px";
    i.disabled = !estudio.rascunho;
    i.addEventListener("input", () => { x[campo] = passo === "1" ? Math.trunc(Number(i.value)) : Number(i.value); });
    return i;
  };
  const botao = (texto, acao) => { const b = criar("button", "botao botao-pequeno botao-secundario", texto); b.type = "button"; b.addEventListener("click", acao); return b; };
  const desenhar = () => {
    caixa.replaceChildren(criar("strong", "", `${rotulo} (até ${tetos.grupos} grupos, ${tetos.itens} itens cada)`));
    gs.forEach((g, gi) => {
      const bloco = criar("div", "recursos-linha");
      const marca = criar("input"); marca.type = "checkbox"; marca.checked = g.sorteia_um; marca.disabled = !estudio.rascunho;
      marca.addEventListener("change", () => { g.sorteia_um = marca.checked; });
      const l = criar("label", "recursos-marca"); l.append(marca, ` Grupo ${gi + 1}: sorteia um`);
      bloco.appendChild(l);
      if (estudio.rascunho) bloco.appendChild(botao("Remover grupo", () => { gs.splice(gi, 1); desenhar(); }));
      caixa.appendChild(bloco);
      g.itens.forEach((it, ii) => {
        const linha = criar("div", "recursos-linha");
        linha.append(" Item ", numero(it, "id", "1"), " Qtd. ", numero(it, "quantidade", "1"), " Peso ", numero(it, "probabilidade", "any"));
        if (estudio.rascunho) linha.appendChild(botao("Remover", () => { g.itens.splice(ii, 1); desenhar(); }));
        caixa.appendChild(linha);
      });
      if (estudio.rascunho && g.itens.length < tetos.itens) caixa.appendChild(botao("+ Item", () => { g.itens.push({ id: 0, comum: true, quantidade: 1, probabilidade: 1, validade: 0 }); desenhar(); }));
    });
    if (estudio.rascunho && gs.length < tetos.grupos) caixa.appendChild(botao("+ Grupo", () => { gs.push({ sorteia_um: false, itens: [] }); desenhar(); }));
  };
  desenhar();
  return { caixa, valor: () => gs };
}

/* Diálogos (B266): título e opções são text[64] (até 63 caracteres); o texto da janela tem tamanho livre. */
const ROTULOS_DOS_DIALOGOS = { entrega: "Ao oferecer", nao_qualificado: "Sem requisitos", entrega_de_item: "Ao entregar item", execucao: "Durante", premio: "Ao premiar" };

function editorDeDialogos(dialogos) {
  const caixa = criar("div");
  const mudancas = [];
  const campo = (valor, longo, onde) => {
    const i = criar(longo ? "textarea" : "input", "campo");
    i.value = valor;
    if (longo) { i.rows = 2; i.style.width = "100%"; } else { i.maxLength = 63; i.style.flex = "1"; }
    i.disabled = !estudio.rascunho;
    mudancas.push({ i, valor, onde });
    return i;
  };
  dialogos.forEach((d, di) => {
    if (!d.texto && !d.janelas.length) return;
    const det = criar("details");
    det.appendChild(criar("summary", "", `Diálogo: ${ROTULOS_DOS_DIALOGOS[d.nome] || d.nome} (${d.janelas.length} janelas)`));
    const t = criar("div", "recursos-linha");
    t.append("Título ", campo(d.texto, false, { dialogo: di }));
    det.appendChild(t);
    d.janelas.forEach((j, ji) => {
      const jb = criar("div", "missao-objeto");
      jb.appendChild(criar("div", "texto-suave", `Janela ${j.id}${j.pai !== 0xffffffff ? ` (de ${j.pai})` : ""}`));
      jb.appendChild(campo(j.texto, true, { dialogo: di, janela: ji }));
      j.opcoes.forEach((o, oi) => {
        const l = criar("div", "recursos-linha");
        l.append(`Opção → ${o.id} `, campo(o.texto, false, { dialogo: di, janela: ji, opcao: oi }));
        jb.appendChild(l);
      });
      det.appendChild(jb);
    });
    caixa.appendChild(det);
  });
  return { caixa, valor: () => mudancas.filter((m) => m.i.value !== m.valor).map((m) => ({ ...m.onde, texto: m.i.value })) };
}

function formularioDaMissao(f, editaveis, textos, listas, vetores, tetosDosGrupos, dialogos) {
  const fs = criar("fieldset");
  fs.appendChild(criar("legend", "", estudio.rascunho ? `Editar (rascunho #${estudio.rascunho})` : "Editar (escolha ou crie um rascunho no topo do Estúdio)"));
  const g = criar("div", "mapa-campos");
  const entradas = {};
  for (const [nome, tipo] of editaveis) {
    const label = criar("label", tipo === "bool" ? "marca" : "");
    const i = criar("input", tipo === "bool" ? "" : "campo");
    const atual = valorDaFicha(f, nome);
    if (tipo === "bool") { i.type = "checkbox"; i.checked = !!atual; label.append(i, criar("span", "", ROTULOS_DA_MISSAO[nome] || nome)); }
    else {
      if (tipo === "nome") { i.maxLength = 29; i.value = atual || ""; } else { i.type = "number"; i.step = "1"; if (tipo === "u32") i.min = "0"; i.value = atual ?? 0; }
      label.append(criar("span", "", ROTULOS_DA_MISSAO[nome] || nome), i);
    }
    i.disabled = !estudio.rascunho;
    label.title = nome;
    g.appendChild(label);
    entradas[nome] = { i, tipo, atual };
  }
  fs.appendChild(g);
  // Listas de capacidade fixa (B263): ids separados por vírgula; no máximo a capacidade do arquivo.
  for (const [nome, capacidade] of listas) {
    const label = criar("label", "");
    const i = criar("input", "campo");
    const atual = (f[nome] || []).join(", ");
    i.value = atual;
    i.disabled = !estudio.rascunho;
    i.placeholder = `até ${capacidade}`;
    label.append(criar("span", "", `${ROTULOS_DAS_LISTAS[nome] || nome} — até ${capacidade}`), i);
    fs.appendChild(label);
    entradas[nome] = { i, tipo: "lista", atual };
  }
  for (const [nome, teto] of vetores) {
    if (!VETORES_DA_MISSAO[nome]) continue;
    const e = editorDeVetor(nome, teto, f[nome] || []);
    fs.appendChild(e.caixa);
    entradas[nome] = { tipo: "vetor", atual: JSON.stringify(f[nome] || []), valor: e.valor };
  }
  if (tetosDosGrupos) {
    for (const [nome, rotulo, grupos] of [["premio_grupos", "Prêmio em itens", f.rewards?.grupos_de_itens], ["falha_grupos", "Prêmio de falha em itens", f.premio_de_falha?.grupos_de_itens]]) {
      const e = editorDeGrupos(rotulo, grupos || [], tetosDosGrupos);
      fs.appendChild(e.caixa);
      entradas[nome] = { tipo: "vetor", atual: JSON.stringify(grupos || []), valor: e.valor };
    }
  }
  if (dialogos.length) {
    const e = editorDeDialogos(dialogos);
    fs.appendChild(e.caixa);
    entradas.dialogos = { tipo: "dialogos", valor: e.valor };
  }
  // Textos longos (B262): mudam o tamanho da missão; o servidor corrige a tabela de deslocamentos do tasks.data.
  for (const [nome, atual] of Object.entries(textos)) {
    const label = criar("label", "");
    const i = criar("textarea", "campo");
    i.rows = nome === "descricao" ? 4 : 2;
    i.value = atual;
    i.disabled = !estudio.rascunho;
    i.style.width = "100%";
    label.append(criar("span", "", ROTULOS_DOS_TEXTOS[nome] || nome), i);
    fs.appendChild(label);
    entradas[nome] = { i, tipo: "texto", atual };
  }
  fs.appendChild(criar("div", "texto-suave", "Códigos de cor (^ffcb4a) ficam no texto como estão. Os textos mudam o tamanho do tasks.data (a tabela de deslocamentos é corrigida no servidor); o resto é de tamanho fixo. Mudar o NPC que entrega não muda o serviço do NPC no elements (NPC_TASK_OUT_SERVICE): ajuste a ficha dele junto."));
  if (estudio.rascunho) {
    const b = criar("button", "botao botao-pequeno botao-primario", "Salvar no rascunho");
    b.type = "button";
    b.addEventListener("click", async () => {
      const campos = {};
      for (const [nome, { i, tipo, atual }] of Object.entries(entradas)) {
        if (tipo === "dialogos") {
          const v = entradas[nome].valor();
          if (v.length) campos[nome] = v;
          continue;
        }
        if (tipo === "vetor") {
          const v = entradas[nome].valor();
          if (JSON.stringify(v) !== atual) campos[nome] = v;
          continue;
        }
        if (tipo === "lista") {
          const texto = i.value.split(/[\s,;]+/).filter(Boolean).join(", ");
          if (texto !== atual) campos[nome] = texto ? texto.split(", ").map((x) => Math.trunc(Number(x))) : [];
          continue;
        }
        const v = tipo === "bool" ? i.checked : (tipo === "nome" || tipo === "texto") ? i.value : Math.trunc(Number(i.value));
        if (v !== atual) campos[nome] = v;
      }
      if (!Object.keys(campos).length) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
      try {
        const r = await api(rotaDeMissoes(`/${f.id}`), { method: "PUT", body: JSON.stringify({ campos }) });
        alerta("ok", "Estúdio", "Missão salva no rascunho (cliente e realm, o mesmo tasks.data)." + (r.igual_ao_publicado ? " Ficou igual ao publicado: o item saiu do rascunho." : ""), { toast: true });
        await abrirMissao(f.id);
        if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
      } catch (erro) { alerta("erro", "Estúdio", erro.message); }
    });
    fs.appendChild(b);
  }
  return fs;
}

(function ligarMissoes() {
  elemento("estudio-vista-missoes").addEventListener("click", () => mostrarVistaDoEstudio("missoes"));
  let espera = null;
  elemento("missoes-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(() => { missoes.pagina = 0; carregarMissoes(); }, 300); });
  elemento("missoes-topo").addEventListener("change", () => { missoes.pagina = 0; carregarMissoes(); });
  elemento("missoes-anterior").addEventListener("click", () => { if (missoes.pagina > 0) { missoes.pagina--; carregarMissoes(); } });
  elemento("missoes-proxima").addEventListener("click", () => { if ((missoes.pagina + 1) * missoes.por_pagina < missoes.total) { missoes.pagina++; carregarMissoes(); } });
  elemento("estudio-rascunho").addEventListener("change", () => { if (!elemento("estudio-missoes").hidden && missoes.aberta) abrirMissao(missoes.aberta); });
})();
