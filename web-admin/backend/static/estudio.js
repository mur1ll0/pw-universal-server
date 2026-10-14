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
    r = await api(rotaEstudio(`tabelas/${estudio.tabela}`, { busca, pagina: estudio.pagina, por_pagina: estudio.porPagina }));
  } catch (erro) { avisoEstudio(erro.message); return; }
  Object.assign(estudio, { registros: r.registros, total: r.total });
  const lista = elemento("estudio-registros");
  lista.replaceChildren();
  for (const reg of r.registros) {
    const li = criar("li");
    const b = criar("button", estudio.ficha?.indice === reg.indice ? "ativo" : "");
    b.type = "button";
    b.append(criar("span", "", reg.nome || `(registro ${reg.indice})`), criar("small", "", reg.id ?? ""));
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
  try { estudio.ficha = await api(rotaEstudio(`tabelas/${estudio.tabela}/${indice}`)); }
  catch (erro) { avisoEstudio(erro.message); return; }
  desenharFichaDoEstudio();
  document.querySelectorAll("#estudio-registros button").forEach((b, i) => b.classList.toggle("ativo", estudio.registros[i]?.indice === indice));
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
    if (!grupos.has(chave)) grupos.set(chave, { linhas: new Map(), colunas: [] });
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
  entrada.setAttribute("aria-label", c.nome);
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
      const link = criar("a", "", `${c.referencia.nome || "(sem nome)"} · ${c.referencia.tabela}`);
      link.href = "#";
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
        b.append(criar("span", "", x.nome || "(sem nome)"), criar("small", "", `${x.id} · ${x.tabela}`));
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

function desenharFichaDoEstudio() {
  const area = elemento("estudio-ficha");
  area.replaceChildren();
  const f = estudio.ficha;
  if (!f) { area.appendChild(criar("p", "texto-suave", "Escolha uma tabela e um registro.")); return; }
  const editavel = !!estudio.rascunho;
  const topo = criar("div", "estudio-cabecalho-ficha");
  topo.appendChild(criar("h3", "", `${f.nome || f.tabela} · id ${f.id ?? "—"}`));
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
  const grade = criar("div", "estudio-campos");
  for (const c of soltos) {
    const rotulo = criar("label", "", `${c.nome} · ${c.tipo}${c.tipo.endsWith("string") ? ` (${c.tipo === "wstring" ? c.tamanho / 2 : c.tamanho})` : ""}`);
    grade.append(rotulo, entradaDoCampo(c, editavel, entradas));
  }
  area.appendChild(grade);

  for (const [chave, g] of grupos) {
    const bloco = criar("details", "estudio-grupo");
    const linhas = [...g.linhas.entries()];
    const preenchidas = linhas.filter(([, l]) => Object.values(l).some((c) => c.valor && c.valor !== 0)).length;
    bloco.appendChild(criar("summary", "", `${chave} · ${linhas.length} linhas (${preenchidas} preenchidas)`));
    bloco.open = preenchidas > 0 && preenchidas <= 40;
    const tabela = criar("table", "recursos-tabela");
    const cab = criar("tr");
    cab.appendChild(criar("th", "", "#"));
    for (const col of g.colunas) cab.appendChild(criar("th", "", col));
    tabela.appendChild(cab);
    for (const [n, l] of linhas) {
      const tr = criar("tr");
      tr.appendChild(criar("td", "texto-suave", n));
      for (const col of g.colunas) {
        const td = criar("td");
        if (l[col]) td.appendChild(entradaDoCampo(l[col], editavel, entradas));
        tr.appendChild(td);
      }
      tabela.appendChild(tr);
    }
    bloco.appendChild(tabela);
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

  salvar.addEventListener("click", () => {
    const campos = {};
    for (const [c, e] of entradas) {
      if (e.value === String(c.valor ?? "")) continue;
      if (c.tipo === "int32" || c.tipo === "float") {
        const n = Number(e.value);
        if (e.value === "" || !Number.isFinite(n) || (c.tipo === "int32" && !Number.isInteger(n))) {
          alerta("aviso", "Estúdio", `${c.nome}: número inválido.`, { toast: true });
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
