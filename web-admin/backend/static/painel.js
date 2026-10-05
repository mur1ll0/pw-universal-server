"use strict";

const elemento = (id) => document.getElementById(id);
const estado = { sessao: null, realms: [], selecionado: null, pagina: "mundos", carregando: false, geracao: 0, busca: 0, ficha: 0, personagemId: null, conta: null, comando: null, enviando: false, buscaContas: 0 };
const nomesEstados = { disponivel: "Disponível", implementado: "Implementado no servidor", nao_validado: "Não validado", indisponivel: "Indisponível" };

async function api(caminho, opcoes = {}) {
  const cabecalhos = { ...opcoes.headers };
  if (opcoes.body) cabecalhos["Content-Type"] = "application/json";
  if (estado.sessao) cabecalhos["X-CSRF-Token"] = estado.sessao.csrf;
  const resposta = await fetch(caminho, { ...opcoes, headers: cabecalhos, credentials: "same-origin" });
  const dados = resposta.status === 204 ? null : await resposta.json();
  if (!resposta.ok) {
    if (resposta.status === 401 && caminho !== "/api/sessao/entrar") mostrarEntrada();
    const erro = new Error(dados?.detail || dados?.codigo || "Não foi possível concluir a operação.");
    erro.resultado = dados;
    throw erro;
  }
  return dados;
}

function mostrarEntrada() {
  estado.buscaContas++;
  estado.conta = null;
  estado.comando = null;
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
  elemento("lista-realms").replaceChildren();
  elemento("lista-capacidades").replaceChildren();
  elemento("nome-usuario").textContent = "";
  elemento("lista-personagens").replaceChildren();
  elemento("ficha-personagem").hidden = true;
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

function selecionar(id) {
  const geracao = ++estado.geracao;
  estado.busca++;
  estado.ficha++;
  estado.personagemId = null;
  elemento("lista-personagens").replaceChildren();
  elemento("ficha-personagem").hidden = true;
  elemento("aviso-personagens").hidden = true;
  estado.selecionado = estado.realms.find((realm) => realm.id === id) || null;
  elemento("realm-selecionado").value = estado.selecionado?.id || "";
  document.querySelectorAll(".cartao-realm").forEach((cartao) => {
    const selecionado = cartao.dataset.realm === estado.selecionado?.id;
    cartao.classList.toggle("selecionado", selecionado);
    cartao.setAttribute("aria-pressed", String(selecionado));
  });
  const realm = estado.selecionado;
  elemento("detalhes-realm").hidden = !realm;
  elemento("ficha-realm").replaceChildren();
  elemento("lista-capacidades").replaceChildren();
  if (!realm) {
    elemento("titulo-recursos").textContent = "Selecione um realm";
    return;
  }
  elemento("nome-realm").textContent = realm.nome;
  const ficha = [
    ["Porta do gateway", realm.porta],
    ["Personagens persistidos", realm.personagens_persistidos.toLocaleString("pt-BR")],
    ["Estado do mundo", "Ainda não consultado"],
    ["Jogadores online", "Ainda não consultados"],
  ];
  for (const [rotulo, valor] of ficha) {
    const item = criar("dl");
    item.append(criar("dt", "", rotulo), criar("dd", "", valor));
    elemento("ficha-realm").append(item);
  }
  elemento("titulo-recursos").textContent = `Recursos · ${realm.nome}`;
  for (const recurso of realm.capacidades) {
    const linha = criar("article", "recurso");
    linha.dataset.recurso = recurso.id;
    const texto = criar("div");
    texto.append(criar("h3", "", recurso.nome), criar("p", "", recurso.detalhe));
    linha.append(texto, criar("span", `etiqueta ${recurso.estado === "disponivel" ? "" : "neutro"}`, nomesEstados[recurso.estado] || recurso.estado));
    elemento("lista-capacidades").append(linha);
  }
  consultarMundo(realm, geracao);
}

async function consultarMundo(realm, geracao) {
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realm.id)}/estado`);
    if (estado.geracao !== geracao || !estado.sessao) return;
    const campos = elemento("ficha-realm").querySelectorAll("dd");
    campos[2].textContent = dados.estado === "consultado" ? `${dados.mapas.length} mapa(s) servido(s)` : dados.estado === "em_transicao" ? "Em transição" : "Desconhecido";
    campos[3].textContent = dados.jogadores_online === null ? "Desconhecidos" : dados.jogadores_online.toLocaleString("pt-BR");
    if (dados.estado === "consultado") {
      const recurso = elemento("lista-capacidades").querySelector('[data-recurso="consulta_viva"]');
      recurso.querySelector("p").textContent = `Consulta viva autenticada. Mapas: ${dados.mapas.map((mapa) => mapa.mapa).join(", ")}.`;
      const etiqueta = recurso.querySelector("span");
      etiqueta.textContent = "Consulta disponível";
      etiqueta.classList.remove("neutro");
    }
  } catch (erro) {
    if (estado.geracao === geracao && estado.sessao) aviso(erro.message);
  }
}

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
    if (!dados.personagens.length) {
      elemento("lista-personagens").append(criar("p", "nota", "Nenhum personagem encontrado neste realm."));
    }
    for (const personagem of dados.personagens) {
      const botao = criar("button", "personagem");
      botao.type = "button";
      botao.append(criar("strong", "", personagem.nome), criar("span", "", `ID ${personagem.id} · Nível ${personagem.nivel} persistido`), criar("span", "", "Consultar →"));
      botao.addEventListener("click", () => consultarPersonagem(realm.id, personagem.id));
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
  elemento("ficha-personagem").hidden = true;
  try {
    const dados = await api(`/api/realms/${encodeURIComponent(realmId)}/personagens/${id}`);
    if (estado.ficha !== consulta || estado.selecionado?.id !== realmId || !estado.sessao) return;
    const ficha = dados.ficha;
    elemento("titulo-personagem").textContent = `${ficha.nome} · ID ${ficha.id}`;
    elemento("origem-personagem").textContent = dados.origem === "viva" ? "Estado vivo no GS" : "Dados persistidos";
    const presencas = { online: "Online no momento da consulta", ausente_nos_daemons: "Ausente nos daemons consultados", em_transicao: "Presença em transição", desconhecida: "Presença desconhecida" };
    elemento("presenca-personagem").textContent = presencas[dados.presenca] || "Presença desconhecida";
    elemento("observacao-personagem").textContent = dados.aviso || "Fotografia do personagem no servidor de mundo. Atualize para uma nova consulta.";
    elemento("dados-personagem").replaceChildren();
    const inteiro = (valor) => BigInt(valor).toLocaleString("pt-BR");
    const campos = [["Classe (ID)", ficha.classe], ["Nível", ficha.nivel], ["Cultivo (ID)", ficha.cultivo],
      ["EXP", inteiro(ficha.exp)], ["Alma", inteiro(ficha.alma)], ["Dinheiro", inteiro(ficha.dinheiro)],
      ["Vida atual", ficha.vida], ["Mana atual", ficha.mana], ["Força", ficha.forca],
      ["Agilidade", ficha.agilidade], ["Vitalidade", ficha.vitalidade], ["Energia", ficha.energia],
      ["Pontos de atributo", ficha.pontos], ["Mapa", ficha.mapa],
      ["Posição do servidor", `${ficha.posicao.x.toFixed(2)}, ${ficha.posicao.y.toFixed(2)}, ${ficha.posicao.z.toFixed(2)}`]];
    for (const [rotulo, valor] of campos) {
      const campo = criar("dl"); campo.append(criar("dt", "", rotulo), criar("dd", "", valor));
      elemento("dados-personagem").append(campo);
    }
    elemento("ficha-personagem").hidden = false;
    estado.personagemId = id;
  } catch (erro) {
    if (estado.ficha !== consulta || !estado.sessao) return;
    elemento("aviso-personagens").textContent = erro.message;
    elemento("aviso-personagens").hidden = false;
  }
}

function renderizarRealms() {
  elemento("lista-realms").replaceChildren();
  elemento("realm-selecionado").replaceChildren();
  elemento("quantidade-realms").textContent = estado.realms.length;
  for (const realm of estado.realms) {
    const opcao = criar("option", "", `${realm.versao} · ${realm.nome}`);
    opcao.value = realm.id;
    elemento("realm-selecionado").append(opcao);
    const cartao = criar("button", "cartao-realm");
    cartao.type = "button";
    cartao.dataset.realm = realm.id;
    const topo = criar("div", "cartao-topo");
    const acessivel = realm.gateway === "acessivel";
    topo.append(criar("span", "versao", realm.versao), criar("span", `etiqueta ${acessivel ? "" : "neutro"}`, acessivel ? "Gateway acessível" : "Gateway inacessível"));
    const rodape = criar("div", "cartao-rodape");
    rodape.append(criar("span", "", `Porta ${realm.porta}`), criar("span", "", "Selecionar →"));
    cartao.append(topo, criar("h3", "", realm.nome), criar("p", "identificador", realm.id), rodape);
    cartao.addEventListener("click", () => selecionar(realm.id));
    elemento("lista-realms").append(cartao);
  }
  const anterior = estado.selecionado?.id;
  const escolhido = estado.realms.find((realm) => realm.id === anterior) || estado.realms.find((realm) => realm.versao === "1.2.6") || estado.realms[0];
  selecionar(escolhido?.id);
  if (!estado.realms.length) aviso("Nenhum realm configurado foi encontrado neste ambiente.");
}

async function atualizar() {
  if (estado.carregando) return;
  estado.carregando = true;
  elemento("atualizar").disabled = true;
  aviso("");
  try {
    if (estado.pagina === "contas") { await buscarContas(); return; }
    if (estado.pagina === "personagens") {
      if (estado.personagemId && estado.selecionado) await consultarPersonagem(estado.selecionado.id, estado.personagemId);
      else await buscarPersonagens();
      return;
    }
    const dados = await api("/api/realms");
    estado.realms = dados.realms;
    renderizarRealms();
    elemento("ultima-atualizacao").textContent = `Consultado às ${new Date().toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}`;
  } catch (erro) {
    aviso(`Não foi possível atualizar. ${erro.message} Os dados anteriores podem estar desatualizados.`);
  } finally {
    estado.carregando = false;
    elemento("atualizar").disabled = false;
  }
}

function mudarPagina(pagina) {
  estado.pagina = pagina;
  elemento("pagina-mundos").hidden = pagina !== "mundos";
  elemento("pagina-capacidades").hidden = pagina !== "capacidades";
  elemento("pagina-personagens").hidden = pagina !== "personagens";
  elemento("pagina-contas").hidden = pagina !== "contas";
  elemento("titulo-pagina").textContent = { mundos: "Seus mundos", capacidades: "Recursos disponíveis", personagens: "Personagens", contas: "Contas globais" }[pagina];
  elemento("subtitulo-pagina").textContent = { mundos: "Escolha um realm para acompanhar sua configuração e seus recursos.", capacidades: "Veja o que está disponível para o realm selecionado.", personagens: "Consulte o personagem no realm selecionado e confira a origem dos dados.", contas: "As alterações de conta têm alcance em todos os realms." }[pagina];
  document.querySelectorAll(".nav").forEach((botao) => {
    const ativo = botao.dataset.pagina === pagina;
    botao.classList.toggle("ativo", ativo);
    if (ativo) botao.setAttribute("aria-current", "page");
    else botao.removeAttribute("aria-current");
  });
}

async function abrirPortal() {
  estado.sessao = await api("/api/sessao");
  estado.comando = lerComando();
  atualizarComando();
  elemento("nome-usuario").textContent = estado.sessao.usuario;
  elemento("entrada").hidden = true;
  elemento("portal").hidden = false;
  mudarPagina("mundos");
  await atualizar();
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

function chaveComando() { return `pw-admin:comando:${estado.sessao.conta_id}`; }
function lerComando() {
  try {
    const comando = JSON.parse(localStorage.getItem(chaveComando()));
    if (comando && /^[A-Za-z0-9_-]{1,64}$/.test(comando.id) && typeof comando.realm === "string" &&
        typeof comando.usuario === "string" && (
          (comando.tipo === "criar_conta" && /^[a-z0-9_]{1,64}$/.test(comando.usuario)) ||
          ((!comando.tipo || comando.tipo === "trocar_senha" || (comando.tipo === "definir_gm" && typeof comando.habilitado === "boolean")) && Number.isInteger(comando.conta_id) && comando.conta_id > 0))) return comando;
  } catch (_) { /* Sem registro recuperável neste navegador. */ }
  return null;
}
function atualizarComando(mostrarAviso = true) {
  const criacao = estado.comando?.tipo === "criar_conta";
  const alvo = criacao ? null : estado.comando || estado.conta;
  elemento("alvo-senha").textContent = criacao ? `Criação pendente: ${estado.comando.usuario}` : alvo ? `${alvo.usuario} · conta ${alvo.conta_id || alvo.id}` : "Selecione uma conta";
  elemento("aplicar-senha").disabled = !alvo || estado.enviando || criacao || estado.comando?.tipo === "definir_gm";
  elemento("aplicar-senha").textContent = estado.comando ? "Repetir a mesma operação" : "Trocar senha global";
  elemento("aplicar-gm").disabled = !alvo || estado.enviando || !!(estado.comando && estado.comando.tipo !== "definir_gm");
  elemento("aplicar-gm").textContent = estado.comando?.tipo === "definir_gm" ? "Repetir a mesma alteração GM" : "Aplicar GM global";
  elemento("gm-habilitado").disabled = estado.enviando || !!estado.comando;
  if (estado.comando?.tipo === "definir_gm") elemento("gm-habilitado").value = String(estado.comando.habilitado);
  elemento("recuperar-comando").hidden = !estado.comando;
  elemento("recuperar-comando").disabled = estado.enviando;
  elemento("aplicar-criacao").disabled = estado.enviando || !!(estado.comando && !criacao);
  elemento("aplicar-criacao").textContent = criacao ? "Repetir a mesma criação" : "Criar conta global";
  elemento("usuario-novo").readOnly = criacao || estado.enviando;
  if (criacao) elemento("usuario-novo").value = estado.comando.usuario;
  if (estado.comando && mostrarAviso) elemento("resultado-comando").textContent = `Operação ${estado.comando.id} · resultado desconhecido. Consulte o resultado antes de repetir. Para repetir, informe a mesma senha; parâmetros diferentes são recusados.`;
}
async function buscarContas() {
  const geracao = ++estado.buscaContas;
  try {
    const dados = await api(`/api/contas?busca=${encodeURIComponent(elemento("nome-conta").value.trim())}`);
    if (geracao !== estado.buscaContas || !estado.sessao) return;
    elemento("lista-contas").replaceChildren();
    for (const conta of dados.contas) {
      const botao = criar("button", "cartao-conta");
      botao.type = "button";
      botao.append(criar("strong", "", `${conta.usuario} · ${conta.id}`), criar("p", "nota", `GM ${conta.gm} · ${conta.banida ? "Banida" : "Sem banimento"} · Gold: ${conta.gold} unidades (100 = 1 gold)`));
      botao.addEventListener("click", () => {
        if (estado.enviando || estado.comando) { aviso("Consulte o resultado da operação pendente antes de trocar o alvo."); return; }
        estado.conta = conta;
        elemento("gm-habilitado").value = String(conta.gm > 0);
        elemento("nova-senha").value = "";
        elemento("resultado-comando").textContent = "";
        atualizarComando();
      });
      elemento("lista-contas").append(botao);
    }
    if (!dados.contas.length) elemento("lista-contas").append(criar("p", "nota", "Nenhuma conta encontrada."));
  } catch (erro) { if (geracao === estado.buscaContas) aviso(erro.message); }
}
function mostrarResultadoComando(dados, comando) {
  if (estado.comando?.id !== comando.id) return;
  if (dados.codigo === "operacao_em_conflito") {
    atualizarComando();
    elemento("resultado-comando").textContent = `Operação ${comando.id}: os parâmetros da repetição são diferentes. A operação original foi preservada; consulte seu resultado.`;
    return;
  }
  if (["administrador_recusado", "realm_incorreto", "alvo_invalido"].includes(dados.codigo)) {
    atualizarComando();
    elemento("resultado-comando").textContent = `Operação ${comando.id}: envio recusado (${dados.codigo}). Identificador preservado; recupere o resultado antes de repetir.`;
    return;
  }
  if (["pendente", "aplicado", "substituido"].includes(dados.estado)) {
    if (comando.tipo !== "definir_gm" || dados.tipo !== "definir_gm" || dados.conta_id !== comando.conta_id || dados.habilitado !== comando.habilitado) {
      aviso("Resultado de outro alvo; mantenha o identificador."); return;
    }
    if (dados.estado === "pendente") {
      atualizarComando(false);
      elemento("resultado-comando").textContent = `Operação ${comando.id}: GM global salvo; sessões pendentes. Processos aguardados: ${(dados.processos_pendentes || dados.processos || []).join(", ")}. Consulte novamente. Reconexão necessária.`;
      return;
    }
    localStorage.removeItem(chaveComando()); estado.comando = null; atualizarComando(false);
    elemento("resultado-comando").textContent = dados.estado === "aplicado"
      ? `Operação ${comando.id}: GM global salvo e sessões reconciliadas. Entre novamente no jogo para carregar o privilégio.`
      : `Operação ${comando.id}: persistida, substituída por alteração posterior. Consulte o GM atual da conta.`;
    return;
  }
  if (dados.estado === "salvo" || dados.estado === "falha") {
    if (dados.estado === "salvo" && comando.tipo === "criar_conta" &&
        (dados.tipo !== "criar_conta" || dados.usuario !== comando.usuario || !Number.isInteger(dados.conta_id) || dados.conta_id <= 0)) {
      aviso("Resultado de outro alvo; mantenha o identificador e consulte novamente.");
      return;
    }
    localStorage.removeItem(chaveComando());
    estado.comando = null;
    atualizarComando();
    elemento("resultado-comando").textContent = dados.estado === "salvo"
      ? (comando.tipo === "criar_conta"
          ? `Operação ${comando.id}: conta global ${dados.usuario} criada · ID ${dados.conta_id}. Use suas credenciais no cliente 1.2.6 ou 1.5.5. Sem privilégio GM e com saldos iniciais zero.`
          : `Operação ${comando.id}: senha global salva. Use a nova senha no próximo login; sessões de jogo abertas permanecem conectadas.`)
      : `Operação ${comando.id}: recusada (${dados.codigo}).`;
  } else atualizarComando();
}
async function executarComando(recuperar, tipo = "trocar_senha") {
  if (estado.enviando || !estado.sessao) return;
  let comando = estado.comando;
  if (!comando && recuperar) return;
  if (!recuperar && comando && (comando.tipo || "trocar_senha") !== tipo) { aviso("Recupere a operação pendente antes de iniciar outra."); return; }
  const gm = (comando?.tipo || tipo) === "definir_gm";
  const criacao = (comando?.tipo || tipo) === "criar_conta";
  if (!comando && (!estado.selecionado || (!criacao && !estado.conta))) { aviso("Selecione um realm com canal ativo e o alvo da operação."); return; }
  const usuario = comando?.usuario || elemento("usuario-novo").value.toLowerCase();
  if (!recuperar && criacao && !/^[a-z0-9_]{1,64}$/.test(usuario)) { aviso("Use de 1 a 64 letras ASCII, números ou _ no usuário."); return; }
  const campoSenha = elemento(criacao ? "senha-criacao" : "nova-senha");
  const senha = campoSenha.value;
  if (!recuperar && !gm && !/^[\x20-\x7e]{1,64}$/.test(senha)) { aviso("Informe de 1 a 64 caracteres ASCII imprimíveis."); return; }
  try {
    if (!comando) {
      const bytes = crypto.getRandomValues(new Uint8Array(16));
      comando = { id: Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(""), realm: estado.selecionado.id,
                  tipo, ...(gm ? { habilitado: elemento("gm-habilitado").value === "true" } : {}), ...(criacao ? { usuario } : { conta_id: estado.conta.id, usuario: estado.conta.usuario }) };
      // Guardar só metadados antes de enviar; nunca persistir a senha no navegador.
      localStorage.setItem(chaveComando(), JSON.stringify(comando));
      estado.comando = comando;
    }
    estado.enviando = true;
    campoSenha.value = "";
    atualizarComando();
    const base = `/api/realms/${encodeURIComponent(estado.selecionado?.id || comando.realm)}`;
    const dados = await api(recuperar ? `${base}/operacoes/${comando.id}` : (criacao ? `${base}/contas` : `${base}/contas/${comando.conta_id}/${gm ? "gm" : "senha"}`),
      recuperar ? { signal: AbortSignal.timeout(5000) } : { method: "POST", signal: AbortSignal.timeout(5000), body: JSON.stringify({ operacao_id: comando.id, ...(gm ? { habilitado: comando.habilitado } : { senha }), ...(criacao ? { usuario: comando.usuario } : {}) }) });
    mostrarResultadoComando(dados, comando);
  } catch (erro) {
    if (erro.resultado?.estado === "falha") mostrarResultadoComando(erro.resultado, comando);
    aviso(erro.message);
  } finally { estado.enviando = false; atualizarComando(false); }
}
elemento("busca-contas").addEventListener("submit", (evento) => { evento.preventDefault(); buscarContas(); });
elemento("troca-senha").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false); });
elemento("gestao-gm").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "definir_gm"); });
elemento("criacao-conta").addEventListener("submit", (evento) => { evento.preventDefault(); executarComando(false, "criar_conta"); });
elemento("recuperar-comando").addEventListener("click", () => executarComando(true));
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
elemento("busca-personagens").addEventListener("submit", (evento) => { evento.preventDefault(); buscarPersonagens(); });
elemento("ver-recursos").addEventListener("click", () => mudarPagina("capacidades"));
document.querySelectorAll(".nav").forEach((botao) => botao.addEventListener("click", () => mudarPagina(botao.dataset.pagina)));
abrirPortal().catch((erro) => {
  mostrarEntrada();
  if (!erro.message.includes("Entre") && !erro.message.includes("Sessão")) {
    elemento("erro-login").textContent = erro.message;
    elemento("erro-login").hidden = false;
  }
});
