"use strict";

const elemento = (id) => document.getElementById(id);
const estado = {
  sessao: null, realms: [], selecionado: null, pagina: "inicio", carregando: false,
  geracao: 0, busca: 0, ficha: 0, personagemId: null,
  conta: null, cartao: null, comando: null, enviando: false, buscaContas: 0,
  paginaContas: 1, paginaPersonagens: 1, personagemAberto: null, porPagina: 12, totalContas: 0, aba: "resumo", online: {},
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
  templates: ["Templates", ""],
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
/* B216: sucesso, erro e aviso simples saem como toast no canto (somem sozinhos, sem OK, com a
   barra de tempo; clique fecha), como o toast do SweetAlert. Modal só para confirmação, para o
   carregamento das contas e para o aviso que oferece "Consultar agora". */
function alerta(tipo, titulo, texto, opcoes = {}) {
  const comRecuperar = tipo === "aviso" && !!estado.comando && !opcoes.toast;
  const modal = opcoes.modal ?? (comRecuperar || (tipo === "carregando" && !opcoes.toast));
  estado.alertaModal = modal;
  elemento("operacao").classList.toggle("modo-toast", !modal);
  elemento("alerta-icone").className = `alerta-icone ${tipo}`;
  elemento("alerta-titulo").textContent = titulo;
  elemento("resultado-comando").textContent = texto;
  elemento("fechar-operacao").hidden = tipo === "carregando" || !modal;
  elemento("recuperar-comando").hidden = !comRecuperar;
  if (!modal) elemento("confirmar-alerta").hidden = true;
  elemento("operacao").hidden = false;
  if (fecharAlertaEm && temTempo) clearTimeout(fecharAlertaEm);
  const duracao = modal || tipo === "carregando" ? 0 : (tipo === "ok" ? 2600 : 5000);
  const barra = elemento("alerta-tempo");
  if (barra?.style) {
    barra.style.animation = "none";
    void barra.offsetWidth;
    barra.style.animation = duracao ? `esvaziar ${duracao}ms linear forwards` : "none";
  }
  fecharAlertaEm = duracao && temTempo ? setTimeout(fecharAlerta, duracao) : null;
}
function fecharAlerta() { elemento("operacao").hidden = true; elemento("confirmar-alerta").hidden = true; estado.alertaModal = false; }

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
  elemento("consulta-personagens").hidden = false;
  estado.paginaPersonagens = 1;
}

/* ---------------- Navegação ---------------- */

function mudarPagina(pagina) {
  if (!estado.selecionado && ["realm", "personagens", "mapas", "rates", "templates"].includes(pagina)) pagina = "inicio";
  estado.pagina = pagina;
  fecharConta();
  for (const id of ["inicio", "realm", "personagens", "contas", "rates", "mapas", "templates"]) elemento(`pagina-${id}`).hidden = pagina !== id;
  const [titulo, sub] = PAGINAS[pagina];
  const realm = estado.selecionado;
  elemento("titulo-pagina").textContent = pagina === "realm" && realm ? realm.nome : titulo;
  elemento("subtitulo-pagina").textContent = sub || (realm ? (pagina === "realm" ? `Porta ${realm.porta}` : realm.nome) : "");
  document.querySelectorAll(".nav").forEach((botao) => {
    const ativo = botao.dataset.pagina === pagina;
    botao.classList.toggle("ativo", ativo);
    if (ativo) botao.setAttribute("aria-current", "page");
    else botao.removeAttribute("aria-current");
  });
  fecharMenu();
  if (pagina === "contas" && !elemento("lista-contas").children?.length) buscarContas();
  if (pagina === "personagens" && !estado.personagemId) buscarPersonagens();
  if (pagina === "templates") carregarTemplates();
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

// "1 · Mundo" pelo catálogo da versão; sem catálogo (ainda carregando ou mapa fora dele), "Mapa N" (B218).
function nomeDoMapa(realm, mapa) {
  const achado = (estado.catalogoMapas[realm?.id] || []).find((m) => m.mapa === mapa);
  return achado?.nome ? `${mapa} · ${achado.nome}` : `Mapa ${mapa}`;
}

// Catálogo da versão (gs.conf original + nomes do pwadmin), uma vez por realm.
async function carregarCatalogoDeMapas(realm) {
  if (estado.catalogoMapas[realm.id]) return;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/catalogo-mapas`);
    estado.catalogoMapas[realm.id] = dados.mapas || [];
  } catch (_) {
    estado.catalogoMapas[realm.id] = [];
  }
  if (estado.selecionado?.id !== realm.id) return;
  if (estado.pagina === "mapas") renderizarMapas();
  else if (estado.pagina === "realm") renderizarRealm();
  else if (estado.pagina === "templates" && estado.templates && !modoTemplate())
    elemento("lista-templates").replaceChildren(...(estado.templates.templates || []).map(cartaoTemplate));
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
  alerta("aviso", titulo, texto, { modal: true });
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
  elemento("consulta-personagens").hidden = false;
  estado.paginaPersonagens = 1;
  elemento("aviso-personagens").hidden = true;
  estado.templates = null;
  estado.templateAberto = null;
  elemento("ficha-template").hidden = true;
  elemento("consulta-templates").hidden = false;
  elemento("lista-templates").replaceChildren();
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
    if (!estado.catalogoMapas[realm.id]) carregarCatalogoDeMapas(realm);
    for (const mapa of vivo.mapas) {
      const item = criar("li");
      item.append(criar("span", "", nomeDoMapa(realm, mapa.mapa)), criar("span", "etiqueta", `${fmt(mapa.jogadores_online)} online`));
      mapas.append(item);
    }
  } else if (situacaoDoMundo(realm) === "sem_resposta") {
    const item = criar("li");
    item.append(avisoSemResposta(realm));
    mapas.append(item);
  } else {
    mapas.append(criar("li", "texto-suave", realm.canal_administrativo ? "Consultando…" : "Sem canal administrativo"));
  }
  const nomes = { disponivel: ["Disponível", ""], implementado: ["Implementado", ""], nao_validado: ["Não validado", "etiqueta-neutra"], parcial: ["Parcial", "etiqueta-neutra"], indisponivel: ["Indisponível", "etiqueta-neutra"] };
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

/* Personagens em cartões (B190): todas as contas do realm, busca por personagem ou conta,
   paginação como a de Contas. Clicar abre a tela do personagem; "Voltar" retorna à consulta. */
async function buscarPersonagens(pagina = estado.paginaPersonagens) {
  const realm = estado.selecionado;
  if (!realm) return;
  const busca = ++estado.busca;
  estado.paginaPersonagens = Math.max(1, pagina);
  fecharTelaPersonagem();
  elemento("aviso-personagens").hidden = true;
  try {
    const termo = encodeURIComponent(elemento("nome-personagem").value.trim());
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens?busca=${termo}&pagina=${estado.paginaPersonagens}&por_pagina=${estado.porPagina}`);
    if (estado.busca !== busca || estado.selecionado?.id !== realm.id || !estado.sessao) return;
    const lista = elemento("lista-personagens");
    lista.replaceChildren();
    for (const personagem of dados.personagens) lista.append(cartaoPersonagem(realm, personagem));
    if (!dados.personagens.length) lista.append(criar("p", "nota-vazia", "Nenhum personagem encontrado."));
    const total = dados.total ?? dados.personagens.length;
    const paginas = Math.max(1, Math.ceil(total / estado.porPagina));
    elemento("resumo-personagens").textContent = `${fmt(total)} personage${total === 1 ? "m" : "ns"}`;
    elemento("indicador-personagens").textContent = `Página ${estado.paginaPersonagens} de ${paginas}`;
    elemento("personagens-anterior").disabled = estado.paginaPersonagens <= 1;
    elemento("personagens-seguinte").disabled = estado.paginaPersonagens >= paginas;
  } catch (erro) {
    if (estado.busca !== busca || !estado.sessao) return;
    elemento("aviso-personagens").textContent = erro.message;
    elemento("aviso-personagens").hidden = false;
  }
}

function cartaoPersonagem(realm, personagem) {
  const botao = criar("button", "cartao-conta cartao-personagem");
  botao.type = "button";
  const avatar = criar("span", `avatar ${corAvatar(personagem.classe)}`, personagem.nome.slice(0, 1));
  const nome = criar("div");
  nome.append(criar("div", "nome", personagem.nome), criar("div", "id", `${personagem.usuario} · #${personagem.id}`));
  const rodape = criar("div", "rodape");
  rodape.append(criar("span", "etiqueta etiqueta-neutra", personagem.classe_nome || `Classe ${personagem.classe}`),
    criar("span", "gold", `Nv. ${personagem.nivel}`));
  botao.append(avatar, nome, rodape);
  botao.addEventListener("click", () => {
    estado.personagemAberto = personagem;
    consultarPersonagem(realm.id, personagem.id);
  });
  return botao;
}

function fecharTelaPersonagem() {
  estado.ficha++;
  estado.personagemId = null;
  esconderDica();
  elemento("ficha-personagem").hidden = true;
  elemento("consulta-personagens").hidden = false;
}

function abrirTelaPersonagem() {
  elemento("consulta-personagens").hidden = true;
  elemento("ficha-personagem").hidden = false;
}

/* ---------------- Ícones (B216) ----------------
   Traço de 24×24 desenhado aqui (sem biblioteca: a CSP só permite 'self'). `[data-icone]` no HTML
   é preenchido na carga; `icone(nome)` serve ao que o JS monta. */
const ICONES = {
  usuario: "M12 4a4 4 0 1 0 0 8a4 4 0 1 0 0-8M4 21c0-4 3.6-6 8-6s8 2 8 6",
  grupo: "M9 5a3 3 0 1 0 0 6a3 3 0 1 0 0-6M17 7a2.5 2.5 0 1 0 0 5M3 20c0-3.3 2.7-5 6-5s6 1.7 6 5M15 15c3 0 6 1.4 6 4.5",
  editar: "M4 20h4L19 9l-4-4L4 16zM14 6l4 4",
  mais: "M12 5v14M5 12h14",
  busca: "M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14M20 20l-4-4",
  filtro: "M3 5h18l-7 8v6l-4 2v-8z",
  lixeira: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
  nivel: "M12 3l2.6 5.6 6.1.6-4.6 4.1 1.3 6-5.4-3.1-5.4 3.1 1.3-6L3.3 9.2l6.1-.6z",
  escudo: "M12 3l8 3v6c0 4.5-3.4 8-8 9-4.6-1-8-4.5-8-9V6z",
  cultivo: "M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9L7 7M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1",
  exp: "M3 17l6-6 4 4 8-8M15 7h6v6",
  brilho: "M12 3l1.8 4.7 4.7 1.8-4.7 1.8L12 16l-1.8-4.7-4.7-1.8 4.7-1.8zM19 15l.8 2.2 2.2.8-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z",
  moeda: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 7v10M15 9.5c0-1.4-1.3-2-3-2s-3 .8-3 2 1.3 1.8 3 2.2 3 .9 3 2.3-1.3 2-3 2-3-.7-3-2",
  vida: "M12 20s-7-4.4-9-9.2C1.8 7.7 3.9 4.5 7.2 4.5c2 0 3.6 1.1 4.8 2.8 1.2-1.7 2.8-2.8 4.8-2.8 3.3 0 5.4 3.2 4.2 6.3C19 15.6 12 20 12 20z",
  mana: "M12 3s6 6.6 6 11a6 6 0 0 1-12 0c0-4.4 6-11 6-11z",
  pontos: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 8v8M8 12h8",
  forca: "M6 7v10M3 9v6M18 7v10M21 9v6M6 12h12",
  agilidade: "M3 8h11a3 3 0 1 0-3-3M3 12h16a3 3 0 1 1-3 3M3 16h7",
  vitalidade: "M3 12h4l3-8 4 16 3-8h4",
  energia: "M13 2L4 14h7l-1 8 9-12h-7z",
  mapa: "M3 6l6-3 6 3 6-3v15l-6 3-6-3-6 3zM9 3v15M15 6v15",
  posicao: "M12 21s-7-6.2-7-11.5A7 7 0 0 1 19 9.5C19 14.8 12 21 12 21zM12 7a2.5 2.5 0 1 0 0 5a2.5 2.5 0 1 0 0-5",
  camadas: "M12 3l9 5-9 5-9-5zM3 13l9 5 9-5",
  chave: "M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.5 2.5-2.4-.6-.6-2.4z",
  refino: "M12 19V5M5 12l7-7 7 7",
  gema: "M6 3h12l4 6-10 12L2 9zM2 9h20M12 21L8 9l4-6 4 6z",
  martelo: "M14 5l5 5-3 3-5-5zM11 8l-8 8 3 3 8-8",
  furo: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 10a2 2 0 1 0 0 4a2 2 0 1 0 0-4",
  etiqueta: "M3 12V4h8l10 10-8 8zM7.5 7.5h.01",
  fome: "M7 3v8M5 3v5a2 2 0 0 0 4 0V3M7 11v10M17 3c-2 1-3 3-3 6s1 4 3 4v8",
  medalha: "M12 13a5 5 0 1 0 0-10a5 5 0 1 0 0 10M8.5 12L7 21l5-3 5 3-1.5-9",
  pergaminho: "M8 4h11v13a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3v-1h11v1a3 3 0 0 0 3 3M8 4a3 3 0 0 0-3 3v9M11 8h5M11 12h5",
  livro: "M4 5a2 2 0 0 1 2-2h14v16H6a2 2 0 0 0-2 2zM4 19V5",
  pata: "M8 5a1.7 1.7 0 1 0 0 3.4a1.7 1.7 0 1 0 0-3.4M16 5a1.7 1.7 0 1 0 0 3.4a1.7 1.7 0 1 0 0-3.4M5 10a1.6 1.6 0 1 0 0 3.2a1.6 1.6 0 1 0 0-3.2M19 10a1.6 1.6 0 1 0 0 3.2a1.6 1.6 0 1 0 0-3.2M12 12c-3 0-5 3.5-5 5.5S9 20 12 20s5-.5 5-2.5S15 12 12 12z",
  caixa: "M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10",
  bolsa: "M6 8h12l1 12H5zM9 8V6a3 3 0 0 1 6 0v2",
  roupa: "M8 3l-5 4 3 3 2-1v12h8V9l2 1 3-3-5-4c-.5 1.5-2 2.5-4 2.5S8.5 4.5 8 3z",
  olho: "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6",
  check: "M5 12l5 5 9-10",
  fechar: "M6 6l12 12M18 6L6 18",
  relogio: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 7v5l3 2",
  // B218: menu lateral, topo e popup de contas (antes glifos/emoji soltos).
  casa: "M3 11l9-8 9 8M5 9v12h14V9M10 21v-6h4v6",
  servidor: "M4 4h16v6H4zM4 14h16v6H4zM8 7h.01M8 17h.01",
  porcentagem: "M19 5L5 19M7 5a2 2 0 1 0 0 4a2 2 0 1 0 0-4M17 15a2 2 0 1 0 0 4a2 2 0 1 0 0-4",
  espada: "M20 4l-9 9M20 4h-4M20 4v4M11 13l-3 3M5 14l5 5M7 17l-3 3",
  menu: "M4 6h16M4 12h16M4 18h16",
  atualizar: "M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7",
  bloqueio: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M5.6 5.6l12.8 12.8",
  sair: "M10 17l5-5-5-5M15 12H3M14 4h6v16h-6",
  tema: "M12 3a9 9 0 1 0 9 9a7 7 0 0 1-9-9z",
};
function icone(nome, classe = "ico") {
  const caixa = criar("span", classe);
  caixa.setAttribute("aria-hidden", "true");
  if (typeof document.createElementNS !== "function" || !ICONES[nome]) return caixa;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  const traco = document.createElementNS("http://www.w3.org/2000/svg", "path");
  traco.setAttribute("d", ICONES[nome]);
  svg.append(traco);
  caixa.append(svg);
  return caixa;
}
function preencherIcones(raiz = document) {
  for (const alvo of raiz.querySelectorAll?.("[data-icone]") || []) {
    if (alvo.firstChild) continue;
    const feito = icone(alvo.dataset.icone, "");
    if (feito.firstChild) alvo.append(feito.firstChild);
  }
}

/* Modais (B216): `.fechar-modal` e o fundo fecham o modal de onde vieram. */
function abrirModal(id) {
  esconderMenuDoSlot();
  esconderDica();
  elemento(id).hidden = false;
  const foco = elemento(id).querySelector?.("input:not([disabled]):not([readonly]), select");
  if (foco && temTempo) setTimeout(() => foco.focus?.(), 30);
}
function fecharModal(id) {
  elemento(id).hidden = true;
  if (id === "modal-item") estado.itemEmEdicao = null;
}
const MODAIS_DA_FICHA = ["modal-estatisticas", "modal-item", "modal-criar-item", "modal-busca-habilidade", "modal-habilidade",
  "modal-busca-mascote", "modal-mascote", "modal-daimon", "modal-missoes", "modal-template-estatisticas", "modal-template-item"];

/* Texto com os códigos de cor do cliente (`^RRGGBB`) pintado; sem código, a cor do painel. */
function textoPintado(texto, classe = "") {
  const caixa = criar("span", classe);
  let cor = null;
  for (const parte of String(texto || "").split(/(\^[0-9a-fA-F]{6})/)) {
    if (/^\^[0-9a-fA-F]{6}$/.test(parte)) { cor = parte.slice(1); continue; }
    if (!parte) continue;
    const span = criar("span", "", parte);
    if (cor) span.style.color = `#${cor}`;
    caixa.append(span);
  }
  return caixa;
}
const semCores = (texto) => String(texto || "").replace(/\^[0-9a-fA-F]{6}/g, "");

/* Ícone recortado do atlas do cliente (`/api/icones/<conjunto>/<arquivo>.png`) ou as iniciais. */
function imagemDoIcone(conjunto, arquivo, nome, classe = "") {
  if (!arquivo) return criar("span", `sem-icone ${classe}`, semCores(nome).slice(0, 2));
  const img = criar("img", classe);
  img.src = `/api/icones/${conjunto}/${arquivo}.png`;
  img.alt = semCores(nome);
  img.addEventListener("error", () => img.replaceWith(criar("span", `sem-icone ${classe}`, semCores(nome).slice(0, 2))));
  return img;
}

/* Classes do cliente para os filtros de habilidade (`/api/classes`), por versão. */
const classesPorVersao = {};
async function classesDoRealm() {
  const versao = estado.selecionado?.versao || "1.5.5";
  if (!classesPorVersao[versao]) {
    try { classesPorVersao[versao] = (await api(`/api/classes?versao=${encodeURIComponent(versao)}`)).classes || []; }
    catch (_) { return []; }
  }
  return classesPorVersao[versao];
}
/* Grupos de habilidade pelo `cls` do stub do cliente: as classes, e os três grupos sem classe. */
async function preencherGruposDeHabilidade(select, padrao = "") {
  const opcoes = [["", "Todas as classes"], ...(await classesDoRealm()).map((n, i) => [String(i), n]),
    ["127", "Sem classe (monstros e mascotes)"], ["255", "Vida e especiais"], ["258", "Grupo 258"]];
  select.replaceChildren(...opcoes.map(([v, n]) => { const o = criar("option", "", n); o.value = v; return o; }));
  select.value = padrao;
}

/* ---------------- Ficha: estatísticas ---------------- */

async function consultarPersonagem(realmId, id) {
  const consulta = ++estado.ficha;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realmId)}/personagens/${id}`);
    if (estado.ficha !== consulta || estado.selecionado?.id !== realmId || !estado.sessao) return;
    const ficha = dados.ficha;
    elemento("titulo-personagem").textContent = ficha.nome;
    const conta = estado.personagemAberto?.id === id ? ` · conta ${estado.personagemAberto.usuario}` : "";
    const presencas = { online: "Online agora", ausente_nos_daemons: "Offline", em_transicao: "Entrando ou saindo", desconhecida: "Presença desconhecida" };
    elemento("presenca-personagem").textContent = `${ficha.classe_nome || `Classe ${ficha.classe}`} · Nv. ${ficha.nivel}${conta} · #${ficha.id} · ${presencas[dados.presenca] || "Presença desconhecida"}`;
    elemento("origem-personagem").textContent = dados.origem === "viva" ? "Ao vivo" : "Salvo no banco";
    elemento("origem-personagem").className = `etiqueta ${dados.origem === "viva" ? "" : "etiqueta-neutra"}`;
    elemento("observacao-personagem").textContent = dados.aviso || "";
    const inteiro = (valor) => BigInt(valor).toLocaleString("pt-BR");
    // Um campo de cada; força a energia em linha com os ícones do jogo.
    const campos = [["nivel", "Nível", ficha.nivel], ["escudo", "Classe", ficha.classe_nome || ficha.classe],
      ["cultivo", "Cultivo", ficha.cultivo], ["exp", "EXP", inteiro(ficha.exp)], ["brilho", "Alma (SP)", inteiro(ficha.alma)],
      ["moeda", "Dinheiro", inteiro(ficha.dinheiro)], ["vida", "Vida", fmt(ficha.vida)], ["mana", "Mana", fmt(ficha.mana)],
      ["pontos", "Pontos livres", fmt(ficha.pontos)], ["forca", "Força", fmt(ficha.forca)], ["agilidade", "Agilidade", fmt(ficha.agilidade)],
      ["vitalidade", "Vitalidade", fmt(ficha.vitalidade)], ["energia", "Energia", fmt(ficha.energia)], ["mapa", "Mapa", ficha.mapa],
      ["posicao", "Posição", `${ficha.posicao.x.toFixed(0)}, ${ficha.posicao.y.toFixed(0)}, ${ficha.posicao.z.toFixed(0)}`]];
    elemento("dados-personagem").replaceChildren(...campos.map(([ico, rotulo, valor]) => {
      const caixa = criar("div", `atributo atributo-${ico}`);
      const texto = criar("div");
      texto.append(criar("span", "", rotulo), criar("strong", "", String(valor)));
      caixa.append(icone(ico), texto);
      return caixa;
    }));
    abrirTelaPersonagem();
    estado.personagemId = id;
    const online = dados.presenca === "online";
    estado.personagemOnline = online;
    elemento("dica-edicao").textContent = online
      ? "Em jogo: as edições valem na hora e o jogador vê a mudança."
      : "Fora do jogo: grava no banco; EXP/SP só com o personagem em jogo.";
    elemento("est-resumo").textContent = `${ficha.nome} · ${online ? "em jogo" : "fora do jogo"}`;
    elemento("form-exp").querySelector?.("button")?.toggleAttribute?.("disabled", !online);
    preencherCultivos(estado.selecionado?.versao, ficha.cultivo);
    preencherAtributos(ficha);
    preencherPosicao(ficha);
    carregarInventario(estado.selecionado, id);
    carregarHabilidades(estado.selecionado, id);
    carregarMascotes(estado.selecionado, id);
    carregarDaimon(estado.selecionado, id);
    carregarMissoes(estado.selecionado, id);
  } catch (erro) {
    if (estado.ficha !== consulta || !estado.sessao) return;
    elemento("aviso-personagens").textContent = erro.message;
    elemento("aviso-personagens").hidden = false;
  }
}

/* ---------------- Edição de personagem (E5) ---------------- */

/* Cultivos que o cliente mostra: 0–8 em todas; 20–22 e 30–32 só no 1.5.5 (B182). */
function preencherCultivos(versao, atual, alvo = "cultivo-valor") {
  const lista = elemento(alvo);
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
  precisa_estar_online: "Isto só com o personagem em jogo (EXP, SP ou prêmio de missão).",
  missao_inexistente: "Este realm não tem essa missão.",
  nao_e_topo: "Só a missão principal (não uma etapa).",
  escolha_a_submissao: "Esta missão pede que se escolha a etapa.",
  missao_ja_ativa: "O personagem já tem essa missão.",
  lista_cheia: "A lista de missões do personagem está cheia.",
  missao_ausente: "A missão não está mais na lista; atualize.",
  ja_finalizada: "A missão já está pronta para entregar.",
  missao_ativa: "A missão está ativa; cancele antes de esquecer.",
  sem_registro: "A missão não está nas concluídas.",
  nivel_invalido: "O nível novo tem de ser maior que o atual e até o teto do realm.",
  nivel_invalido_ou_personagem_inexistente: "O nível novo tem de ser maior que o atual (ou o personagem não existe).",
  cultivo_invalido: "Cultivo que esta versão não tem.",
  atributos_invalidos: "Algum atributo ficaria abaixo do mínimo ou a soma passa do total (atributos + livres).",
  atributos_invalidos_ou_personagem_inexistente: "Algum atributo ficaria abaixo do mínimo ou a soma passa do total (ou o personagem não existe).",
  sem_mudanca: "Nada mudaria.",
  sem_daimon: "O personagem não tem Daimon vestido.",
  mapa_indisponivel: "O mapa de destino não está ligado neste realm.",
  fora_do_mapa: "Essas coordenadas ficam fora do terreno do mapa.",
  altura_obrigatoria: "Este mapa não tem terreno carregado: informe o Y.",
  bolsa_cheia: "Não cabe tudo na bolsa (nada foi dado).",
  slot_mudou: "O item mudou de lugar desde a consulta; atualize a ficha.",
  slot_ocupado: "O slot já tem um item; atualize a ficha.",
  recipiente_errado: "Item de missão só vai à bolsa de missão, e os outros só à bolsa ou ao armazém.",
  quantidade_invalida: "Quantidade maior que a pilha.",
  precisa_estar_offline: "Isto só com o personagem fora do jogo.",
  posicao_invalida: "Esta peça não vai nesse slot.",
  requisito: "O personagem não atende o requisito da peça (nível, classe ou atributos).",
  reputacao: "O personagem não tem a reputação que a peça exige.",
  habilidade_repetida: "O mesmo item de habilidade já está no outro slot de habilidade.",
  movimento_invalido: "O jogo não move itens entre esses dois lugares.",
  slot_invalido: "Slot fora do recipiente.",
  equipamento_travado: "O equipamento está trancado (Forma Sombria).",
  nao_aplicado: "O jogo não confirmou a troca; atualize a ficha.",
  mascote_invocado: "O mascote está invocado; recolha-o no jogo antes de editar.",
  nivel_do_mascote_invalido: "Nível acima do máximo do modelo do mascote.",
  ovo_inexistente: "Este realm não tem esse ovo de mascote.",
  jaula_cheia: "A jaula do personagem não tem vaga livre.",
  classe_nao_permitida: "A classe do personagem não pode ter este mascote.",
  item_inexistente: "Este realm não tem esse item.",
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
  alerta("carregando", "Aplicando…", descricao, { toast: true });
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
      return dados.estado;
    } else if (dados.estado === "desconhecido") {
      alerta("aviso", "Sem confirmação", `Operação ${operacao}: o servidor não confirmou. Confira a ficha antes de repetir.`, { toast: true });
    } else if (dados.codigo === "precisa_estar_offline") {
      estado.enviando = false;
      return desconectarEAplicar(corpo, descricao);
    } else {
      alerta("erro", "Não aplicado", MOTIVOS_EDICAO[dados.codigo] || dados.mensagem || dados.codigo);
    }
  } catch (erro) {
    if (erro.resultado?.codigo === "precisa_estar_offline") { estado.enviando = false; return desconectarEAplicar(corpo, descricao); }
    alerta("erro", "Não aplicado", MOTIVOS_EDICAO[erro.resultado?.codigo] || erro.message);
  } finally { estado.enviando = false; }
}

/* Online sem pacote comprovado (decisão de 2026-10-07): recusa e oferece desconectar a conta
   (o mesmo desconectar da E4, que salva) e aplicar com o personagem fora do jogo. */
async function desconectarEAplicar(corpo, descricao) {
  const realm = estado.selecionado;
  const id = estado.personagemId;
  const conta = estado.personagemAberto?.id === id ? estado.personagemAberto : null;
  if (!realm || !id || !conta) { alerta("erro", "Não aplicado", MOTIVOS_EDICAO.precisa_estar_offline); return; }
  const sim = await confirmar("Personagem em jogo",
    `${MOTIVOS_EDICAO.precisa_estar_offline} Desconectar a conta ${conta.usuario} e aplicar?`, "Desconectar e aplicar");
  if (!sim || estado.personagemId !== id) return;
  alerta("carregando", "Desconectando…", `Salvando ${conta.nome}.`, { toast: true });
  try {
    await api(`/api/contas/${conta.conta_id}/desconectar`, { method: "POST" });
    // Espera o GS declarar a ausência (a saída grava antes de soltar o personagem).
    for (let i = 0; i < 20 && temTempo; i++) {
      const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}`);
      if (dados.presenca === "ausente_nos_daemons") return await editarPersonagem(corpo, descricao);
      await new Promise((r) => setTimeout(r, 1000));
    }
    alerta("aviso", "Ainda em jogo", "A conta não saiu a tempo; nada foi alterado. Tente de novo.", { toast: true });
  } catch (erro) {
    alerta("erro", "Não aplicado", erro.message);
  }
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

/* ---------------- Itens (E6, B186–B194; B216) ----------------
   Grade de itens (B188): ícones recortados do atlas do cliente, oito por linha; o equipamento com
   o nome de cada slot (`EQUIPIVTR_*`, `EC_IvtrTypes.h:56-95`). Tamanhos de `gs/config.h:12-17`:
   bolsa e bolsa de missão 32, armazém 16 (ou mais, se o banco tiver slot acima). */
const RECIPIENTES = [["bolsa", "Bolsa"], ["equipamento", "Equipamento"], ["armazem", "Armazém"], ["missao", "Bolsa de missão"]];
const TAMANHOS = { bolsa: 32, armazem: 16, missao: 32 };
const SLOTS_DE_EQUIPAMENTO = ["Arma", "Cabeça", "Colar", "Manto", "Peito", "Cinto", "Pernas", "Pés", "Pulsos",
  "Anel 1", "Anel 2", "Munição", "Voo", "Moda: corpo", "Moda: pernas", "Moda: pés", "Moda: pulsos", "Runa", "Tomo",
  "Alto-falante", "Amuleto HP", "Amuleto MP", "Bolso", "Gênio", "Certificado", "Moda: cabeça", "Ficha de força",
  "Habilidade 1", "Habilidade 2", "Moda: arma", "—", "—", "Carta 1", "Carta 2", "Carta 3", "Carta 4", "Carta 5", "Carta 6",
  "Astrolábio"];
function celulaDeItem(chave, rotulo, item, sexo, legenda, slot) {
  const celula = criar("div", `celula-item ${item ? "ocupada" : "vazia"}`);
  prepararArrasto(celula, chave, slot, item);
  if (legenda) celula.title = legenda;
  if (item) {
    const nome = item.nome || `Item ${item.id}`;
    celula.title = `${legenda ? `${legenda}: ` : ""}${semCores(nome)}${item.quantidade > 1 ? ` ×${item.quantidade}` : ""} (ID ${item.id}, slot ${item.slot})`;
    celula.append(imagemDoIcone(sexo, item.icone, nome));
    if (item.quantidade > 1) celula.append(criar("span", "qtd-item", fmt(item.quantidade)));
    celula.addEventListener("mouseenter", () => mostrarDica(chave, item, celula));
    celula.addEventListener("mouseleave", () => esconderDica());
    celula.addEventListener("click", () => abrirEdicaoDeItem(chave, rotulo, item, celula));
  } else {
    if (legenda) celula.append(criar("span", "rotulo-slot", legenda));
    celula.addEventListener("click", (evento) => { evento.stopPropagation?.(); mostrarMenuDoSlot(celula, { chave, rotulo, slot, legenda }); });
  }
  return celula;
}
/* Slot vazio (B216): o menu "Criar item" ancorado na célula. */
function mostrarMenuDoSlot(celula, destino) {
  esconderDica();
  estado.slotDoMenu = destino;
  const menu = elemento("menu-slot");
  menu.hidden = false;
  const r = celula.getBoundingClientRect?.();
  if (!r) return;
  const largura = menu.offsetWidth || 160;
  menu.style.left = `${Math.max(8, Math.min(r.left, (window.innerWidth || 1200) - largura - 8))}px`;
  menu.style.top = `${r.bottom + 4}px`;
}
function esconderMenuDoSlot() { elemento("menu-slot").hidden = true; }

/* Arrastar (B191): HTML5 drag-and-drop entre as janelas. Soltar num slot troca com o que estiver
   lá, pelos mesmos caminhos do jogo (o GS confere par, posição no corpo e requisitos). */
let arrasto = null;
function prepararArrasto(celula, chave, slot, item) {
  if (item) {
    celula.draggable = true;
    celula.addEventListener("dragstart", (evento) => {
      arrasto = { recipiente: chave, slot: item.slot, id: item.id, nome: semCores(item.nome) || `Item ${item.id}` };
      evento.dataTransfer?.setData?.("text/plain", arrasto.nome);
      if (evento.dataTransfer) evento.dataTransfer.effectAllowed = "move";
      esconderDica();
      celula.classList.add("arrastando");
    });
    celula.addEventListener("dragend", () => { celula.classList.remove("arrastando"); arrasto = null; });
  }
  celula.addEventListener("dragover", (evento) => {
    if (!arrasto) return;
    evento.preventDefault();
    celula.classList.add("alvo-arrasto");
  });
  celula.addEventListener("dragleave", () => celula.classList.remove("alvo-arrasto"));
  celula.addEventListener("drop", (evento) => {
    evento.preventDefault();
    celula.classList.remove("alvo-arrasto");
    const origem = arrasto;
    arrasto = null;
    if (!origem || (origem.recipiente === chave && origem.slot === slot)) return;
    moverItem(origem, chave, slot);
  });
}
function moverItem(origem, para, slot) {
  const nomes = Object.fromEntries(RECIPIENTES);
  return editarPersonagem({ mover_item: { de: origem.recipiente, slot_de: origem.slot, id: origem.id, para, slot_para: slot } },
    `${origem.nome}: ${nomes[origem.recipiente]} ${origem.slot} → ${nomes[para]} ${slot}`);
}

/* Dica (B189): pedida ao passar o mouse e guardada até a grade recarregar. Cada linha vem com
   os códigos de cor do cliente (`^RRGGBB`), pintados aqui. */
const dicas = new Map();
let dicaPedida = 0;
function linhaColorida(texto) {
  const p = criar("p");
  let cor = "ffffff";
  for (const parte of texto.split(/(\^[0-9a-fA-F]{6})/)) {
    if (/^\^[0-9a-fA-F]{6}$/.test(parte)) { cor = parte.slice(1); continue; }
    if (!parte) continue;
    const span = criar("span", "", parte);
    span.style.color = `#${cor}`;
    p.append(span);
  }
  return p;
}
async function mostrarDica(chave, item, celula) {
  const realm = estado.selecionado;
  const id = estado.personagemId;
  if (!realm || !id || !elemento("menu-slot").hidden) return;
  const caixa = elemento("dica-item");
  const marca = `${realm.id}:${id}:${chave}:${item.slot}:${item.id}`;
  const minha = ++dicaPedida;
  let linhas = dicas.get(marca);
  if (!linhas) {
    caixa.replaceChildren(linhaColorida(`^ffffff${item.nome || `Item ${item.id}`}`), linhaColorida("^b0b0b0…"));
    posicionarDica(caixa, celula);
    try {
      const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/itens/${chave}/${item.slot}/dica`);
      linhas = dados.linhas;
      dicas.set(marca, linhas);
    } catch (erro) {
      linhas = [`^ffffff${item.nome || `Item ${item.id}`}`, `^ff0000${erro.message}`];
    }
    if (minha !== dicaPedida) return;
  }
  caixa.replaceChildren(...linhas.map(linhaColorida));
  posicionarDica(caixa, celula);
}
function posicionarDica(caixa, celula) {
  caixa.hidden = false;
  const r = celula.getBoundingClientRect?.();
  if (!r) return;
  const largura = caixa.offsetWidth || 300;
  const x = r.right + 8 + largura > (window.innerWidth || 1200) ? r.left - largura - 8 : r.right + 8;
  caixa.style.left = `${Math.max(4, x)}px`;
  caixa.style.top = `${Math.max(4, r.top)}px`;
}
function esconderDica() {
  dicaPedida++;
  elemento("dica-item").hidden = true;
}

/* Equipamento e roupas (B190): as roupas são slots do próprio equipamento —
   `EQUIPIVTR_FASHION_BODY/LEG/FOOT/WRIST` 13–16, `_HEAD` 25 e `_WEAPON` 29 (`EC_IvtrTypes.h:56-85`). */
const SLOTS_DE_ROUPA = [13, 14, 15, 16, 25, 29];
const JANELAS_DE_ITENS = { bolsa: "janela-bolsa", armazem: "janela-armazem", missao: "janela-missao" };
function gradeDeItens(chave, rotulo, slots, porSlot, sexo) {
  const grade = criar("div", `grade-itens ${chave === "equipamento" ? "grade-corpo" : ""}`);
  for (const s of slots) {
    const legenda = chave === "equipamento" ? (SLOTS_DE_EQUIPAMENTO[s] || `Slot ${s}`) : "";
    grade.append(celulaDeItem(chave, rotulo, porSlot.get(s), sexo, legenda, s));
  }
  return grade;
}
const intervalo = (n) => Array.from({ length: n }, (_, i) => i);
function alvosDeItens() {
  return ["janela-equipamento", "janela-roupas", ...Object.values(JANELAS_DE_ITENS)].map(elemento);
}
async function carregarInventario(realm, id) {
  dicas.clear();
  esconderDica();
  esconderMenuDoSlot();
  for (const alvo of alvosDeItens()) alvo.replaceChildren(criar("p", "nota-vazia", "Carregando itens…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/inventario`);
    if (estado.personagemId !== id) return;
    const sexo = dados.sexo === "f" ? "f" : "m";
    estado.sexoDaFicha = sexo;
    for (const [chave, rotulo] of RECIPIENTES) {
      const itens = dados.recipientes?.[chave] || [];
      const porSlot = new Map(itens.map((i) => [i.slot, i]));
      const maior = itens.reduce((m, i) => Math.max(m, i.slot + 1), 0);
      if (chave === "equipamento") {
        const corpo = intervalo(Math.max(30, maior)).filter((s) => !SLOTS_DE_ROUPA.includes(s));
        elemento("janela-equipamento").replaceChildren(gradeDeItens(chave, rotulo, corpo, porSlot, sexo));
        elemento("janela-roupas").replaceChildren(gradeDeItens(chave, rotulo, SLOTS_DE_ROUPA, porSlot, sexo));
        continue;
      }
      const total = Math.max(TAMANHOS[chave], Math.ceil(maior / 8) * 8);
      const resumo = criar("p", "resumo-grade");
      resumo.append(criar("span", "", `${itens.length} de ${TAMANHOS[chave]} ocupados`), criar("span", "texto-suave", "slot vazio: criar item"));
      elemento(JANELAS_DE_ITENS[chave]).replaceChildren(resumo, gradeDeItens(chave, rotulo, intervalo(total), porSlot, sexo));
    }
  } catch (erro) {
    for (const alvo of alvosDeItens()) alvo.replaceChildren(criar("p", "nota-vazia", `Itens indisponíveis: ${erro.message}`));
  }
}

/* Editar item (B194; B216 no modal largo): abre com o detalhe do slot (a mesma consulta da dica),
   mostra a dica como o jogo e manda só os campos que mudaram; o GS confere o formato e regrava o
   bloco. Remover (B187) fica no rodapé: em jogo só bolsa e bolsa de missão. */
const CAMPOS_DE_REQUISITO = [["nivel", "ei-req-nivel"], ["forca", "ei-req-forca"], ["agilidade", "ei-req-agilidade"],
  ["vitalidade", "ei-req-vitalidade"], ["energia", "ei-req-energia"]];
const ORIGEM_DO_EFEITO = 0x8000 | 0x10000 | 0x20000;
/* B206: a descrição do efeito como a dica do cliente, refeita enquanto se digita. */
let esperaDaDescricao = new WeakMap();
async function descreverEfeito(linha) {
  const [id, args] = linha.querySelectorAll("input");
  const alvo = linha.querySelector(".descricao-efeito");
  const n = Math.trunc(Number(id.value));
  if (!alvo) return;
  if (!(n >= 1 && n <= 8191)) { alvo.textContent = ""; return; }
  try {
    const d = await api(`/api/efeitos/${n}?args=${encodeURIComponent(args.value.replace(/[^-0-9, ]/g, ""))}`);
    alvo.textContent = (d.linhas || []).join(" ") || "Efeito sem descrição no cliente.";
    marcarHabilidadeDoEfeito(linha, d.habilidade);
  } catch (_) { alvo.textContent = ""; }
}
function agendarDescricao(linha) {
  if (esperaDaDescricao.get(linha)) clearTimeout(esperaDaDescricao.get(linha));
  esperaDaDescricao.set(linha, temTempo ? setTimeout(() => descreverEfeito(linha), 300) : null);
}
/* B209: o GS diz, para a peça aberta, o valor do id, se o valor é editável, a faixa e se o efeito
   serve (`/api/realms/{realm}/efeitos?item=…&ids=…`). Fixo: o campo trava com o valor do id. */
function aplicarInfoDoEfeito(linha, info) {
  const [, args] = linha.querySelectorAll("input");
  const nota = linha.querySelector(".faixa-efeito");
  if (!info || !nota) return;
  if (info.recusa) {
    args.readOnly = false;
    nota.textContent = info.recusa === "efeito_inexistente" ? "Não existe neste realm."
      : "Não serve nesta peça (é de outra família).";
    return;
  }
  const valor = (info.args || []).join(", ");
  const unidade = info.porcento ? "%" : "";
  const faixa = info.faixa ? (info.faixa[0] === info.faixa[1] ? `${info.faixa[0]}${unidade}`
    : `${info.faixa[0]}–${info.faixa[1]}${unidade}`) : "";
  args.readOnly = !info.editavel;
  if (!info.editavel && args.value !== valor) { args.value = valor; agendarDescricao(linha); }
  if (info.editavel && !args.value) { args.value = valor; agendarDescricao(linha); }
  nota.textContent = (info.editavel ? `Editável${unidade ? " (em %)" : ""} · valor do id: ${faixa}`
    : `Fixo pelo id: ${valor}`) + (info.age === false ? " · ainda não age no jogo" : "");
}
async function informarEfeitos(linhas) {
  const em = estado.itemEmEdicao;
  const realm = estado.selecionado;
  const ids = linhas.map((l) => Math.trunc(Number(l.querySelector("input").value))).filter((n) => n >= 1 && n <= 8191);
  if (!em || !realm || !ids.length) return;
  try {
    const d = await api(`/api/realms/${encodeURIComponent(realm.id)}/efeitos?item=${em.item.id}&ids=${[...new Set(ids)].join(",")}`);
    const porId = new Map((d.efeitos || []).map((e) => [e.id, e]));
    for (const l of linhas) aplicarInfoDoEfeito(l, porId.get(Math.trunc(Number(l.querySelector("input").value))));
  } catch (_) { /* sem o GS: a linha fica livre, o GS recusa ao gravar */ }
}
/* B213: efeito que dá uma habilidade (`item_skill_addon`, tipo 55 do cliente) — ícone da habilidade
   ou, sem ícone no cliente, o selo ✦; a dica diz qual habilidade é. */
function seloDeHabilidade(h) {
  const selo = criar("span", "selo-habilidade");
  selo.title = `Habilidade do item: ${h.nome} (ID ${h.id})`;
  if (h.icone) {
    const img = criar("img");
    img.src = `/api/icones/habilidade/${h.icone}.png`;
    img.alt = h.nome; img.width = 24; img.height = 24;
    img.addEventListener("error", () => img.replaceWith(document.createTextNode("✦")));
    selo.append(img);
  } else {
    selo.textContent = "✦";
  }
  return selo;
}
function marcarHabilidadeDoEfeito(linha, h) {
  linha.querySelector(".selo-habilidade")?.remove();
  linha.classList.toggle("efeito-habilidade", Boolean(h));
  if (h) linha.querySelector(".descricao-efeito")?.prepend(seloDeHabilidade(h));
}
let esperaDaInfo = new WeakMap();
function linhaDeEfeito(efeito = { id: "", args: [] }) {
  const linha = criar("div", "linha-efeito");
  const id = criar("input");
  id.type = "number"; id.min = "1"; id.max = "8191"; id.value = efeito.id; id.placeholder = "id"; id.setAttribute("aria-label", "Id do efeito");
  const args = criar("input");
  args.value = (efeito.args || []).join(", "); args.placeholder = "parâmetros (até 3)"; args.setAttribute("aria-label", "Parâmetros do efeito");
  const tirar = criar("button", "botao-icone botao-icone-pequeno");
  tirar.append(icone("lixeira"));
  tirar.type = "button"; tirar.setAttribute("aria-label", "Tirar efeito"); tirar.title = "Tirar efeito";
  tirar.addEventListener("click", () => linha.remove());
  linha.append(id, args, tirar);
  const descricao = criar("small", "descricao-efeito", efeito.texto || "");
  linha.append(descricao, criar("small", "faixa-efeito", ""));
  id.addEventListener("input", () => {
    args.value = ""; args.readOnly = false;
    agendarDescricao(linha);
    if (esperaDaInfo.get(linha)) clearTimeout(esperaDaInfo.get(linha));
    esperaDaInfo.set(linha, temTempo ? setTimeout(() => informarEfeitos([linha]), 300) : null);
  });
  args.addEventListener("input", () => agendarDescricao(linha));
  if (!efeito.texto && efeito.id) agendarDescricao(linha);
  if (efeito.habilidade) marcarHabilidadeDoEfeito(linha, efeito.habilidade);
  if (efeito.editavel !== undefined) aplicarInfoDoEfeito(linha, efeito);
  return linha;
}
async function abrirEdicaoDeItem(chave, rotulo, item, celula) {
  const realm = estado.selecionado;
  const id = estado.personagemId;
  if (!realm || !id) return;
  esconderDica();
  for (const outra of document.querySelectorAll?.(".celula-item.escolhido") || []) outra.classList.remove("escolhido");
  celula.classList.add("escolhido");
  let dados;
  try {
    dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/itens/${chave}/${item.slot}/dica`);
  } catch (erro) { alerta("erro", "Item indisponível", erro.message); return; }
  if (estado.personagemId !== id) return;
  const atual = dados.item || {};
  const equip = atual.equipamento || null;
  estado.itemEmEdicao = { chave, rotulo, item, atual };
  elemento("titulo-item").replaceChildren(textoPintado(atual.nome || `Item ${item.id}`));
  elemento("resumo-item").textContent = `${rotulo}, slot ${item.slot} · ID ${item.id}${atual.pilha > 1 ? ` · pilha de ${fmt(atual.pilha)}` : ""}`;
  elemento("ei-icone").replaceChildren(imagemDoIcone(estado.sexoDaFicha || "m", atual.icone || item.icone, atual.nome || ""));
  elemento("ei-previa").replaceChildren(...(dados.linhas || []).map(linhaColorida));
  elemento("ei-quantidade").value = String(atual.quantidade ?? item.quantidade);
  elemento("ei-quantidade").max = String(atual.pilha || 100000);
  elemento("remover-quantidade").max = String(item.quantidade);
  elemento("remover-quantidade").value = String(item.quantidade);
  elemento("ei-equipamento").hidden = !equip;
  if (equip) {
    elemento("ei-durabilidade").value = String(equip.durabilidade ?? 0);
    elemento("ei-durabilidade-max").value = String(equip.durabilidade_maxima ?? 0);
    elemento("ei-fabricante").value = equip.fabricante || "";
    const refino = elemento("ei-refino");
    refino.replaceChildren(...Array.from({ length: 13 }, (_, n) => {
      const o = criar("option", "", n ? `+${n}` : "Sem refino"); o.value = String(n); return o;
    }));
    refino.value = String(equip.refino || 0);
    const req = equip.requisitos || {};
    for (const [campo, alvo] of CAMPOS_DE_REQUISITO) elemento(alvo).value = String(req[campo] ?? 0);
    const classes = elemento("ei-classes");
    classes.replaceChildren(...(dados.classes || []).map((nome, i) => {
      const rotuloClasse = criar("label", "chip");
      const caixa = criar("input"); caixa.type = "checkbox"; caixa.value = String(i);
      caixa.checked = ((req.classes ?? 0xFFFF) & (1 << i)) !== 0;
      rotuloClasse.append(caixa, document.createTextNode(nome));
      return rotuloClasse;
    }));
    elemento("ei-pedras").value = (equip.furos || []).map((f) => f.id || 0).join(", ");
    elemento("ei-furos").value = String((equip.furos || []).length);
    elemento("ei-busca-pedra").value = "";
    elemento("ei-resultados-pedra").hidden = true;
    elemento("ei-busca-efeito").value = "";
    elemento("ei-resultados-efeito").hidden = true;
    const linhas = (equip.efeitos || []).filter((e) => !((e.tipo || 0) & ORIGEM_DO_EFEITO)).map((e) => linhaDeEfeito(e));
    elemento("ei-efeitos").replaceChildren(...linhas);
    informarEfeitos(linhas);
  }
  abrirModal("modal-item");
}
function fecharEdicaoDeItem() { fecharModal("modal-item"); }
const inteiro = (alvo) => Math.trunc(Number(elemento(alvo).value));
function lerEfeitos() {
  const lista = [];
  for (const linha of elemento("ei-efeitos").children) {
    const [id, args] = linha.querySelectorAll("input");
    if (!id.value) continue;
    lista.push({ id: Math.trunc(Number(id.value)),
      args: args.value.split(/[,\s]+/).filter(Boolean).map((v) => Math.trunc(Number(v))) });
  }
  return lista;
}
async function salvarEdicaoDeItem() {
  const em = estado.itemEmEdicao;
  if (!em) return;
  const atual = em.atual;
  const equip = atual.equipamento || null;
  const edicao = {};
  const q = inteiro("ei-quantidade");
  if (q !== (atual.quantidade ?? em.item.quantidade)) edicao.quantidade = q;
  if (equip) {
    const d = inteiro("ei-durabilidade"), m = inteiro("ei-durabilidade-max");
    if (d !== equip.durabilidade) edicao.durabilidade = d;
    if (m !== equip.durabilidade_maxima) edicao.durabilidade_maxima = m;
    const r = inteiro("ei-refino");
    if (r !== (equip.refino || 0)) edicao.refino = r;
    const fabricante = elemento("ei-fabricante").value;
    if (fabricante !== (equip.fabricante || "")) edicao.fabricante = fabricante;
    const req = Object.fromEntries(CAMPOS_DE_REQUISITO.map(([campo, alvo]) => [campo, inteiro(alvo)]));
    req.classes = [...elemento("ei-classes").querySelectorAll("input")].reduce((m, c) => (c.checked ? m | (1 << Number(c.value)) : m),
      (equip.requisitos?.classes ?? 0) & ~((1 << elemento("ei-classes").children.length) - 1));
    const antes = equip.requisitos || {};
    if (["nivel", "classes", "forca", "agilidade", "vitalidade", "energia"].some((c) => req[c] !== (antes[c] ?? 0))) edicao.requisitos = req;
    const pedras = elemento("ei-pedras").value.split(/[,\s]+/).filter(Boolean).map((v) => Math.trunc(Number(v)));
    if (pedras.join(",") !== (equip.furos || []).map((f) => f.id || 0).join(",")) edicao.pedras = pedras;
    const efeitos = lerEfeitos();
    const efeitosAntes = (equip.efeitos || []).filter((e) => !((e.tipo || 0) & ORIGEM_DO_EFEITO)).map((e) => ({ id: e.id, args: e.args || [] }));
    if (JSON.stringify(efeitos) !== JSON.stringify(efeitosAntes)) edicao.efeitos = efeitos;
  }
  if (!Object.keys(edicao).length) { alerta("aviso", "Nada mudou", "Altere algum campo antes de salvar.", { toast: true }); return; }
  const nome = semCores(atual.nome) || `Item ${em.item.id}`;
  const resultado = await editarPersonagem({ editar_item: { recipiente: em.chave, slot: em.item.slot, id: em.item.id, edicao } },
    `${nome}: ${Object.keys(edicao).join(", ")}`);
  if (resultado === "aplicado" || resultado === "salvo") fecharEdicaoDeItem();
}
/* Remover (B187): do rodapé do modal, com a quantidade (a pilha inteira por padrão). */
async function removerItem() {
  const em = estado.itemEmEdicao;
  if (!em) return;
  const maximo = em.item.quantidade;
  const nome = semCores(em.atual.nome) || `Item ${em.item.id}`;
  const quantidade = Math.trunc(Number(elemento("remover-quantidade").value));
  if (!Number.isFinite(quantidade) || quantidade < 1 || quantidade > maximo) { alerta("erro", "Valor inválido", `De 1 a ${fmt(maximo)}.`); return; }
  const sim = await confirmar("Remover item?", `${fmt(quantidade)} × ${nome}. Não há como desfazer.`, "Remover");
  if (!sim) { fecharAlerta(); return; }
  fecharEdicaoDeItem();
  await editarPersonagem({ remover_item: { recipiente: em.chave, slot: em.item.slot, id: em.item.id, quantidade } },
    `Removido ${fmt(quantidade)} × ${nome}`);
}

/* Criar item (B216): o slot vazio da bolsa, do armazém ou da bolsa de missão recebe uma pilha
   gerada como o drop (`item` com `recipiente`/`slot`); o do corpo filtra pelo que veste nele
   (`slot:N`) e o item nasce na bolsa, para arrastar. "Dar item" (sem slot) é o prêmio de missão:
   qualquer quantidade, em lotes de uma pilha. */
const CATEGORIAS_DE_ITEM = [["", "Todas as categorias"], ["arma", "Armas"], ["armadura", "Armaduras"], ["ornamento", "Ornamentos"],
  ["roupa", "Roupas"], ["voo", "Voo"], ["municao", "Munição"], ["pocao", "Poções e amuletos"], ["material", "Materiais"],
  ["pedra", "Pedras"], ["refino", "Refino"], ["missao", "Itens de missão"], ["mascote", "Mascotes"], ["habilidade", "Livros de habilidade"],
  ["daimon", "Daimon"], ["outros", "Outros"]];
const NOMES_DE_CATEGORIA = Object.fromEntries(CATEGORIAS_DE_ITEM);
function abrirCriarItem(destino) {
  estado.criacao = destino;
  estado.itemParaCriar = null;
  const categoria = elemento("ci-categoria");
  const opcoes = destino.chave === "equipamento"
    ? [[`slot:${destino.slot}`, `Veste em: ${destino.legenda}`], ...CATEGORIAS_DE_ITEM]
    : CATEGORIAS_DE_ITEM;
  categoria.replaceChildren(...opcoes.map(([v, n]) => { const o = criar("option", "", n); o.value = v; return o; }));
  categoria.value = destino.chave === "equipamento" ? `slot:${destino.slot}` : (destino.chave === "missao" ? "missao" : "");
  const nomes = Object.fromEntries(RECIPIENTES);
  elemento("titulo-criar-item").textContent = destino.template ? "Pôr item no template" : destino.slot === undefined ? "Dar item" : "Criar item";
  elemento("ci-destino").textContent = destino.template
    ? `${destino.chave === "equipamento" ? `Corpo, ${destino.legenda}` : `Bolsa, slot ${destino.slot}`}: o personagem novo nasce com este item aqui.`
    : destino.slot === undefined
    ? "Vai para a bolsa (item de missão, para a bolsa de missão), em lotes de uma pilha."
    : destino.chave === "equipamento"
      ? `${destino.legenda}: o item nasce na bolsa — depois arraste-o para o slot para vestir.`
      : `${nomes[destino.chave]}, slot ${destino.slot}: uma pilha nasce neste slot.`;
  elemento("ci-busca").value = "";
  elemento("ci-quantidade").value = "1";
  elemento("ci-escolhido").replaceChildren(criar("span", "texto-suave", "Escolha um item da lista."));
  elemento("ci-resultados").replaceChildren();
  abrirModal("modal-criar-item");
  if (categoria.value) buscarParaCriar();
}
let buscaDeCriacao = 0;
async function buscarParaCriar() {
  const realm = estado.selecionado;
  const texto = elemento("ci-busca").value.trim();
  const categoria = elemento("ci-categoria").value;
  const alvo = elemento("ci-resultados");
  if (!realm || (!categoria && texto.length < 2 && !/^\d+$/.test(texto))) {
    alvo.replaceChildren(criar("p", "nota-vazia", "Digite o nome ou o ID, ou escolha uma categoria."));
    return;
  }
  const minha = ++buscaDeCriacao;
  alvo.replaceChildren(criar("p", "nota-vazia", "Buscando…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/itens?busca=${encodeURIComponent(texto)}`
      + (categoria ? `&categoria=${encodeURIComponent(categoria)}` : ""));
    if (minha !== buscaDeCriacao) return;
    alvo.replaceChildren(...(dados.itens || []).map((item) => cartaoDeResultado(
      imagemDoIcone(estado.sexoDaFicha || "m", item.icone, item.nome || ""), item.nome || `Item ${item.id}`,
      `ID ${item.id} · ${NOMES_DE_CATEGORIA[item.categoria] || item.categoria || "—"} · pilha ${fmt(item.pilha)}${item.missao ? " · missão" : ""}`,
      (cartao) => escolherParaCriar(item, cartao))));
    if (!(dados.itens || []).length) alvo.append(criar("p", "nota-vazia", "Nenhum item encontrado."));
  } catch (erro) {
    alvo.replaceChildren(criar("p", "nota-vazia", erro.message));
  }
}
/* Um cartão clicável de resultado de busca (ícone, nome pintado, detalhe). */
function cartaoDeResultado(figura, nome, detalhe, escolher) {
  const cartao = criar("button", "cartao-resultado");
  cartao.type = "button";
  const texto = criar("span", "texto-resultado");
  texto.append(textoPintado(nome, "nome-resultado"), criar("small", "", detalhe));
  cartao.append(criar("span", "figura-resultado"), texto);
  cartao.firstChild.append(figura);
  cartao.addEventListener("click", () => {
    for (const outro of cartao.parentNode?.children || []) outro.classList?.remove("escolhido");
    cartao.classList.add("escolhido");
    escolher(cartao);
  });
  return cartao;
}
function escolherParaCriar(item) {
  estado.itemParaCriar = item;
  const destino = estado.criacao || {};
  const quantidade = elemento("ci-quantidade");
  const comSlot = destino.template || (destino.slot !== undefined && destino.chave !== "equipamento");
  quantidade.max = String(comSlot ? item.pilha || 1 : 100000);
  if (Number(quantidade.value) > Number(quantidade.max)) quantidade.value = quantidade.max;
  const resumo = criar("span", "escolhido-linha");
  resumo.append(imagemDoIcone(estado.sexoDaFicha || "m", item.icone, item.nome || ""), textoPintado(item.nome || `Item ${item.id}`),
    criar("small", "texto-suave", `ID ${item.id}${comSlot ? ` · até ${fmt(item.pilha)} (uma pilha)` : ""}`));
  elemento("ci-escolhido").replaceChildren(resumo);
}
async function criarItem() {
  const item = estado.itemParaCriar;
  const destino = estado.criacao || {};
  if (!item) { alerta("erro", "Escolha o item", "Busque e clique num item da lista."); return; }
  const quantidade = Math.trunc(Number(elemento("ci-quantidade").value));
  const comSlot = destino.template || (destino.slot !== undefined && destino.chave !== "equipamento");
  const maximo = comSlot ? (item.pilha || 1) : 100_000;
  if (!Number.isFinite(quantidade) || quantidade < 1 || quantidade > maximo) { alerta("erro", "Quantidade inválida", `De 1 a ${fmt(maximo)}.`); return; }
  const nome = semCores(item.nome) || `item ${item.id}`;
  if (destino.template) {
    const feito = await editarTemplate({ item: { recipiente: destino.chave, slot: destino.slot, id: item.id, quantidade } },
      `${fmt(quantidade)} × ${nome} no template (${destino.chave === "equipamento" ? destino.legenda : `bolsa, slot ${destino.slot}`})`);
    if (feito) fecharModal("modal-criar-item");
    return;
  }
  const pedido = { id: item.id, quantidade, ...(comSlot ? { recipiente: destino.chave, slot: destino.slot } : {}) };
  const nomes = Object.fromEntries(RECIPIENTES);
  const r = await editarPersonagem({ item: pedido },
    comSlot ? `Criado ${fmt(quantidade)} × ${nome} em ${nomes[destino.chave]}, slot ${destino.slot}` : `Dado ${fmt(quantidade)} × ${nome}`);
  if (r === "aplicado" || r === "salvo") fecharModal("modal-criar-item");
}

/* ---------------- Habilidades (B195–B196; B216) ----------------
   As aprendidas, do banco (a fonte do GS), com o nome do `skillstr.txt` e o ícone do atlas
   `IconList_Skill`; o nível no canto, como na janela do jogo. Clicar abre o modal do nível
   (subir, descer ou remover); "Dar habilidade" busca no cliente por nome, ID e classe. O GS manda
   o `LEARN_SKILL` em jogo, nas duas versões. */
async function carregarHabilidades(realm, id) {
  const alvo = elemento("janela-habilidades");
  alvo.replaceChildren(criar("p", "nota-vazia", "Carregando habilidades…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/habilidades`);
    if (estado.personagemId !== id) return;
    const lista = dados.habilidades || [];
    estado.habilidadesDaFicha = lista;
    const grade = criar("div", "grade-itens grade-habilidades");
    for (const h of lista) {
      const celula = criar("button", "celula-item ocupada");
      celula.type = "button";
      celula.title = `${h.nome} · nível ${h.nivel}${h.nivel_maximo ? `/${h.nivel_maximo}` : ""} (ID ${h.id})`;
      celula.append(imagemDoIcone("habilidade", h.icone, h.nome), criar("span", "qtd-item", String(h.nivel)));
      celula.addEventListener("click", () => abrirHabilidade(h, true));
      grade.append(celula);
    }
    const resumo = criar("p", "resumo-grade");
    resumo.append(criar("span", "", `${lista.length} habilidade${lista.length === 1 ? "" : "s"}`), criar("span", "texto-suave", "clique para editar"));
    alvo.replaceChildren(resumo, grade);
    if (!lista.length) alvo.append(criar("p", "nota-vazia", "Nenhuma habilidade aprendida."));
  } catch (erro) {
    alvo.replaceChildren(criar("p", "nota-vazia", `Habilidades indisponíveis: ${erro.message}`));
  }
}
/* `possuida`: a do personagem (nível atual, remover); senão, ensinar a do catálogo. */
function abrirHabilidade(h, possuida) {
  const atual = possuida ? h : habilidadesAtuais().find((x) => x.id === h.id);
  const dono = modoTemplate() ? "o template" : "o personagem";
  estado.habilidade = { ...h, possuida: Boolean(atual), nivel: atual?.nivel };
  elemento("hab-qual").textContent = h.nome;
  elemento("hab-icone").replaceChildren(imagemDoIcone("habilidade", h.icone, h.nome));
  elemento("hab-detalhe").textContent = `ID ${h.id} · nível máximo ${h.nivel_maximo ?? "?"}`
    + (atual ? ` · ${dono} tem no nível ${atual.nivel}` : ` · ${dono} ainda não tem`);
  elemento("hab-nivel").max = String(h.nivel_maximo || 255);
  elemento("hab-nivel").value = String(atual?.nivel || 1);
  elemento("hab-remover").hidden = !atual;
  elemento("hab-aplicar").textContent = atual ? "Aplicar nível" : "Ensinar";
  fecharModal("modal-busca-habilidade");
  abrirModal("modal-habilidade");
}
async function aplicarHabilidade(nivel) {
  const h = estado.habilidade;
  if (!h) return;
  const n = Math.trunc(Number(nivel));
  if (!Number.isFinite(n) || n < 0 || n > (h.nivel_maximo || 255) || (n === 0 && !h.possuida)) {
    alerta("erro", "Nível inválido", `De 1 a ${h.nivel_maximo || 255}.`); return;
  }
  if (modoTemplate()) {
    if (n === 0 && !(await confirmar("Tirar habilidade?", `${h.nome} deixa de vir com o personagem novo.`, "Tirar"))) return;
    const feito = await editarTemplate({ habilidade: { id: h.id, nivel: n } },
      n ? `${h.nome}: nível ${n} no template` : `${h.nome}: tirada do template`);
    if (feito) { fecharModal("modal-habilidade"); estado.habilidade = null; }
    return;
  }
  if (n === 0 && !(await confirmar("Remover habilidade?", `${h.nome} sai das habilidades do personagem.`, "Remover"))) return;
  const r = await editarPersonagem({ habilidade: { id: h.id, nivel: n } },
    n ? `${h.nome}: ${h.possuida ? "nível" : "ensinada no nível"} ${n}` : `${h.nome}: removida`);
  if (r === "aplicado" || r === "salvo") { fecharModal("modal-habilidade"); estado.habilidade = null; }
}
async function abrirBuscaDeHabilidade(classe = "") {
  elemento("hb-busca").value = "";
  elemento("hb-resultados").replaceChildren(criar("p", "nota-vazia", "Digite o nome ou o ID, ou escolha a classe."));
  await preencherGruposDeHabilidade(elemento("hb-classe"), classe);
  abrirModal("modal-busca-habilidade");
  if (classe !== "") buscarHabilidade();
}
/* Busca no catálogo de habilidades do cliente; `classe` é o `cls` do stub ("" = todas). */
async function catalogoDeHabilidades(texto, classe) {
  const params = new URLSearchParams();
  if (texto) params.set("busca", texto);
  if (classe !== "") params.set("classe", classe);
  return (await api(`/api/habilidades?${params}`)).habilidades || [];
}
let buscaDeHabilidade = 0;
async function buscarHabilidade() {
  const texto = elemento("hb-busca").value.trim();
  const classe = elemento("hb-classe").value;
  const alvo = elemento("hb-resultados");
  if (classe === "" && texto.length < 2 && !/^\d+$/.test(texto)) {
    alvo.replaceChildren(criar("p", "nota-vazia", "Digite o nome ou o ID, ou escolha a classe."));
    return;
  }
  const minha = ++buscaDeHabilidade;
  try {
    const lista = await catalogoDeHabilidades(texto, classe);
    if (minha !== buscaDeHabilidade) return;
    const possuidas = new Map(habilidadesAtuais().map((h) => [h.id, h.nivel]));
    alvo.replaceChildren(...lista.map((h) => cartaoDeResultado(imagemDoIcone("habilidade", h.icone, h.nome), h.nome,
      `ID ${h.id} · até nível ${h.nivel_maximo ?? "?"}${possuidas.has(h.id) ? ` · tem nível ${possuidas.get(h.id)}` : ""}`,
      () => abrirHabilidade(h, false))));
    if (!lista.length) alvo.append(criar("p", "nota-vazia", "Nenhuma habilidade encontrada."));
  } catch (erro) {
    alvo.replaceChildren(criar("p", "nota-vazia", erro.message));
  }
}

/* ---------------- Daimon (E6, B215) ----------------
   O Daimon vestido (slot 23) — a ficha do bloco do item, em jogo (memória) ou no banco; "Dar EXP"
   segue a regra do jogo (`InsertExp`): sobe quantos níveis couberem, sem passar o dono. */
async function carregarDaimon(realm, id) {
  const alvo = elemento("janela-daimon");
  const janela = alvo.closest(".janela");
  if (janela) janela.hidden = false;
  elemento("abrir-daimon-exp").hidden = true;
  alvo.replaceChildren(criar("p", "nota-vazia", "Carregando Daimon…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/daimon`);
    if (estado.personagemId !== id) return;
    // B218: a versão sem Daimon (1.2.6 — o elements v7 não tem `GOBLIN_ESSENCE`) não mostra a janela.
    if (dados.existe_na_versao === false) { if (janela) janela.hidden = true; return; }
    const d = dados.daimon;
    if (!d) { alvo.replaceChildren(criar("p", "nota-vazia", "Sem Daimon vestido.")); return; }
    const linha = (ico, rotulo, valor) => {
      const caixa = criar("div", "atributo");
      const texto = criar("div");
      texto.append(criar("span", "", rotulo), criar("strong", "", String(valor)));
      caixa.append(icone(ico), texto);
      return caixa;
    };
    const titulo = criar("p", "titulo-daimon");
    titulo.append(textoPintado(d.nome || `Daimon ${d.item_id}`), criar("span", "etiqueta etiqueta-neutra", d.origem === "em_jogo" ? "em jogo" : "gravado"));
    const grade = criar("div", "grade-atributos");
    grade.append(linha("nivel", "Nível", d.nivel),
      linha("exp", "EXP", `${Number(d.exp).toLocaleString("pt-BR")} / ${Number(d.exp_para_subir).toLocaleString("pt-BR")}`),
      linha("forca", "Força", d.forca), linha("agilidade", "Agilidade", d.agilidade),
      linha("vitalidade", "Vitalidade", d.vitalidade), linha("energia", "Energia", d.energia),
      linha("pontos", "Pontos livres", d.pontos_livres), linha("brilho", "Gênios", (d.genios || []).join(" · ")),
      linha("pontos", "Pontos de gênio", d.pontos_de_genio), linha("refino", "Refino", d.refino), linha("vitalidade", "Vigor", d.vigor));
    const habilidades = criar("p", "dica", `Habilidades: ${(d.habilidades || []).map((h) => `${h.nome} ${h.nivel}`).join(", ") || "—"}`);
    alvo.replaceChildren(titulo, grade, habilidades);
    elemento("abrir-daimon-exp").hidden = false;
  } catch (erro) {
    alvo.replaceChildren(criar("p", "nota-vazia", `Daimon indisponível: ${erro.message}`));
  }
}
async function darExpAoDaimon() {
  const n = Math.trunc(Number(elemento("daimon-exp").value));
  if (!(n >= 1 && n <= 2000000000)) { alerta("erro", "EXP inválida", "De 1 a 2.000.000.000."); return; }
  const r = await editarPersonagem({ daimon_exp: n }, `Daimon: +${n.toLocaleString("pt-BR")} de EXP`);
  if (r === "aplicado" || r === "salvo") fecharModal("modal-daimon");
}

/* ---------------- Mascotes (B197; B216) ----------------
   A jaula com o ícone do modelo (`IconList_Pet`), nível e qual está invocado. Clicar abre o modal
   de edição (campos do registro e as habilidades por busca); "Libertar" tira da jaula. "Dar
   mascote" busca nos ovos do realm por nome, ID, tipo e ambiente e dá pela incubação do jogo. Em
   jogo o GS manda `PET_ROOM`/`FREE_PET`/`GAIN_PET`; o mascote invocado não se edita por fora. */
const TIPOS_DE_MASCOTE = ["Montaria", "Combate", "Seguidor", "Invocação", "Planta", "Evolução"];
const AMBIENTES = ["Terrestre", "Aquático", "Voador", "Terra e água", "Terra e ar", "Água e ar", "Todos"];
async function carregarMascotes(realm, id) {
  const alvo = elemento("janela-mascotes");
  alvo.replaceChildren(criar("p", "nota-vazia", "Carregando mascotes…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/mascotes`);
    if (estado.personagemId !== id) return;
    alvo.replaceChildren();
    for (const m of dados.mascotes || []) {
      const cartao = criar("button", "cartao-mascote");
      cartao.type = "button";
      const figura = criar("span", "figura-resultado");
      figura.append(imagemDoIcone("mascote", m.icone, m.nome || m.modelo || "?"));
      const texto = criar("span", "texto-resultado");
      texto.append(textoPintado(m.nome || m.modelo || `Mascote ${m.tid}`, "nome-resultado"),
        criar("small", "", `${TIPOS_DE_MASCOTE[m.classe] || "Mascote"} · ${semCores(m.modelo) || `Modelo ${m.tid}`} · slot ${m.slot}`));
      const lado = criar("span", m.invocado ? "etiqueta" : "etiqueta etiqueta-neutra", m.invocado ? "Invocado" : `Nv. ${m.nivel}`);
      cartao.append(figura, texto, lado);
      cartao.addEventListener("click", () => abrirMascote(m));
      alvo.append(cartao);
    }
    if (!(dados.mascotes || []).length) alvo.append(criar("p", "nota-vazia", "Jaula vazia."));
  } catch (erro) {
    alvo.replaceChildren(criar("p", "nota-vazia", `Mascotes indisponíveis: ${erro.message}`));
  }
}
const CAMPOS_DO_MASCOTE = [["nivel", "pet-nivel"], ["exp", "pet-exp"], ["lealdade", "pet-lealdade"], ["fome", "pet-fome"], ["pontos", "pet-pontos"]];
/* `m` da jaula (editar) ou, com `novo`, o ovo do catálogo (dar). */
async function abrirMascote(m, novo = false) {
  estado.mascote = novo ? null : m;
  estado.ovoParaDar = novo ? m : null;
  elemento("mascote-qual").replaceChildren(textoPintado(m.nome || m.modelo || `Mascote ${m.tid || m.mascote}`));
  elemento("pet-icone").replaceChildren(imagemDoIcone("mascote", m.icone, m.nome || m.modelo || "?"));
  elemento("mascote-detalhe").textContent = novo
    ? `${TIPOS_DE_MASCOTE[m.tipo] || "Mascote"} · ${AMBIENTES[m.ambiente] || "—"} · ovo ${m.ovo} (${semCores(m.nome_do_ovo) || "—"}) · nível ${m.nivel}`
    : `${TIPOS_DE_MASCOTE[m.classe] || "Mascote"} · ${semCores(m.modelo) || `Modelo ${m.tid}`} · slot ${m.slot}${m.invocado ? " · invocado — recolha antes de editar" : ""}`;
  for (const [campo, alvo] of CAMPOS_DO_MASCOTE) {
    elemento(alvo).value = String(novo ? (campo === "nivel" ? m.nivel : 0) : (m[campo] ?? 0));
    elemento(alvo).disabled = novo;
  }
  elemento("pet-nivel").max = String((!novo && m.nivel_maximo) || 999);
  elemento("pet-nome").value = novo ? "" : (m.nome || "");
  elemento("pet-nome").disabled = novo;
  estado.habilidadesDoMascote = (m.habilidades || []).map((h) => ({ ...h }));
  desenharHabilidadesDoMascote(novo);
  elemento("pet-hab-adicionar").hidden = novo;
  elemento("pet-hab-resultados").replaceChildren();
  elemento("pet-hab-busca").value = "";
  elemento("pet-libertar").hidden = novo;
  elemento("pet-salvar").textContent = novo ? "Dar mascote" : "Salvar";
  if (!novo) await preencherGruposDeHabilidade(elemento("pet-hab-classe"), "127");
  fecharModal("modal-busca-mascote");
  abrirModal("modal-mascote");
}
/* As habilidades do mascote (até 8): ícone, nome, nível editável e tirar. */
function desenharHabilidadesDoMascote(somenteLeitura = false) {
  const lista = estado.habilidadesDoMascote || [];
  elemento("pet-hab-total").textContent = `${lista.length} de 8`;
  elemento("pet-hab-lista").replaceChildren(...lista.map((h, i) => {
    const linha = criar("div", "linha-hab-mascote");
    const nivel = criar("input");
    nivel.type = "number"; nivel.min = "1"; nivel.max = String(h.nivel_maximo || 255); nivel.value = String(h.nivel);
    nivel.disabled = somenteLeitura; nivel.setAttribute("aria-label", `Nível de ${h.nome}`);
    nivel.addEventListener("input", () => { h.nivel = Math.trunc(Number(nivel.value)) || 1; });
    const nome = criar("span", "texto-resultado");
    nome.append(criar("span", "nome-resultado", h.nome || `Habilidade ${h.id}`), criar("small", "", `ID ${h.id}`));
    const figura = criar("span", "figura-resultado");
    figura.append(imagemDoIcone("habilidade", h.icone, h.nome || "?"));
    linha.append(figura, nome, nivel);
    if (!somenteLeitura) {
      const tirar = criar("button", "botao-icone botao-icone-pequeno");
      tirar.type = "button"; tirar.title = "Tirar"; tirar.setAttribute("aria-label", `Tirar ${h.nome}`);
      tirar.append(icone("lixeira"));
      tirar.addEventListener("click", () => { lista.splice(i, 1); desenharHabilidadesDoMascote(); });
      linha.append(tirar);
    }
    return linha;
  }));
  if (!lista.length) elemento("pet-hab-lista").append(criar("p", "nota-vazia", "Sem habilidades."));
}
let buscaDeHabilidadeDoMascote = 0;
async function buscarHabilidadeDoMascote() {
  const texto = elemento("pet-hab-busca").value.trim();
  const classe = elemento("pet-hab-classe").value;
  const alvo = elemento("pet-hab-resultados");
  if (classe === "" && texto.length < 2 && !/^\d+$/.test(texto)) { alvo.replaceChildren(); return; }
  const minha = ++buscaDeHabilidadeDoMascote;
  try {
    const lista = await catalogoDeHabilidades(texto, classe);
    if (minha !== buscaDeHabilidadeDoMascote) return;
    alvo.replaceChildren(...lista.map((h) => cartaoDeResultado(imagemDoIcone("habilidade", h.icone, h.nome), h.nome,
      `ID ${h.id} · até nível ${h.nivel_maximo ?? "?"}`, () => {
        const atuais = estado.habilidadesDoMascote || [];
        if (atuais.some((x) => x.id === h.id)) { alerta("aviso", "Já tem", `${h.nome} já está nas habilidades do mascote.`, { toast: true }); return; }
        if (atuais.length >= 8) { alerta("aviso", "Sem espaço", "O mascote tem no máximo 8 habilidades.", { toast: true }); return; }
        atuais.push({ id: h.id, nivel: 1, nome: h.nome, icone: h.icone, nivel_maximo: h.nivel_maximo });
        desenharHabilidadesDoMascote();
      })));
    if (!lista.length) alvo.append(criar("p", "nota-vazia", "Nenhuma habilidade encontrada."));
  } catch (erro) { alvo.replaceChildren(criar("p", "nota-vazia", erro.message)); }
}
async function salvarMascote(libertar = false) {
  const ovo = estado.ovoParaDar;
  if (ovo && !libertar) {
    const r = await editarPersonagem({ dar_mascote: { ovo: ovo.ovo } }, `Mascote dado: ${semCores(ovo.nome) || `ovo ${ovo.ovo}`}`);
    if (r === "aplicado" || r === "salvo") fecharModal("modal-mascote");
    return;
  }
  const m = estado.mascote;
  if (!m) return;
  let edicao;
  if (libertar) {
    const sim = await confirmar("Libertar mascote", `${semCores(m.nome || m.modelo)} sai da jaula.`, "Libertar");
    if (!sim) return;
    edicao = { libertar: true };
  } else {
    edicao = {};
    for (const [campo, alvo] of CAMPOS_DO_MASCOTE) {
      const v = Math.trunc(Number(elemento(alvo).value));
      if (v !== (m[campo] ?? 0)) edicao[campo] = v;
    }
    const nome = elemento("pet-nome").value.trim();
    if (nome && nome !== (m.nome || "")) edicao.nome = nome;
    const habilidades = (estado.habilidadesDoMascote || []).map((h) => [h.id, h.nivel]);
    if (JSON.stringify(habilidades) !== JSON.stringify((m.habilidades || []).map((h) => [h.id, h.nivel]))) edicao.habilidades = habilidades;
    if (!Object.keys(edicao).length) { alerta("aviso", "Nada mudou", "Altere algum campo antes de salvar.", { toast: true }); return; }
  }
  const r = await editarPersonagem({ mascote: { slot: m.slot, tid: m.tid, edicao } },
    `${semCores(m.nome || m.modelo)}: ${libertar ? "libertado" : Object.keys(edicao).join(", ")}`);
  if (r === "aplicado" || r === "salvo") fecharModal("modal-mascote");
}
function abrirBuscaDeMascote() {
  elemento("mb-busca").value = "";
  elemento("mb-tipo").value = "";
  elemento("mb-ambiente").value = "";
  elemento("mb-resultados").replaceChildren(criar("p", "nota-vazia", "Digite o nome ou o ID, ou escolha o tipo ou o ambiente."));
  abrirModal("modal-busca-mascote");
}
let buscaDeMascote = 0;
async function buscarMascote() {
  const realm = estado.selecionado;
  const texto = elemento("mb-busca").value.trim();
  const tipo = elemento("mb-tipo").value;
  const ambiente = elemento("mb-ambiente").value;
  const alvo = elemento("mb-resultados");
  if (!realm || (!tipo && !ambiente && texto.length < 2 && !/^\d+$/.test(texto))) {
    alvo.replaceChildren(criar("p", "nota-vazia", "Digite o nome ou o ID, ou escolha o tipo ou o ambiente."));
    return;
  }
  const params = new URLSearchParams();
  if (texto) params.set("busca", texto);
  if (tipo) params.set("tipo", tipo);
  if (ambiente) params.set("ambiente", ambiente);
  const minha = ++buscaDeMascote;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/mascotes?${params}`);
    if (minha !== buscaDeMascote) return;
    alvo.replaceChildren(...(dados.mascotes || []).map((o) => cartaoDeResultado(
      imagemDoIcone("mascote", o.icone, o.nome || "?"), o.nome || `Mascote ${o.mascote}`,
      `${TIPOS_DE_MASCOTE[o.tipo] || "—"} · ${AMBIENTES[o.ambiente] || "—"} · nível ${o.nivel} · ovo ${o.ovo}`,
      () => abrirMascote(o, true))));
    if (!(dados.mascotes || []).length) alvo.append(criar("p", "nota-vazia", "Nenhum mascote encontrado."));
  } catch (erro) { alvo.replaceChildren(criar("p", "nota-vazia", erro.message)); }
}

/* ---------------- Missões (B198; B216) ----------------
   A janela mostra só as ativas, com o nome pintado como o jogo (`^RRGGBB` do tasks.data). "Editar"
   abre o modal: busca nas ativas e concluídas (concluir, cancelar, esquecer) e dar missão nova. Dar
   é livre (sem pré-requisitos); concluir força o sucesso (missão de NPC fica pronta para entregar,
   a direta premia); cancelar apaga sem prêmio de falha; esquecer tira das concluídas. */
const PAGINA_DE_CONCLUIDAS = 200;
let buscaDeMissao = 0;
function rotuloDaMissao(m) { return m.nome || `Missão ${m.id}`; }
function situacaoDaMissao(m) {
  return m.finalizada ? (m.sucesso ? ["Pronta", "etiqueta"] : ["Falhou", "etiqueta etiqueta-erro"]) : ["Em curso", "etiqueta etiqueta-neutra"];
}
function linhaDeMissao(m, porId, comAcoes) {
  const linha = criar("div", m.pai ? "linha-missao filha" : "linha-missao");
  const texto = criar("span", "texto-resultado");
  const abates = (m.abates || []).map(([feitos, pedidos]) => `${feitos}/${pedidos}`).join(" · ");
  texto.append(textoPintado(rotuloDaMissao(m), "nome-resultado"),
    criar("small", "", `ID ${m.id}${m.pai ? ` · etapa de ${semCores(rotuloDaMissao(porId.get(m.pai) || { id: m.pai }))}` : ""}${abates ? ` · abates ${abates}` : ""}`));
  const [rotulo, classe] = situacaoDaMissao(m);
  linha.append(texto, criar("span", classe, rotulo));
  if (comAcoes) {
    const acoes = criar("span", "acoes-linha");
    const concluir = criar("button", "botao botao-secundario botao-pequeno", "Concluir");
    concluir.type = "button"; concluir.disabled = !!m.finalizada;
    concluir.addEventListener("click", () => acaoDeMissao("concluir", m));
    acoes.append(concluir);
    if (!m.pai) {
      const cancelar = criar("button", "botao botao-secundario botao-pequeno texto-perigo", "Cancelar");
      cancelar.type = "button";
      cancelar.addEventListener("click", () => acaoDeMissao("cancelar", m));
      acoes.append(cancelar);
    }
    linha.append(acoes);
  }
  return linha;
}
async function carregarMissoes(realm, id, pagina = estado.paginaDeMissoes || 0) {
  const cartao = elemento("missoes-ativas");
  const ativas = elemento("mm-ativas");
  const concluidas = elemento("missoes-concluidas");
  estado.paginaDeMissoes = pagina;
  cartao.replaceChildren(criar("p", "nota-vazia", "Carregando missões…"));
  const filtro = elemento("mis-filtro").value.trim();
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/personagens/${id}/missoes?pagina=${pagina}`
      + (filtro ? `&busca=${encodeURIComponent(filtro)}` : ""));
    if (estado.personagemId !== id) return;
    const todas = dados.ativas || [];
    const porId = new Map(todas.map((m) => [m.id, m]));
    cartao.replaceChildren(...todas.map((m) => linhaDeMissao(m, porId, false)));
    if (!todas.length) cartao.append(criar("p", "nota-vazia", "Nenhuma missão ativa."));
    // No modal: as ativas pelo mesmo filtro (nome ou ID), as concluídas pelo servidor.
    const termo = semCores(filtro).toLowerCase();
    const filtradas = termo ? todas.filter((m) => String(m.id) === termo || semCores(rotuloDaMissao(m)).toLowerCase().includes(termo)) : todas;
    ativas.replaceChildren(...filtradas.map((m) => linhaDeMissao(m, porId, true)));
    if (!filtradas.length) ativas.append(criar("p", "nota-vazia", termo ? "Nenhuma ativa com esse filtro." : "Nenhuma missão ativa."));
    elemento("mis-total-ativas").textContent = `(${filtradas.length})`;
    concluidas.replaceChildren();
    for (const m of dados.concluidas || []) {
      const linha = criar("div", "linha-missao");
      const texto = criar("span", "texto-resultado");
      texto.append(textoPintado(rotuloDaMissao(m), "nome-resultado"), criar("small", "", `ID ${m.id}${m.vezes ? ` · ${m.vezes}×` : ""}`));
      const esquecer = criar("button", "botao botao-secundario botao-pequeno", "Esquecer");
      esquecer.type = "button";
      esquecer.title = "Tirar das concluídas para poder refazer";
      esquecer.addEventListener("click", () => acaoDeMissao("esquecer", m));
      linha.append(texto, criar("span", m.falhou ? "etiqueta etiqueta-erro" : "etiqueta etiqueta-ouro", m.falhou ? "Falhou" : "Concluída"), esquecer);
      concluidas.append(linha);
    }
    const total = dados.total_concluidas || 0;
    if (!total) concluidas.append(criar("p", "nota-vazia", filtro ? "Nenhuma concluída com esse filtro." : "Nenhuma missão concluída."));
    const paginas = Math.max(1, Math.ceil(total / PAGINA_DE_CONCLUIDAS));
    elemento("mis-total").textContent = `(${total})`;
    elemento("mis-pagina").textContent = `${pagina + 1} de ${paginas}`;
    elemento("mis-anterior").disabled = pagina <= 0;
    elemento("mis-proxima").disabled = pagina + 1 >= paginas;
    elemento("paginacao-missoes").hidden = paginas <= 1;
  } catch (erro) {
    cartao.replaceChildren(criar("p", "nota-vazia", `Missões indisponíveis: ${erro.message}`));
  }
}
async function acaoDeMissao(acao, m) {
  if (!m) return;
  const nome = semCores(rotuloDaMissao(m));
  if (acao === "cancelar" && !(await confirmar("Cancelar missão", `${nome} sai da lista, com as etapas, sem prêmio.`, "Cancelar missão"))) return;
  if (acao === "esquecer" && !(await confirmar("Esquecer conclusão", `${nome} sai das concluídas e pode ser feita de novo.`, "Esquecer"))) return;
  const corpo = { acao, id: m.id };
  if (acao === "dar" && m.sub) corpo.sub = m.sub;
  const texto = { dar: "dada", concluir: "concluída", cancelar: "cancelada", esquecer: "esquecida" }[acao];
  return editarPersonagem({ missao: corpo }, `${nome}: ${texto}`);
}
function trocarAbaDeMissao(aba) {
  for (const botao of document.querySelectorAll?.("[data-aba-missao]") || []) botao.classList.toggle("ativa", botao.dataset.abaMissao === aba);
  for (const painel of document.querySelectorAll?.("[data-painel-missao]") || []) painel.hidden = painel.dataset.painelMissao !== aba;
}
function abrirMissoes() {
  trocarAbaDeMissao("lista");
  abrirModal("modal-missoes");
}
async function buscarMissao() {
  const realm = estado.selecionado;
  const texto = elemento("mis-busca").value.trim();
  const lista = elemento("mis-resultados");
  estado.missaoEscolhida = null;
  elemento("mis-escolhida").textContent = "";
  elemento("mis-sub").hidden = true;
  if (!realm || (texto.length < 2 && !/^\d+$/.test(texto))) { lista.replaceChildren(); return; }
  const minha = ++buscaDeMissao;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/missoes?busca=${encodeURIComponent(texto)}`);
    if (minha !== buscaDeMissao) return;
    lista.replaceChildren(...(dados.missoes || []).map((m) => {
      const [minimo, maximo] = m.nivel || [0, 0];
      return cartaoDeResultado(icone("pergaminho"), rotuloDaMissao(m),
        `ID ${m.id}${minimo ? ` · nível ${minimo}${maximo ? `–${maximo}` : "+"}` : ""}${m.filhas ? ` · ${m.filhas} etapas` : ""}`, () => {
          estado.missaoEscolhida = m;
          const sub = elemento("mis-sub");
          sub.replaceChildren(...(m.escolha || []).map((s) => { const o = criar("option", "", semCores(rotuloDaMissao(s))); o.value = String(s.id); return o; }));
          sub.hidden = !(m.escolha || []).length;
          elemento("mis-escolhida").textContent = `Escolhida: ${semCores(rotuloDaMissao(m))} (ID ${m.id})${sub.hidden ? "" : " — escolha a etapa"}.`;
        });
    }));
    if (!(dados.missoes || []).length) lista.append(criar("p", "nota-vazia", "Nenhuma missão com esse nome."));
  } catch (erro) {
    lista.replaceChildren(criar("p", "nota-vazia", erro.message));
  }
}
async function darMissao() {
  const texto = elemento("mis-busca").value.trim();
  const m = estado.missaoEscolhida || (/^\d+$/.test(texto) ? { id: Number(texto) } : null);
  if (!m) { alerta("erro", "Escolha a missão", "Busque pelo nome e clique nela, ou digite o ID."); return; }
  const sub = elemento("mis-sub");
  return acaoDeMissao("dar", { ...m, sub: sub.hidden ? undefined : Number(sub.value) });
}

/* Liga a ficha (B216): botões das barras, modais, buscas com espera e o menu do slot vazio. */
function aoDigitar(id, funcao) {
  let espera = null;
  elemento(id).addEventListener("input", () => {
    if (espera) clearTimeout(espera);
    espera = temTempo ? setTimeout(funcao, 300) : null;
  });
}
function aoEnviar(id, funcao) {
  elemento(id).addEventListener("submit", (evento) => { evento.preventDefault(); funcao(); });
}
/* B206: furos (0–4, `MAX_SOCKET_COUNT`) redimensionam a lista de pedras; a busca de pedra põe o id
   no primeiro furo vazio; a busca de efeito acrescenta a linha com o id. */
function lerPedras() {
  return elemento("ei-pedras").value.split(/[,\s]+/).filter(Boolean).map((v) => Math.trunc(Number(v)) || 0);
}
function listaDeBusca(alvo, itens, rotulo, detalhe, escolher) {
  alvo.replaceChildren();
  for (const x of itens) {
    const li = criar("li");
    li.append(criar("span", "", rotulo(x)), criar("small", "", detalhe(x)));
    li.addEventListener("click", () => { escolher(x); alvo.hidden = true; });
    alvo.append(li);
  }
  if (!itens.length) alvo.append(criar("li", "dica", "Nada encontrado."));
  alvo.hidden = false;
}
async function buscarPedra() {
  const texto = elemento("ei-busca-pedra").value.trim();
  const realm = estado.selecionado;
  if (!realm || !texto) { elemento("ei-resultados-pedra").hidden = true; return; }
  try {
    const d = await api(`/api/realms/${encodeURIComponent(realm.id)}/pedras?busca=${encodeURIComponent(texto)}`);
    listaDeBusca(elemento("ei-resultados-pedra"), d.pedras || [], (p) => semCores(p.nome) || `Pedra ${p.id}`,
      (p) => `ID ${p.id}${p.grau ? ` · grau ${p.grau}` : ""}`, (p) => {
        const pedras = lerPedras();
        const vazio = pedras.indexOf(0);
        if (vazio < 0) { alerta("aviso", "Sem furo vazio", "Aumente os furos ou troque um id na lista.", { toast: true }); return; }
        pedras[vazio] = p.id;
        elemento("ei-pedras").value = pedras.join(", ");
      });
  } catch (erro) { alerta("erro", "Busca de pedras", erro.message); }
}
async function buscarEfeito() {
  const texto = elemento("ei-busca-efeito").value.trim();
  if (!texto) { elemento("ei-resultados-efeito").hidden = true; return; }
  try {
    const em = estado.itemEmEdicao;
    if (!em || !estado.selecionado) return;
    // B209: só o que serve nesta peça e neste realm, com o valor do id ("Acerto +118").
    const d = await api(`/api/realms/${encodeURIComponent(estado.selecionado.id)}/efeitos?item=${em.item.id}&busca=${encodeURIComponent(texto)}`);
    listaDeBusca(elemento("ei-resultados-efeito"), d.efeitos || [], (e) => e.habilidade ? `✦ ${e.texto || `Efeito ${e.id}`} — habilidade ${e.habilidade.nome}` : (e.texto || `Efeito ${e.id}`),
      (e) => `ID ${e.id}${e.editavel ? "" : " · fixo"}${e.age === false ? " · não age" : ""}`,
      (e) => elemento("ei-efeitos").append(linhaDeEfeito(e)));
  } catch (erro) { alerta("erro", "Busca de efeitos", erro.message); }
}
function ligarFichaDoPersonagem() {
  aoEnviar("form-dinheiro", editarDinheiro);
  aoEnviar("form-exp", editarExp);
  aoEnviar("form-pontos", editarPontos);
  aoEnviar("form-nivel", editarNivel);
  aoEnviar("form-cultivo", editarCultivo);
  aoEnviar("form-atributos", editarAtributos);
  aoEnviar("form-posicao", editarPosicao);
  elemento("redistribuir-atributos").addEventListener("click", () => redistribuirAtributos());
  for (const id of CAMPOS_DE_ATRIBUTO) elemento(id).addEventListener("input", () => atualizarSobraDeAtributos());
  elemento("abrir-estatisticas").addEventListener("click", () => abrirModal("modal-estatisticas"));

  aoEnviar("form-editar-item", salvarEdicaoDeItem);
  elemento("ei-remover").addEventListener("click", () => removerItem());
  elemento("ei-novo-efeito").addEventListener("click", () => elemento("ei-efeitos").append(linhaDeEfeito()));
  elemento("ei-furos").addEventListener("input", () => {
    const n = Math.max(0, Math.min(4, Math.trunc(Number(elemento("ei-furos").value)) || 0));
    const pedras = lerPedras().slice(0, n);
    while (pedras.length < n) pedras.push(0);
    elemento("ei-pedras").value = pedras.join(", ");
  });
  aoDigitar("ei-busca-pedra", buscarPedra);
  aoDigitar("ei-busca-efeito", buscarEfeito);

  elemento("menu-criar-item").addEventListener("click", () => { const d = estado.slotDoMenu; esconderMenuDoSlot(); if (d) abrirCriarItem(d); });
  elemento("abrir-dar-item").addEventListener("click", () => abrirCriarItem({ chave: "bolsa" }));
  aoEnviar("form-criar-item", criarItem);
  aoDigitar("ci-busca", buscarParaCriar);
  elemento("ci-categoria").addEventListener("change", () => buscarParaCriar());

  elemento("abrir-dar-habilidade").addEventListener("click", () => abrirBuscaDeHabilidade());
  aoDigitar("hb-busca", buscarHabilidade);
  elemento("hb-classe").addEventListener("change", () => buscarHabilidade());
  aoEnviar("form-habilidade", () => aplicarHabilidade(elemento("hab-nivel").value));
  elemento("hab-remover").addEventListener("click", () => aplicarHabilidade(0));

  elemento("abrir-dar-mascote").addEventListener("click", () => abrirBuscaDeMascote());
  aoDigitar("mb-busca", buscarMascote);
  elemento("mb-tipo").addEventListener("change", () => buscarMascote());
  elemento("mb-ambiente").addEventListener("change", () => buscarMascote());
  aoEnviar("form-mascote", () => salvarMascote(false));
  elemento("pet-libertar").addEventListener("click", () => salvarMascote(true));
  aoDigitar("pet-hab-busca", buscarHabilidadeDoMascote);
  elemento("pet-hab-classe").addEventListener("change", () => buscarHabilidadeDoMascote());

  elemento("abrir-daimon-exp").addEventListener("click", () => { elemento("daimon-exp").value = ""; abrirModal("modal-daimon"); });
  aoEnviar("form-daimon", darExpAoDaimon);

  elemento("abrir-missoes").addEventListener("click", () => abrirMissoes());
  for (const botao of document.querySelectorAll?.("[data-aba-missao]") || []) botao.addEventListener("click", () => trocarAbaDeMissao(botao.dataset.abaMissao));
  aoEnviar("form-filtro-missoes", () => { if (estado.selecionado && estado.personagemId) carregarMissoes(estado.selecionado, estado.personagemId, 0); });
  elemento("mis-anterior").addEventListener("click", () => carregarMissoes(estado.selecionado, estado.personagemId, (estado.paginaDeMissoes || 0) - 1));
  elemento("mis-proxima").addEventListener("click", () => carregarMissoes(estado.selecionado, estado.personagemId, (estado.paginaDeMissoes || 0) + 1));
  aoDigitar("mis-busca", buscarMissao);
  aoEnviar("form-dar-missao", darMissao);

  for (const id of MODAIS_DA_FICHA) {
    const modal = elemento(id);
    modal.addEventListener("mousedown", (evento) => { if (evento.target === modal) fecharModal(id); });
    for (const botao of modal.querySelectorAll?.(".fechar-modal") || []) botao.addEventListener("click", () => fecharModal(id));
  }
  document.addEventListener?.("mousedown", (evento) => {
    const menu = elemento("menu-slot");
    if (!menu.hidden && !menu.contains(evento.target)) esconderMenuDoSlot();
  });
  document.addEventListener?.("scroll", () => esconderMenuDoSlot(), true);
  preencherIcones();
}

/* ---------------- Templates (E8, B217) ----------------
   O que o servidor copia para o personagem novo de cada classe (o molde do `clsconfig` do original,
   aqui o template do realm no banco). Cartões por classe e a tela da classe com as janelas da
   ficha: estatísticas iniciais, corpo, roupas, bolsa e habilidades. Mascotes, Daimon e missões
   não entram (vêm pelo jogo). Toda edição vale para os personagens criados depois. */
const modoTemplate = () => estado.pagina === "templates" && estado.templateAberto !== null && estado.templateAberto !== undefined;
const templateAberto = () => (estado.templates?.templates || []).find((t) => t.classe === estado.templateAberto) || null;
const habilidadesAtuais = () => (modoTemplate() ? templateAberto()?.habilidades : estado.habilidadesDaFicha) || [];
const MOTIVOS_TEMPLATE = {
  ...MOTIVOS_EDICAO,
  nivel_invalido: "Nível fora de 1 até o máximo do realm.",
  atributos_invalidos: "Algum atributo ficou abaixo do piso da versão.",
  recipiente_errado: "O template só tem bolsa e corpo, e item de missão não entra.",
  mapa_indisponivel: "Este mapa não está carregado no servidor do realm.",
  posicao_invalida: "Esta peça não vai nesse slot.",
  requisito: "A classe, o nível ou os atributos do template não atendem o requisito da peça.",
  requisito_dos_itens: "Com esses valores, uma peça vestida deixaria de atender o requisito; tire-a antes.",
  reputacao_dos_itens: "Uma peça vestida exige reputação, que o personagem novo não tem.",
  reputacao: "A peça exige reputação, que o personagem novo não tem.",
  slot_vazio: "O slot já está vazio; atualize.",
  habilidade_ausente: "O template não tem essa habilidade.",
  classe_invalida: "Esta versão não cria essa classe.",
  template_inexistente: "O realm não tem template para essa classe.",
};

async function carregarTemplates() {
  const realm = estado.selecionado;
  if (!realm) return;
  const aviso = elemento("aviso-templates");
  aviso.hidden = true;
  elemento("lista-templates").replaceChildren(criar("p", "nota-vazia", "Carregando templates…"));
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/templates`);
    if (estado.selecionado?.id !== realm.id) return;
    estado.templates = dados;
    elemento("lista-templates").replaceChildren(...(dados.templates || []).map(cartaoTemplate));
    if (modoTemplate()) desenharTemplate();
  } catch (erro) {
    elemento("lista-templates").replaceChildren();
    aviso.textContent = `Templates indisponíveis: ${erro.message}`;
    aviso.hidden = false;
  }
}

function cartaoTemplate(t) {
  const botao = criar("button", "cartao-conta cartao-personagem cartao-template");
  botao.type = "button";
  const nome = t.classe_nome || `Classe ${t.classe}`;
  const avatar = criar("span", `avatar ${corAvatar(t.classe)}`, nome.slice(0, 1));
  const texto = criar("div");
  if (!t.existe) {
    texto.append(criar("div", "nome", nome), criar("div", "id", "Sem template neste realm"));
    botao.disabled = true;
    botao.append(avatar, texto);
    return botao;
  }
  const e = t.estatisticas;
  const itens = (t.recipientes?.bolsa?.length || 0) + (t.recipientes?.equipamento?.length || 0);
  if (estado.selecionado && !estado.catalogoMapas[estado.selecionado.id]) carregarCatalogoDeMapas(estado.selecionado);
  texto.append(criar("div", "nome", nome), criar("div", "id", `${nomeDoMapa(estado.selecionado, e.mapa)} · ${itens} itens · ${t.habilidades.length} habilidades`));
  const rodape = criar("div", "rodape");
  rodape.append(criar("span", "etiqueta etiqueta-neutra", `${e.forca}/${e.agilidade}/${e.vitalidade}/${e.energia}`),
    criar("span", "gold", `Nv. ${e.nivel}`));
  botao.append(avatar, texto, rodape);
  botao.addEventListener("click", () => abrirTemplate(t.classe));
  return botao;
}

function abrirTemplate(classe) {
  estado.templateAberto = classe;
  estado.sexoDaFicha = "m";
  elemento("consulta-templates").hidden = true;
  elemento("ficha-template").hidden = false;
  desenharTemplate();
}
function fecharTemplate() {
  estado.templateAberto = null;
  esconderMenuDoSlot();
  elemento("ficha-template").hidden = true;
  elemento("consulta-templates").hidden = false;
}

function desenharTemplate() {
  const t = templateAberto();
  if (!t) { fecharTemplate(); return; }
  const e = t.estatisticas;
  elemento("titulo-template").textContent = t.classe_nome || `Classe ${t.classe}`;
  elemento("resumo-template").textContent = `${estado.selecionado?.nome || ""} · versão ${estado.templates.versao} · nível máximo ${estado.templates.nivel_maximo}`;
  const campos = [["nivel", "Nível", e.nivel], ["cultivo", "Cultivo", e.cultivo], ["exp", "EXP", fmt(e.exp)],
    ["brilho", "Alma (SP)", fmt(e.sp)], ["moeda", "Dinheiro", fmt(e.dinheiro)], ["pontos", "Pontos livres", fmt(e.pontos)],
    ["vida", "Vida (calculada)", fmt(t.vida)], ["mana", "Mana (calculada)", fmt(t.mana)],
    ["forca", "Força", e.forca], ["agilidade", "Agilidade", e.agilidade], ["vitalidade", "Vitalidade", e.vitalidade],
    ["energia", "Energia", e.energia], ["mapa", "Mapa", e.mapa],
    ["posicao", "Posição", `${Number(e.x).toFixed(0)}, ${Number(e.y).toFixed(0)}, ${Number(e.z).toFixed(0)}`]];
  elemento("dados-template").replaceChildren(...campos.map(([ico, rotulo, valor]) => {
    const caixa = criar("div", `atributo atributo-${ico}`);
    const texto = criar("div");
    texto.append(criar("span", "", rotulo), criar("strong", "", String(valor)));
    caixa.append(icone(ico), texto);
    return caixa;
  }));
  const porSlot = (chave) => new Map((t.recipientes?.[chave] || []).map((i) => [i.slot, i]));
  const corpo = porSlot("equipamento");
  const maior = Math.max(30, ...[...corpo.keys()].map((s) => s + 1));
  const grade = (chave, slots, mapa) => {
    const g = criar("div", `grade-itens ${chave === "equipamento" ? "grade-corpo" : ""}`);
    for (const s of slots) g.append(celulaDoTemplate(chave, s, mapa.get(s), chave === "equipamento" ? (SLOTS_DE_EQUIPAMENTO[s] || `Slot ${s}`) : ""));
    return g;
  };
  elemento("tpl-equipamento").replaceChildren(grade("equipamento", intervalo(maior).filter((s) => !SLOTS_DE_ROUPA.includes(s)), corpo));
  elemento("tpl-roupas").replaceChildren(grade("equipamento", SLOTS_DE_ROUPA, corpo));
  const bolsa = porSlot("bolsa");
  const resumo = criar("p", "resumo-grade");
  resumo.append(criar("span", "", `${bolsa.size} de ${TAMANHOS.bolsa} ocupados`), criar("span", "texto-suave", "slot vazio: pôr item · arraste para mover"));
  elemento("tpl-bolsa").replaceChildren(resumo, grade("bolsa", intervalo(TAMANHOS.bolsa), bolsa));
  const habilidades = criar("div", "grade-itens grade-habilidades");
  for (const h of t.habilidades) {
    const celula = criar("button", "celula-item ocupada");
    celula.type = "button";
    celula.title = `${h.nome} · nível ${h.nivel}${h.nivel_maximo ? `/${h.nivel_maximo}` : ""} (ID ${h.id})`;
    celula.append(imagemDoIcone("habilidade", h.icone, h.nome), criar("span", "qtd-item", String(h.nivel)));
    celula.addEventListener("click", () => abrirHabilidade(h, true));
    habilidades.append(celula);
  }
  const resumoH = criar("p", "resumo-grade");
  resumoH.append(criar("span", "", `${t.habilidades.length} habilidade${t.habilidades.length === 1 ? "" : "s"}`), criar("span", "texto-suave", "clique para editar"));
  elemento("tpl-habilidades").replaceChildren(resumoH, habilidades);
  if (!t.habilidades.length) elemento("tpl-habilidades").append(criar("p", "nota-vazia", "O personagem novo nasce sem habilidades."));
}

/* Célula de item do template: dica simples (nome, quantidade, id); clique abre o modal da
   quantidade; slot vazio, o menu "Criar item"; arrastar move/troca dentro do template. */
let arrastoDoTemplate = null;
function celulaDoTemplate(chave, slot, item, legenda) {
  const celula = criar("div", `celula-item ${item ? "ocupada" : "vazia"}`);
  if (legenda) celula.title = legenda;
  if (item) {
    const nome = item.nome || `Item ${item.id}`;
    celula.title = `${legenda ? `${legenda}: ` : ""}${semCores(nome)}${item.quantidade > 1 ? ` ×${item.quantidade}` : ""} (ID ${item.id})`;
    celula.append(imagemDoIcone("m", item.icone, nome));
    if (item.quantidade > 1) celula.append(criar("span", "qtd-item", fmt(item.quantidade)));
    celula.addEventListener("click", () => abrirItemDoTemplate(chave, item, legenda));
    celula.draggable = true;
    celula.addEventListener("dragstart", (evento) => {
      arrastoDoTemplate = { chave, slot, nome: semCores(nome) };
      evento.dataTransfer?.setData?.("text/plain", semCores(nome));
      celula.classList.add("arrastando");
    });
    celula.addEventListener("dragend", () => { celula.classList.remove("arrastando"); arrastoDoTemplate = null; });
  } else {
    if (legenda) celula.append(criar("span", "rotulo-slot", legenda));
    celula.addEventListener("click", (evento) => { evento.stopPropagation?.(); mostrarMenuDoSlot(celula, { template: true, chave, slot, legenda }); });
  }
  celula.addEventListener("dragover", (evento) => { if (arrastoDoTemplate) { evento.preventDefault(); celula.classList.add("alvo-arrasto"); } });
  celula.addEventListener("dragleave", () => celula.classList.remove("alvo-arrasto"));
  celula.addEventListener("drop", (evento) => {
    evento.preventDefault();
    celula.classList.remove("alvo-arrasto");
    const origem = arrastoDoTemplate;
    arrastoDoTemplate = null;
    if (!origem || (origem.chave === chave && origem.slot === slot)) return;
    const nomes = Object.fromEntries(RECIPIENTES);
    editarTemplate({ mover_item: { de: origem.chave, slot_de: origem.slot, para: chave, slot_para: slot } },
      `${origem.nome}: ${nomes[origem.chave]} ${origem.slot} → ${nomes[chave]} ${slot}`);
  });
  return celula;
}

function abrirItemDoTemplate(chave, item, legenda) {
  estado.itemDoTemplate = { chave, ...item };
  const nome = item.nome || `Item ${item.id}`;
  elemento("tpl-item-icone").replaceChildren(imagemDoIcone("m", item.icone, nome));
  elemento("tpl-item-nome").replaceChildren(textoPintado(nome));
  const onde = chave === "equipamento" ? `corpo, ${legenda || `slot ${item.slot}`}` : `bolsa, slot ${item.slot}`;
  elemento("tpl-item-detalhe").textContent = `ID ${item.id} · ${onde} · pilha até ${fmt(item.pilha || 1)}`;
  const quantidade = elemento("tpl-item-quantidade");
  quantidade.max = String(Math.max(1, item.pilha || 1));
  quantidade.value = String(item.quantidade);
  abrirModal("modal-template-item");
}
async function salvarItemDoTemplate() {
  const i = estado.itemDoTemplate;
  if (!i) return;
  const quantidade = Math.trunc(Number(elemento("tpl-item-quantidade").value));
  const maximo = Math.max(1, i.pilha || 1);
  if (!Number.isFinite(quantidade) || quantidade < 1 || quantidade > maximo) { alerta("erro", "Quantidade inválida", `De 1 a ${fmt(maximo)}.`); return; }
  const r = await editarTemplate({ item: { recipiente: i.chave, slot: i.slot, id: i.id, quantidade } },
    `${semCores(i.nome) || `Item ${i.id}`}: quantidade ${fmt(quantidade)}`);
  if (r) fecharModal("modal-template-item");
}
async function removerItemDoTemplate() {
  const i = estado.itemDoTemplate;
  if (!i) return;
  const nome = semCores(i.nome) || `Item ${i.id}`;
  if (!(await confirmar("Tirar do template?", `${nome} deixa de vir com o personagem novo.`, "Tirar"))) return;
  const r = await editarTemplate({ remover_item: { recipiente: i.chave, slot: i.slot } }, `${nome}: tirado do template`);
  if (r) fecharModal("modal-template-item");
}

function abrirEstatisticasDoTemplate() {
  const t = templateAberto();
  if (!t) return;
  const e = t.estatisticas;
  elemento("tpl-est-resumo").textContent = `${t.classe_nome} · vale para os personagens criados depois`;
  for (const campo of ["nivel", "exp", "sp", "dinheiro", "pontos", "forca", "agilidade", "vitalidade", "energia"]) {
    elemento(`tpl-${campo}`).value = String(e[campo]);
  }
  elemento("tpl-nivel").max = String(estado.templates.nivel_maximo || 1);
  const piso = estado.templates.piso || {};
  elemento("tpl-piso").textContent = `mínimo ${piso.forca}/${piso.agilidade}/${piso.vitalidade}/${piso.energia}`;
  for (const [campo, minimo] of Object.entries(piso)) elemento(`tpl-${campo}`).min = String(minimo);
  preencherCultivos(estado.templates.versao, e.cultivo, "tpl-cultivo");
  // Mapa: os carregados agora no realm (o GS confere), com os nomes do catálogo.
  const realm = estado.selecionado;
  if (realm && !estado.catalogoMapas[realm.id]) carregarCatalogoDeMapas(realm);
  const nomes = new Map((estado.catalogoMapas[realm?.id] || []).map((m) => [m.mapa, m.nome]));
  const carregados = (estado.online[realm?.id]?.mapas || []).filter((m) => !m.carregando).map((m) => m.mapa).sort((a, b) => a - b);
  if (!carregados.includes(Number(e.mapa))) carregados.unshift(Number(e.mapa));
  elemento("tpl-mapa").replaceChildren(...carregados.map((mapa) => {
    const opcao = criar("option", "", nomes.has(mapa) ? `${mapa} · ${nomes.get(mapa)}` : `Mapa ${mapa}`);
    opcao.value = String(mapa);
    opcao.selected = mapa === Number(e.mapa);
    return opcao;
  }));
  elemento("tpl-x").value = Number(e.x).toFixed(1);
  elemento("tpl-y").value = Number(e.y).toFixed(1);
  elemento("tpl-z").value = Number(e.z).toFixed(1);
  abrirModal("modal-template-estatisticas");
}
async function salvarEstatisticasDoTemplate() {
  const inteiro = (id) => Math.trunc(Number(elemento(id).value));
  const est = {};
  for (const campo of ["nivel", "cultivo", "exp", "sp", "dinheiro", "pontos", "forca", "agilidade", "vitalidade", "energia", "mapa"]) {
    est[campo] = inteiro(`tpl-${campo}`);
  }
  const [x, z] = ["tpl-x", "tpl-z"].map((id) => Number(elemento(id).value));
  const textoY = elemento("tpl-y").value.trim();
  const valido = (v) => Number.isFinite(v) && Math.abs(v) <= 100_000;
  if (!Object.values(est).every((v) => Number.isInteger(v) && v >= 0) || est.nivel < 1 || !valido(x) || !valido(z)
      || (textoY !== "" && !valido(Number(textoY)))) {
    alerta("erro", "Valor inválido", "Confira os números (inteiros, sem negativos) e as coordenadas."); return;
  }
  const corpo = { estatisticas: { ...est, x, z, ...(textoY === "" ? {} : { y: Number(textoY) }) } };
  const r = await editarTemplate(corpo, `Estatísticas iniciais: nível ${est.nivel}, mapa ${est.mapa}`);
  if (r) fecharModal("modal-template-estatisticas");
}

/* Uma edição do template: operação idempotente (mesmo id na reconsulta), toast para todo
   resultado, e a tela recarregada do servidor no sucesso. */
async function editarTemplate(corpo, descricao) {
  const realm = estado.selecionado;
  const classe = estado.templateAberto;
  if (!realm || classe === null || classe === undefined || estado.enviando) return false;
  const operacao = Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0")).join("");
  estado.enviando = true;
  alerta("carregando", "Salvando…", descricao, { toast: true });
  try {
    let dados;
    try {
      dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/templates/${classe}/editar`,
        { method: "POST", signal: AbortSignal.timeout(8000), body: JSON.stringify({ operacao_id: operacao, ...corpo }) });
    } catch (erro) {
      if (erro.resultado?.estado === "falha" || !erro.resultado || erro.resultado.detail) throw erro;
      dados = { estado: "desconhecido" };
    }
    for (let i = 0; dados.estado === "desconhecido" && i < 10 && temTempo; i++) {
      await new Promise((r) => setTimeout(r, 1500));
      try { dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/operacoes/${operacao}`); } catch (_) { /* de novo */ }
    }
    if (dados.estado === "salvo") {
      alerta("ok", "Template salvo", `${descricao}. Vale para os personagens criados depois.`);
      await carregarTemplates();
      return true;
    }
    if (dados.estado === "desconhecido") {
      alerta("aviso", "Sem confirmação", `Operação ${operacao}: o servidor não confirmou. Confira o template antes de repetir.`, { toast: true });
    } else {
      alerta("erro", "Não salvo", MOTIVOS_TEMPLATE[dados.codigo] || dados.mensagem || dados.codigo);
    }
  } catch (erro) {
    const detalhe = erro.resultado?.detail;
    const texto = Array.isArray(detalhe) ? "Valores fora dos limites." : (MOTIVOS_TEMPLATE[erro.resultado?.codigo] || erro.message);
    alerta("erro", "Não salvo", texto);
  } finally { estado.enviando = false; }
  return false;
}

function ligarTemplates() {
  elemento("voltar-templates").addEventListener("click", fecharTemplate);
  elemento("abrir-estatisticas-template").addEventListener("click", abrirEstatisticasDoTemplate);
  aoEnviar("form-template-estatisticas", salvarEstatisticasDoTemplate);
  aoEnviar("form-template-item", salvarItemDoTemplate);
  elemento("tpl-item-remover").addEventListener("click", () => removerItemDoTemplate());
  elemento("abrir-item-template").addEventListener("click", () => {
    const livre = intervalo(TAMANHOS.bolsa).find((s) => !(templateAberto()?.recipientes?.bolsa || []).some((i) => i.slot === s));
    if (livre === undefined) { alerta("erro", "Bolsa cheia", "Tire um item do template antes."); return; }
    abrirCriarItem({ template: true, chave: "bolsa", slot: livre, legenda: "" });
  });
  elemento("abrir-habilidade-template").addEventListener("click", () => abrirBuscaDeHabilidade(String(estado.templateAberto)));
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

/* Tema (B218): claro/escuro pelo sistema; o botão fixa a escolha neste navegador. */
function temaEfetivo() {
  const fixado = document.documentElement.dataset.tema;
  if (fixado) return fixado;
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "claro" : "escuro";
}
try {
  const salvo = localStorage.getItem("pw-painel-tema");
  if (salvo === "claro" || salvo === "escuro") document.documentElement.dataset.tema = salvo;
} catch (_) { /* sem armazenamento: segue o sistema */ }
elemento("tema").addEventListener("click", () => {
  const novo = temaEfetivo() === "escuro" ? "claro" : "escuro";
  document.documentElement.dataset.tema = novo;
  try { localStorage.setItem("pw-painel-tema", novo); } catch (_) { /* só nesta página */ }
});
elemento("cortina").addEventListener("click", fecharMenu);
elemento("busca-personagens").addEventListener("submit", (evento) => { evento.preventDefault(); buscarPersonagens(1); });
elemento("personagens-anterior").addEventListener("click", () => buscarPersonagens(estado.paginaPersonagens - 1));
elemento("personagens-seguinte").addEventListener("click", () => buscarPersonagens(estado.paginaPersonagens + 1));
elemento("voltar-personagens").addEventListener("click", () => {
  fecharTelaPersonagem();
  if (!elemento("lista-personagens").children?.length) buscarPersonagens();
});
/* Janelas da ficha e do template (B217): recolher leva a janela à bandeja no canto da tela, e as
   outras ocupam o espaço; restaurar devolve-a ao lugar de onde saiu (marcado por um espaço
   reservado invisível). A bandeja fica dentro da tela, então some junto com ela. */
function bandejaDe(tela) {
  let bandeja = [...(tela.children || [])].find((filho) => filho.classList?.contains("bandeja-janelas"));
  if (!bandeja) {
    bandeja = criar("div", "bandeja-janelas");
    bandeja.setAttribute("aria-label", "Janelas recolhidas");
    tela.append(bandeja);
  }
  return bandeja;
}
function alternarJanela(botao) {
  const janela = botao.closest(".janela");
  const recolher = !janela.classList.contains("recolhida");
  const tela = janela.closest(".tela-personagem");
  if (recolher && tela) {
    const lugar = criar("span", "lugar-janela");
    lugar.hidden = true;
    janela.before(lugar);
    janela.lugarDeOrigem = lugar;
    bandejaDe(tela).append(janela);
  } else if (!recolher && janela.lugarDeOrigem) {
    janela.lugarDeOrigem.replaceWith(janela);
    janela.lugarDeOrigem = null;
  }
  janela.classList.toggle("recolhida", recolher);
  botao.textContent = recolher ? "+" : "–";
  botao.setAttribute("aria-label", recolher ? "Restaurar" : "Recolher");
}
document.querySelectorAll(".botao-janela").forEach((botao) => botao.addEventListener("click", () => alternarJanela(botao)));
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
  // Toast: um clique fecha; modal: só o fundo, e só se há OK.
  if (!estado.alertaModal) { if (!elemento("confirmar-alerta").hidden) return; fecharAlerta(); return; }
  if (evento.target === elemento("operacao") && !elemento("fechar-operacao").hidden) fecharAlerta();
});
elemento("abrir-pendente").addEventListener("click", () => { atualizarComando(true); });
elemento("troca-senha").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false); });
elemento("gestao-gm").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "definir_gm"); });
elemento("ajuste-gold").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "ajustar_gold"); });
elemento("gestao-ban").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "definir_ban"); });
elemento("aplicar-desconectar").addEventListener("click", desconectarConta);
elemento("form-rates").addEventListener("submit", (evento) => { evento.preventDefault(); salvarRates(); });
elemento("filtro-mapa").addEventListener("input", () => renderizarMapas());
elemento("filtro-status-mapa").addEventListener("change", () => renderizarMapas());
elemento("marcar-mapas").addEventListener("click", () => marcarMapasVisiveis(true));
elemento("desmarcar-mapas").addEventListener("click", () => marcarMapasVisiveis(false));
elemento("ligar-mapas").addEventListener("click", () => aplicarAosMarcados(true));
elemento("desligar-mapas").addEventListener("click", () => aplicarAosMarcados(false));
ligarFichaDoPersonagem();
ligarTemplates();
elemento("criacao-conta").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "criar_conta"); });
elemento("recuperar-comando").addEventListener("click", () => { pararAcompanhamento(); executarComando(true); });
document.querySelectorAll(".nav").forEach((botao) => botao.addEventListener("click", () => mudarPagina(botao.dataset.pagina)));
document.querySelectorAll(".acao, .voltar").forEach((botao) => botao.addEventListener("click", () => trocarAba(botao.dataset.aba)));
// Clique fora do popup fecha (exceto enquanto o alerta está aberto por cima).
document.addEventListener?.("mousedown", (evento) => {
  const popover = elemento("popover-conta");
  if (popover.hidden || (!elemento("operacao").hidden && estado.alertaModal)) return;
  if (!popover.contains(evento.target) && !estado.cartao?.contains(evento.target)) fecharConta();
});
if (typeof window !== "undefined") window.addEventListener("resize", fecharConta);
document.addEventListener?.("scroll", fecharConta, true);
document.addEventListener?.("keydown", (evento) => {
  if (evento.key !== "Escape") return;
  const aberto = MODAIS_DA_FICHA.filter((id) => !elemento(id).hidden).pop();
  if (!elemento("operacao").hidden && estado.alertaModal && !elemento("fechar-operacao").hidden) fecharAlerta();
  else if (!elemento("menu-slot").hidden) esconderMenuDoSlot();
  else if (aberto) fecharModal(aberto);
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
