/* Estúdio, fase S4 (B242): mapa interativo do realm — peças do minimapa do cliente e as criaturas do
   npcgen do realm. Conta do cliente (DlgMiniMap.cpp:1045-1061): 2 m por pixel, peças de 256 px
   "LLCC" (linha, coluna), origem no centro: px = (x + colunas·512)/2, py = (linhas·512 − z)/2.
   Usa `api`, `criar`, `elemento`, `estado` do painel.js e `estudio`/`abrirRegistroDoEstudio` do estudio.js. */

const mapa = {
  lista: [], atual: null, escala: 0.25, x: 0, y: 0, pontos: [], modelos: {}, pecas: new Map(),
  arrastando: null, desenhoPendente: false,
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
    if (a && !a.moveu) detalharPonto(e);
  });
  window.addEventListener("resize", desenharMapa);
})();
