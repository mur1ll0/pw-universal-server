"use strict";

const elemento = (id) => document.getElementById(id);
const estado = {
  sessao: null, realms: [], selecionado: null, pagina: "inicio", carregando: false,
  geracao: 0, busca: 0, ficha: 0, personagemId: null,
  conta: null, cartao: null, comando: null, enviando: false, buscaContas: 0,
  paginaContas: 1, porPagina: 12, totalContas: 0, aba: "resumo", online: {},
  acompanhamento: null, tentativas: 0,
  catalogoMapas: {}, mapasMarcados: new Set(), realmDosMarcados: null,
};
const temTempo = typeof setTimeout === "function";
const fmt = (n) => (n === null || n === undefined ? "—" : Number(n).toLocaleString("pt-BR"));
const PAGINAS = {
  inicio: ["Visão geral", "Realms deste ambiente"],
  contas: ["Contas globais", "Valem para todos os realms"],
  realm: ["Painel do realm", ""],
  personagens: ["Personagens", ""],
  mapas: ["Mapas", ""],
  rates: ["Rates", ""],
  moldes: ["Moldes", ""],
};
const FUTURAS = {
  moldes: "Editar os moldes de criação por classe.",
};
const CAMPOS_RATES = [["exp", "rate-exp"], ["sp", "rate-sp"], ["drop", "rate-drop"], ["moedas", "rate-moedas"]];
const mesmasRates = (a, b) => !!a && !!b && CAMPOS_RATES.every(([c]) => Math.abs(Number(a[c]) - Number(b[c])) < 0.001);

async function api(caminho, opcoes = {}) {
  const cabecalhos = { ...opcoes.headers };
  if (opcoes.body) cabecalhos["Content-Type"] = "application/json";
  if (estado.sessao) cabecalhos["X-CSRF-Token"] = estado.sessao.csrf;
  const resposta = await fetch(caminho, { ...opcoes, headers: cabecalhos, credentials: "same-origin" });
  const dados = resposta.status === 204 ? null : await resposta.json();
  if (!resposta.ok) {
    if (resposta.status === 401 && caminho !== "/api/sessao/entrar") mostrarEntrada();
    const erro = new Error(dados?.mensagem || dados?.detail || dados?.codigo || "Não foi possível concluir a operação.");
    erro.resultado = dados;
    throw erro;
  }
  return dados;
}

function criar(tag, classe, texto) {
  const no = document.createElement(tag);
  if (classe) no.className = classe;
  if (texto !== undefined) no.textContent = texto;
  return no;
}

function aviso(mensagem) {
  elemento("aviso").textContent = mensagem;
  elemento("aviso").hidden = !mensagem;
}

/* ---------------- Alerta padrão (sucesso, erro, aguardando) ---------------- */

let fecharAlertaEm = null;
function alerta(tipo, titulo, texto) {
  elemento("alerta-icone").className = `alerta-icone ${tipo}`;
  elemento("alerta-titulo").textContent = titulo;
  elemento("resultado-comando").textContent = texto;
  elemento("fechar-operacao").hidden = tipo === "carregando";
  elemento("recuperar-comando").hidden = !(tipo === "aviso" && estado.comando);
  elemento("operacao").hidden = false;
  if (fecharAlertaEm && temTempo) clearTimeout(fecharAlertaEm);
  fecharAlertaEm = tipo === "ok" && temTempo ? setTimeout(fecharAlerta, 2200) : null;
}
function fecharAlerta() { elemento("operacao").hidden = true; elemento("confirmar-alerta").hidden = true; }

function mostrarEntrada() {
  estado.buscaContas++;
  estado.conta = null;
  estado.comando = null;
  pararAcompanhamento();
  elemento("lista-contas").replaceChildren();
  elemento("nova-senha").value = "";
  elemento("senha-criacao").value = "";
  elemento("usuario-novo").value = "";
  estado.geracao++;
  estado.busca++;
  estado.ficha++;
  estado.personagemId = null;
  estado.sessao = null;
  estado.realms = [];
  estado.selecionado = null;
  elemento("portal").hidden = true;
  elemento("entrada").hidden = false;
  elemento("popover-conta").hidden = true;
  elemento("modal-criacao").hidden = true;
  elemento("operacao").hidden = true;
  elemento("lista-realms").replaceChildren();
  elemento("lista-capacidades").replaceChildren();
  elemento("nome-usuario").textContent = "";
  elemento("lista-personagens").replaceChildren();
  elemento("ficha-personagem").hidden = true;
}

/* ---------------- Navegação ---------------- */

function mudarPagina(pagina) {
  if (!estado.selecionado && ["realm", "personagens", "mapas", "rates", "moldes"].includes(pagina)) pagina = "inicio";
  estado.pagina = pagina;
  fecharConta();
  for (const id of ["inicio", "realm", "personagens", "contas", "rates", "mapas"]) elemento(`pagina-${id}`).hidden = pagina !== id;
  elemento("pagina-futura").hidden = !FUTURAS[pagina];
  const [titulo, sub] = PAGINAS[pagina];
  const realm = estado.selecionado;
  elemento("titulo-pagina").textContent = pagina === "realm" && realm ? realm.nome : titulo;
  elemento("subtitulo-pagina").textContent = sub || (realm ? (pagina === "realm" ? `Porta ${realm.porta}` : realm.nome) : "");
  if (FUTURAS[pagina]) {
    elemento("titulo-futura").textContent = `${titulo} · em breve`;
    elemento("texto-futura").textContent = FUTURAS[pagina];
  }
  document.querySelectorAll(".nav").forEach((botao) => {
    const ativo = botao.dataset.pagina === pagina;
    botao.classList.toggle("ativo", ativo);
    if (ativo) botao.setAttribute("aria-current", "page");
    else botao.removeAttribute("aria-current");
  });
  fecharMenu();
  if (pagina === "contas" && !elemento("lista-contas").children?.length) buscarContas();
  if (pagina === "personagens" && !estado.personagemId) buscarPersonagens();
  if (pagina === "rates") preencherRates();
  if (pagina === "mapas") renderizarMapas();
}

/* ---------------- Mundo sem resposta (B178) ---------------- */

// Situação da consulta viva de um realm: "ok", "consultando", "sem_canal" ou "sem_resposta".
function situacaoDoMundo(realm) {
  if (!realm?.canal_administrativo) return "sem_canal";
  const vivo = estado.online[realm.id];
  if (!vivo) return "consultando";
  return vivo.estado === "consultado" ? "ok" : "sem_resposta";
}

// Bloco "sem resposta" com o motivo e um botão para consultar de novo.
function avisoSemResposta(realm) {
  const vivo = estado.online[realm.id];
  const caixa = criar("div", "nota-vazia sem-resposta");
  const titulo = vivo?.estado === "em_transicao" ? "O mundo está trocando jogadores de lugar" : "O servidor de mundo não respondeu";
  caixa.append(criar("strong", "", titulo), criar("p", "dica", vivo?.aviso || "Ele pode estar reiniciando ou parado."));
  const botao = criar("button", "botao botao-secundario", "Tentar de novo");
  botao.type = "button";
  botao.addEventListener("click", () => reconsultar(realm));
  caixa.append(botao);
  return caixa;
}

async function reconsultar(realm) {
  delete estado.online[realm.id];
  if (estado.pagina === "mapas") renderizarMapas();
  if (estado.pagina === "rates") preencherRates();
  renderizarRealm();
  await consultarOnline(estado.geracao);
}

/* ---------------- Mapas do realm (E7, B183) ---------------- */

// Catálogo da versão (gs.conf original + nomes do pwadmin), uma vez por realm.
async function carregarCatalogoDeMapas(realm) {
  if (estado.catalogoMapas[realm.id]) return;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/catalogo-mapas`);
    estado.catalogoMapas[realm.id] = dados.mapas || [];
  } catch (_) {
    estado.catalogoMapas[realm.id] = [];
  }
  if (estado.selecionado?.id === realm.id && estado.pagina === "mapas") renderizarMapas();
}

// Catálogo + estado vivo: todo mapa do realm, com status. Ligado = carregado e aceitando
// entrada (ou carregando); o resto é desligado. Mapa vivo fora do catálogo também entra.
function mapasDoRealm(realm) {
  const vivo = estado.online[realm.id] || {};
  const vivos = new Map((vivo.mapas || []).map((m) => [m.mapa, m]));
  const carregaveis = new Set(vivo.carregaveis || []);
  const catalogo = estado.catalogoMapas[realm.id] || [];
  const lista = catalogo.map((c) => ({ mapa: c.mapa, chave: c.chave, nome: c.nome }));
  for (const id of vivos.keys()) {
    if (!catalogo.some((c) => c.mapa === id)) lista.push({ mapa: id, chave: "", nome: `Mapa ${id}` });
  }
  return lista.map((m) => {
    const v = vivos.get(m.mapa);
    const ligado = Boolean(v) && v.ligado !== false;
    return { ...m, ligado, carregando: Boolean(v?.carregando), online: v?.jogadores_online ?? 0,
      carregavel: Boolean(v) || carregaveis.has(m.mapa) };
  });
}

function mapasFiltrados(realm) {
  const texto = (elemento("filtro-mapa").value || "").trim().toLowerCase();
  const status = elemento("filtro-status-mapa").value;
  return mapasDoRealm(realm)
    .filter((m) => !texto || String(m.mapa).includes(texto) || m.nome.toLowerCase().includes(texto)
      || m.chave.toLowerCase().includes(texto))
    .filter((m) => status === "todos" || (status === "ligados") === m.ligado)
    .sort((a, b) => (a.ligado === b.ligado ? a.mapa - b.mapa : a.ligado ? -1 : 1));
}

function renderizarMapas() {
  const realm = estado.selecionado;
  const lista = elemento("lista-mapas-admin");
  lista.replaceChildren();
  elemento("resumo-mapas").textContent = "";
  if (!realm) return;
  if (estado.realmDosMarcados !== realm.id) { estado.mapasMarcados.clear(); estado.realmDosMarcados = realm.id; }
  const situacao = situacaoDoMundo(realm);
  if (situacao === "sem_canal") { lista.append(criar("p", "nota-vazia", "Realm sem canal administrativo.")); return; }
  if (situacao === "consultando") { lista.append(criar("p", "nota-vazia", "Consultando o mundo…")); return; }
  if (situacao === "sem_resposta") { lista.append(avisoSemResposta(realm)); return; }
  if (!estado.catalogoMapas[realm.id]) { carregarCatalogoDeMapas(realm); lista.append(criar("p", "nota-vazia", "Carregando a lista de mapas…")); return; }
  const todos = mapasDoRealm(realm);
  const visiveis = mapasFiltrados(realm);
  const ligados = todos.filter((m) => m.ligado).length;
  elemento("resumo-mapas").textContent = `${ligados} ligado${ligados === 1 ? "" : "s"} de ${todos.length} · `
    + `${estado.mapasMarcados.size} selecionado${estado.mapasMarcados.size === 1 ? "" : "s"}`
    + (visiveis.length !== todos.length ? ` · mostrando ${visiveis.length}` : "");
  if (!visiveis.length) { lista.append(criar("p", "nota-vazia", "Nenhum mapa com esse filtro.")); return; }
  for (const mapa of visiveis) {
    const marcado = estado.mapasMarcados.has(mapa.mapa);
    const linha = criar("div", `linha-mapa ${mapa.ligado ? "" : "desligado"} ${marcado ? "marcada" : ""}`);
    linha.setAttribute("role", "listitem");
    const caixa = criar("input");
    caixa.type = "checkbox";
    caixa.checked = marcado;
    caixa.setAttribute("aria-label", `Selecionar mapa ${mapa.mapa}`);
    const nome = criar("span", "nome-mapa", mapa.nome);
    if (mapa.chave) nome.append(criar("small", "", mapa.chave));
    const rotulo = mapa.carregando ? "Carregando…" : mapa.ligado ? "Ligado" : mapa.carregavel ? "Desligado" : "Sem dados";
    const etiqueta = criar("span", `etiqueta ${mapa.ligado ? "" : mapa.carregavel ? "etiqueta-erro" : "etiqueta-neutra"}`);
    etiqueta.append(criar("span", `ponto ${mapa.ligado ? "ponto-ok" : "ponto-erro"}`), rotulo);
    if (!mapa.carregavel) etiqueta.title = "O servidor de mundo não tem os dados deste mapa.";
    const online = criar("span", "online-mapa", mapa.ligado ? `${fmt(mapa.online)} online` : "");
    linha.append(caixa, criar("span", "numero-mapa", String(mapa.mapa)), nome, etiqueta, online);
    linha.addEventListener("click", () => {
      if (estado.mapasMarcados.has(mapa.mapa)) estado.mapasMarcados.delete(mapa.mapa);
      else estado.mapasMarcados.add(mapa.mapa);
      renderizarMapas();
    });
    lista.append(linha);
  }
}

// Selecionar/desmarcar todos: age sobre o que o filtro mostra.
function marcarMapasVisiveis(marcar) {
  const realm = estado.selecionado;
  if (!realm) return;
  if (marcar) for (const m of mapasFiltrados(realm)) estado.mapasMarcados.add(m.mapa);
  else estado.mapasMarcados.clear();
  renderizarMapas();
}

let resolverConfirmacao = null;
function confirmar(titulo, texto, rotulo) {
  alerta("aviso", titulo, texto);
  elemento("confirmar-alerta").textContent = rotulo;
  elemento("confirmar-alerta").hidden = false;
  elemento("fechar-operacao").textContent = "Cancelar";
  elemento("fechar-operacao").className = "botao botao-secundario";
  return new Promise((resolver) => { resolverConfirmacao = resolver; });
}
function responderConfirmacao(sim) {
  elemento("confirmar-alerta").hidden = true;
  elemento("fechar-operacao").textContent = "OK";
  elemento("fechar-operacao").className = "botao botao-primario";
  const resolver = resolverConfirmacao;
  resolverConfirmacao = null;
  if (resolver) resolver(sim);
}

// Liga ou desliga os selecionados, um por vez. Só envia o que muda (ligar um desligado com
// dados; desligar um ligado). Ligar um mapa descarregado volta "carregando": a tela segue
// consultando o estado sozinha até todos subirem (B183).
async function aplicarAosMarcados(ligar) {
  const realm = estado.selecionado;
  if (!realm || estado.enviando) return;
  const marcados = mapasDoRealm(realm).filter((m) => estado.mapasMarcados.has(m.mapa));
  const alvos = marcados.filter((m) => (ligar ? !m.ligado && m.carregavel : m.ligado));
  if (!marcados.length) { alerta("aviso", "Nada selecionado", "Marque os mapas na lista."); return; }
  if (!alvos.length) {
    alerta("aviso", "Nada a fazer", ligar ? "Os selecionados já estão ligados (ou não têm dados)." : "Os selecionados já estão desligados.");
    return;
  }
  if (!ligar) {
    const n = alvos.reduce((t, m) => t + (m.online || 0), 0);
    const sim = await confirmar(`Desligar ${alvos.length} mapa${alvos.length === 1 ? "" : "s"}?`,
      n ? `${n} jogador${n === 1 ? "" : "es"} ser${n === 1 ? "á" : "ão"} salvo${n === 1 ? "" : "s"} e voltar${n === 1 ? "á" : "ão"} ao login. Ninguém entra até religar.`
        : "Os mapas serão descarregados. Ninguém entra até religar.", "Desligar");
    if (!sim) { fecharAlerta(); return; }
  }
  estado.enviando = true;
  const falhas = [];
  let desconectados = 0;
  try {
    for (const [i, mapa] of alvos.entries()) {
      alerta("carregando", ligar ? "Ligando mapas…" : "Desligando mapas…", `${i + 1} de ${alvos.length}: ${mapa.mapa} · ${mapa.nome}`);
      try {
        const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/mapas/${mapa.mapa}`,
          { method: "POST", body: JSON.stringify({ ligado: ligar }) });
        desconectados += dados.desconectados || 0;
      } catch (erro) {
        falhas.push(`${mapa.mapa}: ${erro.message}`);
      }
    }
  } finally { estado.enviando = false; }
  estado.mapasMarcados.clear();
  const feitos = alvos.length - falhas.length;
  const texto = `${feitos} de ${alvos.length} ${ligar ? "ligado" : "desligado"}${feitos === 1 ? "" : "s"}.`
    + (!ligar && desconectados ? ` ${desconectados} jogador(es) desconectado(s).` : "")
    + (ligar && feitos ? " Os mapas sobem em segundo plano." : "")
    + (falhas.length ? ` Falhas: ${falhas.join("; ")}` : "");
  alerta(falhas.length ? (feitos ? "aviso" : "erro") : "ok", ligar ? "Ligar mapas" : "Desligar mapas", texto);
  await acompanharMapas(realm);
}

// Reconsulta o estado até nenhum mapa estar carregando (até ~45 s).
async function acompanharMapas(realm) {
  for (let i = 0; i < 30; i++) {
    try { estado.online[realm.id] = await api(`/api/realms/${encodeURIComponent(realm.id)}/estado`); } catch (_) { /* tenta de novo */ }
    if (estado.selecionado?.id === realm.id && estado.pagina === "mapas") renderizarMapas();
    if (!(estado.online[realm.id]?.mapas || []).some((m) => m.carregando) || !temTempo) return;
    await new Promise((r) => setTimeout(r, 1500));
  }
}

/* ---------------- Rates do realm (E7) ---------------- */

function preencherRates() {
  const realm = estado.selecionado;
  if (!realm) return;
  const vivo = estado.online[realm.id]?.taxas;
  const valores = vivo || realm.rates || {};
  for (const [chave, id] of CAMPOS_RATES) elemento(id).value = valores[chave] ?? 1;
  const situacao = situacaoDoMundo(realm);
  elemento("origem-rates").textContent = vivo
    ? (mesmasRates(vivo, realm.rates) ? "Valores em vigor no servidor de mundo" : "Em vigor no mundo; diferente do que está gravado")
    : { sem_canal: "Realm sem canal administrativo: não é possível aplicar",
        consultando: "Consultando o mundo…",
        sem_resposta: "Servidor de mundo sem resposta: mostrando o que está gravado" }[situacao] || "";
  elemento("salvar-rates").disabled = situacao !== "ok";
}

async function salvarRates() {
  const realm = estado.selecionado;
  if (!realm || estado.enviando) return;
  const corpo = Object.fromEntries(CAMPOS_RATES.map(([chave, id]) => [chave, Math.round(Number(elemento(id).value) * 10) / 10]));
  if (Object.values(corpo).some((v) => !Number.isFinite(v) || v < 0.1 || v > 99.9)) {
    alerta("erro", "Valor inválido", "Cada rate vai de 0,1× a 99,9×.");
    return;
  }
  estado.enviando = true;
  elemento("salvar-rates").disabled = true;
  alerta("carregando", "Aplicando rates…", realm.nome);
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/rates`, { method: "POST", body: JSON.stringify(corpo) });
    realm.rates = { ...dados.taxas };
    if (estado.online[realm.id]) estado.online[realm.id].taxas = { ...dados.taxas };
    alerta("ok", "Rates aplicadas", `EXP ${dados.taxas.exp}× · SP ${dados.taxas.sp}× · DROP ${dados.taxas.drop}× · MOEDAS ${dados.taxas.moedas}×`);
    renderizarRealms();
    renderizarRealm();
    preencherRates();
  } catch (erro) {
    alerta("erro", "Rates não aplicadas", erro.message);
  } finally {
    estado.enviando = false;
    elemento("salvar-rates").disabled = !realm.canal_administrativo;
  }
}

function abrirMenu() { elemento("lateral").classList.add("aberta"); elemento("cortina").hidden = false; }
function fecharMenu() { elemento("lateral").classList.remove("aberta"); elemento("cortina").hidden = true; }

function preencherSeletor() {
  const seletor = elemento("realm-selecionado");
  seletor.replaceChildren();
  const vazio = criar("option", "", "Selecionar realm…");
  vazio.value = "";
  seletor.append(vazio);
  for (const realm of estado.realms) {
    const opcao = criar("option", "", `${realm.nome} · ${realm.versao}`);
    opcao.value = realm.id;
    seletor.append(opcao);
  }
  seletor.value = estado.selecionado?.id || "";
}

function selecionar(id, irPara = "realm") {
  estado.geracao++;
  estado.busca++;
  estado.ficha++;
  estado.personagemId = null;
  elemento("lista-personagens").replaceChildren();
  elemento("ficha-personagem").hidden = true;
  elemento("aviso-personagens").hidden = true;
  estado.selecionado = estado.realms.find((realm) => realm.id === id) || null;
  elemento("realm-selecionado").value = estado.selecionado?.id || "";
  document.querySelectorAll(".cartao-realm").forEach((cartao) => {
    cartao.classList.toggle("selecionado", cartao.dataset.realm === estado.selecionado?.id);
  });
  elemento("grupo-realm").hidden = !estado.selecionado;
  elemento("rotulo-realm-nav").textContent = estado.selecionado?.versao || "";
  if (!estado.selecionado) { mudarPagina(estado.pagina === "contas" ? "contas" : "inicio"); return; }
  renderizarRealm();
  if (irPara) mudarPagina(irPara);
}

/* ---------------- Visão geral ---------------- */

function rates(realm) {
  const grade = criar("div", "grade-rates");
  for (const [rotulo, chave] of [["EXP", "exp"], ["SP", "sp"], ["DROP", "drop"], ["MOEDAS", "moedas"]]) {
    const caixa = criar("div", "rate");
    const valor = realm.rates ? `${Number(realm.rates[chave]).toLocaleString("pt-BR")}×` : "—";
    caixa.append(criar("span", "", rotulo), criar("strong", "", valor));
    grade.append(caixa);
  }
  return grade;
}

function renderizarRealms() {
  const lista = elemento("lista-realms");
  lista.replaceChildren();
  for (const realm of estado.realms) {
    const cartao = criar("button", "cartao-realm");
    cartao.type = "button";
    cartao.dataset.realm = realm.id;
    const topo = criar("div", "realm-topo");
    const nome = criar("div");
    nome.append(criar("h3", "", realm.nome), criar("p", "realm-sub", realm.nome.includes(realm.versao) ? `Porta ${realm.porta}` : `${realm.versao} · porta ${realm.porta}`));
    const acessivel = realm.gateway === "acessivel";
    const situacao = criar("span", `etiqueta ${acessivel ? "" : "etiqueta-erro"}`);
    situacao.append(criar("span", `ponto ${acessivel ? "ponto-ok" : "ponto-erro"}`), acessivel ? "No ar" : "Fora do ar");
    topo.append(nome, situacao);
    const numeros = criar("div", "realm-estatisticas");
    const online = criar("div", "online");
    const valorOnline = criar("strong", "", realm.canal_administrativo ? "…" : "—");
    valorOnline.dataset.online = realm.id;
    if (!realm.canal_administrativo) { online.title = "Sem canal administrativo com o mundo"; online.classList.add("sem-canal"); }
    online.append(criar("span", "", "Online"), valorOnline);
    const personagens = criar("div");
    personagens.append(criar("span", "", "Personagens"), criar("strong", "", fmt(realm.personagens_persistidos)));
    numeros.append(online, personagens);
    cartao.append(topo, numeros, rates(realm));
    cartao.classList.toggle("selecionado", realm.id === estado.selecionado?.id);
    cartao.addEventListener("click", () => selecionar(realm.id));
    lista.append(cartao);
  }
  if (!estado.realms.length) lista.append(criar("p", "nota-vazia", "Nenhum realm configurado."));
  elemento("total-realms").textContent = fmt(estado.realms.length);
  elemento("total-personagens").textContent = fmt(estado.realms.reduce((s, r) => s + r.personagens_persistidos, 0));
}

async function consultarOnline(geracao) {
  const consultas = estado.realms.filter((r) => r.canal_administrativo).map(async (realm) => {
    try {
      const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/estado`);
      if (estado.geracao !== geracao) return;
      estado.online[realm.id] = dados;
    } catch (_) {
      estado.online[realm.id] = { estado: "desconhecido", jogadores_online: null, mapas: [] };
    }
    if (estado.online[realm.id].estado === "em_transicao" && temTempo && !realm.reconsultado) {
      realm.reconsultado = true;
      setTimeout(() => reconsultar(realm), 2000);
    }
    const alvo = document.querySelector?.(`[data-online="${realm.id}"]`);
    if (alvo) alvo.textContent = fmt(estado.online[realm.id].jogadores_online);
    if (estado.selecionado?.id === realm.id) {
      renderizarRealm();
      if (estado.pagina === "rates") preencherRates();
      if (estado.pagina === "mapas") renderizarMapas();
    }
  });
  await Promise.all(consultas);
  if (estado.geracao !== geracao) return;
  const valores = Object.values(estado.online).map((d) => d.jogadores_online).filter((v) => v !== null && v !== undefined);
  elemento("total-online").textContent = valores.length ? fmt(valores.reduce((a, b) => a + b, 0)) : "—";
}

/* ---------------- Painel do realm ---------------- */

function renderizarRealm() {
  const realm = estado.selecionado;
  if (!realm) return;
  const vivo = estado.online[realm.id];
  const ficha = elemento("ficha-realm");
  ficha.replaceChildren();
  for (const [rotulo, valor] of [
    ["Online", vivo ? fmt(vivo.jogadores_online) : (realm.canal_administrativo ? "…" : "—")],
    ["Personagens", fmt(realm.personagens_persistidos)],
    ["Mapas no ar", vivo?.estado === "consultado" ? fmt(vivo.mapas.length) : "—"],
    ["Gateway", realm.gateway === "acessivel" ? "No ar" : "Fora do ar"],
  ]) {
    const caixa = criar("article", "numero");
    caixa.append(criar("span", "", rotulo), criar("strong", "", valor));
    ficha.append(caixa);
  }
  elemento("rates-realm").replaceChildren(...rates(realm).children);
  const selo = elemento("estado-rates");
  if (vivo?.taxas) {
    const iguais = mesmasRates(vivo.taxas, realm.rates);
    selo.textContent = iguais ? "Em vigor" : "Diferente no mundo";
    selo.className = `etiqueta ${iguais ? "" : "etiqueta-ouro"}`;
    selo.title = iguais ? "O servidor de mundo usa estes valores" : "O mundo usa outros valores; reaplique";
  } else {
    selo.textContent = { sem_canal: "Sem canal", consultando: "Consultando…", sem_resposta: "Mundo sem resposta" }[situacaoDoMundo(realm)];
    selo.className = "etiqueta etiqueta-neutra";
    selo.title = "";
  }
  const mapas = elemento("lista-mapas");
  mapas.replaceChildren();
  if (vivo?.estado === "consultado" && vivo.mapas.length) {
    for (const mapa of vivo.mapas) {
      const item = criar("li");
      item.append(criar("span", "", `Mapa ${mapa.mapa}`), criar("span", "etiqueta", `${fmt(mapa.jogadores_online)} online`));
      mapas.append(item);
    }
  } else if (situacaoDoMundo(realm) === "sem_resposta") {
    const item = criar("li");
    item.append(avisoSemResposta(realm));
    mapas.append(item);
  } else {
    mapas.append(criar("li", "texto-suave", realm.canal_administrativo ? "Consultando…" : "Sem canal administrativo"));
  }
  const nomes = { disponivel: ["Disponível", ""], implementado: ["Implementado", ""], nao_validado: ["Não validado", "etiqueta-neutra"], indisponivel: ["Em breve", "etiqueta-neutra"] };
  const recursos = elemento("lista-capacidades");
  recursos.replaceChildren();
  for (const recurso of realm.capacidades) {
    let [nome, classe] = nomes[recurso.estado] || [recurso.estado, "etiqueta-neutra"];
    if (recurso.id === "consulta_viva" && vivo?.estado === "consultado") [nome, classe] = ["Disponível", ""];
    const linha = criar("div", "recurso");
    linha.title = recurso.detalhe;
    linha.append(criar("strong", "", recurso.nome), criar("span", `etiqueta ${classe}`, nome));
    recursos.append(linha);
  }
}

/* ---------------- Personagens ---------------- */

async function buscarPersonagens() {
  const realm = estado.selecionado;
  if (!realm) return;
  const busca = ++estado.busca;
  estado.ficha++;
  estado.personagemId = null;
  elemento("ficha-personagem").hidden = true;
  elemento("lista-personagens").replaceChildren();
  elemento("aviso-personagens").hidden = true;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens?busca=${encodeURIComponent(elemento("nome-personagem").value)}`);
    if (estado.busca !== busca || estado.selecionado?.id !== realm.id || !estado.sessao) return;
    if (!dados.personagens.length) elemento("lista-personagens").append(criar("p", "nota-vazia", "Nenhum personagem encontrado."));
    for (const personagem of dados.personagens) {
      const botao = criar("button", "personagem");
      botao.type = "button";
      const nome = criar("div");
      nome.append(criar("strong", "", personagem.nome));
      botao.append(nome, criar("span", "", `Nv. ${personagem.nivel} · #${personagem.id}`));
      botao.addEventListener("click", () => {
        document.querySelectorAll(".personagem").forEach((b) => b.classList.toggle("selecionado", b === botao));
        consultarPersonagem(realm.id, personagem.id);
      });
      elemento("lista-personagens").append(botao);
    }
  } catch (erro) {
    if (estado.busca !== busca || !estado.sessao) return;
    elemento("aviso-personagens").textContent = erro.message;
    elemento("aviso-personagens").hidden = false;
  }
}

async function consultarPersonagem(realmId, id) {
  const consulta = ++estado.ficha;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realmId)}/personagens/${id}`);
    if (estado.ficha !== consulta || estado.selecionado?.id !== realmId || !estado.sessao) return;
    const ficha = dados.ficha;
    elemento("titulo-personagem").textContent = ficha.nome;
    const presencas = { online: "Online agora", ausente_nos_daemons: "Offline", em_transicao: "Entrando ou saindo", desconhecida: "Presença desconhecida" };
    elemento("presenca-personagem").textContent = `#${ficha.id} · ${presencas[dados.presenca] || "Presença desconhecida"}`;
    elemento("origem-personagem").textContent = dados.origem === "viva" ? "Ao vivo" : "Salvo no banco";
    elemento("origem-personagem").className = `etiqueta ${dados.origem === "viva" ? "" : "etiqueta-neutra"}`;
    elemento("observacao-personagem").textContent = dados.aviso || "";
    const inteiro = (valor) => BigInt(valor).toLocaleString("pt-BR");
    const campos = [["Nível", ficha.nivel], ["Classe", ficha.classe], ["Cultivo", ficha.cultivo],
      ["EXP", inteiro(ficha.exp)], ["Alma", inteiro(ficha.alma)], ["Dinheiro", inteiro(ficha.dinheiro)],
      ["Vida", ficha.vida], ["Mana", ficha.mana], ["Pontos livres", ficha.pontos],
      ["Força", ficha.forca], ["Agilidade", ficha.agilidade], ["Vitalidade", ficha.vitalidade],
      ["Energia", ficha.energia], ["Mapa", ficha.mapa],
      ["Posição", `${ficha.posicao.x.toFixed(0)}, ${ficha.posicao.y.toFixed(0)}, ${ficha.posicao.z.toFixed(0)}`]];
    elemento("dados-personagem").replaceChildren();
    for (const [rotulo, valor] of campos) {
      const campo = criar("dl");
      campo.append(criar("dt", "", rotulo), criar("dd", "", valor));
      elemento("dados-personagem").append(campo);
    }
    elemento("ficha-personagem").hidden = false;
    estado.personagemId = id;
    const online = dados.presenca === "online";
    elemento("dica-edicao").textContent = online
      ? "Em jogo: aplica na hora e o jogador vê a mudança."
      : "Fora do jogo: grava no banco; EXP/SP só com o personagem em jogo.";
    elemento("form-exp").querySelector?.("button")?.toggleAttribute?.("disabled", !online);
    preencherCultivos(estado.selecionado?.versao, ficha.cultivo);
    preencherAtributos(ficha);
    preencherPosicao(ficha);
  } catch (erro) {
    if (estado.ficha !== consulta || !estado.sessao) return;
    elemento("aviso-personagens").textContent = erro.message;
    elemento("aviso-personagens").hidden = false;
  }
}

/* ---------------- Edição de personagem (E5) ---------------- */

/* Cultivos que o cliente mostra: 0–8 em todas; 20–22 e 30–32 só no 1.5.5 (B182). */
function preencherCultivos(versao, atual) {
  const lista = elemento("cultivo-valor");
  const grupos = [["Base", [0, 1, 2, 3, 4, 5, 6, 7, 8]]];
  if (versao !== "1.2.6") grupos.push(["Deus", [20, 21, 22]], ["Demônio", [30, 31, 32]]);
  lista.replaceChildren();
  for (const [rotulo, valores] of grupos) {
    const grupo = criar("optgroup");
    grupo.label = rotulo;
    for (const v of valores) {
      const opcao = criar("option", "", String(v));
      opcao.value = String(v);
      opcao.selected = v === Number(atual);
      grupo.append(opcao);
    }
    lista.append(grupo);
  }
}

const MOTIVOS_EDICAO = {
  precisa_estar_online: "EXP e SP só com o personagem em jogo.",
  nivel_invalido: "O nível novo tem de ser maior que o atual e até o teto do realm.",
  nivel_invalido_ou_personagem_inexistente: "O nível novo tem de ser maior que o atual (ou o personagem não existe).",
  cultivo_invalido: "Cultivo que esta versão não tem.",
  atributos_invalidos: "Algum atributo ficaria abaixo do mínimo ou a soma passa do total (atributos + livres).",
  atributos_invalidos_ou_personagem_inexistente: "Algum atributo ficaria abaixo do mínimo ou a soma passa do total (ou o personagem não existe).",
  sem_mudanca: "Os atributos já estão assim.",
  mapa_indisponivel: "O mapa de destino não está ligado neste realm.",
  fora_do_mapa: "Essas coordenadas ficam fora do terreno do mapa.",
  altura_obrigatoria: "Este mapa não tem terreno carregado: informe o Y.",
  em_transicao: "O personagem está entrando ou saindo agora; tente de novo em instantes.",
  personagem_inexistente: "O personagem não existe mais.",
  operacao_em_conflito: "Esta operação já foi usada com outros valores.",
  edicao_invalida: "Valores inválidos.",
  banco_indisponivel: "O banco não respondeu; nada foi alterado.",
};

async function editarPersonagem(corpo, descricao) {
  const realm = estado.selecionado;
  const id = estado.personagemId;
  if (!realm || !id || estado.enviando) return;
  const operacao = Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0")).join("");
  estado.enviando = true;
  alerta("carregando", "Aplicando…", descricao);
  try {
    let dados;
    try {
      dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/editar`,
        { method: "POST", signal: AbortSignal.timeout(8000), body: JSON.stringify({ operacao_id: operacao, ...corpo }) });
    } catch (erro) {
      if (erro.resultado?.estado === "falha") throw erro;
      dados = { estado: "desconhecido" };
    }
    // Sem confirmação: consulta o resultado pelo mesmo ID algumas vezes (não reaplica).
    for (let i = 0; dados.estado === "desconhecido" && i < 10 && temTempo; i++) {
      await new Promise((r) => setTimeout(r, 1500));
      try { dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/operacoes/${operacao}`); } catch (_) { /* tenta de novo */ }
    }
    if (dados.estado === "aplicado" || dados.estado === "salvo") {
      const subiu = dados.nivel > dados.nivel_antes ? ` Subiu para o nível ${dados.nivel}.` : "";
      alerta("ok", dados.estado === "aplicado" ? "Aplicado no jogo" : "Gravado",
        `${descricao}.${dados.dinheiro ? ` Dinheiro agora: ${BigInt(dados.dinheiro).toLocaleString("pt-BR")}.` : ""}${subiu}`);
      for (const campo of ["din-valor", "exp-valor", "sp-valor", "pontos-valor", "nivel-valor"]) elemento(campo).value = "";
      consultarPersonagem(realm.id, id);
    } else if (dados.estado === "desconhecido") {
      alerta("aviso", "Sem confirmação", `Operação ${operacao}: o servidor não confirmou. Confira a ficha antes de repetir.`);
    } else {
      alerta("erro", "Não aplicado", MOTIVOS_EDICAO[dados.codigo] || dados.mensagem || dados.codigo);
    }
  } catch (erro) {
    alerta("erro", "Não aplicado", MOTIVOS_EDICAO[erro.resultado?.codigo] || erro.message);
  } finally { estado.enviando = false; }
}

function editarDinheiro() {
  const valor = Math.trunc(Number(elemento("din-valor").value));
  if (!Number.isFinite(valor) || valor < 1 || valor > 2_000_000_000) { alerta("erro", "Valor inválido", "Informe de 1 a 2.000.000.000 moedas."); return; }
  editarPersonagem({ dinheiro: valor }, `Dado ${valor.toLocaleString("pt-BR")} de dinheiro`);
}

function editarExp() {
  const exp = Math.trunc(Number(elemento("exp-valor").value || 0));
  const sp = Math.trunc(Number(elemento("sp-valor").value || 0));
  if (![exp, sp].every((v) => Number.isFinite(v) && v >= 0 && v <= 2_000_000_000) || exp + sp <= 0) {
    alerta("erro", "Valor inválido", "Informe EXP e/ou SP a dar."); return;
  }
  editarPersonagem({ ...(exp ? { exp } : {}), ...(sp ? { sp } : {}) },
    `Dado ${exp.toLocaleString("pt-BR")} de EXP e ${sp.toLocaleString("pt-BR")} de SP`);
}

/* Atributos (B184): os quatro atuais nos campos; a dica mostra quantos livres sobram. O
   total (atributos + livres) não muda: para subir além dele, dar pontos livres antes. */
const CAMPOS_DE_ATRIBUTO = ["attr-forca", "attr-agilidade", "attr-vitalidade", "attr-energia"];
function preencherAtributos(ficha) {
  const atuais = [ficha.forca, ficha.agilidade, ficha.vitalidade, ficha.energia].map(Number);
  estado.totalDeAtributos = atuais.reduce((t, v) => t + v, 0) + Math.max(0, Number(ficha.pontos) || 0);
  CAMPOS_DE_ATRIBUTO.forEach((id, i) => { elemento(id).value = String(atuais[i]); });
  atualizarSobraDeAtributos();
}
function atualizarSobraDeAtributos() {
  const soma = CAMPOS_DE_ATRIBUTO.reduce((t, id) => t + (Math.trunc(Number(elemento(id).value)) || 0), 0);
  const sobra = (estado.totalDeAtributos ?? 0) - soma;
  elemento("sobra-atributos").textContent = sobra >= 0
    ? `Pontos livres depois: ${sobra.toLocaleString("pt-BR")}.`
    : `Passou ${(-sobra).toLocaleString("pt-BR")} do total; dê pontos livres antes.`;
}
function editarAtributos() {
  const valores = CAMPOS_DE_ATRIBUTO.map((id) => Math.trunc(Number(elemento(id).value)));
  if (!valores.every((v) => Number.isFinite(v) && v >= 0 && v <= 100_000)) { alerta("erro", "Valor inválido", "Informe os quatro atributos."); return; }
  editarPersonagem({ atributos: valores }, `Atributos ${valores.join(" / ")}`);
}
function redistribuirAtributos() {
  editarPersonagem({ redistribuir: true }, "Atributos devolvidos aos pontos livres");
}

/* Posição (E6, B185): destino entre os mapas ligados do realm; x/y/z começam na posição
   atual. Y vazio = o chão do destino (o GS sobe para o chão o que estiver abaixo). */
function preencherPosicao(ficha) {
  const realm = estado.selecionado;
  if (!realm) return;
  if (!estado.catalogoMapas[realm.id]) carregarCatalogoDeMapas(realm);
  const nomes = new Map((estado.catalogoMapas[realm.id] || []).map((m) => [m.mapa, m.nome]));
  const ligados = (estado.online[realm.id]?.mapas || []).filter((m) => m.ligado !== false && !m.carregando)
    .map((m) => m.mapa).sort((a, b) => a - b);
  if (!ligados.includes(Number(ficha.mapa))) ligados.unshift(Number(ficha.mapa));
  const lista = elemento("pos-mapa");
  lista.replaceChildren();
  for (const mapa of ligados) {
    const opcao = criar("option", "", nomes.has(mapa) ? `${mapa} · ${nomes.get(mapa)}` : `Mapa ${mapa}`);
    opcao.value = String(mapa);
    opcao.selected = mapa === Number(ficha.mapa);
    lista.append(opcao);
  }
  elemento("pos-x").value = String(Number(ficha.posicao.x).toFixed(1));
  elemento("pos-y").value = String(Number(ficha.posicao.y).toFixed(1));
  elemento("pos-z").value = String(Number(ficha.posicao.z).toFixed(1));
}
function editarPosicao() {
  const mapa = Number(elemento("pos-mapa").value);
  const [x, z] = ["pos-x", "pos-z"].map((id) => Number(elemento(id).value));
  const textoY = elemento("pos-y").value.trim();
  const y = textoY === "" ? null : Number(textoY);
  const valido = (v) => Number.isFinite(v) && Math.abs(v) <= 100_000;
  if (!Number.isInteger(mapa) || mapa < 1 || !valido(x) || !valido(z) || (y !== null && !valido(y))) {
    alerta("erro", "Valor inválido", "Informe o mapa e as coordenadas X e Z (Y vazio = chão)."); return;
  }
  const posicao = { mapa, x, z, ...(y === null ? {} : { y }) };
  editarPersonagem({ posicao }, `Mover para o mapa ${mapa} (${x.toFixed(0)}, ${z.toFixed(0)})`);
}

function editarPontos() {
  const valor = Math.trunc(Number(elemento("pontos-valor").value));
  if (!Number.isFinite(valor) || valor < 1 || valor > 10_000) { alerta("erro", "Valor inválido", "Informe de 1 a 10.000 pontos."); return; }
  editarPersonagem({ pontos: valor }, `Dados ${valor.toLocaleString("pt-BR")} pontos livres`);
}

function editarNivel() {
  const valor = Math.trunc(Number(elemento("nivel-valor").value));
  if (!Number.isFinite(valor) || valor < 2) { alerta("erro", "Valor inválido", "Informe o nível novo."); return; }
  editarPersonagem({ nivel: valor }, `Nível ${valor}`);
}

function editarCultivo() {
  const valor = Number(elemento("cultivo-valor").value);
  if (!Number.isInteger(valor)) return;
  editarPersonagem({ cultivo: valor }, `Cultivo ${valor}`);
}

/* ---------------- Contas globais ---------------- */

const corAvatar = (id) => `c${(Number(id) % 6) + 1}`;
function gold(unidades) {
  try {
    const valor = BigInt(unidades);
    const inteiro = valor / 100n;
    const resto = (valor < 0n ? -valor : valor) % 100n;
    return `${inteiro.toLocaleString("pt-BR")}${resto ? "," + String(resto).padStart(2, "0") : ""}`;
  } catch (_) { return String(unidades); }
}
const data = (iso) => (iso ? new Date(iso).toLocaleDateString("pt-BR") : "nunca");

async function buscarContas(pagina = estado.paginaContas) {
  const geracao = ++estado.buscaContas;
  estado.paginaContas = Math.max(1, pagina);
  const lista = elemento("lista-contas");
  if (document.createElement && !lista.children.length) {
    lista.replaceChildren(...Array.from({ length: 6 }, () => criar("div", "esqueleto")));
  }
  try {
    const termo = encodeURIComponent(elemento("nome-conta").value.trim());
    const dados = await api(`/api/contas?busca=${termo}&pagina=${estado.paginaContas}&por_pagina=${estado.porPagina}`);
    if (geracao !== estado.buscaContas || !estado.sessao) return;
    estado.totalContas = dados.total ?? dados.contas.length;
    lista.replaceChildren();
    for (const conta of dados.contas) {
      lista.append(cartaoConta(conta));
      if (estado.conta?.id === conta.id) estado.conta = conta;
    }
    if (!dados.contas.length) lista.append(criar("p", "nota-vazia", "Nenhuma conta encontrada."));
    const paginas = Math.max(1, Math.ceil(estado.totalContas / estado.porPagina));
    elemento("resumo-contas").textContent = `${fmt(estado.totalContas)} conta${estado.totalContas === 1 ? "" : "s"}`;
    elemento("indicador-pagina").textContent = `Página ${estado.paginaContas} de ${paginas}`;
    elemento("pagina-anterior").disabled = estado.paginaContas <= 1;
    elemento("pagina-seguinte").disabled = estado.paginaContas >= paginas;
    if (!elemento("nome-conta").value.trim()) elemento("total-contas").textContent = fmt(estado.totalContas);
  } catch (erro) {
    if (geracao === estado.buscaContas) { lista.replaceChildren(); aviso(erro.message); }
  }
}

function cartaoConta(conta) {
  const botao = criar("button", "cartao-conta");
  botao.type = "button";
  botao.dataset.conta = conta.id;
  const avatar = criar("span", `avatar ${corAvatar(conta.id)}`, conta.usuario.slice(0, 1));
  const nome = criar("div");
  nome.append(criar("div", "nome", conta.usuario), criar("div", "id", `#${conta.id} · ${fmt(conta.personagens ?? 0)} personagens`));
  const rodape = criar("div", "rodape");
  if (conta.gm > 0) rodape.append(criar("span", "etiqueta etiqueta-ouro", "GM"));
  if (conta.banida) rodape.append(criar("span", "etiqueta etiqueta-erro", "Banida"));
  if (!(conta.gm > 0) && !conta.banida) rodape.append(criar("span", "etiqueta etiqueta-neutra", "Jogador"));
  rodape.append(criar("span", "gold", `${gold(conta.gold)} gold`));
  botao.append(avatar, nome, rodape);
  botao.addEventListener("click", (evento) => { evento.stopPropagation?.(); abrirConta(conta, botao); });
  return botao;
}

function posicionarPopover() {
  const popover = elemento("popover-conta");
  if (!estado.cartao || popover.hidden || !estado.cartao.getBoundingClientRect) return;
  const caixa = estado.cartao.getBoundingClientRect();
  const largura = popover.offsetWidth || 300;
  const altura = popover.offsetHeight || 260;
  const esquerda = Math.max(8, Math.min(caixa.left, window.innerWidth - largura - 8));
  let topo = caixa.bottom + 8;
  if (topo + altura > window.innerHeight - 8) topo = Math.max(8, caixa.top - altura - 8);
  popover.style.left = `${esquerda}px`;
  popover.style.top = `${topo}px`;
}

function abrirConta(conta, cartao = null) {
  if (estado.enviando || (estado.comando && estado.comando.conta_id !== conta.id)) {
    alerta("aviso", "Operação em andamento", "Aguarde a operação atual terminar antes de mexer em outra conta.");
    return;
  }
  estado.conta = conta;
  estado.cartao = cartao;
  document.querySelectorAll(".cartao-conta").forEach((c) => c.classList.toggle("selecionado", Number(c.dataset.conta) === conta.id));
  elemento("gaveta-avatar").textContent = conta.usuario.slice(0, 1);
  elemento("gaveta-avatar").className = `avatar ${corAvatar(conta.id)}`;
  elemento("gaveta-usuario").textContent = conta.usuario;
  elemento("gaveta-meta").textContent = `#${conta.id}${conta.gm > 0 ? " · GM" : ""}${conta.banida ? " · Banida" : ""}`;
  const dados = elemento("dados-conta");
  dados.replaceChildren();
  for (const [rotulo, valor] of [["Gold", gold(conta.gold)], ["Personagens", fmt(conta.personagens ?? 0)],
    ["Último login", data(conta.ultimo_login)]]) {
    const item = criar("div");
    item.append(criar("dt", "", rotulo), criar("dd", "", valor));
    dados.append(item);
  }
  elemento("gm-habilitado").value = String(!(conta.gm > 0));
  elemento("nova-senha").value = "";
  elemento("gold-valor").value = "";
  elemento("ban-motivo").value = "";
  const banir = !conta.banida;
  elemento("rotulo-ban").textContent = banir ? "Banir" : "Desbanir";
  elemento("titulo-ban").textContent = banir ? "Banir" : "Desbanir";
  elemento("ban-motivo").hidden = !banir;
  elemento("dica-ban").textContent = banir ? "Sai do jogo agora e não entra mais" : "Volta a poder entrar no jogo";
  elemento("aplicar-ban").textContent = banir ? "Banir conta" : "Desbanir conta";
  elemento("aplicar-ban").className = `botao ${banir ? "botao-perigo" : "botao-primario"} botao-largo`;
  trocarAba("resumo");
  atualizarComando(false);
  elemento("popover-conta").hidden = false;
  posicionarPopover();
}

function fecharConta() {
  elemento("popover-conta").hidden = true;
  if (!estado.comando) estado.conta = null;
  estado.cartao = null;
  document.querySelectorAll(".cartao-conta").forEach((c) => c.classList.remove("selecionado"));
}

function trocarAba(aba) {
  estado.aba = aba;
  document.querySelectorAll(".painel-aba").forEach((p) => { p.hidden = p.dataset.painel !== aba; });
  posicionarPopover();
  const foco = { senha: "nova-senha", gold: "gold-valor", ban: "ban-motivo" }[aba];
  if (foco) elemento(foco).focus?.();
}

async function desconectarConta() {
  const conta = estado.conta;
  if (!conta || estado.enviando) return;
  estado.enviando = true;
  elemento("aplicar-desconectar").disabled = true;
  alerta("carregando", "Desconectando…", `Salvando os personagens de ${conta.usuario}.`);
  try {
    const dados = await api(`/api/contas/${conta.id}/desconectar`, { method: "POST" });
    fecharConta();
    const n = dados.desconectados;
    alerta(dados.estado === "aplicado" ? "ok" : "aviso",
      n ? "Conta desconectada" : "Ninguém online",
      n ? `${n} personagem${n === 1 ? "" : "s"} de ${conta.usuario} voltaram à tela de login.` : `${conta.usuario} não estava em jogo.`);
  } catch (erro) {
    alerta("erro", "Não foi possível desconectar", erro.message);
  } finally {
    estado.enviando = false;
    elemento("aplicar-desconectar").disabled = false;
  }
}

/* ---------------- Comandos recuperáveis (senha, criação, GM, gold, ban) ---------------- */

// Rota do canal para operações globais: o realm selecionado, se tiver canal; senão o
// primeiro que tenha. A conta é global, o realm só escolhe por qual GS o pedido entra.
function rotaContas() {
  if (estado.selecionado && estado.selecionado.canal_administrativo !== false) return estado.selecionado;
  return estado.realms.find((realm) => realm.canal_administrativo) || null;
}

const ROTA_DO_TIPO = { trocar_senha: "senha", definir_gm: "gm", ajustar_gold: "gold", definir_ban: "ban" };
const NOMES_TIPO = { trocar_senha: "troca de senha", criar_conta: "criação de conta", definir_gm: "alteração de GM",
  ajustar_gold: "ajuste de gold", definir_ban: "banimento" };

function chaveComando() { return `pw-admin:comando:${estado.sessao.conta_id}`; }
function lerComando() {
  try {
    const comando = JSON.parse(localStorage.getItem(chaveComando()));
    const tipo = comando?.tipo || "trocar_senha";
    const daConta = Number.isInteger(comando?.conta_id) && comando.conta_id > 0;
    if (comando && /^[A-Za-z0-9_-]{1,64}$/.test(comando.id) && typeof comando.realm === "string" &&
        typeof comando.usuario === "string" && (
          (tipo === "criar_conta" && /^[a-z0-9_]{1,64}$/.test(comando.usuario)) ||
          (tipo === "trocar_senha" && daConta) ||
          (tipo === "definir_gm" && daConta && typeof comando.habilitado === "boolean") ||
          (tipo === "ajustar_gold" && daConta && Number.isInteger(comando.delta) && comando.delta > 0) ||
          (tipo === "definir_ban" && daConta && typeof comando.banida === "boolean" &&
            (comando.motivo === null || typeof comando.motivo === "string")))) return comando;
  } catch (_) { /* Sem registro recuperável neste navegador. */ }
  return null;
}

function atualizarComando(mostrarAviso = true) {
  const tipo = estado.comando?.tipo || (estado.comando ? "trocar_senha" : null);
  const criacao = tipo === "criar_conta";
  const alvo = criacao ? null : estado.comando || estado.conta;
  const outro = (meu) => !!(estado.comando && tipo !== meu);
  elemento("alvo-senha").textContent = criacao ? `Criação pendente: ${estado.comando.usuario}` : alvo ? `Conta ${alvo.usuario}` : "";
  elemento("aplicar-senha").disabled = !alvo || estado.enviando || criacao || outro("trocar_senha");
  elemento("aplicar-senha").textContent = estado.comando && tipo === "trocar_senha" ? "Repetir a mesma operação" : "Trocar senha";
  elemento("aplicar-gm").disabled = !alvo || estado.enviando || outro("definir_gm");
  elemento("aplicar-gm").textContent = tipo === "definir_gm" ? "Repetir a mesma alteração" : "Aplicar";
  elemento("gm-habilitado").disabled = estado.enviando || !!estado.comando;
  if (tipo === "definir_gm") elemento("gm-habilitado").value = String(estado.comando.habilitado);
  elemento("aplicar-gold").disabled = !alvo || estado.enviando || outro("ajustar_gold");
  elemento("aplicar-ban").disabled = !alvo || estado.enviando || outro("definir_ban");
  elemento("recuperar-comando").disabled = estado.enviando;
  elemento("aplicar-criacao").disabled = estado.enviando || !!(estado.comando && !criacao);
  elemento("aplicar-criacao").textContent = criacao ? "Repetir a mesma criação" : "Criar conta";
  elemento("usuario-novo").readOnly = criacao || estado.enviando;
  if (criacao) elemento("usuario-novo").value = estado.comando.usuario;
  const faixa = estado.comando && !estado.enviando;
  elemento("faixa-comando").hidden = !faixa;
  if (faixa) elemento("texto-faixa-comando").textContent = `Aguardando confirmação: ${NOMES_TIPO[tipo]} · ${estado.comando.usuario}`;
  if (estado.enviando) alerta("carregando", "Aplicando…", `Enviando ${NOMES_TIPO[tipo] || "operação"} ao servidor.`);
  else if (estado.comando && mostrarAviso) {
    alerta("aviso", "Aguardando confirmação",
      `Sem confirmação do servidor para a ${NOMES_TIPO[tipo]}. O painel continua consultando; ao repetir, use os mesmos dados.`);
  }
}

function concluir(tipo, titulo, texto) {
  alerta(tipo, titulo, texto);
  if (tipo === "ok") fecharConta();
}

function encerrarComando() {
  localStorage.removeItem(chaveComando());
  estado.comando = null;
  pararAcompanhamento();
}

function mostrarResultadoComando(dados, comando) {
  if (estado.comando?.id !== comando.id) return;
  if (dados.codigo === "operacao_em_conflito") {
    atualizarComando(false);
    concluir("erro", "Dados diferentes", `Os parâmetros da repetição são diferentes dos originais. A operação ${comando.id} foi preservada e continua sendo consultada.`);
    return;
  }
  if (["administrador_recusado", "realm_incorreto", "alvo_invalido"].includes(dados.codigo)) {
    atualizarComando(false);
    concluir("erro", "Envio recusado", `O servidor recusou o envio (${dados.codigo}). A operação foi preservada; consulte o resultado antes de repetir.`);
    return;
  }
  if (["pendente", "aplicado", "substituido"].includes(dados.estado)) {
    if (comando.tipo !== "definir_gm" || dados.tipo !== "definir_gm" || dados.conta_id !== comando.conta_id || dados.habilitado !== comando.habilitado) {
      aviso("Resultado de outro alvo; a operação continua pendente."); return;
    }
    if (dados.estado === "pendente") {
      atualizarComando(false);
      alerta("carregando", "Aplicando GM…", `GM salvo; sessões pendentes em ${(dados.processos_pendentes || dados.processos || []).join(", ")}.`);
      acompanhar();
      return;
    }
    encerrarComando(); atualizarComando(false);
    concluir("ok", dados.estado === "aplicado" ? "GM aplicado" : "GM substituído",
      dados.estado === "aplicado"
        ? `GM ${comando.habilitado ? "concedido a" : "removido de"} ${comando.usuario}. Entre novamente no jogo para ver a mudança.`
        : "Esta alteração foi substituída por outra posterior.");
    buscarContas();
    return;
  }
  if (dados.estado === "salvo" || dados.estado === "falha") {
    if (dados.estado === "salvo" && comando.tipo === "criar_conta" &&
        (dados.tipo !== "criar_conta" || dados.usuario !== comando.usuario || !Number.isInteger(dados.conta_id) || dados.conta_id <= 0)) {
      aviso("Resultado de outro alvo; a operação continua pendente.");
      return;
    }
    if (dados.estado === "salvo" && ["ajustar_gold", "definir_ban"].includes(comando.tipo) &&
        (dados.tipo !== comando.tipo || dados.conta_id !== comando.conta_id)) {
      aviso("Resultado de outro alvo; a operação continua pendente.");
      return;
    }
    encerrarComando();
    atualizarComando(false);
    if (dados.estado === "salvo") {
      const textos = {
        criar_conta: ["Conta criada", `Pronto: conta global ${dados.usuario} criada · ID ${dados.conta_id}.`],
        trocar_senha: ["Senha trocada", `A nova senha de ${comando.usuario} vale no próximo login.`],
        ajustar_gold: ["Gold dado", `Adicionado ${gold(comando.delta)} gold · saldo ${gold(dados.saldo ?? "0")}.`],
        definir_ban: [comando.banida ? "Conta banida" : "Conta desbanida",
          comando.banida ? `${comando.usuario} foi desconectada e não entra mais.` : `${comando.usuario} pode entrar de novo.`],
      };
      const [titulo, texto] = textos[comando.tipo || "trocar_senha"];
      concluir("ok", titulo, texto);
      if (comando.tipo === "criar_conta") elemento("modal-criacao").hidden = true;
      buscarContas();
    } else if (dados.codigo === "canal_nao_enviado") {
      concluir("erro", "Não enviada", `${dados.mensagem || "Canal administrativo indisponível"}. Nada foi alterado; pode tentar de novo.`);
    } else {
      const motivos = { valor_invalido: "Informe um valor positivo.", proprio_administrador: "Você não pode banir a própria conta.",
        conta_inexistente: "A conta não existe mais.", usuario_existente: "Esse usuário já existe." };
      concluir("erro", "Operação recusada", motivos[dados.codigo] || `O servidor recusou (${dados.codigo}).`);
    }
  } else atualizarComando();
}

/* Acompanhamento automático: enquanto houver operação sem confirmação, consulta o
   resultado sozinho (1,5 s, até ~45 s), sem o administrador precisar clicar. */
function acompanhar() {
  if (!temTempo || !estado.comando || estado.acompanhamento) return;
  estado.acompanhamento = setTimeout(async () => {
    estado.acompanhamento = null;
    if (!estado.comando || !estado.sessao) return;
    if (++estado.tentativas > 30) {
      alerta("aviso", "Sem confirmação", "O servidor ainda não confirmou. Use \"Consultar agora\" mais tarde; não repita com dados diferentes.");
      return;
    }
    await executarComando(true);
  }, 1500);
}
function pararAcompanhamento() {
  if (estado.acompanhamento && temTempo) clearTimeout(estado.acompanhamento);
  estado.acompanhamento = null;
  estado.tentativas = 0;
}

async function executarComando(recuperar, tipo = "trocar_senha") {
  if (estado.enviando || !estado.sessao) return;
  let comando = estado.comando;
  if (!comando && recuperar) return;
  if (!recuperar && comando && (comando.tipo || "trocar_senha") !== tipo) { alerta("aviso", "Operação em andamento", "Aguarde a operação atual terminar antes de iniciar outra."); return; }
  const efetivo = comando?.tipo || tipo;
  const gm = efetivo === "definir_gm";
  const criacao = efetivo === "criar_conta";
  const comSenha = efetivo === "trocar_senha" || criacao;
  const rota = rotaContas();
  if (!comando && (!rota || (!criacao && !estado.conta))) { alerta("erro", "Sem canal", "Nenhum realm com canal administrativo ativo."); return; }
  const usuario = comando?.usuario || elemento("usuario-novo").value.toLowerCase();
  if (!recuperar && criacao && !/^[a-z0-9_]{1,64}$/.test(usuario)) { alerta("erro", "Usuário inválido", "Use de 1 a 64 letras, números ou _."); return; }
  const campoSenha = elemento(criacao ? "senha-criacao" : "nova-senha");
  const senha = comSenha ? campoSenha.value : "";
  if (!recuperar && comSenha && !/^[\x20-\x7e]{1,64}$/.test(senha)) { alerta("erro", "Senha inválida", "Use de 1 a 64 caracteres sem acento."); return; }
  let delta = comando?.delta;
  if (!recuperar && !comando && efetivo === "ajustar_gold") {
    // Só dar (B180): o painel não tira gold.
    const valor = Number(elemento("gold-valor").value);
    delta = Math.round(valor * 100);
    if (!Number.isFinite(valor) || delta < 1 || delta > 2_147_483_647) { alerta("erro", "Valor inválido", "Informe a quantidade de gold a dar."); return; }
  }
  aviso("");
  try {
    if (!comando) {
      const bytes = crypto.getRandomValues(new Uint8Array(16));
      const extra = {
        definir_gm: () => ({ habilitado: elemento("gm-habilitado").value === "true" }),
        ajustar_gold: () => ({ delta }),
        definir_ban: () => ({ banida: !estado.conta.banida, motivo: !estado.conta.banida ? (elemento("ban-motivo").value.trim() || null) : null }),
      }[efetivo];
      comando = { id: Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(""), realm: rota.id,
                  tipo, ...(extra ? extra() : {}), ...(criacao ? { usuario } : { conta_id: estado.conta.id, usuario: estado.conta.usuario }) };
      // Guardar só metadados antes de enviar; nunca persistir a senha no navegador.
      localStorage.setItem(chaveComando(), JSON.stringify(comando));
      estado.comando = comando;
    }
    estado.enviando = true;
    if (comSenha) campoSenha.value = "";
    if (!recuperar) atualizarComando();
    const base = `/api/realms/${encodeURIComponent(rotaContas()?.id || comando.realm)}`;
    const corpo = { operacao_id: comando.id,
      ...(gm ? { habilitado: comando.habilitado } : {}),
      ...(comSenha ? { senha } : {}),
      ...(criacao ? { usuario: comando.usuario } : {}),
      ...(efetivo === "ajustar_gold" ? { delta: comando.delta } : {}),
      ...(efetivo === "definir_ban" ? { banida: comando.banida, motivo: comando.motivo } : {}) };
    const destino = recuperar ? `${base}/operacoes/${comando.id}`
      : (criacao ? `${base}/contas` : `${base}/contas/${comando.conta_id}/${ROTA_DO_TIPO[efetivo]}`);
    const dados = await api(destino, recuperar ? { signal: AbortSignal.timeout(5000) }
      : { method: "POST", signal: AbortSignal.timeout(8000), body: JSON.stringify(corpo) });
    estado.enviando = false;
    mostrarResultadoComando(dados, comando);
  } catch (erro) {
    estado.enviando = false;
    if (erro.resultado?.estado === "falha") mostrarResultadoComando(erro.resultado, comando);
    else if (!recuperar) atualizarComando();
  } finally {
    estado.enviando = false;
    atualizarComando(false);
    if (estado.comando) acompanhar();
  }
}

/* ---------------- Carga e sessão ---------------- */

async function atualizar() {
  if (estado.carregando) return;
  estado.carregando = true;
  elemento("atualizar").disabled = true;
  aviso("");
  try {
    const dados = await api("/api/realms");
    estado.realms = dados.realms;
    estado.online = {};
    const anterior = estado.selecionado?.id;
    estado.selecionado = estado.realms.find((realm) => realm.id === anterior) || null;
    preencherSeletor();
    renderizarRealms();
    selecionar(estado.selecionado?.id, null);
    if (estado.selecionado) renderizarRealm();
    elemento("ultima-atualizacao").textContent = `Atualizado às ${new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`;
    consultarOnline(estado.geracao);
    if (estado.pagina === "contas") buscarContas();
    else if (!elemento("nome-conta").value) buscarTotalContas();
    if (estado.pagina === "personagens") {
      if (estado.personagemId && estado.selecionado) await consultarPersonagem(estado.selecionado.id, estado.personagemId);
      else await buscarPersonagens();
    }
  } catch (erro) {
    aviso(`Não foi possível atualizar: ${erro.message}`);
  } finally {
    estado.carregando = false;
    elemento("atualizar").disabled = false;
  }
}

async function buscarTotalContas() {
  try {
    const dados = await api("/api/contas?por_pagina=1");
    elemento("total-contas").textContent = fmt(dados.total);
  } catch (_) { /* Número da visão geral é opcional. */ }
}

async function abrirPortal() {
  estado.sessao = await api("/api/sessao");
  estado.comando = lerComando();
  atualizarComando(false);
  elemento("nome-usuario").textContent = estado.sessao.usuario;
  elemento("entrada").hidden = true;
  elemento("portal").hidden = false;
  mudarPagina("inicio");
  await atualizar();
  if (estado.comando) acompanhar();
}

elemento("formulario-login").addEventListener("submit", async (evento) => {
  evento.preventDefault();
  elemento("entrar").disabled = true;
  elemento("erro-login").hidden = true;
  try {
    await api("/api/sessao/entrar", { method: "POST", body: JSON.stringify({ usuario: elemento("usuario").value.trim(), senha: elemento("senha").value }) });
    elemento("senha").value = "";
    await abrirPortal();
  } catch (erro) {
    elemento("senha").value = "";
    elemento("erro-login").textContent = erro.message;
    elemento("erro-login").hidden = false;
  } finally { elemento("entrar").disabled = false; }
});
elemento("mostrar-senha").addEventListener("click", () => {
  const mostrar = elemento("senha").type === "password";
  elemento("senha").type = mostrar ? "text" : "password";
  elemento("mostrar-senha").textContent = mostrar ? "Ocultar" : "Mostrar";
  elemento("mostrar-senha").setAttribute("aria-pressed", String(mostrar));
});
elemento("sair").addEventListener("click", async () => {
  elemento("sair").disabled = true;
  try { await api("/api/sessao/sair", { method: "POST" }); mostrarEntrada(); }
  catch (erro) { aviso(erro.message); }
  finally { elemento("sair").disabled = false; }
});
elemento("atualizar").addEventListener("click", atualizar);
elemento("realm-selecionado").addEventListener("change", (evento) => selecionar(evento.target.value));
elemento("menu").addEventListener("click", abrirMenu);
elemento("cortina").addEventListener("click", fecharMenu);
elemento("busca-personagens").addEventListener("submit", (evento) => { evento.preventDefault(); buscarPersonagens(); });
elemento("busca-contas").addEventListener("submit", (evento) => { evento.preventDefault(); buscarContas(1); });
elemento("pagina-anterior").addEventListener("click", () => buscarContas(estado.paginaContas - 1));
elemento("pagina-seguinte").addEventListener("click", () => buscarContas(estado.paginaContas + 1));
elemento("nova-conta").addEventListener("click", () => {
  if (estado.comando && estado.comando.tipo !== "criar_conta") { alerta("aviso", "Operação em andamento", "Aguarde a operação atual terminar."); return; }
  fecharConta();
  elemento("modal-criacao").hidden = false;
  atualizarComando(false);
  elemento("usuario-novo").focus?.();
});
elemento("fechar-criacao").addEventListener("click", () => { elemento("modal-criacao").hidden = true; });
elemento("modal-criacao").addEventListener("click", (evento) => { if (evento.target === elemento("modal-criacao")) elemento("modal-criacao").hidden = true; });
elemento("fechar-gaveta").addEventListener("click", fecharConta);
elemento("fechar-operacao").addEventListener("click", () => { responderConfirmacao(false); fecharAlerta(); });
elemento("confirmar-alerta").addEventListener("click", () => responderConfirmacao(true));
elemento("operacao").addEventListener("click", (evento) => {
  if (evento.target === elemento("operacao") && !elemento("fechar-operacao").hidden) fecharAlerta();
});
elemento("abrir-pendente").addEventListener("click", () => { atualizarComando(true); });
elemento("troca-senha").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false); });
elemento("gestao-gm").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "definir_gm"); });
elemento("ajuste-gold").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "ajustar_gold"); });
elemento("gestao-ban").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "definir_ban"); });
elemento("aplicar-desconectar").addEventListener("click", desconectarConta);
elemento("form-rates").addEventListener("submit", (evento) => { evento.preventDefault(); salvarRates(); });
elemento("form-dinheiro").addEventListener("submit", (evento) => { evento.preventDefault(); editarDinheiro(); });
elemento("form-exp").addEventListener("submit", (evento) => { evento.preventDefault(); editarExp(); });
elemento("filtro-mapa").addEventListener("input", () => renderizarMapas());
elemento("filtro-status-mapa").addEventListener("change", () => renderizarMapas());
elemento("marcar-mapas").addEventListener("click", () => marcarMapasVisiveis(true));
elemento("desmarcar-mapas").addEventListener("click", () => marcarMapasVisiveis(false));
elemento("ligar-mapas").addEventListener("click", () => aplicarAosMarcados(true));
elemento("desligar-mapas").addEventListener("click", () => aplicarAosMarcados(false));
elemento("form-pontos").addEventListener("submit", (evento) => { evento.preventDefault(); editarPontos(); });
elemento("form-nivel").addEventListener("submit", (evento) => { evento.preventDefault(); editarNivel(); });
elemento("form-atributos").addEventListener("submit", (evento) => { evento.preventDefault(); editarAtributos(); });
elemento("redistribuir-atributos").addEventListener("click", () => redistribuirAtributos());
for (const id of CAMPOS_DE_ATRIBUTO) elemento(id).addEventListener("input", () => atualizarSobraDeAtributos());
elemento("form-posicao").addEventListener("submit", (evento) => { evento.preventDefault(); editarPosicao(); });
elemento("form-cultivo").addEventListener("submit", (evento) => { evento.preventDefault(); editarCultivo(); });
elemento("criacao-conta").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "criar_conta"); });
elemento("recuperar-comando").addEventListener("click", () => { pararAcompanhamento(); executarComando(true); });
document.querySelectorAll(".nav").forEach((botao) => botao.addEventListener("click", () => mudarPagina(botao.dataset.pagina)));
document.querySelectorAll(".acao, .voltar").forEach((botao) => botao.addEventListener("click", () => trocarAba(botao.dataset.aba)));
// Clique fora do popup fecha (exceto enquanto o alerta está aberto por cima).
document.addEventListener?.("mousedown", (evento) => {
  const popover = elemento("popover-conta");
  if (popover.hidden || !elemento("operacao").hidden) return;
  if (!popover.contains(evento.target) && !estado.cartao?.contains(evento.target)) fecharConta();
});
if (typeof window !== "undefined") window.addEventListener("resize", fecharConta);
document.addEventListener?.("scroll", fecharConta, true);
document.addEventListener?.("keydown", (evento) => {
  if (evento.key !== "Escape") return;
  if (!elemento("operacao").hidden && !elemento("fechar-operacao").hidden) fecharAlerta();
  else if (!elemento("modal-criacao").hidden) elemento("modal-criacao").hidden = true;
  else if (!elemento("popover-conta").hidden) fecharConta();
  else fecharMenu();
});
abrirPortal().catch((erro) => {
  mostrarEntrada();
  if (!erro.message.includes("Entre") && !erro.message.includes("Sessão")) {
    elemento("erro-login").textContent = erro.message;
    elemento("erro-login").hidden = false;
  }
});
