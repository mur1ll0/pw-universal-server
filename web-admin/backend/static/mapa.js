/* Estúdio, fase S4 (B242): mapa interativo do realm — peças do minimapa do cliente e as criaturas do
   npcgen do realm. Conta do cliente (DlgMiniMap.cpp:1045-1061): 2 m por pixel, peças de 256 px
   "LLCC" (linha, coluna), origem no centro: px = (x + colunas·512)/2, py = (linhas·512 − z)/2.
   Usa `api`, `criar`, `elemento`, `estado` do painel.js e `estudio`/`abrirRegistroDoEstudio` do estudio.js. */

const mapa = {
  lista: [], atual: null, escala: 0.25, x: 0, y: 0, pontos: [], modelos: {}, pecas: new Map(),
  arrastando: null, desenhoPendente: false,
  // Edição do npcgen (B243): resumo das áreas no rascunho, área aberta e "posicionar pelo clique".
  editar: false, ng: null, area: null, posicionando: false,
  // Distritos (precinct.sev) e portais (region.sev), B244.
  distritos: null, portais: null,
  // Destinos de teleporte (world_targets.sev), B245.
  teleportes: null,
  // Rastreador de NPCs do cliente (task_npc.data), B246.
  rastreador: null,
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
  desenharListaDeControladores();
  mapa.distritos = null;
  mapa.portais = null;
  mapa.teleportes = null;
  mapa.rastreador = null;
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
  await carregarCamadas();
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
  if (mapa.editar && mapa.ng) desenharAreas(ctx, tipos);
  else {
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
  desenharCamadas(ctx);
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
      else if (!escolherDistritoOuPortal(e)) detalharPonto(e);
    }
  });
  window.addEventListener("resize", desenharMapa);
  elemento("mapa-editar").addEventListener("change", async (e) => {
    mapa.editar = e.target.checked;
    fecharArea();
    if (mapa.editar) await carregarNpcgen();
    desenharListaDeControladores();
    desenharMapa();
  });
  for (const id of ["mapa-distritos", "mapa-portais", "mapa-teleportes", "mapa-rastreador"]) {
    elemento(id).addEventListener("change", async () => { fecharArea(); await carregarCamadas(); desenharMapa(); });
  }
  elemento("estudio-rascunho").addEventListener("change", async () => {
    if (!mapa.atual) return;
    if (!mapa.editar) {
      const aberto = mapa.area && mapa.area.camada ? { camada: mapa.area.camada, indice: mapa.area.indice } : null;
      await carregarCamadas();
      if (aberto) abrirDaCamada(aberto.camada, aberto.indice); else desenharMapa();
      return;
    }
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
  // Objeto dinâmico e controlador (B247; comentários de `npcgendata.h:205-282`).
  dwDynObjID: "Objeto (id)", dir_0: "Direção 1", dir_1: "Direção 2", rad: "Raio", idController: "Controlador", scale: "Escala",
  id: "Id (citado pelas áreas)", iControllerID: "Gatilho (id do aipolicy)", bActived: "Ligado ao subir",
  iWaitTime: "Espera após ligar (s)", iStopTime: "Desliga após (s, 0 = nunca)", bActiveTimeInvalid: "Sem horário para ligar",
  bStopTimeInvalid: "Sem horário para desligar", iActiveTimeRange: "Janela de ativação (s)",
  ActiveTime_iYear: "Liga: ano (-1 qualquer)", ActiveTime_iMouth: "Liga: mês 0-11", ActiveTime_iWeek: "Liga: dia da semana (0 dom.)",
  ActiveTime_iDay: "Liga: dia 1-31", ActiveTime_iHours: "Liga: hora", ActiveTime_iMinutes: "Liga: minuto",
  StopTime_iYear: "Desliga: ano (-1 qualquer)", StopTime_iMouth: "Desliga: mês 0-11", StopTime_iWeek: "Desliga: dia da semana",
  StopTime_iDay: "Desliga: dia 1-31", StopTime_iHours: "Desliga: hora", StopTime_iMinutes: "Desliga: minuto",
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
    desenharListaDeControladores();
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
  if (tipos.has("objeto")) for (const a of ng.objetos || []) saida.push(["objetos", a]);
  return saida;
}

function desenharAreas(ctx, tipos) {
  const tela = elemento("mapa-tela");
  for (const [tipo, a] of linhasDasAreas(tipos)) {
    const [, classe, x, z, ex, ez] = a;
    const [sx, sy] = paraTela(x, z);
    if (tipo === "objetos") {
      // Objeto dinâmico (B247): quadrado no ponto.
      const sel = mapa.area && mapa.area.tipo === "objetos" && mapa.area.indice === a[0];
      ctx.fillStyle = sel ? "#ffffff" : CORES_DO_MAPA.objeto;
      ctx.fillRect(sx - 4, sy - 4, 8, 8);
      continue;
    }
    // tamanho em metros → meia-medida (÷2) → pixels da tela (2 m por pixel: ÷2)
    const w = Math.max(4, ex / 4 * mapa.escala), h = Math.max(4, ez / 4 * mapa.escala);
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
    const w = a[4] / 4 * mapa.escala, h = a[5] / 4 * mapa.escala;
    const n = d <= 12 ? d : tipo === "objetos" ? Infinity : (Math.abs(sx - mx) <= w && Math.abs(sy - my) <= h ? 1000 + w * h : Infinity);
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
  // Objeto dinâmico e controlador (B247): registro sem filhos.
  const simples = tipo === "objetos" || tipo === "controladores";
  const campos = mapa.ng.campos;
  const camposDaArea = tipo === "objetos" ? campos.objeto : tipo === "controladores" ? campos.controlador
    : recurso ? campos.area_de_recurso : campos.area;
  const camposDoFilho = recurso ? campos.recurso : campos.gerador;
  const chaveDoModelo = recurso ? "idTemplate" : "dwID";
  const editavel = !!estudio.rascunho;
  const nomeDoFilho = recurso ? "Recurso" : "Gerador";

  const titulo = criar("div", "recursos-linha");
  const NOMES_DO_TIPO = { areas: "Área de criatura", recursos: "Área de recurso", objetos: "Objeto dinâmico", controladores: "Controlador" };
  // Objeto dinâmico (B250): o modelo do DynamicObjects.data do cliente — o nome do arquivo, o caminho na dica.
  const nomeCurtoDoRegistro = tipo === "objetos" && dados.nome ? dados.nome.split("\\").pop().replace(/\.ecm$/i, "") : dados.nome;
  const forte = criar("strong", "", `${NOMES_DO_TIPO[tipo]} #${indice}${nomeCurtoDoRegistro ? `: ${nomeCurtoDoRegistro}` : ""}`);
  if (dados.nome) forte.title = dados.nome;
  titulo.append(forte,
    criar("span", "texto-suave", editavel ? ` · rascunho #${estudio.rascunho}` : " · só leitura: escolha ou crie um rascunho no topo do Estúdio"));
  ed.appendChild(titulo);

  const area = criar("fieldset");
  area.appendChild(criar("legend", "", simples ? NOMES_DO_TIPO[tipo] : "Área"));
  area.appendChild(grupoDeCampos(camposDaArea, dados.campos, editavel));
  ed.appendChild(area);
  // O id do controlador é o que as áreas citam: não se edita (clone para ter outro).
  const idCtrl = area.querySelector('input[data-campo="id"]');
  if (tipo === "controladores" && idCtrl) idCtrl.disabled = true;
  if (tipo === "controladores") {
    const usos = dados.usado_por || [];
    const linha = criar("div", "texto-suave", usos.length ? `Citado por ${usos.length}: ` : "Nenhuma área ou objeto cita este controlador.");
    usos.slice(0, 30).forEach(([t, i], k) => {
      const a = criar("a", "", `${k ? ", " : ""}${t === "areas" ? "área" : t === "recursos" ? "área de recurso" : "objeto"} #${i}`);
      a.href = "#";
      a.addEventListener("click", (e) => { e.preventDefault(); abrirArea(t, i); });
      linha.appendChild(a);
    });
    ed.appendChild(linha);
  } else if (!simples || tipo === "objetos") {
    // Área/objeto: o controlador citado, com link.
    const c = dados.campos.idCtrl ?? dados.campos.idController;
    if (c) {
      const ctrl = (mapa.ng.controladores || []).find((x) => x[1] === c);
      const linha = criar("div", "texto-suave", ctrl ? "Controlador: " : `Controlador ${c}: não existe no arquivo (referência pendurada; o GS trata como ligado).`);
      if (ctrl) {
        const a = criar("a", "", `${ctrl[3] || "(sem nome)"} (id ${ctrl[1]}, gatilho ${ctrl[2]}, ${ctrl[4] ? "ligado" : "desligado"} ao subir)`);
        a.href = "#";
        a.addEventListener("click", (e) => { e.preventDefault(); abrirArea("controladores", ctrl[0]); });
        linha.appendChild(a);
      }
      ed.appendChild(linha);
    }
  }

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
    // Área de NPC (`iNPCType` = 1): a posição do rastreador de missões do cliente para este NPC (B246).
    if (!recurso && dados.campos.iNPCType === 1) linhaDoRastreador(f, valores[chaveDoModelo], area);
    filhos.appendChild(f);
  };
  if (!simples) {
    dados.filhos.forEach(desenharFilho);
    ed.appendChild(filhos);
  }

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
  if (!simples) {
    botao(`+ ${nomeDoFilho.toLowerCase()} (cópia do último)`, "botao-secundario", () => {
      const ultimos = [...filhos.querySelectorAll("fieldset[data-filho]")];
      desenharFilho(ultimos.length ? valoresDosCampos(ultimos[ultimos.length - 1]) : {}, ultimos.length);
    });
  }
  if (tipo !== "controladores") {
    const pos = botao("Posicionar pelo clique", "botao-secundario", () => {
      mapa.posicionando = !mapa.posicionando;
      pos.textContent = mapa.posicionando ? "Clique no mapa…" : "Posicionar pelo clique";
    });
  }
  const nomeCurto = NOMES_DO_TIPO[tipo].toLowerCase();
  botao("Salvar no rascunho", "botao-primario", () => acaoNaArea(async () => {
    const corpo = { campos: valoresDosCampos(area) };
    if (!simples) corpo.filhos = [...filhos.querySelectorAll("fieldset[data-filho]")].map(valoresDosCampos);
    return api(rotaDoMapa(rota), { method: "PUT", body: JSON.stringify(corpo) });
  }, () => "Salvo no rascunho.", () => indice));
  botao("Clonar", "botao-secundario", () => acaoNaArea(
    () => api(rotaDoMapa(rota.replace("?", "/clonar?")), { method: "POST" }),
    (r) => `Clonado: #${r.resultado.indice}${tipo === "controladores" ? " (id novo)" : " (mesma posição; mova-o)"}.`, (r) => r.resultado.indice));
  botao("Remover", "botao-secundario texto-perigo", () => {
    if (!confirm(`Remover ${nomeCurto} #${indice}? Os seguintes sobem um número.`)) return;
    acaoNaArea(() => api(rotaDoMapa(rota), { method: "DELETE" }), () => "Removido no rascunho.", () => null);
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
  const ix = ed.querySelector('input[data-campo="vPos_x"], input[data-pos="x"]');
  const iz = ed.querySelector('input[data-campo="vPos_z"], input[data-pos="z"]');
  if (ix && iz) { ix.value = x; iz.value = z; }
  // Altura do chão no ponto (B248), dos .hmap do realm: preenche o Y quando o editor tem um.
  const iy = ed.querySelector('input[data-campo="vPos_y"], input[data-pos="y"]');
  if (iy && !iy.disabled) {
    api(rotaDoMapa(`/${encodeURIComponent(m.caminho)}/altura?x=${x}&z=${z}`))
      .then((r) => {
        if (r.y == null) { alerta("aviso", "Estúdio", "Sem terreno do realm neste ponto: confira o Y à mão.", { toast: true }); return; }
        iy.value = r.y;
      })
      .catch((erro) => alerta("aviso", "Estúdio", `Altura do chão: ${erro.message}`, { toast: true }));
  }
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

/* ---------------- Distritos e portais (S4 parte 3, B244) ----------------
   Distrito (`precinct.sev`): renascer na cidade leva ao ponto de cidade do distrito onde se morreu
   (`city_region::GetCityPos` → `CELPrecinctSet::IsPointIn`); nome, id e marcas vêm do `precinct.clt` do cliente.
   Portal (`region.sev`): o cliente acha a caixa pela própria `region.clt` e manda só o índice
   (`EC_World.cpp:2360-2373`), então no servidor só o ponto de chegada é editável. O carimbo dos dois arquivos
   (conferido pelo cliente) nunca muda. */

const COR_DO_DISTRITO = "#5ec8ff", COR_DO_PORTAL = "#e07bff", COR_DO_TELEPORTE = "#ffa94d";

async function carregarCamadas() {
  const m = mapa.atual;
  if (!m) return;
  const q = estudio.rascunho ? `?rascunho=${estudio.rascunho}` : "";
  const carregar = async (id, nome) => {
    if (!elemento(id).checked) return null;
    try { return await api(rotaDoMapa(`/${encodeURIComponent(m.caminho)}/${nome}${q}`)); }
    catch (erro) { alerta("aviso", "Estúdio", `${nome}: ${erro.message}`, { toast: true }); return null; }
  };
  const [d, p, t, rs] = await Promise.all([carregar("mapa-distritos", "distritos"), carregar("mapa-portais", "portais"),
    carregar("mapa-teleportes", "teleportes"), carregar("mapa-rastreador", "rastreador")]);
  if (mapa.atual !== m) return;
  mapa.rastreador = rs;
  if (rs && !rs.usado_pelo_cliente) alerta("aviso", "Estúdio", "O cliente só consulta o rastreador no mapa principal (mapa 1): neste mapa ele não é usado.", { toast: true });
  mapa.distritos = d;
  mapa.portais = p;
  mapa.teleportes = t;
}

function nomeDoMapa(id) {
  const m = mapa.lista.find((x) => x.id === id);
  return m ? `${m.nome} (${id})` : `mapa ${id}`;
}

function desenharCamadas(ctx) {
  const m = mapa.atual;
  const escolhido = (camada, i) => mapa.area && mapa.area.camada === camada && mapa.area.indice === i;
  if (mapa.distritos) {
    for (const d of mapa.distritos.distritos) {
      if (d.mapa !== m.id || d.vertices.length < 2) continue;
      const sel = escolhido("distritos", d.indice);
      ctx.strokeStyle = sel ? "#ffffff" : COR_DO_DISTRITO;
      ctx.lineWidth = sel ? 2.5 : 1;
      ctx.setLineDash([6, 4]);
      ctx.beginPath();
      d.vertices.forEach(([x, z], k) => { const [sx, sy] = paraTela(x, z); if (k) ctx.lineTo(sx, sy); else ctx.moveTo(sx, sy); });
      ctx.closePath();
      ctx.stroke();
      ctx.setLineDash([]);
      if (d.mapa_do_ponto === m.id) {
        const [sx, sy] = paraTela(d.ponto[0], d.ponto[2]);
        ctx.fillStyle = sel ? "#ffffff" : COR_DO_DISTRITO;
        ctx.fillRect(sx - 4, sy - 4, 8, 8);
      }
      if (mapa.escala > 0.25) {
        ctx.fillStyle = "rgba(255,255,255,0.85)";
        ctx.font = "11px sans-serif";
        for (const mk of d.marcas || []) {
          const [sx, sy] = paraTela(mk.x, mk.z);
          ctx.fillText(mk.nome, sx + 3, sy - 3);
        }
      }
    }
  }
  if (mapa.rastreador) {
    // Ponto do rastreador; linha até a área mais perto quando está a mais de 20 m dela.
    for (const n of mapa.rastreador.npcs) {
      if (!n.rastreador) continue;
      const [sx, sy] = paraTela(n.rastreador[0], n.rastreador[2]);
      const longe = n.distancia > 20;
      ctx.strokeStyle = ctx.fillStyle = longe ? "#ff4d4d" : "#9ef01a";
      if (longe) {
        const perto = n.areas.reduce((a, b) => (Math.hypot(b[1] - n.rastreador[0], b[2] - n.rastreador[2]) < Math.hypot(a[1] - n.rastreador[0], a[2] - n.rastreador[2]) ? b : a));
        const [ax, ay] = paraTela(perto[1], perto[2]);
        ctx.setLineDash([3, 3]);
        ctx.beginPath(); ctx.moveTo(sx, sy); ctx.lineTo(ax, ay); ctx.stroke();
        ctx.setLineDash([]);
      }
      ctx.beginPath();
      ctx.arc(sx, sy, escolhido("rastreador", n.id) ? 5 : 3, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  if (mapa.teleportes) {
    ctx.font = "11px sans-serif";
    for (const t of mapa.teleportes.pontos) {
      const [sx, sy] = paraTela(t.pos[0], t.pos[2]);
      const sel = escolhido("teleportes", t.id);
      ctx.fillStyle = sel ? "#ffffff" : COR_DO_TELEPORTE;
      ctx.beginPath();
      ctx.moveTo(sx, sy - 6); ctx.lineTo(sx + 6, sy); ctx.lineTo(sx, sy + 6); ctx.lineTo(sx - 6, sy);
      ctx.closePath();
      ctx.fill();
      if (mapa.escala > 0.12 && t.nome) ctx.fillText(t.nome, sx + 8, sy + 4);
    }
  }
  if (mapa.portais) {
    for (const c of mapa.portais.caixas) {
      if (c.origem !== m.id) continue;
      const sel = escolhido("portais", c.indice);
      const [sx, sy] = paraTela(c.pos[0], c.pos[2]);
      // `exts` é meia-medida no region.sev (`el_region.h:184-192`).
      const w = Math.max(4, c.exts[0] * mapa.escala / 2), h = Math.max(4, c.exts[2] * mapa.escala / 2);
      ctx.strokeStyle = sel ? "#ffffff" : COR_DO_PORTAL;
      ctx.lineWidth = sel ? 2.5 : 1.5;
      ctx.strokeRect(sx - w, sy - h, w * 2, h * 2);
      if (c.destino === m.id) {
        const [ax, ay] = paraTela(c.alvo[0], c.alvo[2]);
        ctx.beginPath();
        ctx.moveTo(sx, sy);
        ctx.lineTo(ax, ay);
        ctx.stroke();
        ctx.fillStyle = ctx.strokeStyle;
        ctx.beginPath();
        ctx.arc(ax, ay, 4, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }
}

function escolherDistritoOuPortal(ev) {
  if (!mapa.distritos && !mapa.portais && !mapa.teleportes && !mapa.rastreador) return false;
  const m = mapa.atual;
  const r = elemento("mapa-tela").getBoundingClientRect();
  const mx = ev.clientX - r.left, my = ev.clientY - r.top;
  for (const t of mapa.teleportes?.pontos || []) {
    const [sx, sy] = paraTela(t.pos[0], t.pos[2]);
    if (Math.hypot(sx - mx, sy - my) < 9) { abrirDaCamada("teleportes", t.id); return true; }
  }
  for (const n of mapa.rastreador?.npcs || []) {
    if (!n.rastreador) continue;
    const [sx, sy] = paraTela(n.rastreador[0], n.rastreador[2]);
    if (Math.hypot(sx - mx, sy - my) < 7) { abrirDaCamada("rastreador", n.id); return true; }
  }
  // Portal (caixa ou chegada perto), depois ponto de cidade perto, depois o distrito que contém o clique.
  for (const c of mapa.portais?.caixas || []) {
    if (c.origem !== m.id) continue;
    const [sx, sy] = paraTela(c.pos[0], c.pos[2]);
    const w = Math.max(6, c.exts[0] * mapa.escala / 2), h = Math.max(6, c.exts[2] * mapa.escala / 2);
    let perto = Math.abs(sx - mx) <= w && Math.abs(sy - my) <= h;
    if (!perto && c.destino === m.id) { const [ax, ay] = paraTela(c.alvo[0], c.alvo[2]); perto = Math.hypot(ax - mx, ay - my) < 8; }
    if (perto) { abrirDaCamada("portais", c.indice); return true; }
  }
  const lista = (mapa.distritos?.distritos || []).filter((d) => d.mapa === m.id);
  for (const d of lista) {
    if (d.mapa_do_ponto !== m.id) continue;
    const [sx, sy] = paraTela(d.ponto[0], d.ponto[2]);
    if (Math.hypot(sx - mx, sy - my) < 8) { abrirDaCamada("distritos", d.indice); return true; }
  }
  // Mundo: coordenadas do clique; o distrito de menor prioridade que o contém (`el_precinct.cpp:379-394`).
  const x = 2 * (mx - mapa.x) / mapa.escala - m.colunas * 512, z = m.linhas * 512 - 2 * (my - mapa.y) / mapa.escala;
  let melhor = null;
  for (const d of lista) {
    let dentro = false;
    for (let i = 0, j = d.vertices.length - 1; i < d.vertices.length; j = i++) {
      const [xi, zi] = d.vertices[i], [xj, zj] = d.vertices[j];
      if ((zi > z) !== (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi) dentro = !dentro;
    }
    if (dentro && (!melhor || d.prioridade <= melhor.prioridade)) melhor = d;
  }
  if (melhor) { abrirDaCamada("distritos", melhor.indice); return true; }
  return false;
}

function campoNumerico(rotulo, valor, editavel, pos) {
  const label = criar("label");
  const input = criar("input", "campo");
  input.type = "number";
  input.step = "any";
  input.value = valor;
  input.disabled = !editavel;
  if (pos) input.dataset.pos = pos;
  label.append(criar("span", "", rotulo), input);
  return [label, input];
}

function abrirDaCamada(camada, indice) {
  const ed = elemento("mapa-editor");
  ed.replaceChildren();
  const fonte = camada === "distritos" ? mapa.distritos?.distritos : camada === "portais" ? mapa.portais?.caixas
    : camada === "teleportes" ? mapa.teleportes?.pontos : mapa.rastreador?.npcs;
  const porId = camada === "teleportes" || camada === "rastreador";
  const item = fonte && fonte.find((x) => (porId ? x.id : x.indice) === indice);
  if (!item) { fecharArea(); desenharMapa(); return; }
  mapa.area = { camada, indice };
  ed.hidden = false;
  const editavel = !!estudio.rascunho;
  const m = mapa.atual;
  const titulo = criar("div", "recursos-linha");
  const grade = criar("div", "mapa-campos");
  const entradas = {};
  let corpo;
  if (camada === "distritos") {
    titulo.append(criar("strong", "", `Distrito #${indice}${item.nome ? `: ${item.nome}` : ""}`),
      criar("span", "texto-suave", `${item.id != null ? ` · id ${item.id}` : ""} · ${nomeDoMapa(item.mapa)}${item.dominio ? ` · domínio ${item.dominio}` : ""}`));
    const [lp, ip] = campoNumerico("Prioridade (menor ganha)", item.prioridade, editavel);
    const lm = criar("label");
    const sel = criar("select", "campo");
    sel.disabled = !editavel;
    for (const x of mapa.lista) { const o = criar("option", "", `${x.nome} (${x.id})`); o.value = x.id; sel.appendChild(o); }
    if (!mapa.lista.some((x) => x.id === item.mapa_do_ponto)) { const o = criar("option", "", `mapa ${item.mapa_do_ponto}`); o.value = item.mapa_do_ponto; sel.appendChild(o); }
    sel.value = item.mapa_do_ponto;
    lm.append(criar("span", "", "Mapa do ponto de renascer"), sel);
    const [lx, ix] = campoNumerico("Renascer X", item.ponto[0], editavel, "x");
    const [ly, iy] = campoNumerico("Renascer Y (altura)", item.ponto[1], editavel, "y");
    const [lz, iz] = campoNumerico("Renascer Z", item.ponto[2], editavel, "z");
    grade.append(lp, lm, lx, ly, lz);
    Object.assign(entradas, { ip, sel, ix, iy, iz });
    if (mapa.distritos.versao_do_arquivo >= 7) {
      const lk = criar("label", "marca");
      const ck = criar("input");
      ck.type = "checkbox";
      ck.checked = item.protege_pk;
      ck.disabled = !editavel;
      lk.append(ck, criar("span", "", "Protege de PK"));
      grade.appendChild(lk);
      entradas.ck = ck;
    }
    corpo = () => {
      const c = {};
      if (Number(ip.value) !== item.prioridade) c.prioridade = Math.trunc(Number(ip.value));
      if (Number(sel.value) !== item.mapa_do_ponto) c.mapa_do_ponto = Number(sel.value);
      const ponto = [Number(ix.value), Number(iy.value), Number(iz.value)];
      if (ponto.some((v, k) => v !== item.ponto[k])) c.ponto = ponto;
      if (entradas.ck && entradas.ck.checked !== item.protege_pk) c.protege_pk = entradas.ck.checked;
      return c;
    };
    ed.append(titulo, grade);
    if (item.marcas?.length) ed.appendChild(criar("div", "texto-suave", `Marcas do cliente: ${item.marcas.map((k) => k.nome).join(", ")}`));
    if (item.mapa_do_ponto !== m.id) ed.appendChild(criar("div", "texto-suave", "O ponto de renascer fica em outro mapa: \"Posicionar pelo clique\" usaria as coordenadas deste."));
  } else if (camada === "rastreador") {
    titulo.append(criar("strong", "", `Rastreador: ${item.nome || `NPC ${item.id}`}`),
      criar("span", "texto-suave", ` · id ${item.id}${item.distancia != null ? ` · ${item.distancia} m da área mais perto` : ""}`));
    const p0 = item.rastreador || [0, 0, 0];
    const [lx, ix] = campoNumerico("X", p0[0], editavel, "x");
    const [ly, iy] = campoNumerico("Y (altura)", p0[1], editavel);
    const [lz, iz] = campoNumerico("Z", p0[2], editavel, "z");
    grade.append(lx, ly, lz);
    corpo = () => {
      const pos = [Number(ix.value), Number(iy.value), Number(iz.value)];
      return pos.some((v, k) => Math.round(v) !== p0[k]) ? { pos } : {};
    };
    ed.append(titulo, grade, criar("div", "texto-suave",
      "Arquivo do cliente (data\\task_npc.data): o launcher entrega ao publicar. Coordenadas inteiras (short); o arquivo não guarda o mapa."));
  } else if (camada === "teleportes") {
    titulo.append(criar("strong", "", `Destino de teleporte ${item.id}${item.nome ? `: ${item.nome}` : ""}`),
      criar("span", "texto-suave", ` · domínio ${item.dominio}`));
    const [lx, ix] = campoNumerico("Chegada X", item.pos[0], editavel, "x");
    const [ly, iy] = campoNumerico("Chegada Y (altura)", item.pos[1], editavel, "y");
    const [lz, iz] = campoNumerico("Chegada Z", item.pos[2], editavel, "z");
    grade.append(lx, ly, lz);
    corpo = () => {
      const pos = [Number(ix.value), Number(iy.value), Number(iz.value)];
      return pos.some((v, k) => v !== item.pos[k]) ? { pos } : {};
    };
    ed.append(titulo, grade);
    const lista = criar("div", "texto-suave");
    if (!item.usado_por.length) lista.textContent = "Nenhuma transportadora do elements leva a este destino.";
    for (const u of item.usado_por) {
      const linha = criar("div", "", `${u.nome} (serviço ${u.servico}) · preço ${u.preco} · nível ${u.nivel}${u.npcs.length ? " · NPCs: " : ""}`);
      u.npcs.forEach((n, k) => {
        const a = criar("a", "", (k ? ", " : "") + n.nome);
        a.href = "#";
        a.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio("NPC_ESSENCE", n.indice); });
        linha.appendChild(a);
      });
      lista.appendChild(linha);
    }
    ed.appendChild(lista);
  } else {
    titulo.append(criar("strong", "", `Portal #${indice}`),
      criar("span", "texto-suave", ` · leva a ${nomeDoMapa(item.destino)}${item.nivel_minimo > 1 ? ` · nível ${item.nivel_minimo}` : ""}`));
    const [lx, ix] = campoNumerico("Chegada X", item.alvo[0], editavel, "x");
    const [ly, iy] = campoNumerico("Chegada Y (altura)", item.alvo[1], editavel, "y");
    const [lz, iz] = campoNumerico("Chegada Z", item.alvo[2], editavel, "z");
    grade.append(lx, ly, lz);
    corpo = () => {
      const alvo = [Number(ix.value), Number(iy.value), Number(iz.value)];
      return alvo.some((v, k) => v !== item.alvo[k]) ? { alvo } : {};
    };
    ed.append(titulo, grade,
      criar("div", "texto-suave", `Caixa em (${item.pos.map((v) => v.toFixed(1)).join(", ")}), meia-medida (${item.exts.map((v) => v.toFixed(1)).join(", ")}): o cliente decide pela region.clt dele, por isso posição, tamanho e destino não se editam aqui.`));
    if (item.destino !== m.id) ed.appendChild(criar("div", "texto-suave", `A chegada fica em ${nomeDoMapa(item.destino)}: abra aquele mapa para posicionar pelo clique.`));
  }
  if (!editavel) {
    titulo.appendChild(criar("span", "texto-suave", " · só leitura: escolha ou crie um rascunho no topo do Estúdio"));
    desenharMapa();
    return;
  }
  const botoes = criar("div", "recursos-linha");
  const pos = criar("button", "botao botao-pequeno botao-secundario", "Posicionar pelo clique");
  pos.type = "button";
  pos.addEventListener("click", () => { mapa.posicionando = !mapa.posicionando; pos.textContent = mapa.posicionando ? "Clique no mapa…" : "Posicionar pelo clique"; });
  const salvar = criar("button", "botao botao-pequeno botao-primario", "Salvar no rascunho");
  salvar.type = "button";
  salvar.addEventListener("click", async () => {
    const campos = corpo();
    if (!Object.keys(campos).length) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
    try {
      const rota = porId
        ? `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/${camada}/${indice}?rascunho=${estudio.rascunho}`
        : rotaDoMapa(`/${encodeURIComponent(m.caminho)}/${camada}/${indice}?rascunho=${estudio.rascunho}`);
      const r = await api(rota, { method: "PUT", body: JSON.stringify({ campos }) });
      alerta("ok", "Estúdio", "Salvo no rascunho." + (r.igual_ao_publicado ? " Ficou igual ao realm: o item saiu do rascunho." : ""), { toast: true });
      await carregarCamadas();
      abrirDaCamada(camada, indice);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  botoes.append(pos, salvar);
  ed.appendChild(botoes);
  desenharMapa();
}

/* Rastreador no editor de área de NPC (B246): mostra a posição que o task_npc.data do cliente tem para o NPC e
   põe nele a posição da área (o centro, como está nos campos da tela). */
async function linhaDoRastreador(fieldset, idNpc, campoDaArea) {
  const linha = criar("div", "texto-suave", "Rastreador: carregando…");
  fieldset.appendChild(linha);
  try {
    if (!mapa.rastreador) {
      const q = estudio.rascunho ? `?rascunho=${estudio.rascunho}` : "";
      mapa.rastreador = await api(rotaDoMapa(`/${encodeURIComponent(mapa.atual.caminho)}/rastreador${q}`));
    }
  } catch (erro) { linha.textContent = `Rastreador: ${erro.message}`; return; }
  const n = mapa.rastreador.npcs.find((x) => x.id === idNpc);
  linha.textContent = n?.rastreador
    ? `Rastreador de missões: (${n.rastreador.join(", ")})${n.distancia != null ? ` · ${n.distancia} m da área mais perto` : ""} `
    : "Rastreador de missões: este NPC não tem posição no task_npc.data do cliente. ";
  if (!mapa.rastreador.usado_pelo_cliente) linha.textContent += "(O cliente só usa o rastreador no mapa principal.) ";
  if (!estudio.rascunho) return;
  const b = criar("button", "botao botao-pequeno botao-secundario", "Usar a posição da área no rastreador");
  b.type = "button";
  b.addEventListener("click", async () => {
    const v = (c) => Number(campoDaArea.querySelector(`input[data-campo="${c}"]`)?.value || 0);
    try {
      await api(`/api/realms/${encodeURIComponent(estudio.realm)}/estudio/rastreador/${idNpc}?rascunho=${estudio.rascunho}`,
        { method: "PUT", body: JSON.stringify({ campos: { pos: [v("vPos_x"), v("vPos_y"), v("vPos_z")] } }) });
      alerta("ok", "Estúdio", "Rastreador atualizado no rascunho (arquivo do cliente).", { toast: true });
      mapa.rastreador = null;
      linha.remove();
      linhaDoRastreador(fieldset, idNpc, campoDaArea);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  linha.appendChild(b);
}


/* Controladores do npcgen (B247): lista com busca por nome, id ou gatilho; clique abre o controlador no editor. */
function desenharListaDeControladores() {
  let caixa = elemento("mapa-controladores");
  if (!caixa) {
    caixa = criar("details", "mapa-editor");
    caixa.id = "mapa-controladores";
    elemento("mapa-editor").before(caixa);
  }
  caixa.replaceChildren();
  const ng = mapa.ng;
  caixa.hidden = !mapa.editar || !ng;
  if (!ng) return;
  const lista = ng.controladores || [];
  const resumo = criar("summary", "", `Controladores do npcgen (${lista.length})`
    + (ng.controladores_inexistentes_citados?.length ? ` · citados e inexistentes: ${ng.controladores_inexistentes_citados.join(", ")}` : ""));
  const busca = criar("input", "campo");
  busca.type = "search";
  busca.placeholder = "Nome, id ou gatilho";
  const saida = criar("div", "texto-suave");
  const filtrar = () => {
    const t = busca.value.trim().toLowerCase();
    saida.replaceChildren();
    const achados = lista.filter((c) => !t || String(c[1]) === t || String(c[2]) === t || (c[3] || "").toLowerCase().includes(t));
    for (const c of achados.slice(0, 60)) {
      const a = criar("a", "", `#${c[0]} ${c[3] || "(sem nome)"} · id ${c[1]} · gatilho ${c[2]} · ${c[4] ? "ligado" : "desligado"} · ${c[5]} citações`);
      a.href = "#";
      a.style.display = "block";
      a.addEventListener("click", (e) => { e.preventDefault(); abrirArea("controladores", c[0]); });
      saida.appendChild(a);
    }
    if (achados.length > 60) saida.appendChild(criar("div", "", `… e mais ${achados.length - 60}; refine a busca.`));
  };
  busca.addEventListener("input", filtrar);
  caixa.append(resumo, busca, saida);
  filtrar();
}
