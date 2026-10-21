/* Estúdio, fase S1 (B237): editor genérico do elements.data do realm selecionado.
   Toda edição vai para um rascunho da versão de cliente do realm (Recursos do cliente);
   publicar entrega ao launcher e grava o arquivo no config do realm (reiniciar o realm).
   Usa `api`, `criar`, `elemento`, `alerta`, `confirmar` e `estado` do painel.js. */

const estudio = {
  realm: "", versao: "", rascunho: null, tabelas: [], tabela: "", pagina: 0, porPagina: 50,
  registros: [], total: 0, ficha: null,
};

const rotaEstudio = (resto, extra = {}) => {
  const q = new URLSearchParams(extra);
  if (estudio.rascunho) q.set("rascunho", estudio.rascunho);
  const consulta = q.toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/${resto}${consulta ? `?${consulta}` : ""}`;
};

function avisoEstudio(texto) {
  elemento("estudio-aviso").textContent = texto || "";
  elemento("estudio-aviso").hidden = !texto;
}

async function carregarEstudio() {
  const realm = estado.selecionado?.id;
  if (!realm) return;
  avisoEstudio("");
  if (estudio.realm !== realm) Object.assign(estudio, { realm, rascunho: null, tabela: "", ficha: null, pagina: 0 });
  try {
    estudio.versao = (await api(`/api/realms/${encodeURIComponent(realm)}/estudio`)).versao;
    verificarReinicio();
    await carregarRascunhosDoEstudio();
    await carregarTabelasDoEstudio();
  } catch (erro) {
    avisoEstudio(erro.message);
  }
}

async function carregarRascunhosDoEstudio() {
  const todas = await api(`/api/recursos/${encodeURIComponent(estudio.versao)}/alteracoes?limite=100`);
  const rascunhos = todas.filter((a) => a.estado === "rascunho");
  if (estudio.rascunho && !rascunhos.some((r) => r.id === estudio.rascunho)) estudio.rascunho = null;
  const sel = elemento("estudio-rascunho");
  sel.replaceChildren();
  const nenhum = criar("option", "", "— só ver o publicado —");
  nenhum.value = "";
  sel.appendChild(nenhum);
  for (const r of rascunhos) {
    const o = criar("option", "", `#${r.id} ${r.descricao || ""}`);
    o.value = r.id;
    sel.appendChild(o);
  }
  sel.value = estudio.rascunho || "";
  elemento("estudio-situacao").textContent = estudio.rascunho
    ? `Cliente ${estudio.versao} · editando o rascunho #${estudio.rascunho} (publique em Recursos do cliente)`
    : `Cliente ${estudio.versao} · somente leitura: escolha ou crie um rascunho para editar`;
}

async function carregarTabelasDoEstudio() {
  const r = await api(rotaEstudio("tabelas"));
  estudio.tabelas = r.tabelas;
  desenharTabelasDoEstudio();
  if (estudio.tabela) await listarRegistrosDoEstudio();
}

function desenharTabelasDoEstudio() {
  const filtro = elemento("estudio-filtro-tabela").value.trim().toUpperCase();
  const lista = elemento("estudio-tabelas");
  lista.replaceChildren();
  for (const t of estudio.tabelas) {
    if (filtro && !t.nome.includes(filtro)) continue;
    const li = criar("li");
    const b = criar("button", t.nome === estudio.tabela ? "ativo" : "");
    b.type = "button";
    b.append(criar("span", "", t.nome), criar("small", "", String(t.registros)));
    b.title = t.espaco ? `espaço de id ${t.espaco}` : "espaço de id desconhecido: não aceita registro novo";
    b.addEventListener("click", () => {
      Object.assign(estudio, { tabela: t.nome, pagina: 0, ficha: null });
      elemento("estudio-busca-texto").value = "";
      desenharTabelasDoEstudio();
      listarRegistrosDoEstudio();
      desenharFichaDoEstudio();
    });
    li.appendChild(b);
    lista.appendChild(li);
  }
}

async function listarRegistrosDoEstudio() {
  if (!estudio.tabela) return;
  const busca = elemento("estudio-busca-texto").value.trim();
  let r;
  try {
    const caminho = estudio.tabela === "TALK_PROC" ? "conversas" : `tabelas/${estudio.tabela}`;
    r = await api(rotaEstudio(caminho, { busca, pagina: estudio.pagina, por_pagina: estudio.porPagina }));
  } catch (erro) { avisoEstudio(erro.message); return; }
  Object.assign(estudio, { registros: r.registros, total: r.total });
  const lista = elemento("estudio-registros");
  lista.replaceChildren();
  for (const reg of r.registros) {
    const li = criar("li");
    const b = criar("button", estudio.ficha?.indice === reg.indice ? "ativo" : "");
    b.type = "button";
    const nome = criar("span", "estudio-nome-com-icone");
    nome.append(iconeDoEstudio(reg.icone, reg.nome, "estudio-icone-pequeno"), criar("span", "", reg.nome || `(registro ${reg.indice})`));
    b.append(nome, criar("small", "", reg.id ?? ""));
    b.addEventListener("click", () => abrirFichaDoEstudio(reg.indice));
    li.appendChild(b);
    lista.appendChild(li);
  }
  if (!r.registros.length) lista.appendChild(criar("li", "texto-suave", "Nada encontrado."));
  const paginas = Math.max(1, Math.ceil(r.total / estudio.porPagina));
  const nav = elemento("estudio-paginas");
  nav.replaceChildren(criar("span", "texto-suave", `${r.total} registro(s) · página ${estudio.pagina + 1} de ${paginas}`));
  for (const [rotulo, delta] of [["‹", -1], ["›", 1]]) {
    const b = criar("button", "botao botao-fantasma botao-pequeno", rotulo);
    b.type = "button";
    b.disabled = estudio.pagina + delta < 0 || estudio.pagina + delta >= paginas;
    b.addEventListener("click", () => { estudio.pagina += delta; listarRegistrosDoEstudio(); });
    nav.appendChild(b);
  }
}

async function abrirFichaDoEstudio(indice) {
  const conversa = estudio.tabela === "TALK_PROC";
  try {
    estudio.ficha = await api(rotaEstudio(conversa ? `conversas/${indice}` : `tabelas/${estudio.tabela}/${indice}`));
    if (conversa) {
      estudio.ficha.tabela = "TALK_PROC";
      if (!estudio.funcoes) estudio.funcoes = await api(rotaEstudio("funcoes"));
    }
  }
  catch (erro) { avisoEstudio(erro.message); return; }
  if (conversa) desenharConversaDoEstudio(); else desenharFichaDoEstudio();
  document.querySelectorAll("#estudio-registros button").forEach((b, i) => b.classList.toggle("ativo", estudio.registros[i]?.indice === indice));
}

const GRUPOS_DO_ESTUDIO = ["Identificação", "Serviços do NPC", "Páginas", "Fabricação", "Teleporte", "Missões",
  "Requisitos", "Combate", "Comércio", "Durabilidade e fabricação", "Efeitos", "Drops", "Comportamento", "Habilidades",
  "Visual", "Outros"];

/* Ícone do registro (o `file_icon` em hex GBK, rota de ícones do painel, B188/B239). */
function iconeDoEstudio(hex, nome, classe = "estudio-icone") {
  return imagemDoIcone("m", hex || "", nome || "?", classe);
}

/* Agrupa campos repetidos (`drop_matters_1_id`, `drop_matters_1_probability`, … ou `id_tasks_1`, …)
   em tabelas: prefixo + número da linha + sufixo da coluna (B238, ESTUDIO §5 "campos repetidos em lista"). */
function agruparCamposDoEstudio(campos) {
  const grupos = new Map();
  const soltos = [];
  for (const c of campos) {
    let m = c.nome.match(/^(.*?)_(\d+)_(.+)$/);
    let chave, linha, coluna;
    if (m) { [, chave, linha, coluna] = m; }
    else if ((m = c.nome.match(/^(.*\D)_?(\d+)$/)) && !/^ID$/.test(c.nome)) { [, chave, linha] = m; coluna = "valor"; }
    if (chave === undefined) { soltos.push(c); continue; }
    if (!grupos.has(chave)) grupos.set(chave, { linhas: new Map(), colunas: [], grupo: c.grupo });
    const g = grupos.get(chave);
    if (!g.colunas.includes(coluna)) g.colunas.push(coluna);
    if (!g.linhas.has(linha)) g.linhas.set(linha, {});
    g.linhas.get(linha)[coluna] = c;
  }
  // Grupo de uma linha só não é lista: volta para os soltos.
  for (const [chave, g] of [...grupos]) {
    if (g.linhas.size < 2) { for (const l of g.linhas.values()) soltos.push(...Object.values(l)); grupos.delete(chave); }
  }
  return { soltos, grupos };
}

function entradaDoCampo(c, editavel, entradas) {
  const caixa = criar("div", "estudio-entrada");
  const entrada = criar("input", "campo");
  entrada.disabled = !editavel || c.nome === "ID";
  entrada.setAttribute("aria-label", c.rotulo || c.nome);
  entrada.title = `${c.nome} · ${c.tipo}`;
  if (c.tipo === "int32") { entrada.type = "number"; entrada.step = "1"; }
  else if (c.tipo === "float") { entrada.type = "number"; entrada.step = "any"; }
  else entrada.maxLength = c.tipo === "wstring" ? c.tamanho / 2 : c.tamanho;
  entrada.value = c.valor ?? "";
  entrada.addEventListener("input", () => entrada.classList.toggle("mudou", entrada.value !== String(c.valor ?? "")));
  caixa.appendChild(entrada);
  entradas.push([c, entrada]);
  if (c.cita) {
    const ref = criar("span", "estudio-ref");
    if (c.referencia?.ausente) {
      ref.textContent = "id inexistente";
      ref.classList.add("texto-erro");
    } else if (c.referencia) {
      ref.appendChild(iconeDoEstudio(c.referencia.icone, c.referencia.nome, "estudio-icone-pequeno"));
      const link = criar("a", "", c.referencia.nome || `(sem nome) · ${c.referencia.tabela}`);
      link.href = "#";
      link.title = `${c.referencia.tabela} · id ${c.referencia.id}`;
      link.addEventListener("click", (e) => { e.preventDefault(); abrirRegistroDoEstudio(c.referencia.tabela, c.referencia.indice); });
      ref.appendChild(link);
    }
    if (editavel) {
      const buscar = criar("button", "botao botao-fantasma botao-pequeno", "Buscar");
      buscar.type = "button";
      buscar.title = `Escolher um registro do espaço ${c.cita}`;
      buscar.addEventListener("click", () => seletorDeReferencia(c.cita, entrada));
      ref.appendChild(buscar);
    }
    caixa.appendChild(ref);
  } else if (c.duvidoso) {
    const d = criar("span", "estudio-ref texto-suave", "?");
    d.title = "Parece citar um id, mas falta evidência no fonte ou nos dados (specs/estudio/referencias.json, duvidosos).";
    caixa.appendChild(d);
  }
  return caixa;
}

function seletorDeReferencia(espaco, entrada) {
  const caixa = entrada.parentElement;
  const aberto = caixa.querySelector(".estudio-seletor");
  if (aberto) { aberto.remove(); return; }
  const painel = criar("div", "estudio-seletor");
  const busca = criar("input", "campo");
  busca.placeholder = `Buscar no espaço ${espaco}: id ou parte do nome`;
  busca.setAttribute("aria-label", `Buscar no espaço ${espaco}`);
  const lista = criar("ul", "estudio-lista");
  painel.append(busca, lista);
  caixa.appendChild(painel);
  busca.focus();
  let espera = null;
  busca.addEventListener("input", () => {
    clearTimeout(espera);
    espera = setTimeout(async () => {
      const texto = busca.value.trim();
      lista.replaceChildren();
      if (!texto) return;
      let achados;
      try { achados = await api(rotaEstudio(`espacos/${espaco}`, { busca: texto, limite: 30 })); }
      catch (erro) { lista.appendChild(criar("li", "texto-erro", erro.message)); return; }
      if (!achados.length) lista.appendChild(criar("li", "texto-suave", "Nada encontrado."));
      for (const x of achados) {
        const li = criar("li");
        const b = criar("button");
        b.type = "button";
        const nome = criar("span", "estudio-nome-com-icone");
        nome.append(iconeDoEstudio(x.icone, x.nome, "estudio-icone-pequeno"), criar("span", "", x.nome || "(sem nome)"));
        b.append(nome, criar("small", "", `${x.id} · ${x.tabela}`));
        b.addEventListener("click", () => {
          entrada.value = x.id;
          entrada.dispatchEvent(new Event("input"));
          painel.remove();
        });
        li.appendChild(b);
        lista.appendChild(li);
      }
    }, 300);
  });
  busca.addEventListener("keydown", (e) => { if (e.key === "Escape") painel.remove(); });
}

async function abrirRegistroDoEstudio(tabela, indice) {
  Object.assign(estudio, { tabela, pagina: 0 });
  elemento("estudio-busca-texto").value = "";
  desenharTabelasDoEstudio();
  await listarRegistrosDoEstudio();
  await abrirFichaDoEstudio(indice);
}

/* Tabela de um campo repetido: só as linhas preenchidas, mais duas vazias para acrescentar; "mostrar
   todas" abre o resto (os 32 drops do monstro ficam legíveis). */
function tabelaDoGrupoRepetido(g, editavel, entradas) {
  const linhas = [...g.linhas.entries()];
  const preenchida = ([, l]) => Object.values(l).some((c) => c.valor && c.valor !== 0 && c.valor !== "0");
  const cheias = linhas.filter(preenchida);
  const vazias = linhas.filter((x) => !preenchida(x));
  const tabela = criar("table", "recursos-tabela");
  const cab = criar("tr");
  cab.appendChild(criar("th", "", "#"));
  for (const col of g.colunas) {
    const exemplo = linhas.find(([, l]) => l[col])?.[1][col];
    cab.appendChild(criar("th", "", exemplo?.rotulo || col));
  }
  tabela.appendChild(cab);
  const desenhar = ([n, l], oculta) => {
    const tr = criar("tr");
    if (oculta) tr.hidden = true;
    tr.appendChild(criar("td", "texto-suave", n));
    for (const col of g.colunas) {
      const td = criar("td");
      if (l[col]) td.appendChild(entradaDoCampo(l[col], editavel, entradas));
      tr.appendChild(td);
    }
    tabela.appendChild(tr);
    return tr;
  };
  for (const x of cheias) desenhar(x, false);
  const ocultas = vazias.map((x, i) => desenhar(x, i >= (editavel ? 2 : 0)));
  const envoltorio = criar("div");
  envoltorio.appendChild(tabela);
  const escondidas = ocultas.filter((tr) => tr.hidden);
  if (escondidas.length) {
    const mais = criar("button", "botao botao-fantasma botao-pequeno", `Mostrar todas as ${linhas.length} linhas`);
    mais.type = "button";
    mais.addEventListener("click", () => { escondidas.forEach((tr) => (tr.hidden = false)); mais.remove(); });
    envoltorio.appendChild(mais);
  }
  return envoltorio;
}

function desenharFichaDoEstudio() {
  const area = elemento("estudio-ficha");
  area.replaceChildren();
  const f = estudio.ficha;
  if (!f) { area.appendChild(criar("p", "texto-suave", "Escolha uma tabela e um registro.")); return; }
  const editavel = !!estudio.rascunho;
  const topo = criar("div", "estudio-cabecalho-ficha");
  const titulo = criar("div", "estudio-nome-com-icone");
  titulo.append(iconeDoEstudio(f.icone, f.nome || f.tabela), criar("h3", "", `${f.nome || f.tabela} · id ${f.id ?? "—"}`));
  topo.appendChild(titulo);
  const acoes = criar("div", "recursos-linha");
  const salvar = criar("button", "botao botao-primario botao-pequeno", "Salvar no rascunho");
  const clonar = criar("button", "botao botao-secundario botao-pequeno", "Novo (clonar)");
  const remover = criar("button", "botao botao-fantasma botao-pequeno", "Remover");
  for (const b of [salvar, clonar, remover]) { b.type = "button"; b.disabled = !editavel; acoes.appendChild(b); }
  clonar.disabled = !editavel || !f.espaco;
  if (!f.espaco) clonar.title = "Tabela sem espaço de id conhecido: não aceita registro novo.";
  topo.appendChild(acoes);
  area.appendChild(topo);
  area.appendChild(criar("p", "texto-suave", `${f.tabela} · registro ${f.indice}${f.espaco ? ` · espaço ${f.espaco}` : ""}`));
  if (!editavel) area.appendChild(criar("p", "texto-suave", "Somente leitura: escolha um rascunho no alto da página para editar."));

  const entradas = [];
  const { soltos, grupos } = agruparCamposDoEstudio(f.campos);
  let abertos = 0;
  // Grupo que o servidor mandou e esta lista não conhece aparece antes de "Outros" (nunca some).
  const conhecidos = new Set(GRUPOS_DO_ESTUDIO);
  const extras = [...new Set(f.campos.map((c) => c.grupo || "Outros"))].filter((g) => !conhecidos.has(g));
  const ordem = [...GRUPOS_DO_ESTUDIO.slice(0, -1), ...extras, "Outros"];
  for (const nomeGrupo of ordem) {
    const meus = soltos.filter((c) => (c.grupo || "Outros") === nomeGrupo);
    const listas = [...grupos].filter(([, g]) => (g.grupo || "Outros") === nomeGrupo);
    if (!meus.length && !listas.length) continue;
    const bloco = criar("details", "estudio-grupo");
    bloco.appendChild(criar("summary", "", `${nomeGrupo} · ${meus.length + listas.length}`));
    bloco.open = abertos < 3 && nomeGrupo !== "Outros";
    if (bloco.open) abertos += 1;
    if (meus.length) {
      const grade = criar("div", "estudio-campos");
      for (const c of meus) {
        const rotulo = criar("label", "", c.rotulo || c.nome);
        rotulo.title = `${c.nome} · ${c.tipo}${c.tipo.endsWith("string") ? ` (${c.tipo === "wstring" ? c.tamanho / 2 : c.tamanho})` : ""}`;
        grade.append(rotulo, entradaDoCampo(c, editavel, entradas));
        // B279: campo de modelo (`file_model`… com um .ecm/.smd/.ski) abre o Modelos — no .ecm, com o diagnóstico das
        // ações que o cliente pede.
        if (typeof c.valor === "string" && /\.(ecm|smd|ski)$/i.test(c.valor.trim()) && typeof abrirModelo === "function") {
          const ir = criar("a", "estudio-link-modelo", /\.ecm$/i.test(c.valor.trim()) ? "↗ ver o modelo e o diagnóstico de ações" : "↗ ver o modelo");
          ir.href = "#";
          // B281: mascote que recebe o sufixo de ambiente — invocado (`id_type` 28752, `EC_Pet.cpp:285-288`) ou
          // `inhabit_type` 6 (`EC_NPCModel.cpp:588-590`) — abre com o diagnóstico de mascote do seu habitat.
          const valor = (n) => (f.campos.find((x) => x.nome === n) || {}).valor;
          const habitat = valor("inhabit_type");
          const comSufixo = typeof habitat === "number" && (valor("id_type") === 28752 || habitat === 6);
          // B283: modelo de voo — `WINGMANWING_ESSENCE` é asa; `FLYSWORD_ESSENCE` pelo `fly_mode` (`EC_Player.cpp:4262-4273`).
          const voo = f.tabela === "WINGMANWING_ESSENCE" ? 1 : f.tabela === "FLYSWORD_ESSENCE" ? (valor("fly_mode") ?? 0) : null;
          ir.addEventListener("click", (ev) => { ev.preventDefault(); mostrarVistaDoEstudio("modelos"); abrirModelo(c.valor.trim(), comSufixo ? habitat : null, voo); });
          grade.append(criar("span"), ir);
        }
      }
      bloco.appendChild(grade);
    }
    for (const [chave, g] of listas) {
      bloco.appendChild(criar("h4", "estudio-subtitulo", chave));
      bloco.appendChild(tabelaDoGrupoRepetido(g, editavel, entradas));
    }
    area.appendChild(bloco);
  }

  const usado = criar("details", "estudio-grupo");
  usado.appendChild(criar("summary", "", `Usado por · ${f.usado_por.length}${f.usado_por.length === 100 ? "+" : ""}`));
  usado.open = f.usado_por.length > 0 && f.usado_por.length <= 20;
  const lista = criar("ul", "estudio-lista");
  for (const u of f.usado_por) {
    const li = criar("li");
    const b = criar("button");
    b.type = "button";
    b.append(criar("span", "", `${u.nome || "(sem nome)"} · ${u.tabela}`), criar("small", "", `${u.campo} · id ${u.id ?? "—"}`));
    b.addEventListener("click", () => abrirRegistroDoEstudio(u.tabela, u.indice));
    li.appendChild(b);
    lista.appendChild(li);
  }
  if (!f.usado_por.length) lista.appendChild(criar("li", "texto-suave", f.espaco ? "Nenhum registro cita este." : "Tabela sem espaço de id conhecido."));
  usado.appendChild(lista);
  area.appendChild(usado);

  // B284: onde nasce — as áreas do npcgen de todos os mapas que citam este id (só busca ao abrir).
  if (["MONSTER_ESSENCE", "NPC_ESSENCE", "MINE_ESSENCE"].includes(f.tabela) && f.id) {
    const nasce = criar("details", "estudio-grupo");
    nasce.appendChild(criar("summary", "", "Onde nasce (npcgen de todos os mapas)"));
    const ln = criar("ul", "estudio-lista");
    nasce.appendChild(ln);
    nasce.addEventListener("toggle", async () => {
      if (!nasce.open || nasce.dataset.carregado) return;
      nasce.dataset.carregado = "1";
      ln.replaceChildren(criar("li", "texto-suave", "Procurando nos mapas…"));
      try {
        const q = new URLSearchParams({ id: f.id, ...(estudio.rascunho ? { rascunho: estudio.rascunho } : {}) });
        const r = await api(`/api/realms/${encodeURIComponent(estudio.realm)}/estudio/nascimentos?${q}`);
        ln.replaceChildren();
        nasce.firstChild.textContent = `Onde nasce · ${r.locais.length} área(s) em ${new Set(r.locais.map((x) => x.caminho)).size} mapa(s) de ${r.mapas_lidos}`;
        for (const l of r.locais) {
          const li = criar("li");
          const b = criar("button");
          b.type = "button";
          b.append(criar("span", "", `${l.mapa} (${l.caminho}) · ${l.tipo} · área ${l.area}`), criar("small", "", `posição (${l.x}, ${l.z}) · quantidade ${l.quantidade}`));
          b.addEventListener("click", () => irParaArea(l.caminho, l.tipo === "recurso" ? "recursos" : "areas", l.area));
          li.appendChild(b);
          ln.appendChild(li);
        }
        if (!r.locais.length) ln.appendChild(criar("li", "texto-suave", `Nenhuma área de npcgen cita o id ${f.id} (${r.mapas_lidos} mapas lidos).`));
      } catch (erro) { ln.replaceChildren(criar("li", "texto-suave", erro.message)); delete nasce.dataset.carregado; }
    });
    area.appendChild(nasce);
  }

  salvar.addEventListener("click", () => {
    const campos = {};
    for (const [c, e] of entradas) {
      if (e.value === String(c.valor ?? "")) continue;
      if (c.tipo === "int32" || c.tipo === "float") {
        const n = Number(e.value);
        if (e.value === "" || !Number.isFinite(n) || (c.tipo === "int32" && !Number.isInteger(n))) {
          alerta("aviso", "Estúdio", `${c.rotulo || c.nome}: número inválido.`, { toast: true });
          return;
        }
        campos[c.nome] = n;
      } else campos[c.nome] = e.value;
    }
    if (!Object.keys(campos).length) { alerta("aviso", "Estúdio", "Nenhum campo mudou.", { toast: true }); return; }
    executarNoEstudio(() => api(rotaEstudio(`tabelas/${f.tabela}/${f.indice}`), { method: "PUT", body: JSON.stringify({ campos }) }),
      (r) => (r.igual_ao_publicado ? "Igual ao publicado: o elements saiu do rascunho." : `${r.resultado.campos_mudados} campo(s) salvos no rascunho.`),
      f.indice);
  });
  clonar.addEventListener("click", () => executarNoEstudio(
    () => api(rotaEstudio(`tabelas/${f.tabela}/${f.indice}/clonar`), { method: "POST" }),
    (r) => `Registro novo com id ${r.resultado.id}.`, null, (r) => r.resultado.indice));
  remover.addEventListener("click", async () => {
    if (!(await confirmar("Remover o registro?", `${f.tabela} id ${f.id}: a remoção fica no rascunho e só vale ao publicar.`, "Remover"))) return;
    executarNoEstudio(() => api(rotaEstudio(`tabelas/${f.tabela}/${f.indice}`), { method: "DELETE" }), () => "Remoção no rascunho.", null);
  });
}

async function executarNoEstudio(acao, mensagem, reabrir, indiceNovo) {
  try {
    const r = await acao();
    alerta("ok", "Estúdio", mensagem(r), { toast: true });
    await carregarTabelasDoEstudio();
    const alvo = indiceNovo ? indiceNovo(r) : reabrir;
    if (alvo !== null && alvo !== undefined) await abrirFichaDoEstudio(alvo);
    else { estudio.ficha = null; desenharFichaDoEstudio(); }
  } catch (erro) {
    alerta("erro", "Estúdio", erro.message);
  }
}

/* ---------------- Conversas (TALK_PROC, S3, B240) ----------------
   Uma conversa = janelas (texto + opções). Cada opção leva a outra janela da mesma conversa ou a uma
   função pré-definida (bit alto do id, `enum SERVICE_TYPE` do cliente: loja, teleporte, encerrar…). */

function desenharConversaDoEstudio() {
  const area = elemento("estudio-ficha");
  area.replaceChildren();
  const c = structuredClone(estudio.ficha);
  const editavel = !!estudio.rascunho;
  const funcoes = estudio.funcoes || [];

  const topo = criar("div", "estudio-cabecalho-ficha");
  topo.appendChild(criar("h3", "", `Conversa · id ${c.id}`));
  const acoes = criar("div", "recursos-linha");
  const salvar = criar("button", "botao botao-primario botao-pequeno", "Salvar no rascunho");
  const copiar = criar("button", "botao botao-secundario botao-pequeno", "Nova a partir desta");
  const nova = criar("button", "botao botao-secundario botao-pequeno", "Nova vazia");
  const remover = criar("button", "botao botao-fantasma botao-pequeno", "Remover");
  for (const b of [salvar, copiar, nova, remover]) { b.type = "button"; b.disabled = !editavel; acoes.appendChild(b); }
  topo.appendChild(acoes);
  area.appendChild(topo);
  if (!editavel) area.appendChild(criar("p", "texto-suave", "Somente leitura: escolha um rascunho no alto da página para editar."));

  const titulo = criar("input", "campo");
  titulo.maxLength = 63;
  titulo.value = c.texto;
  titulo.disabled = !editavel;
  titulo.setAttribute("aria-label", "Título da conversa");
  titulo.addEventListener("input", () => (c.texto = titulo.value));
  const linhaTitulo = criar("div", "estudio-campos");
  linhaTitulo.append(criar("label", "", "Título (até 63 caracteres)"), titulo);
  area.appendChild(linhaTitulo);

  const lista = criar("div");
  area.appendChild(lista);
  const destinos = () => [
    ...c.janelas.map((j) => ({ valor: j.id, rotulo: `→ janela ${j.id}` })),
    ...funcoes.map((f) => ({ valor: f.codigo, rotulo: `★ ${f.rotulo}` })),
  ];

  function redesenhar() {
    lista.replaceChildren();
    for (const [ji, j] of c.janelas.entries()) {
      const cartao = criar("section", "recursos-rascunho estudio-janela");
      const cab = criar("header", "recursos-linha");
      cab.appendChild(criar("strong", "", `Janela ${j.id}`));
      const pai = criar("select", "campo");
      pai.disabled = !editavel;
      pai.setAttribute("aria-label", `Janela de origem da janela ${j.id}`);
      for (const [valor, rotulo] of [[-1, "início da conversa"], ...c.janelas.filter((x) => x.id !== j.id).map((x) => [x.id, `vem da janela ${x.id}`])]) {
        const o = criar("option", "", rotulo); o.value = valor; pai.appendChild(o);
      }
      pai.value = j.id_pai;
      pai.addEventListener("change", () => (j.id_pai = Number(pai.value)));
      cab.appendChild(pai);
      if (editavel && c.janelas.length > 1) {
        const tirar = criar("button", "botao botao-fantasma botao-pequeno", "Remover janela");
        tirar.type = "button";
        tirar.addEventListener("click", () => {
          if (c.janelas.some((x) => x.id !== j.id && (x.id_pai === j.id || x.opcoes.some((o) => o.id === j.id)))) {
            alerta("aviso", "Estúdio", `Outra janela ou opção leva à janela ${j.id}: mude-a antes.`, { toast: true });
            return;
          }
          c.janelas.splice(ji, 1);
          redesenhar();
        });
        cab.appendChild(tirar);
      }
      cartao.appendChild(cab);

      const texto = criar("textarea", "campo estudio-texto-janela");
      texto.value = j.texto;
      texto.disabled = !editavel;
      texto.rows = 3;
      texto.setAttribute("aria-label", `Texto da janela ${j.id}`);
      texto.addEventListener("input", () => (j.texto = texto.value));
      cartao.appendChild(texto);

      const tabela = criar("table", "recursos-tabela");
      const th = criar("tr");
      for (const t of ["Opção (até 63)", "Leva a", "Parâmetro", ""]) th.appendChild(criar("th", "", t));
      tabela.appendChild(th);
      for (const [oi, o] of j.opcoes.entries()) {
        const tr = criar("tr");
        const tx = criar("input", "campo"); tx.maxLength = 63; tx.value = o.texto; tx.disabled = !editavel;
        tx.setAttribute("aria-label", "Texto da opção");
        tx.addEventListener("input", () => (o.texto = tx.value));
        const dest = criar("select", "campo"); dest.disabled = !editavel;
        dest.setAttribute("aria-label", "Destino da opção");
        const opcoesDestino = destinos();
        if (!opcoesDestino.some((d) => d.valor === o.id)) opcoesDestino.push({ valor: o.id, rotulo: `? ${o.id}` });
        for (const d of opcoesDestino) { const op = criar("option", "", d.rotulo); op.value = d.valor; dest.appendChild(op); }
        dest.value = o.id;
        dest.addEventListener("change", () => (o.id = Number(dest.value)));
        const par = criar("input", "campo"); par.type = "number"; par.min = "0"; par.value = o.param; par.disabled = !editavel;
        par.setAttribute("aria-label", "Parâmetro da opção");
        par.addEventListener("input", () => (o.param = Number(par.value) || 0));
        const acoesO = criar("td");
        if (editavel) {
          const tirar = criar("button", "botao botao-fantasma botao-pequeno", "Tirar");
          tirar.type = "button";
          tirar.addEventListener("click", () => { j.opcoes.splice(oi, 1); redesenhar(); });
          acoesO.appendChild(tirar);
        }
        for (const el of [tx, dest, par]) { const td = criar("td"); td.appendChild(el); tr.appendChild(td); }
        tr.appendChild(acoesO);
        tabela.appendChild(tr);
      }
      cartao.appendChild(tabela);
      if (editavel) {
        const maisOpcao = criar("button", "botao botao-fantasma botao-pequeno", "+ opção");
        maisOpcao.type = "button";
        maisOpcao.addEventListener("click", () => {
          j.opcoes.push({ id: 0x80000012, texto: "Até logo", param: 0 }); // TALK_EXIT
          redesenhar();
        });
        cartao.appendChild(maisOpcao);
      }
      lista.appendChild(cartao);
    }
    if (editavel) {
      const maisJanela = criar("button", "botao botao-secundario botao-pequeno", "+ janela");
      maisJanela.type = "button";
      maisJanela.addEventListener("click", () => {
        const id = Math.max(0, ...c.janelas.map((x) => x.id)) + 1;
        c.janelas.push({ id, id_pai: c.janelas[0]?.id ?? -1, texto: "", opcoes: [{ id: 0x80000011, texto: "Voltar", param: 0 }] }); // TALK_RETURN
        redesenhar();
      });
      lista.appendChild(maisJanela);
    }
  }
  redesenhar();

  const usado = criar("details", "estudio-grupo");
  usado.appendChild(criar("summary", "", `Usado por · ${c.usado_por.length}`));
  usado.open = c.usado_por.length > 0 && c.usado_por.length <= 20;
  const ul = criar("ul", "estudio-lista");
  for (const u of c.usado_por) {
    const li = criar("li"); const b = criar("button"); b.type = "button";
    b.append(criar("span", "", `${u.nome || "(sem nome)"} · ${u.tabela}`), criar("small", "", `${u.campo} · id ${u.id ?? "—"}`));
    b.addEventListener("click", () => abrirRegistroDoEstudio(u.tabela, u.indice));
    li.appendChild(b); ul.appendChild(li);
  }
  if (!c.usado_por.length) ul.appendChild(criar("li", "texto-suave", "Nenhum serviço usa esta conversa."));
  usado.appendChild(ul);
  area.appendChild(usado);

  salvar.addEventListener("click", () => executarNoEstudio(
    () => api(rotaEstudio(`conversas/${c.indice}`), { method: "PUT", body: JSON.stringify({ conversa: c }) }),
    (r) => (r.resultado.mudou ? "Conversa salva no rascunho." : "Nada mudou."), c.indice));
  copiar.addEventListener("click", () => executarNoEstudio(
    () => api(rotaEstudio("conversas", { base: c.indice }), { method: "POST" }),
    (r) => `Conversa nova com id ${r.resultado.id}.`, null, (r) => r.resultado.indice));
  nova.addEventListener("click", () => executarNoEstudio(
    () => api(rotaEstudio("conversas"), { method: "POST" }),
    (r) => `Conversa nova com id ${r.resultado.id}.`, null, (r) => r.resultado.indice));
  remover.addEventListener("click", async () => {
    if (!(await confirmar("Remover a conversa?", `Conversa ${c.id}: a remoção fica no rascunho e só vale ao publicar.`, "Remover"))) return;
    executarNoEstudio(() => api(rotaEstudio(`conversas/${c.indice}`), { method: "DELETE" }), () => "Conversa removida no rascunho.", null);
  });
}

elemento("estudio-rascunho").addEventListener("change", async (e) => {
  estudio.rascunho = e.target.value ? Number(e.target.value) : null;
  await carregarRascunhosDoEstudio();
  await carregarTabelasDoEstudio();
  if (estudio.ficha) await abrirFichaDoEstudio(estudio.ficha.indice);
});
elemento("estudio-novo").addEventListener("submit", async (e) => {
  e.preventDefault();
  const descricao = elemento("estudio-novo-descricao").value.trim();
  if (!descricao) { alerta("aviso", "Estúdio", "Descreva o rascunho.", { toast: true }); return; }
  try {
    const r = await api(`/api/recursos/${encodeURIComponent(estudio.versao)}/rascunhos`, {
      method: "POST", body: JSON.stringify({ descricao, operacao_id: (crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`).slice(0, 64) }),
    });
    elemento("estudio-novo-descricao").value = "";
    estudio.rascunho = r.id;
    await carregarRascunhosDoEstudio();
    if (estudio.ficha) await abrirFichaDoEstudio(estudio.ficha.indice);
    else desenharFichaDoEstudio();
  } catch (erro) { alerta("erro", "Estúdio", erro.message); }
});
elemento("estudio-filtro-tabela").addEventListener("input", desenharTabelasDoEstudio);
elemento("estudio-busca").addEventListener("submit", (e) => { e.preventDefault(); estudio.pagina = 0; listarRegistrosDoEstudio(); });

/* ---------------- Aguarda reinício (B249) ----------------
   O GS lê os arquivos do config (elements, npcgen, precinct, region, world_targets) uma vez, na partida. Se a última
   publicação que gravou arquivo no realm é mais nova que a partida do GS, o realm ainda roda com os dados velhos. */
async function verificarReinicio(antes) {
  const caixa = elemento("estudio-reinicio");
  const realm = estudio.realm;
  let s;
  try { s = await api(`/api/realms/${encodeURIComponent(realm)}/servidor`); }
  catch (erro) { caixa.hidden = true; return null; }
  if (estudio.realm !== realm) return null;
  caixa.replaceChildren();
  caixa.hidden = s.aguarda_reinicio !== true;
  if (s.aguarda_reinicio === true) {
    const quando = new Date(s.ultima.publicada_em * 1000).toLocaleString();
    caixa.append(criar("span", "", `A versão ${s.ultima.numero} (${quando}) gravou arquivos no realm depois que o servidor de mundo subiu: `
      + `reinicie para valer. ${s.jogadores_online ? `${s.jogadores_online} jogador(es) online serão desconectados (salvos).` : "Ninguém online."} `));
    const b = criar("button", "botao botao-pequeno botao-primario", "Reiniciar o realm");
    b.type = "button";
    b.addEventListener("click", () => reiniciarRealm(s));
    // B331: reler os dados sem desconectar. Vale para tabelas e catálogos (itens, habilidades, missões, serviços,
    // distritos, portais, destinos); o que já nasceu no mapa (npcgen) só muda reiniciando.
    const r = criar("button", "botao botao-pequeno", s.recarregando ? "Recarregando…" : "Recarregar os dados (sem desconectar)");
    r.type = "button";
    r.disabled = !!s.recarregando;
    r.title = "Relê os arquivos do config do realm no servidor de mundo, sem desconectar ninguém. As criaturas do npcgen continuam as de antes até reiniciar.";
    r.addEventListener("click", () => recarregarRealm(s));
    caixa.append(" ", b, " ", r);
  }
  if (s.falhas_da_recarga && s.falhas_da_recarga.length) {
    caixa.hidden = false;
    caixa.append(criar("div", "texto-suave", `A última recarga foi recusada (os dados em uso continuam): ${s.falhas_da_recarga.join(", ")}`));
  }
  return s;
}

async function recarregarRealm(situacao) {
  const realm = estudio.realm;
  try {
    await api(`/api/realms/${encodeURIComponent(realm)}/recarregar`, { method: "POST", body: JSON.stringify({}) });
    alerta("ok", "Estúdio", "Relendo os dados do realm no servidor de mundo…", { toast: true });
  } catch (erro) { alerta("erro", "Estúdio", erro.message); return; }
  const antes = situacao.dados_em || situacao.iniciado_em || 0;
  for (let i = 0; i < 40; i++) {
    await new Promise((ok) => setTimeout(ok, 3000));
    const s = await verificarReinicio().catch(() => null);
    if (s && !s.recarregando && s.dados_em && s.dados_em > antes) {
      alerta("ok", "Estúdio", "Dados novos em uso, sem desconectar ninguém.", { toast: true });
      return;
    }
    if (s && !s.recarregando && s.falhas_da_recarga && s.falhas_da_recarga.length) {
      alerta("erro", "Estúdio", `Recarga recusada: ${s.falhas_da_recarga.join(", ")}`);
      return;
    }
  }
  alerta("aviso", "Estúdio", "A recarga ainda não terminou depois de 120 s; confira o log do servidor de mundo.");
}

async function reiniciarRealm(situacao) {
  const realm = estudio.realm;
  const digitado = window.prompt(`Reiniciar o servidor de mundo de ${realm}? Todos são desconectados (o personagem é salvo) `
    + "e o realm volta em alguns segundos. Digite o id do realm para confirmar:", "");
  if (digitado === null) return;
  try {
    const r = await api(`/api/realms/${encodeURIComponent(realm)}/reiniciar`, { method: "POST", body: JSON.stringify({ confirmar: digitado.trim() }) });
    alerta("ok", "Estúdio", `Reiniciando (${r.processos} processo(s), ${r.online} desconectado(s)). Acompanhando a volta…`, { toast: true });
  } catch (erro) { alerta("erro", "Estúdio", erro.message); return; }
  const antes = situacao.iniciado_em;
  for (let i = 0; i < 30; i++) {
    await new Promise((ok) => setTimeout(ok, 3000));
    const s = await verificarReinicio().catch(() => null);
    if (s && s.iniciado_em && s.iniciado_em > antes) {
      alerta("ok", "Estúdio", "O servidor de mundo voltou com os arquivos novos.", { toast: true });
      return;
    }
  }
  alerta("aviso", "Estúdio", "O servidor de mundo ainda não respondeu depois de 90 s; confira o contêiner.");
}
