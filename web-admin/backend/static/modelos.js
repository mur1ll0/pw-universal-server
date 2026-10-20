/* Estúdio, fase S5 (B272): modelos 3D. Lista os `.ski` do cliente publicado, mostra a ficha (ossos, texturas, materiais,
   malhas) e desenha o GLB que o pw-recursos gera (`pw-data-loader/src/glb.rs`) num visualizador WebGL próprio — sem
   biblioteca externa (CSP `self`). Textura (B273: PNG embutido no GLB, alfa por corte), luz direcional e (B276) a
   animação de uma ação: canais avaliados e pele deformada na CPU a cada quadro.
   Usa `api`, `criar`, `elemento`, `alerta`, `estudio` e `mostrarVistaDoEstudio`. */

function rotaDeModelos(resto, q = {}) {
  const p = new URLSearchParams(q).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/modelos${resto}${p ? `?${p}` : ""}`;
}

async function carregarModelos() {
  if (!estudio.realm) return;
  elemento("modelos-situacao").textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeModelos("", { busca: elemento("modelos-busca").value.trim() })); }
  catch (erro) { elemento("modelos-situacao").textContent = erro.message; return; }
  const pt = r.por_tipo || {};
  elemento("modelos-situacao").textContent = `${pt.ecm ?? 0} .ecm · ${pt.smd ?? 0} .smd · ${pt.ski ?? 0} .ski${r.total > r.linhas.length ? ` (até 200 de cada)` : ""}`;
  const area = elemento("modelos-lista");
  area.replaceChildren();
  for (const l of r.linhas) {
    const ext = l.caminho.toLowerCase().slice(-4);
    const b = criar("button", "", `${ext === ".ecm" ? "◆ " : ext === ".smd" ? "▣ " : "· "}${l.caminho.replace(/^models\\/i, "")}`);
    b.type = "button";
    b.title = l.caminho;
    b.addEventListener("click", () => abrirModelo(l.caminho));
    area.appendChild(b);
  }
  if (!r.linhas.length) area.appendChild(criar("div", "texto-suave", "Nenhum modelo."));
}

const MODOS_DE_VOO = ["espada (fly_mode 0)", "asa (fly_mode 1, WINGMANWING)", "rodas (fly_mode 2)"];
const HABITATS = ["chão", "água", "ar", "chão + água", "chão + ar", "água + ar", "chão, água e ar"];

/* Tabela de diagnóstico (B277/B281) com o "Criar" do B280 em cada falta. */
function tabelaDeDiagnostico(f, caminho, titulo, lista, habitat, voo = null) {
  const tab = criar("table", "tabela-simples");
  tab.appendChild(criar("caption", "", titulo));
  for (const d of lista) {
    const tr = criar("tr");
    // B283: "ausente, como na maioria dos colegas" (voo) não é alerta — mas ainda pode ser criada.
    const semAlerta = d.estado === "ok" || d.estado.startsWith("ausente, como");
    tr.append(criar("td", "", d.pedido), criar("td", semAlerta ? "texto-suave" : "", semAlerta ? d.estado : `⚠ ${d.estado}`));
    // B280: corrigir — ação combinada nova clonando uma existente, opcionalmente com outra ação-base do .smd.
    if (d.estado !== "ok" && estudio.rascunho && f.combinadas.length) {
      const td = criar("td");
      const origem = criar("select", "campo");
      for (const c of f.combinadas) origem.appendChild(new Option(`copiar ${c.nome}`, c.nome));
      const base = criar("select", "campo");
      base.appendChild(new Option("(mesma ação-base)", ""));
      for (const b of f.bases_do_smd || []) base.appendChild(new Option(`base ${b}`, b));
      const b = criar("button", "botao botao-pequeno botao-secundario", "Criar");
      b.type = "button";
      b.addEventListener("click", async () => {
        try {
          await api(rotaDeModelos("/acao", { rascunho: estudio.rascunho }), { method: "POST", body: JSON.stringify({ caminho, origem: origem.value, nova: d.pedido, base: base.value || undefined }) });
          alerta("ok", "Estúdio", `Ação ${d.pedido} criada no rascunho.`, { toast: true });
          if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
          abrirModelo(caminho, habitat, voo);
        } catch (erro) { alerta("erro", "Estúdio", erro.message); }
      });
      td.append(origem, base, b);
      // B285: copiar a ação de outro .ecm com o mesmo esqueleto (e mesma versão de arquivo) que a tenha.
      const doar = criar("button", "botao botao-pequeno botao-secundario", "Copiar de outro modelo…");
      doar.type = "button";
      doar.addEventListener("click", async () => {
        doar.disabled = true;
        doar.textContent = "Procurando (a 1ª vez lê todos os .ecm)…";
        try {
          const r = await api(rotaDeModelos("/doadores", { caminho, acao: d.pedido }));
          const sel = criar("select", "campo");
          for (const x of r.doadores) {
            // B285: outro esqueleto anima igual (as bases vêm do .smd do destino), mas os efeitos podem citar ganchos que ele não tem.
            const esq = x.mesmo_smd ? " · mesmo .smd" : x.mesmo_esqueleto ? " · mesmo esqueleto" : " · outro esqueleto (efeitos podem citar ganchos que este não tem)";
            const o = new Option(`${x.caminho.split("\\").pop()}${esq}${x.bases_faltando.length ? ` · ⚠ faltam no .smd: ${x.bases_faltando.join(", ")}` : ""}`, x.caminho);
            o.disabled = x.bases_faltando.length > 0;
            o.title = x.caminho;
            sel.appendChild(o);
          }
          if (!r.doadores.length) { doar.replaceWith(criar("span", "texto-suave", r.motivo || `Nenhum modelo com o mesmo esqueleto tem ${d.pedido}.`)); return; }
          const copiar = criar("button", "botao botao-pequeno", "Copiar");
          copiar.type = "button";
          copiar.addEventListener("click", async () => {
            try {
              await api(rotaDeModelos("/acao", { rascunho: estudio.rascunho }), { method: "POST", body: JSON.stringify({ caminho, de: sel.value, origem: d.pedido, nova: d.pedido }) });
              alerta("ok", "Estúdio", `Ação ${d.pedido} copiada de ${sel.value.split("\\").pop()} no rascunho.`, { toast: true });
              if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
              abrirModelo(caminho, habitat, voo);
            } catch (erro) { alerta("erro", "Estúdio", erro.message); }
          });
          doar.replaceWith(criar("span", "texto-suave", `${r.total} doador(es): `), sel, copiar);
        } catch (erro) { doar.disabled = false; doar.textContent = "Copiar de outro modelo…"; alerta("erro", "Estúdio", erro.message); }
      });
      td.appendChild(doar);
      tr.appendChild(td);
    }
    tab.appendChild(tr);
  }
  return tab;
}

/** B290 (S6): "Substituir por GLB…" — a pele `destino` vira a do glTF escolhido, com os ossos do esqueleto do `.smd`
    `smd`, no rascunho; mostra o relatório do que foi ajustado ou deixado de fora. */
function botaoDeImportar(smd, destino) {
  const rotulo = criar("label", "botao botao-pequeno botao-secundario", "Substituir por GLB…");
  const arquivo = criar("input");
  arquivo.type = "file";
  arquivo.accept = ".glb,model/gltf-binary";
  arquivo.hidden = true;
  rotulo.appendChild(arquivo);
  arquivo.addEventListener("change", async () => {
    const f = arquivo.files[0];
    arquivo.value = "";
    if (!f) return;
    if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio antes de importar.", { toast: true }); return; }
    try {
      const resp = await fetch(rotaDeModelos("/importar", { rascunho: estudio.rascunho, smd, destino }), {
        method: "POST", credentials: "same-origin", body: await f.arrayBuffer(),
        headers: { "Content-Type": "application/octet-stream", "X-CSRF-Token": estado.sessao.csrf },
      });
      const dados = await resp.json();
      if (!resp.ok) throw new Error(dados?.mensagem || dados?.detail || "Não foi possível importar.");
      const r = dados.resultado || {};
      const avisos = (r.relatorio || []).length ? ` Relatório: ${r.relatorio.join(" · ")}` : "";
      alerta("ok", "Estúdio", `${destino.split("\\").pop()} substituída no rascunho: ${r.malhas} malhas, ${r.vertices} vértices, ${r.ossos} ossos, texturas ${(r.texturas || []).join(", ") || "(nenhuma)"}.${avisos}`);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  return rotulo;
}

/** B291 (S6): pele nova no modelo — o GLB vira `<pasta do .smd>\<nome>.ski` e o nome entra na lista de peles do `.smd`,
    no rascunho. (NPC e monstro usam `<.ecm>.ski`, não as peles do `.smd` — B282.) */
function controleDePeleNova(smd) {
  const caixa = criar("span");
  const nome = criar("input", "campo");
  nome.placeholder = "nome da pele nova (.ski)";
  nome.size = 24;
  const rotulo = criar("label", "botao botao-pequeno botao-secundario", "Pele nova a partir de GLB…");
  const arquivo = criar("input");
  arquivo.type = "file";
  arquivo.accept = ".glb,model/gltf-binary";
  arquivo.hidden = true;
  rotulo.appendChild(arquivo);
  arquivo.addEventListener("change", async () => {
    const f = arquivo.files[0];
    arquivo.value = "";
    if (!f) return;
    if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio antes de importar.", { toast: true }); return; }
    let n = nome.value.trim() || f.name.replace(/\.glb$/i, "");
    if (!/\.ski$/i.test(n)) n += ".ski";
    try {
      const resp = await fetch(rotaDeModelos("/pele_nova", { rascunho: estudio.rascunho, smd, nome: n }), {
        method: "POST", credentials: "same-origin", body: await f.arrayBuffer(),
        headers: { "Content-Type": "application/octet-stream", "X-CSRF-Token": estado.sessao.csrf },
      });
      const dados = await resp.json();
      if (!resp.ok) throw new Error(dados?.mensagem || dados?.detail || "Não foi possível criar a pele.");
      const r = dados.resultado || {};
      const avisos = (r.relatorio || []).length ? ` Relatório: ${r.relatorio.join(" · ")}` : "";
      alerta("ok", "Estúdio", `${r.pele} criada e ligada ao modelo no rascunho (${r.peles_do_modelo} peles): ${r.malhas} malhas, ${r.vertices} vértices, ${r.ossos} ossos.${avisos}`);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  caixa.append(nome, " ", rotulo);
  return caixa;
}

/** B295 (S6): as animações de um GLB viram ações do `.smd` (uma `.stck` por ação na pasta das trilhas), no rascunho.
    Para o jogo tocar, a ação ainda precisa de uma ação combinada no `.ecm` (o "Criar" do diagnóstico, com a base). */
function botaoDeAcoes(smd) {
  const rotulo = criar("label", "botao botao-pequeno botao-secundario", "Importar ações de GLB…");
  const arquivo = criar("input");
  arquivo.type = "file";
  arquivo.accept = ".glb,model/gltf-binary";
  arquivo.hidden = true;
  rotulo.appendChild(arquivo);
  arquivo.addEventListener("change", async () => {
    const f = arquivo.files[0];
    arquivo.value = "";
    if (!f) return;
    if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio antes de importar.", { toast: true }); return; }
    try {
      const resp = await fetch(rotaDeModelos("/acoes_glb", { rascunho: estudio.rascunho, smd }), {
        method: "POST", credentials: "same-origin", body: await f.arrayBuffer(),
        headers: { "Content-Type": "application/octet-stream", "X-CSRF-Token": estado.sessao.csrf },
      });
      const dados = await resp.json();
      if (!resp.ok) throw new Error(dados?.mensagem || dados?.detail || "Não foi possível importar as ações.");
      const r = dados.resultado || {};
      const lista = (r.acoes || []).map((a) => `${a.nome} (${a.acao}, ${a.quadros} quadros a ${a.fps}/s)`).join(", ");
      const avisos = (r.relatorio || []).length ? ` Relatório: ${r.relatorio.join(" · ")}` : "";
      alerta("ok", "Estúdio", `Ações no rascunho: ${lista}. O modelo agora tem ${r.acoes_do_modelo} ações.${avisos}`);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  return rotulo;
}

async function abrirModelo(caminho, habitat = null, voo = null) {
  const ficha = elemento("modelos-ficha");
  ficha.textContent = "Carregando…";
  let f;
  const q = { caminho };
  if (estudio.rascunho) q.rascunho = estudio.rascunho;
  if (habitat !== null && habitat !== undefined) q.habitat = habitat;
  if (voo !== null && voo !== undefined) q.voo = voo;
  try { f = await api(rotaDeModelos("/ficha", q)); } catch (erro) { ficha.textContent = erro.message; return; }
  ficha.replaceChildren();
  ficha.appendChild(criar("h3", "", caminho.split("\\").pop()));
  if (f.tipo === "ecm") {
    // B277: o modelo do jogo — diagnóstico das ações que o cliente pede (monstro; B281: mascote, com o habitat).
    ficha.appendChild(criar("div", "texto-suave", `${caminho} · versão ${f.versao} · ${f.combinadas.length} ações combinadas · tabela do cliente com ${f.tabela_de_acoes} nomes`));
    const abre = criar("a", "", `Abrir só o .smd (${f.smd.split("\\").pop()})`);
    abre.href = "#";
    abre.addEventListener("click", (ev) => { ev.preventDefault(); abrirModelo(f.smd); });
    ficha.appendChild(abre);
    if (!f.smd_lido) ficha.appendChild(criar("div", "texto-suave", "O .smd citado não foi lido: as ações-base não puderam ser conferidas."));
    // B282: o modelo como o jogo o monta — o .smd do .ecm, sem as peles dele, com a pele <.ecm>.ski (EC_NPC.cpp:186-222).
    const tela = criar("canvas", "modelo-tela");
    tela.width = 640; tela.height = 480;
    tela.style.width = "100%"; tela.style.maxWidth = "640px"; tela.style.background = "#20242a"; tela.style.borderRadius = "6px";
    ficha.appendChild(tela);
    const baixar = criar("a", "", "Baixar GLB (Blender)");
    ficha.appendChild(baixar);
    if ((f.bases_do_smd || []).length) {
      const linha = criar("div", "recursos-linha");
      const acoes = criar("select", "campo");
      acoes.appendChild(new Option("(parado, pose de ligação)", ""));
      for (const a of f.bases_do_smd) acoes.appendChild(new Option(a, a));
      acoes.addEventListener("change", () => carregarGlb(tela, caminho, acoes.value || null, baixar));
      linha.append(criar("span", "texto-suave", "Ação do .smd: "), acoes);
      ficha.appendChild(linha);
    }
    carregarGlb(tela, caminho, null, baixar);
    const perfil = criar("div", "recursos-linha");
    const sel = criar("select", "campo");
    sel.appendChild(new Option("diagnóstico como monstro", ""));
    HABITATS.forEach((h, i) => sel.appendChild(new Option(`como mascote invocado/de combate — habitat ${i}: ${h}`, String(i))));
    // B283: modelo de voo (`fly_mode`) — o encaixe e as ações que os modelos de jogador mandam aos filhos.
    MODOS_DE_VOO.forEach((m, i) => sel.appendChild(new Option(`como modelo de voo — ${m}`, `v${i}`)));
    sel.value = f.voo !== null && f.voo !== undefined ? `v${f.voo}` : f.habitat === null || f.habitat === undefined ? "" : String(f.habitat);
    sel.addEventListener("change", () => {
      if (sel.value.startsWith("v")) abrirModelo(caminho, null, Number(sel.value.slice(1)));
      else abrirModelo(caminho, sel.value === "" ? null : Number(sel.value));
    });
    perfil.append(criar("span", "texto-suave", "Perfil: "), sel);
    ficha.appendChild(perfil);
    if (!estudio.rascunho) ficha.appendChild(criar("div", "texto-suave", "Escolha um rascunho no alto do Estúdio para corrigir as ações que faltam."));
    if (f.diagnostico_de_voo) {
      const d = f.diagnostico_de_voo;
      const enc = d.encaixe_presente === null || d.encaixe_presente === undefined
        ? `⚠ esqueleto não lido (${d.esqueleto_erro || "?"}): o encaixe ${d.encaixe} não pôde ser conferido`
        : d.encaixe_presente ? `encaixe ${d.encaixe} presente no esqueleto — pendura em ${d.gancho_do_jogador || "nos pés"} do jogador`
          : `⚠ falta o gancho ${d.encaixe} no esqueleto: o jogo não posiciona o modelo no jogador (A3DSkinModelAux.cpp:378-407)`;
      ficha.appendChild(criar("div", d.encaixe_presente ? "texto-suave" : "", enc));
      const linhas = d.acoes.map((x) => ({ pedido: `${x.nome} (mandada por ${x.jogadores} de ${d.jogadores} modelos de jogador; ${x.colegas} de ${d.colegas} outros modelos de ${d.tipo} têm)`, estado: x.estado, nome: x.nome }));
      const tab = tabelaDeDiagnostico(f, caminho, `Diagnóstico de voo — ${d.tipo}: as ações que o jogador manda ao modelo de voo (ChildActName)`, linhas.map((x) => ({ ...x, pedido: x.nome })), null, f.voo);
      // A coluna do nome mostra também as contagens.
      [...tab.querySelectorAll("tr")].forEach((tr, i) => { tr.firstChild.textContent = linhas[i].pedido; });
      ficha.appendChild(tab);
    } else if (f.diagnostico_de_mascote) {
      ficha.appendChild(tabelaDeDiagnostico(f, caminho, `Diagnóstico de mascote — habitat ${f.habitat} (${HABITATS[f.habitat] || "?"}): nomes + _地面/_水中/_空中`, f.diagnostico_de_mascote, f.habitat));
    } else {
      ficha.appendChild(tabelaDeDiagnostico(f, caminho, "Diagnóstico de monstro (nomes de Configs\\actions_npc.txt + 起/落 nos ataques)", f.diagnostico_de_monstro, null));
    }
    const lista = criar("ul");
    for (const c of f.combinadas) lista.appendChild(criar("li", "", `${c.nome} ← ${c.bases.length ? c.bases.join(", ") : "(sem ação-base)"}${c.bases_ausentes.length ? ` — ausentes no .smd: ${c.bases_ausentes.join(", ")}` : ""}`));
    ficha.appendChild(criar("div", "texto-suave", "Ações combinadas do .ecm (← ações-base do .smd):"));
    ficha.appendChild(lista);
    return;
  }
  if (f.tipo === "modelo") {
    // B274: o modelo inteiro (.smd) — esqueleto e peles.
    const e = f.esqueleto;
    ficha.appendChild(criar("div", "texto-suave", `${caminho} · ${f.acoes} ações · esqueleto ${e.arquivo || "(nenhum)"}${e.erro ? ` — não lido: ${e.erro}` : ` (v${e.versao}, ${e.ossos.length} ossos, ${e.juntas} juntas, ${e.ganchos.length} ganchos)`}`));
    if (e.ganchos && e.ganchos.length) ficha.appendChild(criar("div", "texto-suave", `Ganchos: ${e.ganchos.join(", ")}`));
    if (!f.peles.some((p) => !p.erro)) ficha.appendChild(criar("div", "texto-suave", f.peles.length
      ? "Nenhuma pele citada existe no cliente: mostra-se só o esqueleto."
      : "O .smd não cita pele (a pele vem do equipamento em jogo): mostra-se só o esqueleto."));
  } else {
    ficha.appendChild(criar("div", "texto-suave", `${caminho} · versão ${f.versao} · ${f.malhas.length} malhas · ${f.ossos.length} ossos na pele (esqueleto: ${f.ossos_do_esqueleto})`));
  }
  const tela = criar("canvas", "modelo-tela");
  tela.width = 640; tela.height = 480;
  tela.style.width = "100%"; tela.style.maxWidth = "640px"; tela.style.background = "#20242a"; tela.style.borderRadius = "6px";
  ficha.appendChild(tela);
  const baixar = criar("a", "", "Baixar GLB (Blender)");
  baixar.href = rotaDeModelos("/glb", { caminho });
  ficha.appendChild(baixar);
  const lista = criar("ul");
  if (f.tipo === "modelo") {
    for (const p of f.peles) {
      const li = criar("li");
      if (p.erro) li.textContent = `${p.caminho} — não lida: ${p.erro}`;
      if (p.erro && /não existe/.test(p.erro)) li.textContent += " (o arquivo citado falta no próprio cliente)";
      else { const a = criar("a", "", p.caminho.split("\\").pop()); a.href = "#"; a.addEventListener("click", (ev) => { ev.preventDefault(); abrirModelo(p.caminho); }); li.append(a, ` — ${p.malhas} malhas, ${p.triangulos} triângulos`); }
      if (!p.erro) li.append(" ", botaoDeImportar(caminho, p.caminho));
      lista.appendChild(li);
    }
    if (!e.erro) { const li = criar("li"); li.appendChild(controleDePeleNova(caminho)); lista.appendChild(li); }
    if (!e.erro) { const li = criar("li"); li.appendChild(botaoDeAcoes(caminho)); lista.appendChild(li); }
  } else {
    for (const m of f.malhas) lista.appendChild(criar("li", "", `${m.nome || "(sem nome)"} — ${m.tipo}, ${m.vertices} vértices, ${m.triangulos} triângulos, textura ${f.texturas[m.textura] ?? m.textura}, material ${f.materiais[m.material]?.nome ?? m.material}`));
  }
  ficha.appendChild(lista);
  if (f.texturas && f.texturas.length) {
    const faltam = f.texturas.filter((_, i) => !(f.texturas_achadas || [])[i]);
    ficha.appendChild(criar("div", "texto-suave", `Texturas: ${f.texturas.join(", ")}${faltam.length ? ` — não achadas no cliente (nem em Textures\\ nem em Tex_<nome>\\): ${faltam.join(", ")}` : ""}.`));
  }
  // B276: ações do modelo — escolher uma traz o GLB com a animação dela.
  if (f.tipo === "modelo" && (f.lista_de_acoes || []).length) {
    const caixa = criar("div", "recursos-linha");
    caixa.appendChild(criar("span", "texto-suave", f.acoes_descartadas ? "Ações (o motor as descarta: .smd < 7 com .bon v6): " : "Ações: "));
    const sel = criar("select", "campo");
    sel.appendChild(new Option("(parado, pose de ligação)", ""));
    for (const a of f.lista_de_acoes) sel.appendChild(new Option(`${a.nome} (${Math.round(a.fim_ms - a.inicio_ms)} ms)`, a.nome));
    sel.disabled = !!f.acoes_descartadas;
    sel.addEventListener("change", () => carregarGlb(tela, caminho, sel.value || null, baixar));
    caixa.appendChild(sel);
    ficha.insertBefore(caixa, baixar.nextSibling);
  }
  await carregarGlb(tela, caminho, null, baixar);
}

async function carregarGlb(tela, caminho, acao, link) {
  const q = acao ? { caminho, acao } : { caminho };
  if (link) link.href = rotaDeModelos("/glb", q);
  try {
    const resp = await fetch(rotaDeModelos("/glb", q), { credentials: "same-origin" });
    if (!resp.ok) throw new Error(`GLB: HTTP ${resp.status}${resp.status === 400 ? " — " + ((await resp.json().catch(() => ({}))).detail || "") : ""}`);
    desenharGlb(tela, await resp.arrayBuffer());
  } catch (erro) { alerta("erro", "Estúdio", erro.message); }
}

/* ---- visualizador WebGL mínimo: GLB → buffers → Lambert + órbita; esqueleto e animação na CPU (B276) ---- */

function lerGlb(buf) {
  const dv = new DataView(buf);
  if (dv.getUint32(0, true) !== 0x46546c67) throw new Error("não é GLB");
  const nj = dv.getUint32(12, true);
  const doc = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 20, nj)));
  const bin = 20 + nj + 8;
  const acessor = (i) => {
    const a = doc.accessors[i], v = doc.bufferViews[a.bufferView];
    const n = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }[a.type] * a.count;
    if (a.componentType === 5126) return new Float32Array(buf.slice(bin + v.byteOffset, bin + v.byteOffset + 4 * n));
    return new Uint16Array(buf.slice(bin + v.byteOffset, bin + v.byteOffset + 2 * n));
  };
  const imagens = (doc.images || []).map((im) => { const v = doc.bufferViews[im.bufferView]; return new Blob([new Uint8Array(buf, bin + v.byteOffset, v.byteLength)], { type: im.mimeType }); });
  const malhas = [];
  doc.nodes.forEach((no) => {
    if (no.mesh === undefined) return;
    for (const p of doc.meshes[no.mesh].primitives) {
      const mat = p.material !== undefined ? doc.materials[p.material] : null;
      const tex = mat?.pbrMetallicRoughness?.baseColorTexture;
      malhas.push({ pos: acessor(p.attributes.POSITION), nor: acessor(p.attributes.NORMAL), uv: acessor(p.attributes.TEXCOORD_0), idx: acessor(p.indices),
        juntas: p.attributes.JOINTS_0 !== undefined && no.skin !== undefined ? acessor(p.attributes.JOINTS_0) : null,
        pesos: p.attributes.WEIGHTS_0 !== undefined && no.skin !== undefined ? acessor(p.attributes.WEIGHTS_0) : null,
        cor: mat ? mat.pbrMetallicRoughness.baseColorFactor : [0.75, 0.75, 0.75, 1], min: doc.accessors[p.attributes.POSITION].min, max: doc.accessors[p.attributes.POSITION].max,
        imagem: tex ? imagens[doc.textures[tex.index].source] : null, corte: mat?.alphaMode === "MASK" ? (mat.alphaCutoff ?? 0.5) : -1 });
    }
  });
  // Esqueleto e animação (coluna em ordem de coluna, como o glTF).
  const skin = (doc.skins || [])[0];
  let esqueleto = null;
  if (skin) {
    const mae = new Array(doc.nodes.length).fill(-1);
    doc.nodes.forEach((n, i) => (n.children || []).forEach((c) => { mae[c] = i; }));
    const local = doc.nodes.map((n) => n.matrix ? Float32Array.from(n.matrix) : Float32Array.from([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]));
    const anim = (doc.animations || [])[0];
    const canais = anim ? anim.channels.map((c) => ({ no: c.target.node, caminho: c.target.path, t: acessor(anim.samplers[c.sampler].input), v: acessor(anim.samplers[c.sampler].output) })) : [];
    const duracao = canais.reduce((d, c) => Math.max(d, c.t[c.t.length - 1] || 0), 0);
    esqueleto = { mae, local, juntas: skin.joints, ibm: acessor(skin.inverseBindMatrices), canais, duracao, nome: anim?.name };
  }
  return { malhas, esqueleto };
}

function multiplicar4(a, b) {
  const r = new Float32Array(16);
  for (let c = 0; c < 4; c++) for (let l = 0; l < 4; l++) { let s = 0; for (let k = 0; k < 4; k++) s += a[k * 4 + l] * b[c * 4 + k]; r[c * 4 + l] = s; }
  return r;
}

/* Matrizes das juntas no tempo t (s): locais animados (translação + rotação, linear entre amostras), mundo, × inversa. */
function matrizesDasJuntas(e, t) {
  const local = e.local.map((m) => m);
  const trs = new Map();
  for (const c of e.canais) {
    const n = c.t.length; let i = 0;
    while (i < n - 1 && c.t[i + 1] < t) i++;
    const j = Math.min(i + 1, n - 1); const f = j > i && c.t[j] > c.t[i] ? Math.min(1, Math.max(0, (t - c.t[i]) / (c.t[j] - c.t[i]))) : 0;
    const d = c.caminho === "rotation" ? 4 : 3;
    const v = [];
    let sinal = 1;
    if (d === 4) { let p = 0; for (let k = 0; k < 4; k++) p += c.v[i * 4 + k] * c.v[j * 4 + k]; if (p < 0) sinal = -1; }
    for (let k = 0; k < d; k++) v.push(c.v[i * d + k] * (1 - f) + c.v[j * d + k] * sinal * f);
    if (d === 4) { const l = Math.hypot(...v) || 1; for (let k = 0; k < 4; k++) v[k] /= l; }
    const x = trs.get(c.no) || { t: null, r: null };
    if (d === 4) x.r = v; else x.t = v;
    trs.set(c.no, x);
  }
  for (const [no, { t: tr, r }] of trs) {
    const m = Float32Array.from(e.local[no]);
    if (r) {
      const [x, y, z, w] = r;
      m.set([1 - 2 * (y * y + z * z), 2 * (x * y + z * w), 2 * (x * z - y * w), 0, 2 * (x * y - z * w), 1 - 2 * (x * x + z * z), 2 * (y * z + x * w), 0,
        2 * (x * z + y * w), 2 * (y * z - x * w), 1 - 2 * (x * x + y * y), 0], 0);
    }
    if (tr) m.set(tr, 12);
    local[no] = m;
  }
  const mundo = new Array(local.length).fill(null);
  const calc = (i) => { if (mundo[i]) return mundo[i]; mundo[i] = e.mae[i] >= 0 ? multiplicar4(calc(e.mae[i]), local[i]) : local[i]; return mundo[i]; };
  const juntas = e.juntas.map((j, k) => multiplicar4(calc(j), e.ibm.subarray(16 * k, 16 * k + 16)));
  juntas.mundo = e.juntas.map((j) => calc(j));
  return juntas;
}

/* Linhas do esqueleto (B278): de cada junta à junta-mãe, nas posições do mundo do tempo t. */
function linhasDoEsqueleto(e, t) {
  const J = matrizesDasJuntas(e, t);
  const indice = new Map(e.juntas.map((j, k) => [j, k]));
  const v = [];
  e.juntas.forEach((j, k) => {
    const m = e.mae[j];
    if (m < 0 || !indice.has(m)) return;
    const a = J.mundo[k], b = J.mundo[indice.get(m)];
    v.push(a[12], a[13], a[14], b[12], b[13], b[14]);
  });
  return new Float32Array(v);
}

let visorAtual = null;

function desenharGlb(tela, buf) {
  if (visorAtual) { visorAtual.parar = true; visorAtual = null; }
  const gl = tela.getContext("webgl");
  if (!gl) { tela.replaceWith(criar("div", "texto-suave", "WebGL indisponível neste navegador.")); return; }
  const { malhas, esqueleto } = lerGlb(buf);
  const sh = (tipo, fonte) => { const s = gl.createShader(tipo); gl.shaderSource(s, fonte); gl.compileShader(s); return s; };
  const prog = gl.createProgram();
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, "attribute vec3 p; attribute vec3 n; attribute vec2 t; uniform mat4 mvp; uniform mat4 mdl; varying vec3 vn; varying vec2 vt;" +
    "void main(){ vn = mat3(mdl) * n; vt = t; gl_Position = mvp * vec4(p, 1.0); }"));
  gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, "precision mediump float; varying vec3 vn; varying vec2 vt; uniform vec4 cor; uniform sampler2D img; uniform float usa; uniform float corte;" +
    "void main(){ vec4 c = usa > 0.5 ? texture2D(img, vt) : vec4(1.0); if (corte >= 0.0 && c.a < corte) discard;" +
    " float l = 0.35 + 0.65 * max(dot(normalize(vn), normalize(vec3(0.4, 0.8, 0.5))), 0.0); gl_FragColor = vec4(cor.rgb * c.rgb * l, 1.0); }"));
  gl.linkProgram(prog);
  gl.useProgram(prog);
  const loc = { p: gl.getAttribLocation(prog, "p"), n: gl.getAttribLocation(prog, "n"), t: gl.getAttribLocation(prog, "t"), mvp: gl.getUniformLocation(prog, "mvp"), mdl: gl.getUniformLocation(prog, "mdl"),
    cor: gl.getUniformLocation(prog, "cor"), usa: gl.getUniformLocation(prog, "usa"), corte: gl.getUniformLocation(prog, "corte") };
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity];
  let desenhar = () => {};
  const objs = malhas.map((m) => {
    for (let k = 0; k < 3; k++) { min[k] = Math.min(min[k], m.min[k]); max[k] = Math.max(max[k], m.max[k]); }
    const b = (alvo, dados, uso) => { const x = gl.createBuffer(); gl.bindBuffer(alvo, x); gl.bufferData(alvo, dados, uso || gl.STATIC_DRAW); return x; };
    const animada = esqueleto && esqueleto.canais.length && m.juntas;
    const o = { m, pos: b(gl.ARRAY_BUFFER, m.pos, animada ? gl.DYNAMIC_DRAW : null), nor: b(gl.ARRAY_BUFFER, m.nor, animada ? gl.DYNAMIC_DRAW : null), uv: b(gl.ARRAY_BUFFER, m.uv),
      idx: b(gl.ELEMENT_ARRAY_BUFFER, m.idx), n: m.idx.length, cor: m.cor, tex: null, corte: m.corte, animada,
      p2: animada ? new Float32Array(m.pos.length) : null, n2: animada ? new Float32Array(m.nor.length) : null };
    if (m.imagem) createImageBitmap(m.imagem).then((bmp) => {
      const tx = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tx);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bmp);
      const pot = (x) => (x & (x - 1)) === 0;
      if (pot(bmp.width) && pot(bmp.height)) { gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
      else { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); }
      o.tex = tx; desenhar();
    }).catch(() => {});
    return o;
  });
  // B278: sem malha (pele ausente no cliente, ou corpo cuja pele vem do equipamento) — desenha o esqueleto.
  let linhas = null;
  if (!objs.length && esqueleto) {
    const v0 = linhasDoEsqueleto(esqueleto, 0);
    for (let i = 0; i < v0.length; i += 3) for (let k = 0; k < 3; k++) { min[k] = Math.min(min[k], v0[i + k]); max[k] = Math.max(max[k], v0[i + k]); }
    linhas = { buf: gl.createBuffer(), n: v0.length / 3 };
    gl.bindBuffer(gl.ARRAY_BUFFER, linhas.buf); gl.bufferData(gl.ARRAY_BUFFER, v0, gl.DYNAMIC_DRAW);
  }
  const centro = [0, 1, 2].map((k) => (min[k] + max[k]) / 2);
  const raio = Math.max(1e-3, Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2);
  const estado = { yaw: 0.6, pitch: 0.3, dist: raio * 2.8, t0: performance.now(), parar: false };
  visorAtual = estado;
  // Pele na CPU: v' = Σ peso_k · (junta_k · v); a normal pela parte 3×3.
  const deformar = (t) => {
    if (linhas) { gl.bindBuffer(gl.ARRAY_BUFFER, linhas.buf); gl.bufferSubData(gl.ARRAY_BUFFER, 0, linhasDoEsqueleto(esqueleto, t)); return; }
    const J = matrizesDasJuntas(esqueleto, t);
    for (const o of objs) {
      if (!o.animada) continue;
      const { pos, nor, juntas, pesos } = o.m;
      for (let v = 0; v < pos.length / 3; v++) {
        let px = 0, py = 0, pz = 0, nx = 0, ny = 0, nz = 0;
        const x = pos[3 * v], y = pos[3 * v + 1], z = pos[3 * v + 2], a = nor[3 * v], bb = nor[3 * v + 1], c = nor[3 * v + 2];
        for (let k = 0; k < 4; k++) {
          const w = pesos[4 * v + k]; if (!w) continue;
          const M = J[juntas[4 * v + k]];
          px += w * (M[0] * x + M[4] * y + M[8] * z + M[12]); py += w * (M[1] * x + M[5] * y + M[9] * z + M[13]); pz += w * (M[2] * x + M[6] * y + M[10] * z + M[14]);
          nx += w * (M[0] * a + M[4] * bb + M[8] * c); ny += w * (M[1] * a + M[5] * bb + M[9] * c); nz += w * (M[2] * a + M[6] * bb + M[10] * c);
        }
        o.p2[3 * v] = px; o.p2[3 * v + 1] = py; o.p2[3 * v + 2] = pz; o.n2[3 * v] = nx; o.n2[3 * v + 1] = ny; o.n2[3 * v + 2] = nz;
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, o.pos); gl.bufferSubData(gl.ARRAY_BUFFER, 0, o.p2);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.nor); gl.bufferSubData(gl.ARRAY_BUFFER, 0, o.n2);
    }
  };
  desenhar = () => {
    gl.viewport(0, 0, tela.width, tela.height);
    gl.clearColor(0.125, 0.14, 0.165, 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT); gl.enable(gl.DEPTH_TEST);
    const f = 1 / Math.tan(Math.PI / 8), a = tela.width / tela.height, pe = raio / 50, lo = raio * 50;
    const proj = new Float32Array([f / a, 0, 0, 0, 0, f, 0, 0, 0, 0, (lo + pe) / (pe - lo), -1, 0, 0, (2 * lo * pe) / (pe - lo), 0]);
    const cy = Math.cos(estado.yaw), sy = Math.sin(estado.yaw), cp = Math.cos(estado.pitch), spp = Math.sin(estado.pitch);
    const rot = new Float32Array([cy, spp * sy, -cp * sy, 0, 0, cp, spp, 0, sy, -spp * cy, cp * cy, 0, 0, 0, 0, 1]);
    const tr = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -centro[0], -centro[1], -centro[2], 1]);
    const cam = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, -estado.dist, 1]);
    const mvp = multiplicar4(proj, multiplicar4(cam, multiplicar4(rot, tr)));
    gl.uniformMatrix4fv(loc.mvp, false, mvp); gl.uniformMatrix4fv(loc.mdl, false, rot);
    for (const o of objs) {
      gl.bindBuffer(gl.ARRAY_BUFFER, o.pos); gl.enableVertexAttribArray(loc.p); gl.vertexAttribPointer(loc.p, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.nor); gl.enableVertexAttribArray(loc.n); gl.vertexAttribPointer(loc.n, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.uv); gl.enableVertexAttribArray(loc.t); gl.vertexAttribPointer(loc.t, 2, gl.FLOAT, false, 0, 0);
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, o.tex); gl.uniform1f(loc.usa, o.tex ? 1 : 0); gl.uniform1f(loc.corte, o.corte);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, o.idx); gl.uniform4fv(loc.cor, o.cor);
      gl.drawElements(gl.TRIANGLES, o.n, gl.UNSIGNED_SHORT, 0);
    }
    if (linhas) {
      gl.bindBuffer(gl.ARRAY_BUFFER, linhas.buf); gl.enableVertexAttribArray(loc.p); gl.vertexAttribPointer(loc.p, 3, gl.FLOAT, false, 0, 0);
      gl.disableVertexAttribArray(loc.n); gl.vertexAttrib3f(loc.n, 0.4, 0.8, 0.5);
      gl.disableVertexAttribArray(loc.t); gl.vertexAttrib2f(loc.t, 0, 0);
      gl.uniform1f(loc.usa, 0); gl.uniform1f(loc.corte, -1); gl.uniform4fv(loc.cor, [1.0, 0.85, 0.3, 1]);
      gl.drawArrays(gl.LINES, 0, linhas.n);
    }
  };
  let arrasto = null;
  tela.onmousedown = (e) => { arrasto = [e.clientX, e.clientY]; };
  window.addEventListener("mouseup", () => { arrasto = null; });
  tela.onmousemove = (e) => {
    if (!arrasto) return;
    estado.yaw += (e.clientX - arrasto[0]) * 0.01; estado.pitch = Math.max(-1.5, Math.min(1.5, estado.pitch + (e.clientY - arrasto[1]) * 0.01));
    arrasto = [e.clientX, e.clientY]; desenhar();
  };
  tela.onwheel = (e) => { e.preventDefault(); estado.dist *= e.deltaY > 0 ? 1.1 : 0.9; desenhar(); };
  if (!objs.length && !linhas) { tela.replaceWith(criar("div", "texto-suave", "Nada a desenhar: sem malha e sem esqueleto.")); return; }
  if (esqueleto && esqueleto.canais.length && esqueleto.duracao > 0) {
    // Laço da animação (repete).
    const quadro = () => {
      if (estado.parar || !tela.isConnected) return;
      const t = ((performance.now() - estado.t0) / 1000) % esqueleto.duracao;
      deformar(t); desenhar();
      requestAnimationFrame(quadro);
    };
    requestAnimationFrame(quadro);
  } else {
    desenhar();
  }
}

(function ligarModelos() {
  elemento("estudio-vista-modelos").addEventListener("click", () => mostrarVistaDoEstudio("modelos"));
  let espera = null;
  elemento("modelos-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(carregarModelos, 300); });
})();
