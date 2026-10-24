/* Recursos do cliente (P3, B235): versões publicadas, rascunhos, publicar e reverter.
   As rotas são as de web-admin/backend/painel/recursos.py, que repassam ao pw-recursos.
   Usa `api`, `criar`, `elemento`, `alerta`, `confirmar` e `estado` do painel.js. */

const recursos = { versoes: [], versao: "", rascunhos: [], historico: [], selecionados: new Set() };

const rotaRecursos = (resto) => `/api/recursos/${encodeURIComponent(recursos.versao)}/${resto}`;
const operacaoNova = () => (crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`).slice(0, 64);

function bytesLegiveis(n) {
  if (n === null || n === undefined) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1048576).toFixed(1)} MB`;
}

const ACOES = { adicionar: "acrescenta", substituir: "troca", remover: "remove" };

async function carregarRecursos() {
  elemento("recursos-aviso").hidden = true;
  try {
    recursos.versoes = await api("/api/recursos/versoes");
  } catch (erro) {
    elemento("recursos-aviso").textContent = erro.message;
    elemento("recursos-aviso").hidden = false;
    return;
  }
  if (!recursos.versoes.some((v) => v.versao === recursos.versao)) recursos.versao = recursos.versoes[0]?.versao || "";
  desenharVersoes();
  await recarregarVersao();
}

function desenharVersoes() {
  const lista = elemento("recursos-versoes");
  lista.replaceChildren();
  for (const v of recursos.versoes) {
    const b = criar("button", `botao botao-pequeno ${v.versao === recursos.versao ? "botao-primario" : "botao-secundario"}`, v.versao);
    b.type = "button";
    b.setAttribute("role", "tab");
    b.setAttribute("aria-selected", v.versao === recursos.versao ? "true" : "false");
    b.addEventListener("click", () => { recursos.versao = v.versao; recursos.selecionados.clear(); desenharVersoes(); recarregarVersao(); });
    lista.appendChild(b);
  }
  const atual = recursos.versoes.find((v) => v.versao === recursos.versao);
  elemento("recursos-situacao").textContent = atual
    ? `Publicada: versão ${atual.atual} · mínima aceita pelo launcher: ${atual.minima}`
    : "Nenhuma versão de cliente importada.";
}

async function recarregarVersao() {
  if (!recursos.versao) return;
  try {
    const todas = await api(rotaRecursos("alteracoes?limite=100"));
    recursos.rascunhos = todas.filter((a) => a.estado === "rascunho");
    recursos.historico = todas.filter((a) => a.estado !== "rascunho");
    for (const id of [...recursos.selecionados]) if (!recursos.rascunhos.some((r) => r.id === id)) recursos.selecionados.delete(id);
  } catch (erro) {
    alerta("erro", "Recursos do cliente", erro.message);
    return;
  }
  desenharRascunhos();
  desenharHistorico();
}

/* B365: diferenças lado a lado de um item (onde · campo · antes · depois), a partir do `resumo` que os Editores gravam:
   `campos` [{tabela, id, campo, antes, depois}] e `registros` do elements (spec 07 §6b) e `mudancas` dos outros
   editores (`campos` {nome: [antes, depois]} ou lista de nomes, `antes_*`/`depois`, o resto como está). */
function linhasDeDiferenca(resumo) {
  const linhas = [];
  const texto = (v) => (v === null || v === undefined ? "—" : typeof v === "object" ? JSON.stringify(v) : String(v));
  for (const c of resumo?.campos || []) {
    if (c && typeof c === "object" && "campo" in c) linhas.push([`${c.tabela || ""} ${c.id ?? ""}`.trim(), c.campo, texto(c.antes), texto(c.depois)]);
  }
  for (const r of resumo?.registros || []) linhas.push([`${r.tabela || ""} ${r.id ?? ""}`.trim(), r.acao || "registro", r.base != null ? `cópia de ${r.base}` : "—", r.acao === "removido" ? "—" : "(novo)"]);
  for (const m of resumo?.mudancas || []) {
    const onde = [m.tipo, m.id ?? m.indice, m.nome].filter((x) => x !== undefined && x !== null && x !== "").join(" ");
    if (m.campos && !Array.isArray(m.campos) && typeof m.campos === "object") {
      for (const [k, v] of Object.entries(m.campos)) {
        if (Array.isArray(v) && v.length === 2) linhas.push([onde, k, texto(v[0]), texto(v[1])]);
        else linhas.push([onde, k, "—", texto(v)]);
      }
      if (!Object.keys(m.campos).length) linhas.push([onde, m.acao || "", "—", "(sem mudança de valor)"]);
    } else if (Array.isArray(m.campos)) {
      linhas.push([onde, `${m.acao || "editar"}: ${m.campos.join(", ")}`, "—", "—"]);
    } else {
      const antesK = Object.keys(m).find((k) => k.startsWith("antes"));
      const resto = Object.fromEntries(Object.entries(m).filter(([k]) => !["acao", "tipo", "id", "indice", "nome", "depois"].includes(k) && k !== antesK));
      linhas.push([onde, m.acao || "", antesK ? texto(m[antesK]) : "—", "depois" in m ? texto(m.depois) : Object.keys(resto).length ? texto(resto) : "—"]);
    }
  }
  return linhas;
}

function tabelaDeDiferencas(resumo) {
  const linhas = linhasDeDiferenca(resumo);
  if (!linhas.length) return criar("div", "texto-suave", "Arquivo inteiro trocado: sem diferença por campo registrada.");
  const t = criar("table", "recursos-tabela recursos-diferencas");
  const cab = criar("tr");
  for (const h of ["Onde", "Campo / ação", "Antes", "Depois"]) cab.appendChild(criar("th", "", h));
  t.appendChild(cab);
  for (const [onde, campo, antes, depois] of linhas.slice(0, 500)) {
    const tr = criar("tr");
    tr.append(criar("td", "", onde), criar("td", "", campo), criar("td", "recursos-antes", antes), criar("td", "recursos-depois", depois));
    t.appendChild(tr);
  }
  if (linhas.length > 500) t.appendChild(criar("caption", "", `… e mais ${linhas.length - 500} linhas.`));
  return t;
}

function tabelaDeItens(alteracao, editavel) {
  const tabela = criar("table", "recursos-tabela");
  const cabeca = criar("tr");
  for (const t of ["Arquivo", "O que muda", "Antes", "Depois", ""]) cabeca.appendChild(criar("th", "", t));
  tabela.appendChild(cabeca);
  for (const i of alteracao.lista) {
    const linha = criar("tr");
    linha.appendChild(criar("td", "recursos-caminho", i.caminho));
    linha.appendChild(criar("td", "", `${ACOES[i.acao] || i.acao}${i.destino !== "cliente" ? ` (${i.destino})` : ""}`));
    linha.appendChild(criar("td", "", bytesLegiveis(i.resumo?.tamanho_antes)));
    linha.appendChild(criar("td", "", i.acao === "remover" ? "—" : bytesLegiveis(i.resumo?.tamanho_depois ?? i.tamanho)));
    const acoes = criar("td");
    if (editavel) {
      const tirar = criar("button", "botao botao-fantasma botao-pequeno", "Tirar");
      tirar.type = "button";
      tirar.addEventListener("click", () => executarRecurso(
        () => api(rotaRecursos(`rascunhos/${alteracao.id}/arquivo?caminho=${encodeURIComponent(i.caminho)}`), { method: "DELETE" }),
        "Arquivo tirado do rascunho."));
      acoes.appendChild(tirar);
    }
    linha.appendChild(acoes);
    tabela.appendChild(linha);
    // B365: as diferenças lado a lado do item, abrindo sob a linha.
    const n = linhasDeDiferenca(i.resumo).length;
    const linhaDif = criar("tr");
    const celula = criar("td");
    celula.colSpan = 5;
    const det = criar("details");
    det.appendChild(criar("summary", "texto-suave", n ? `Diferenças (${n})` : "Diferenças"));
    det.addEventListener("toggle", () => { if (det.open && det.children.length === 1) det.appendChild(tabelaDeDiferencas(i.resumo)); }, { once: false });
    celula.appendChild(det);
    linhaDif.appendChild(celula);
    tabela.appendChild(linhaDif);
  }
  return tabela;
}

function desenharRascunhos() {
  const area = elemento("recursos-rascunhos");
  area.replaceChildren();
  if (!recursos.rascunhos.length) area.appendChild(criar("p", "texto-suave", "Nenhum rascunho. Crie um para trocar, acrescentar ou remover arquivos do cliente."));
  for (const r of recursos.rascunhos) {
    const bloco = criar("section", "recursos-rascunho");
    const topo = criar("header", "recursos-linha");
    const marca = criar("label", "recursos-marca");
    const caixa = criar("input");
    caixa.type = "checkbox";
    caixa.checked = recursos.selecionados.has(r.id);
    caixa.disabled = !r.itens;
    caixa.addEventListener("change", () => { caixa.checked ? recursos.selecionados.add(r.id) : recursos.selecionados.delete(r.id); atualizarPublicar(); });
    marca.append(caixa, criar("strong", "", ` #${r.id} ${r.descricao || "(sem descrição)"}`));
    topo.append(marca, criar("span", "texto-suave", `${r.autor_nome || "—"} · ${new Date(r.criada_em).toLocaleString("pt-BR")} · ${r.itens} arquivo(s)`));
    const descartar = criar("button", "botao botao-fantasma botao-pequeno", "Descartar");
    descartar.type = "button";
    descartar.addEventListener("click", async () => {
      if (!(await confirmar("Descartar o rascunho?", `#${r.id} ${r.descricao}: os arquivos enviados não serão publicados.`, "Descartar"))) return;
      executarRecurso(() => api(rotaRecursos(`rascunhos/${r.id}/descartar`), { method: "POST" }), "Rascunho descartado.");
    });
    topo.appendChild(descartar);
    bloco.appendChild(topo);
    if (r.lista.length) bloco.appendChild(tabelaDeItens(r, true));
    bloco.appendChild(formularioDeItem(r));
    area.appendChild(bloco);
  }
  atualizarPublicar();
}

function formularioDeItem(r) {
  const form = criar("form", "recursos-linha recursos-item");
  const caminho = criar("input", "campo");
  caminho.placeholder = "Caminho no cliente (ex.: data\\elements.data)";
  caminho.setAttribute("aria-label", "Caminho no cliente");
  caminho.dataset.rascunho = r.id;
  caminho.className = "campo recursos-campo-caminho";
  const arquivo = criar("input");
  arquivo.type = "file";
  arquivo.setAttribute("aria-label", "Arquivo novo");
  arquivo.addEventListener("change", () => {
    if (!caminho.value && arquivo.files[0]) caminho.value = arquivo.files[0].name;
  });
  const destino = criar("select", "campo");
  for (const [valor, rotulo] of [["cliente", "só cliente"], ["ambos", "cliente e servidor"], ["servidor", "só servidor"]]) {
    const o = criar("option", "", rotulo);
    o.value = valor;
    destino.appendChild(o);
  }
  destino.setAttribute("aria-label", "Destino");
  const enviar = criar("button", "botao botao-secundario botao-pequeno", "Enviar arquivo");
  enviar.type = "submit";
  const remover = criar("button", "botao botao-fantasma botao-pequeno", "Marcar remoção");
  remover.type = "button";
  form.append(caminho, arquivo, destino, enviar, remover);
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    if (!caminho.value.trim() || !arquivo.files[0]) { alerta("aviso", "Enviar arquivo", "Informe o caminho no cliente e escolha o arquivo.", { toast: true }); return; }
    executarRecurso(() => enviarArquivo(r.id, caminho.value.trim(), arquivo.files[0], destino.value), "Arquivo no rascunho.");
  });
  remover.addEventListener("click", async () => {
    if (!caminho.value.trim()) { alerta("aviso", "Marcar remoção", "Informe o caminho do arquivo a remover.", { toast: true }); return; }
    executarRecurso(() => api(rotaRecursos(`rascunhos/${r.id}/remover?caminho=${encodeURIComponent(caminho.value.trim())}`), { method: "POST" }),
      "Remoção marcada no rascunho.");
  });
  return form;
}

async function enviarArquivo(rascunho, caminho, arquivo, destino) {
  const url = rotaRecursos(`rascunhos/${rascunho}/arquivo?caminho=${encodeURIComponent(caminho)}&destino=${destino}`);
  const resposta = await fetch(url, {
    method: "PUT", body: arquivo, credentials: "same-origin",
    headers: { "Content-Type": "application/octet-stream", "X-CSRF-Token": estado.sessao.csrf },
  });
  const dados = await resposta.json().catch(() => null);
  if (!resposta.ok) throw new Error(dados?.detail || `Falha no envio (${resposta.status}).`);
  return dados;
}

function atualizarPublicar() {
  const n = recursos.selecionados.size;
  elemento("recursos-publicar").disabled = !n;
  elemento("recursos-publicar").textContent = n ? `Publicar ${n} rascunho${n === 1 ? "" : "s"}` : "Publicar selecionados";
}

async function publicarRecursos() {
  const ids = [...recursos.selecionados].sort((a, b) => a - b);
  const atual = recursos.versoes.find((v) => v.versao === recursos.versao);
  const obrigar = elemento("recursos-obrigar").checked;
  // B378: reinício dos mundos depois de publicar, com aviso aos jogadores (0 = na hora).
  const reiniciar = elemento("recursos-reiniciar").checked;
  const minutos = Number(elemento("recursos-reiniciar-minutos").value);
  if (reiniciar && !(Number.isInteger(minutos) && minutos >= 0 && minutos <= 120)) {
    alerta("aviso", "Publicar", "Tempo até reiniciar: de 0 a 120 minutos (0 = na hora).", { toast: true });
    return;
  }
  const texto = `Publica a versão ${atual.atual + 1} do cliente ${recursos.versao} com ${ids.map((i) => `#${i}`).join(", ")}. ` +
    (obrigar ? "O launcher vai exigir a atualização antes de jogar." : "Quem estiver na versão anterior ainda pode jogar.") +
    " Arquivos de destino \"servidor\"/\"ambos\" (elements, npcgen, precinct, region, world_targets) são gravados no realm, que precisa ser reiniciado para lê-los." +
    (reiniciar ? ` Os mundos dos realms ${recursos.versao} ${minutos ? `reiniciam em ${minutos} min, com aviso aos jogadores` : "reiniciam assim que publicar"}.` : " Sem reinício: reinicie pelo aviso no topo dos Editores.");
  // B365: o total de diferenças registradas nos rascunhos escolhidos (o detalhe lado a lado fica em cada item).
  const escolhidos = recursos.rascunhos.filter((r) => ids.includes(r.id));
  const total = escolhidos.reduce((s, r) => s + r.lista.reduce((t, i) => t + linhasDeDiferenca(i.resumo).length, 0), 0);
  const arquivos = escolhidos.reduce((s, r) => s + r.lista.length, 0);
  if (!(await confirmar("Publicar versão nova?", `${texto} São ${arquivos} arquivo(s) e ${total} diferença(s) registrada(s) — confira em "Diferenças" em cada item.`, "Publicar"))) return;
  executarRecurso(async () => {
    const r = await api(rotaRecursos("publicar"), { method: "POST", body: JSON.stringify({ rascunhos: ids, minima: obrigar ? atual.atual + 1 : null }) });
    recursos.selecionados.clear();
    elemento("recursos-obrigar").checked = false;
    if (reiniciar) await reiniciarRealmsDaVersao(recursos.versao, minutos);
    return r;
  }, reiniciar ? "Versão publicada: o launcher já oferece a atualização." : "Versão publicada: o launcher já oferece a atualização. Se havia arquivo do servidor, reinicie o realm pelo aviso no topo dos Editores.");
}

/* B378: depois de publicar, cada realm da versão reinicia (agora ou em `minutos`, com aviso aos jogadores). */
async function reiniciarRealmsDaVersao(versao, minutos) {
  const { realms = [] } = await api("/api/realms");
  const alvos = realms.filter((r) => r.versao === versao);
  if (!alvos.length) return;
  const linhas = [];
  for (const realm of alvos) {
    try {
      const r = await api(`/api/realms/${encodeURIComponent(realm.id)}/reiniciar`, {
        method: "POST", body: JSON.stringify({ confirmar: realm.id, em_minutos: minutos }),
      });
      linhas.push(r.estado === "agendado"
        ? `${realm.id}: reinicia às ${new Date(r.em * 1000).toLocaleTimeString()} (jogadores avisados; cancelar no topo dos Editores).`
        : `${realm.id}: reiniciando agora (${r.online ?? 0} online, salvos).`);
    } catch (erro) { linhas.push(`${realm.id}: não reiniciou — ${erro.message}`); }
  }
  alerta("ok", "Reinício dos mundos", linhas.join(" "));
}

/* B333: canal de teste do GM — os rascunhos marcados, por cima da versão atual, sem publicar. O painel mostra o
   endereço com `#teste=<token>` para colar no campo de servidor do launcher; sem o `#teste=…`, o launcher volta ao
   oficial desfazendo o que o teste trocou. */
async function canalDeTeste() {
  const ids = [...recursos.selecionados].sort((a, b) => a - b);
  if (!ids.length) { alerta("aviso", "Canal de teste", "Marque os rascunhos do teste.", { toast: true }); return; }
  try {
    const r = await api(rotaRecursos("teste"), { method: "POST", body: JSON.stringify({ rascunhos: ids }) });
    const endereco = `http://<servidor>:8100#teste=${r.token}`;
    alerta("ok", "Canal de teste",
      `Rascunhos ${ids.map((i) => `#${i}`).join(", ")} prontos para o GM. No launcher, no campo do servidor, use ${endereco} `
      + "e clique em Atualizar. Para voltar ao oficial, tire o #teste=… e atualize de novo.");
  } catch (erro) { alerta("erro", "Canal de teste", erro.message); }
}

function desenharHistorico() {
  const area = elemento("recursos-historico");
  area.replaceChildren();
  if (!recursos.historico.length) area.appendChild(criar("p", "texto-suave", "Só a versão 0 (o cliente original importado)."));
  for (const a of recursos.historico) {
    const bloco = criar("details", "recursos-historico-item");
    const resumo = criar("summary");
    const etiqueta = a.estado === "publicada" ? `versão ${a.versao}` : "descartada";
    resumo.append(criar("strong", "", `#${a.id} ${a.descricao || ""}`),
      criar("span", "texto-suave", ` · ${etiqueta} · ${a.autor_nome || "—"} · ${new Date(a.criada_em).toLocaleString("pt-BR")} · ${a.itens} arquivo(s)`));
    bloco.appendChild(resumo);
    if (a.lista.length) bloco.appendChild(tabelaDeItens(a, false));
    if (a.estado === "publicada") {
      const reverter = criar("button", "botao botao-secundario botao-pequeno", "Reverter");
      reverter.type = "button";
      reverter.addEventListener("click", async () => {
        if (!(await confirmar("Reverter a alteração?", `Cria um rascunho que desfaz #${a.id}. Ele só vale depois de publicado.`, "Criar rascunho"))) return;
        executarRecurso(() => api(rotaRecursos(`alteracoes/${a.id}/reverter`), { method: "POST", body: JSON.stringify({ operacao_id: operacaoNova() }) }),
          "Rascunho de reversão criado: confira e publique.");
      });
      bloco.appendChild(reverter);
    }
    area.appendChild(bloco);
  }
}

async function buscarArquivosDeRecurso(evento) {
  evento.preventDefault();
  const busca = elemento("recursos-busca-texto").value.trim();
  const area = elemento("recursos-arquivos");
  area.replaceChildren();
  let lista;
  try { lista = await api(rotaRecursos(`arquivos?busca=${encodeURIComponent(busca)}&limite=100`)); }
  catch (erro) { area.appendChild(criar("p", "texto-erro", erro.message)); return; }
  if (!lista.length) { area.appendChild(criar("p", "texto-suave", "Nenhum arquivo com esse caminho.")); return; }
  const tabela = criar("table", "recursos-tabela");
  for (const a of lista) {
    const linha = criar("tr");
    linha.appendChild(criar("td", "recursos-caminho", a.caminho));
    linha.appendChild(criar("td", "", bytesLegiveis(a.tamanho)));
    const acoes = criar("td");
    const baixar = criar("a", "botao botao-fantasma botao-pequeno", "Baixar");
    baixar.href = rotaRecursos(`arquivo?caminho=${encodeURIComponent(a.caminho)}`);
    const usar = criar("button", "botao botao-fantasma botao-pequeno", "Usar no rascunho");
    usar.type = "button";
    usar.addEventListener("click", () => {
      const campo = document.querySelector(".recursos-campo-caminho");
      if (!campo) { alerta("aviso", "Usar no rascunho", "Crie um rascunho primeiro.", { toast: true }); return; }
      campo.value = a.caminho;
      campo.focus();
    });
    acoes.append(baixar, usar);
    linha.appendChild(acoes);
    tabela.appendChild(linha);
  }
  if (lista.length === 100) area.appendChild(criar("p", "texto-suave", "Mostrando os 100 primeiros: refine a busca."));
  area.appendChild(tabela);
}

async function executarRecurso(acao, sucesso) {
  try {
    await acao();
    alerta("ok", "Recursos do cliente", sucesso, { toast: true });
  } catch (erro) {
    alerta("erro", "Recursos do cliente", erro.message);
  }
  await carregarRecursos();
}

elemento("recursos-novo").addEventListener("submit", (e) => {
  e.preventDefault();
  const descricao = elemento("recursos-novo-descricao").value.trim();
  if (!descricao) { alerta("aviso", "Novo rascunho", "Descreva o que a alteração faz.", { toast: true }); return; }
  executarRecurso(async () => {
    await api(rotaRecursos("rascunhos"), { method: "POST", body: JSON.stringify({ descricao, operacao_id: operacaoNova() }) });
    elemento("recursos-novo-descricao").value = "";
  }, "Rascunho criado.");
});
elemento("recursos-busca").addEventListener("submit", buscarArquivosDeRecurso);
elemento("recursos-publicar").addEventListener("click", publicarRecursos);

/* ---------------- Envio do cliente original (B332) ----------------
   A pasta escolhida vai arquivo a arquivo em blocos de 8 MiB para o pw-recursos (`.envios/<id>/`). O bloco só acrescenta:
   se o deslocamento não é o que já chegou, o serviço responde 409 com o tamanho, e o envio continua dali. O id do envio
   fica guardado no navegador para a mesma pasta (nome + tamanho total): escolher a pasta de novo retoma. */
const BLOCO_DO_ENVIO = 8 << 20;

async function enviarBloco(id, caminho, deslocamento, dados) {
  const consulta = new URLSearchParams({ caminho, deslocamento: String(deslocamento) });
  const r = await fetch(`/api/recursos/envios/${encodeURIComponent(id)}/arquivo?${consulta}`, {
    method: "PUT", body: dados, credentials: "same-origin",
    headers: { "Content-Type": "application/octet-stream", ...(estado.sessao ? { "X-CSRF-Token": estado.sessao.csrf } : {}) },
  });
  const corpo = await r.json().catch(() => ({}));
  if (r.ok || r.status === 409) return corpo.tamanho;
  throw new Error(corpo.detail || `falha no bloco de ${caminho} (${r.status})`);
}

async function enviarClienteOriginal(e) {
  e.preventDefault();
  const arquivos = Array.from(elemento("envio-pasta").files || []);
  const versao = elemento("envio-versao").value.trim();
  const situacao = elemento("envio-situacao");
  if (!arquivos.length || !versao) { alerta("aviso", "Enviar o cliente", "Escolha a pasta e dê o nome da linha.", { toast: true }); return; }
  const total = arquivos.reduce((s, f) => s + f.size, 0);
  const subpasta = arquivos[0].webkitRelativePath.split("/")[0];
  const chave = `envio:${subpasta}:${arquivos.length}:${total}`;
  let id = null;
  try { id = localStorage.getItem(chave); } catch (_) { id = null; }
  let recebidos = {};
  if (id) {
    try { recebidos = Object.fromEntries((await api(`/api/recursos/envios/${encodeURIComponent(id)}`)).arquivos || []); }
    catch (_) { id = null; recebidos = {}; }
  }
  if (!id) {
    id = (await api("/api/recursos/envios", { method: "POST", body: JSON.stringify({ descricao: `${versao}: ${subpasta}` }) })).id;
    try { localStorage.setItem(chave, id); } catch (_) { /* sem armazenamento: o envio não retoma, mas segue */ }
  }
  let enviados = 0;
  const inicio = Date.now();
  for (const f of arquivos) {
    let desl = Math.min(recebidos[f.webkitRelativePath] || 0, f.size);
    enviados += desl;
    if (desl === f.size && f.size > 0) continue;
    do {
      const fim = Math.min(desl + BLOCO_DO_ENVIO, f.size);
      const novo = await enviarBloco(id, f.webkitRelativePath, desl, f.slice(desl, fim));
      enviados += novo - desl;
      desl = novo;
      const mbs = enviados / 1048576 / Math.max(1, (Date.now() - inicio) / 1000);
      situacao.textContent = `Enviando ${f.webkitRelativePath}: ${bytesLegiveis(enviados)} de ${bytesLegiveis(total)} (${mbs.toFixed(1)} MB/s)`;
    } while (desl < f.size);
  }
  situacao.textContent = `Enviado (${bytesLegiveis(total)}). Importando como a versão 0 de ${versao}…`;
  await api(`/api/recursos/envios/${encodeURIComponent(id)}/importar`, {
    method: "POST", body: JSON.stringify({ versao, subpasta, refazer: elemento("envio-refazer").checked }),
  });
  for (;;) {
    await new Promise((ok) => setTimeout(ok, 3000));
    const v = await api(`/api/recursos/envios/${encodeURIComponent(id)}`);
    const imp = v.importacao || {};
    if (imp.estado === "em_andamento") { situacao.textContent = `Importando ${versao}… (pode levar alguns minutos)`; continue; }
    if (imp.estado === "concluida") {
      situacao.textContent = imp.mensagem;
      alerta("ok", "Enviar o cliente", `Linha ${versao} importada.`, { toast: true });
      try { localStorage.removeItem(chave); } catch (_) { /* nada */ }
      await carregarRecursos();
    } else {
      situacao.textContent = `A importação falhou: ${imp.mensagem || "sem detalhe"}. Os arquivos continuam no envio ${id}.`;
    }
    return;
  }
}
elemento("envio-cliente").addEventListener("submit", (e) => enviarClienteOriginal(e).catch((erro) => {
  elemento("envio-situacao").textContent = `${erro.message} — escolha a mesma pasta e envie de novo para continuar.`;
}));
elemento("recursos-teste").addEventListener("click", canalDeTeste);
