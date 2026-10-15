/* Estúdio, fase S4 (B242): mapa interativo do realm — peças do minimapa do cliente e as criaturas do
   npcgen do realm. Conta do cliente (DlgMiniMap.cpp:1045-1061): 2 m por pixel, peças de 256 px
   "LLCC" (linha, coluna), origem no centro: px = (x + colunas·512)/2, py = (linhas·512 − z)/2.
   Usa `api`, `criar`, `elemento`, `estado` do painel.js e `estudio`/`abrirRegistroDoEstudio` do estudio.js. */

const mapa = {
  lista: [], atual: null, escala: 0.25, x: 0, y: 0, pontos: [], modelos: {}, pecas: new Map(),
  arrastando: null, desenhoPendente: false,
  // Edição do npcgen (B243): resumo das áreas no rascunho, área aberta e "posicionar pelo clique".
  editar: false, ng: null, area: null, posicionando: false,
};
const CORES_DO_MAPA = { monstro: "#ff6b6b", npc: "#ffd166", recurso: "#7bd88f", objeto: "#8ab4ff" };

function rotaDoMapa(resto) {
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/mapas${resto}`;
}

function mostrarVistaDoEstudio(vista) {
  const mapas = vista === "mapas";
  elemento("estudio-mapa").hidden = !mapas;
  document.querySelector("#pagina-estudio .estudio-grade").hidden = mapas;
  for (const [id, ativo] of [["estudio-vista-tabelas", !mapas], ["estudio-vista-mapas", mapas]]) {
    elemento(id).className = `botao botao-pequeno ${ativo ? "botao-primario" : "botao-secundario"}`;
    elemento(id).setAttribute("aria-selected", ativo ? "true" : "false");
  }
  if (mapas) carregarMapas();
}

async function carregarMapas() {
  if (!estudio.realm) return;
  try { mapa.lista = await api(rotaDoMapa("")); }
  catch (erro) { elemento("mapa-situacao").textContent = erro.message; return; }
  const sel = elemento("mapa-escolha");
  const anterior = mapa.atual?.caminho;
  sel.replaceChildren();
  for (const m of mapa.lista) {
    const o = criar("option", "", `${m.nome} (${m.caminho}, id ${m.id})${m.npcgen ? "" : " — sem npcgen"}`);
    o.value = m.caminho;
    sel.appendChild(o);
  }
  sel.value = mapa.lista.some((m) => m.caminho === anterior) ? anterior : (mapa.lista[0]?.caminho || "");
  await abrirMapa(sel.value);
}

async function abrirMapa(caminho) {
  const m = mapa.lista.find((x) => x.caminho === caminho);
  if (!m) return;
  mapa.atual = m;
  mapa.pontos = [];
  mapa.modelos = {};
  mapa.pecas.forEach((img) => img.remove());
  mapa.pecas.clear();
  const tela = elemento("mapa-tela");
  const largura = m.colunas * 512, altura = m.linhas * 512;
  mapa.escala = Math.min(tela.clientWidth / largura, tela.clientHeight / altura);
  mapa.x = (tela.clientWidth - largura * mapa.escala) / 2;
  mapa.y = (tela.clientHeight - altura * mapa.escala) / 2;
  elemento("mapa-situacao").textContent = "Carregando criaturas…";
  elemento("mapa-detalhe").textContent = "Clique numa criatura para ver os detalhes.";
  mapa.ng = null;
  fecharArea();
  desenharMapa();
  if (!m.npcgen) { elemento("mapa-situacao").textContent = "O realm não tem npcgen para este mapa."; return; }
  try {
    const r = await api(rotaDoMapa(`/${encodeURIComponent(caminho)}/criaturas`));
    if (mapa.atual !== m) return;
    mapa.pontos = r.pontos;
    mapa.modelos = r.modelos;
    const contagem = {};
    for (const p of r.pontos) contagem[p[0]] = (contagem[p[0]] || 0) + 1;
    elemento("mapa-situacao").textContent = Object.entries(contagem).map(([t, n]) => `${n} ${t}`).join(" · ");
  } catch (erro) { elemento("mapa-situacao").textContent = erro.message; }
  if (mapa.editar) await carregarNpcgen();
  desenharMapa();
}

function paraTela(x, z) {
  const m = mapa.atual;
  const px = (x + m.colunas * 512) / 2, py = (m.linhas * 512 - z) / 2;
  return [mapa.x + px * mapa.escala, mapa.y + py * mapa.escala];
}

function desenharMapa() {
  if (mapa.desenhoPendente) return;
  mapa.desenhoPendente = true;
  requestAnimationFrame(() => { mapa.desenhoPendente = false; desenharMapaAgora(); });
}

function desenharMapaAgora() {
  const m = mapa.atual;
  const tela = elemento("mapa-tela");
  const camada = elemento("mapa-pecas");
  if (!m) return;
  camada.style.transform = `translate(${mapa.x}px, ${mapa.y}px) scale(${mapa.escala})`;
  // Peças visíveis: carrega as que faltam.
  const l0 = Math.max(0, Math.floor(-mapa.y / mapa.escala / 256)), l1 = Math.min(m.linhas * 2 - 1, Math.floor((tela.clientHeight - mapa.y) / mapa.escala / 256));
  const c0 = Math.max(0, Math.floor(-mapa.x / mapa.escala / 256)), c1 = Math.min(m.colunas * 2 - 1, Math.floor((tela.clientWidth - mapa.x) / mapa.escala / 256));
  for (let l = l0; l <= l1; l++) {
    for (let c = c0; c <= c1; c++) {
      const nome = `${String(l).padStart(2, "0")}${String(c).padStart(2, "0")}`;
      if (mapa.pecas.has(nome)) continue;
      const img = criar("img");
      img.alt = "";
      img.style.left = `${c * 256}px`;
      img.style.top = `${l * 256}px`;
      img.src = rotaDoMapa(`/${encodeURIComponent(m.caminho)}/pecas/${nome}.png`);
      img.addEventListener("error", () => { img.style.visibility = "hidden"; });
      camada.appendChild(img);
      mapa.pecas.set(nome, img);
    }
  }
  // Pontos.
  const canvas = elemento("mapa-pontos");
  const dpr = window.devicePixelRatio || 1;
  if (canvas.width !== tela.clientWidth * dpr || canvas.height !== tela.clientHeight * dpr) {
    canvas.width = tela.clientWidth * dpr;
    canvas.height = tela.clientHeight * dpr;
  }
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, tela.clientWidth, tela.clientHeight);
  const tipos = new Set([...document.querySelectorAll(".mapa-tipo:checked")].map((c) => c.value));
  const raio = Math.max(2, Math.min(6, mapa.escala * 6));
  if (mapa.editar && mapa.ng) { desenharAreas(ctx, tipos); return; }
  for (const p of mapa.pontos) {
    if (!tipos.has(p[0])) continue;
    const [sx, sy] = paraTela(p[2], p[3]);
    if (sx < -10 || sy < -10 || sx > tela.clientWidth + 10 || sy > tela.clientHeight + 10) continue;
    ctx.fillStyle = CORES_DO_MAPA[p[0]] || "#fff";
    ctx.beginPath();
    ctx.arc(sx, sy, raio, 0, Math.PI * 2);
    ctx.fill();
  }
}

function detalharPonto(ev) {
  const tela = elemento("mapa-tela");
  const r = tela.getBoundingClientRect();
  const mx = ev.clientX - r.left, my = ev.clientY - r.top;
  const tipos = new Set([...document.querySelectorAll(".mapa-tipo:checked")].map((c) => c.value));
  let melhor = null, distancia = 10;
  for (const p of mapa.pontos) {
    if (!tipos.has(p[0])) continue;
    const [sx, sy] = paraTela(p[2], p[3]);
    const d = Math.hypot(sx - mx, sy - my);
    if (d < distancia) { distancia = d; melhor = p; }
  }
  const area = elemento("mapa-detalhe");
  area.replaceChildren();
  if (!melhor) { area.textContent = "Nada aqui. Aproxime (roda do mouse) para separar as criaturas."; return; }
  const modelo = mapa.modelos[String(melhor[1])] || {};
  const iguais = mapa.pontos.filter((p) => p[1] === melhor[1]).length;
  area.append(criar("strong", "", modelo.nome || `(modelo ${melhor[1]} sem nome)`),
    criar("span", "", ` · ${melhor[0]} · modelo ${melhor[1]} · posição (${melhor[2]}, ${melhor[3]}) · renasce em ${melhor[4]} s · ${iguais} no mapa`));
  if (modelo.tabela) {
    const link = criar("a", "", " · abrir a ficha");
    link.href = "#";
    link.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio(modelo.tabela, modelo.indice); });
    area.appendChild(link);
  }
}

(function ligarMapa() {
  const tela = elemento("mapa-tela");
  elemento("estudio-vista-tabelas").addEventListener("click", () => mostrarVistaDoEstudio("tabelas"));
  elemento("estudio-vista-mapas").addEventListener("click", () => mostrarVistaDoEstudio("mapas"));
  elemento("mapa-escolha").addEventListener("change", (e) => abrirMapa(e.target.value));
  document.querySelectorAll(".mapa-tipo").forEach((c) => c.addEventListener("change", desenharMapa));
  tela.addEventListener("wheel", (e) => {
    e.preventDefault();
    const r = tela.getBoundingClientRect();
    const mx = e.clientX - r.left, my = e.clientY - r.top;
    const fator = e.deltaY < 0 ? 1.25 : 0.8;
    const nova = Math.min(8, Math.max(0.03, mapa.escala * fator));
    mapa.x = mx - (mx - mapa.x) * (nova / mapa.escala);
    mapa.y = my - (my - mapa.y) * (nova / mapa.escala);
    mapa.escala = nova;
    desenharMapa();
  }, { passive: false });
  tela.addEventListener("pointerdown", (e) => { mapa.arrastando = { x: e.clientX, y: e.clientY, ox: mapa.x, oy: mapa.y, moveu: false }; tela.setPointerCapture(e.pointerId); });
  tela.addEventListener("pointermove", (e) => {
    const a = mapa.arrastando;
    if (!a) return;
    if (Math.abs(e.clientX - a.x) + Math.abs(e.clientY - a.y) > 3) a.moveu = true;
    mapa.x = a.ox + e.clientX - a.x;
    mapa.y = a.oy + e.clientY - a.y;
    desenharMapa();
  });
  tela.addEventListener("pointerup", (e) => {
    const a = mapa.arrastando;
    mapa.arrastando = null;
    if (a && !a.moveu) {
      if (mapa.posicionando) posicionarPeloClique(e);
      else if (mapa.editar) escolherArea(e);
      else detalharPonto(e);
    }
  });
  window.addEventListener("resize", desenharMapa);
  elemento("mapa-editar").addEventListener("change", async (e) => {
    mapa.editar = e.target.checked;
    fecharArea();
    if (mapa.editar) await carregarNpcgen();
    desenharMapa();
  });
  elemento("estudio-rascunho").addEventListener("change", async () => {
    if (!mapa.editar || !mapa.atual) return;
    const aberta = mapa.area && { tipo: mapa.area.tipo, indice: mapa.area.indice };
    await carregarNpcgen();
    if (aberta) await abrirArea(aberta.tipo, aberta.indice); else desenharMapa();
  });
})();

/* ---------------- Edição do npcgen (S4 parte 2, B243) ----------------
   As áreas do npcgen do realm (`npcgendata.h:57-102`): área de criatura (AREA) com geradores (NPCGEN) e
   área de recurso (RESAREA) com recursos (RES). A edição vai para o rascunho escolhido no topo do Estúdio
   como item só do servidor (`servidor\<mapa>\npcgen.data`); publicar grava no realm, que precisa
   reiniciar para ler. `vExts`/`fExtX`/`fExtZ` são o tamanho da caixa, metade para cada lado do centro. */

const ROTULOS_DO_NPCGEN = {
  iType: "Forma (0 no chão, 1 caixa)", vPos_x: "Posição X", vPos_y: "Posição Y", vPos_z: "Posição Z",
  vDir_x: "Direção X", vDir_y: "Direção Y", vDir_z: "Direção Z", vExts_x: "Largura (X)", vExts_y: "Altura (Y)",
  vExts_z: "Profundidade (Z)", iNPCType: "Tipo (0 monstro, 1 NPC)", iGroupType: "Tipo de grupo",
  bInitGen: "Nasce ao subir", bAutoRevive: "Renasce", bValidOnce: "Só uma vez", dwGenID: "Id do editor",
  idCtrl: "Controlador", iLifeTime: "Vida da área (s)", iMaxNum: "Máximo vivo",
  dwID: "Modelo", dwNum: "Quantidade", iRefresh: "Renascer (s)", dwDiedTimes: "Mortes até sumir",
  dwAggressive: "Agressivo", fOffsetWater: "Acima da água", fOffsetTrn: "Acima do chão", dwFaction: "Facção",
  dwFacHelper: "Facções que ajuda", dwFacAccept: "Aceita ajuda de", bNeedHelp: "Pede ajuda",
  bDefFaction: "Facção padrão", bDefFacHelper: "Ajuda padrão", bDefFacAccept: "Aceita padrão", iPathID: "Caminho",
  iLoopType: "Laço do caminho", iSpeedFlag: "Corre no caminho", iDeadTime: "Corpo some (s)",
  iRefreshLower: "Renascer mínimo (s)",
  fExtX: "Largura (X)", fExtZ: "Profundidade (Z)", iResType: "Tipo de recurso", idTemplate: "Modelo",
  dwRefreshTime: "Renascer (s)", dwNumber: "Quantidade", fHeiOff: "Altura",
};
const CORES_DAS_AREAS = { monstro: "#ff6b6b", npc: "#ffd166", recurso: "#7bd88f" };

async function carregarNpcgen() {
  const m = mapa.atual;
  if (!m || !m.npcgen) { mapa.ng = null; return; }
  try {
    const ng = await api(rotaDoMapa(`/${encodeURIComponent(m.caminho)}/npcgen${estudio.rascunho ? `?rascunho=${estudio.rascunho}` : ""}`));
    if (mapa.atual !== m) return;
    mapa.ng = ng;
    Object.assign(mapa.modelos, ng.modelos);
    elemento("mapa-situacao").textContent = `${ng.areas.length} áreas de criatura · ${ng.recursos.length} de recurso`
      + (ng.no_rascunho ? ` · com as mudanças do rascunho #${estudio.rascunho}` : " · como está no realm")
      + (estudio.rascunho ? "" : " · escolha um rascunho para editar");
  } catch (erro) { mapa.ng = null; elemento("mapa-situacao").textContent = erro.message; }
}

function linhasDasAreas(tipos) {
  const ng = mapa.ng;
  const saida = [];
  for (const a of ng.areas) if (tipos.has(a[1])) saida.push(["areas", a]);
  if (tipos.has("recurso")) for (const a of ng.recursos) saida.push(["recursos", a]);
  return saida;
}

function desenharAreas(ctx, tipos) {
  const tela = elemento("mapa-tela");
  for (const [tipo, a] of linhasDasAreas(tipos)) {
    const [, classe, x, z, ex, ez] = a;
    const [sx, sy] = paraTela(x, z);
    const w = Math.max(4, ex / 2 * mapa.escala), h = Math.max(4, ez / 2 * mapa.escala);
    if (sx + w < 0 || sy + h < 0 || sx - w > tela.clientWidth || sy - h > tela.clientHeight) continue;
    const escolhida = mapa.area && mapa.area.tipo === tipo && mapa.area.indice === a[0];
    ctx.strokeStyle = escolhida ? "#ffffff" : CORES_DAS_AREAS[classe];
    ctx.lineWidth = escolhida ? 2.5 : 1;
    ctx.globalAlpha = a[9] ? 1 : 0.45; // área que só nasce por controlador fica apagada
    ctx.strokeRect(sx - w, sy - h, w * 2, h * 2);
    ctx.fillStyle = ctx.strokeStyle;
    ctx.beginPath();
    ctx.arc(sx, sy, escolhida ? 4 : 2.5, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}

function escolherArea(ev) {
  if (!mapa.ng) return;
  const r = elemento("mapa-tela").getBoundingClientRect();
  const mx = ev.clientX - r.left, my = ev.clientY - r.top;
  const tipos = new Set([...document.querySelectorAll(".mapa-tipo:checked")].map((c) => c.value));
  // O centro mais perto (até 12 px) ganha; senão a menor caixa que contém o clique.
  let melhor = null, nota = Infinity;
  for (const [tipo, a] of linhasDasAreas(tipos)) {
    const [sx, sy] = paraTela(a[2], a[3]);
    const d = Math.hypot(sx - mx, sy - my);
    const w = a[4] / 2 * mapa.escala, h = a[5] / 2 * mapa.escala;
    const n = d <= 12 ? d : (Math.abs(sx - mx) <= w && Math.abs(sy - my) <= h ? 1000 + w * h : Infinity);
    if (n < nota) { nota = n; melhor = [tipo, a[0]]; }
  }
  if (!melhor) { fecharArea(); elemento("mapa-detalhe").textContent = "Nenhuma área aqui."; desenharMapa(); return; }
  abrirArea(melhor[0], melhor[1]);
}

function fecharArea() {
  mapa.area = null;
  mapa.posicionando = false;
  const ed = elemento("mapa-editor");
  ed.hidden = true;
  ed.replaceChildren();
}

async function abrirArea(tipo, indice) {
  const m = mapa.atual;
  try {
    const dados = await api(rotaDoMapa(`/${encodeURIComponent(m.caminho)}/npcgen/${tipo}/${indice}${estudio.rascunho ? `?rascunho=${estudio.rascunho}` : ""}`));
    mapa.area = { tipo, indice, dados };
    desenharEditorDeArea();
  } catch (erro) { fecharArea(); alerta("erro", "Estúdio", erro.message); }
  desenharMapa();
}

function campoDoNpcgen(nome, tipo, valor, editavel) {
  const rotulo = ROTULOS_DO_NPCGEN[nome] || nome;
  const input = criar("input");
  input.dataset.campo = nome;
  input.dataset.tipo = tipo;
  input.disabled = !editavel;
  let label;
  if (tipo === "bool") {
    input.type = "checkbox";
    input.checked = !!valor;
    label = criar("label", "marca");
    label.append(input, criar("span", "", rotulo));
  } else {
    input.type = "number";
    input.className = "campo";
    input.step = tipo === "f32" ? "any" : "1";
    if (tipo === "u32") input.min = "0";
    input.value = valor ?? 0;
    label = criar("label");
    label.append(criar("span", "", rotulo), input);
  }
  label.title = nome;
  return label;
}

function valoresDosCampos(raiz) {
  const saida = {};
  for (const i of raiz.querySelectorAll(":scope > .mapa-campos input[data-campo]")) {
    const t = i.dataset.tipo;
    saida[i.dataset.campo] = t === "bool" ? i.checked : t === "f32" ? Number(i.value) : Math.trunc(Number(i.value));
  }
  return saida;
}

function grupoDeCampos(lista, valores, editavel) {
  const g = criar("div", "mapa-campos");
  for (const [nome, tipo] of lista) g.appendChild(campoDoNpcgen(nome, tipo, valores[nome], editavel));
  return g;
}

function desenharEditorDeArea() {
  const ed = elemento("mapa-editor");
  ed.replaceChildren();
  ed.hidden = false;
  const { tipo, indice, dados } = mapa.area;
  const recurso = tipo === "recursos";
  const campos = mapa.ng.campos;
  const camposDaArea = recurso ? campos.area_de_recurso : campos.area;
  const camposDoFilho = recurso ? campos.recurso : campos.gerador;
  const chaveDoModelo = recurso ? "idTemplate" : "dwID";
  const editavel = !!estudio.rascunho;
  const nomeDoFilho = recurso ? "Recurso" : "Gerador";

  const titulo = criar("div", "recursos-linha");
  titulo.append(criar("strong", "", `${recurso ? "Área de recurso" : "Área de criatura"} #${indice}`),
    criar("span", "texto-suave", editavel ? ` · rascunho #${estudio.rascunho}` : " · só leitura: escolha ou crie um rascunho no topo do Estúdio"));
  ed.appendChild(titulo);

  const area = criar("fieldset");
  area.appendChild(criar("legend", "", "Área"));
  area.appendChild(grupoDeCampos(camposDaArea, dados.campos, editavel));
  ed.appendChild(area);

  const filhos = criar("div");
  const desenharFilho = (valores, n) => {
    const f = criar("fieldset");
    f.dataset.filho = "1";
    const modelo = mapa.modelos[String(valores[chaveDoModelo])] || {};
    const legenda = criar("legend", "", `${nomeDoFilho} ${n + 1}: ${modelo.nome || `modelo ${valores[chaveDoModelo]}`}`);
    f.appendChild(legenda);
    f.appendChild(grupoDeCampos(camposDoFilho, valores, editavel));
    const acoes = criar("div", "recursos-linha");
    if (modelo.tabela) {
      const link = criar("a", "", "abrir a ficha do modelo");
      link.href = "#";
      link.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio(modelo.tabela, modelo.indice); });
      acoes.appendChild(link);
    }
    if (editavel) {
      const tirar = criar("button", "botao botao-secundario botao-pequeno texto-perigo", `Tirar ${nomeDoFilho.toLowerCase()}`);
      tirar.type = "button";
      tirar.addEventListener("click", () => { if (filhos.children.length > 1) f.remove(); else alerta("aviso", "Estúdio", "A área precisa de ao menos um; para tirá-la, remova a área.", { toast: true }); });
      acoes.appendChild(tirar);
    }
    f.appendChild(acoes);
    filhos.appendChild(f);
  };
  dados.filhos.forEach(desenharFilho);
  ed.appendChild(filhos);

  if (!editavel) return;
  const rota = `/${encodeURIComponent(mapa.atual.caminho)}/npcgen/${tipo}/${indice}?rascunho=${estudio.rascunho}`;
  const botoes = criar("div", "recursos-linha");
  const botao = (texto, classe, acao) => {
    const b = criar("button", `botao botao-pequeno ${classe}`, texto);
    b.type = "button";
    b.addEventListener("click", acao);
    botoes.appendChild(b);
    return b;
  };
  botao(`+ ${nomeDoFilho.toLowerCase()} (cópia do último)`, "botao-secundario", () => {
    const ultimos = [...filhos.querySelectorAll("fieldset[data-filho]")];
    desenharFilho(ultimos.length ? valoresDosCampos(ultimos[ultimos.length - 1]) : {}, ultimos.length);
  });
  const pos = botao("Posicionar pelo clique", "botao-secundario", () => {
    mapa.posicionando = !mapa.posicionando;
    pos.textContent = mapa.posicionando ? "Clique no mapa…" : "Posicionar pelo clique";
  });
  botao("Salvar no rascunho", "botao-primario", () => acaoNaArea(async () => {
    const corpo = { campos: valoresDosCampos(area), filhos: [...filhos.querySelectorAll("fieldset[data-filho]")].map(valoresDosCampos) };
    return api(rotaDoMapa(rota), { method: "PUT", body: JSON.stringify(corpo) });
  }, () => "Área salva no rascunho.", () => indice));
  botao("Clonar área", "botao-secundario", () => acaoNaArea(
    () => api(rotaDoMapa(rota.replace("?", "/clonar?")), { method: "POST" }),
    (r) => `Área clonada: #${r.resultado.indice} (mesma posição; mova-a).`, (r) => r.resultado.indice));
  botao("Remover área", "botao-secundario texto-perigo", () => {
    if (!confirm(`Remover a ${recurso ? "área de recurso" : "área"} #${indice}? As seguintes sobem um número.`)) return;
    acaoNaArea(() => api(rotaDoMapa(rota), { method: "DELETE" }), () => "Área removida no rascunho.", () => null);
  });
  ed.appendChild(botoes);
}

function posicionarPeloClique(ev) {
  const m = mapa.atual;
  const r = elemento("mapa-tela").getBoundingClientRect();
  const px = (ev.clientX - r.left - mapa.x) / mapa.escala, py = (ev.clientY - r.top - mapa.y) / mapa.escala;
  // Inverso de paraTela: x = 2·px − colunas·512, z = linhas·512 − 2·py
  const x = Math.round((2 * px - m.colunas * 512) * 10) / 10, z = Math.round((m.linhas * 512 - 2 * py) * 10) / 10;
  const ed = elemento("mapa-editor");
  const ix = ed.querySelector('input[data-campo="vPos_x"]'), iz = ed.querySelector('input[data-campo="vPos_z"]');
  if (ix && iz) { ix.value = x; iz.value = z; }
  mapa.posicionando = false;
  const b = [...ed.querySelectorAll("button")].find((x) => x.textContent === "Clique no mapa…");
  if (b) b.textContent = "Posicionar pelo clique";
  alerta("ok", "Estúdio", `Posição (${x}, ${z}). Salve para gravar no rascunho.`, { toast: true });
}

async function acaoNaArea(acao, mensagem, reabrir) {
  const tipo = mapa.area.tipo;
  try {
    const r = await acao();
    alerta("ok", "Estúdio", mensagem(r) + (r.igual_ao_publicado ? " Ficou igual ao realm: o item saiu do rascunho." : ""), { toast: true });
    await carregarNpcgen();
    const alvo = reabrir(r);
    if (alvo === null || alvo === undefined) { fecharArea(); desenharMapa(); } else await abrirArea(tipo, alvo);
    if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
  } catch (erro) { alerta("erro", "Estúdio", erro.message); }
}
