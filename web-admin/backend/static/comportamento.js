/* Estúdio, fase S7 (B258): comportamento dos monstros (`aipolicy.data` do realm, só leitura). Os gatilhos, condições
   e operações são os do leitor do GS (`pw-data-loader/src/aipolicy.rs`, porte de `cgame/gs/ai/policy.cpp`), que o mundo
   interpreta desde o B127 (`pw_gs::politica`). Usa `api`, `criar`, `elemento` e `estudio`/`mostrarVistaDoEstudio`/
   `abrirRegistroDoEstudio`. */

function rotaDeComportamento(resto, q = {}) {
  if (estudio.rascunho) q = { ...q, rascunho: estudio.rascunho };
  const p = new URLSearchParams(q).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/comportamentos${resto}${p ? `?${p}` : ""}`;
}

async function carregarComportamentos() {
  if (!estudio.realm) return;
  elemento("comportamento-situacao").textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeComportamento("", { busca: elemento("comportamento-busca").value.trim() })); }
  catch (erro) { elemento("comportamento-situacao").textContent = erro.message; return; }
  elemento("comportamento-situacao").textContent = `${r.politicas} políticas · versão ${r.versao_do_arquivo} · ${r.total} na lista${r.total > 500 ? " (500 primeiras)" : ""}${r.sem_politica ? ` · ${r.sem_politica} ids citados por monstros sem política` : ""}`;
  const area = elemento("comportamento-lista");
  area.replaceChildren();
  for (const [id, gatilhos, monstros] of r.linhas) {
    const b = criar("button", "", `Política ${id} · ${gatilhos} gatilhos · ${monstros} monstros`);
    b.type = "button";
    b.addEventListener("click", () => abrirComportamento(id));
    area.appendChild(b);
  }
  if (!r.linhas.length) area.appendChild(criar("div", "texto-suave", "Nenhuma política."));
}

function parametroEmTexto(p) {
  if (p === null || p === undefined) return "";
  if (typeof p !== "object") return String(p);
  const partes = Object.entries(p).filter(([, v]) => v !== null && !(Array.isArray(v) && !v.length));
  return partes.map(([k, v]) => `${k} ${typeof v === "object" ? JSON.stringify(v) : v}`).join(", ");
}

/* Condição em árvore: nó com tipo (nome do enum do leitor, ou o número cru), parâmetro e filhos. */
function desenharCondicao(no) {
  if (!no) return criar("span", "texto-suave", "sempre");
  const nome = no.tipo || `tipo ${no.tipo_cru}`;
  const caixa = criar("div", "missao-objeto");
  const p = parametroEmTexto(no.parametro);
  caixa.appendChild(criar("div", "", `${nome}${p ? ` (${p})` : ""}`));
  if (no.esquerda) caixa.appendChild(desenharCondicao(no.esquerda));
  if (no.direita) caixa.appendChild(desenharCondicao(no.direita));
  return caixa;
}

async function abrirComportamento(id) {
  const area = elemento("comportamento-ficha");
  area.textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeComportamento(`/${id}`)); } catch (erro) { area.textContent = erro.message; return; }
  area.replaceChildren();
  const pol = r.politica;
  area.appendChild(criar("h3", "", `Política ${pol.id} (versão ${pol.versao})`));
  const usos = criar("div", "texto-suave");
  usos.append(r.monstros.length ? `Usada por ${r.monstros.length} monstro(s): ` : "Nenhum monstro do elements usa esta política.");
  r.monstros.slice(0, 40).forEach((m, k) => {
    const a = criar("a", "", `${k ? ", " : ""}${m.nome} (${m.id})`);
    a.href = "#";
    a.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio("MONSTER_ESSENCE", m.indice); });
    usos.appendChild(a);
  });
  if (r.monstros.length > 40) usos.append(` … e mais ${r.monstros.length - 40}.`);
  area.appendChild(usos);
  pol.triggers.forEach((t, ti) => {
    const fs = criar("fieldset");
    const marcas = [t.ativo ? "ligado de início" : "desligado", t.rodando ? "só quando chamado" : null, t.so_em_combate ? "só em combate" : null].filter(Boolean).join(" · ");
    fs.appendChild(criar("legend", "", `Gatilho ${t.id}${t.nome ? `: ${t.nome}` : ""} — ${marcas}`));
    const quando = criar("div");
    quando.append(criar("strong", "", "Quando: "), desenharCondicao(t.condicao));
    fs.appendChild(quando);
    const faz = criar("div");
    faz.appendChild(criar("strong", "", "Faz:"));
    const ol = criar("ol");
    for (const o of t.operacoes) {
      const alvo = o.alvo ? `${o.alvo.tipo || `alvo ${o.alvo.tipo_cru}`}` : "";
      const p = parametroEmTexto(o.parametro);
      ol.appendChild(criar("li", "", `${o.tipo || `operação ${o.tipo_cru}`}${p ? ` (${p})` : ""}${alvo ? ` → ${alvo}` : ""}`));
    }
    faz.appendChild(ol);
    fs.appendChild(faz);
    fs.appendChild(editorDoGatilho(pol, t, ti, (r.editaveis || [])[ti] || { condicoes: {}, operacoes: {} }));
    area.appendChild(fs);
  });
}

/* Edição no lugar (B259): flags do gatilho e os parâmetros de tamanho fixo de cada operação e condição. */
function editorDoGatilho(pol, t, ti, ed) {
  const caixa = criar("details");
  caixa.appendChild(criar("summary", "", estudio.rascunho ? "Editar este gatilho" : "Editar (escolha um rascunho no topo do Estúdio)"));
  if (!estudio.rascunho) return caixa;
  const salvar = async (corpo, msg) => {
    try {
      const r = await api(rotaDeComportamento(`/${pol.id}`), { method: "PUT", body: JSON.stringify({ gatilho: ti, ...corpo }) });
      alerta("ok", "Estúdio", msg + (r.igual_ao_publicado ? " Ficou igual ao realm: o item saiu do rascunho." : ""), { toast: true });
      await abrirComportamento(pol.id);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  };
  const flags = criar("div", "recursos-linha");
  const marcas = {};
  for (const [k, rot] of [["ativo", "Ligado de início"], ["rodando", "Só quando chamado"], ["so_em_combate", "Só em combate"]]) {
    const l = criar("label", "recursos-marca");
    const c = criar("input");
    c.type = "checkbox";
    c.checked = !!t[k];
    l.append(c, ` ${rot}`);
    flags.appendChild(l);
    marcas[k] = c;
  }
  const bf = criar("button", "botao botao-pequeno botao-secundario", "Salvar flags");
  bf.type = "button";
  bf.addEventListener("click", () => {
    const f = {};
    for (const [k, c] of Object.entries(marcas)) if (c.checked !== !!t[k]) f[k] = c.checked;
    if (!Object.keys(f).length) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
    salvar({ flags: f }, "Flags salvas no rascunho.");
  });
  flags.appendChild(bf);
  caixa.appendChild(flags);
  const linhaDeCampos = (rotulo, campos, atuais, aoSalvar) => {
    const l = criar("div", "recursos-linha");
    l.appendChild(criar("span", "", rotulo));
    const entradas = {};
    for (const [nome, tipo] of campos) {
      const i = criar("input", "campo");
      i.type = "number";
      i.step = tipo === "f32" ? "any" : "1";
      i.value = atuais[nome] ?? 0;
      i.title = nome;
      i.style.maxWidth = "110px";
      l.append(` ${nome} `, i);
      entradas[nome] = { i, tipo };
    }
    const b = criar("button", "botao botao-pequeno botao-secundario", "Salvar");
    b.type = "button";
    b.addEventListener("click", () => {
      const v = {};
      for (const [nome, { i, tipo }] of Object.entries(entradas)) {
        const x = tipo === "f32" ? Number(i.value) : Math.trunc(Number(i.value));
        if (x !== atuais[nome]) v[nome] = x;
      }
      if (!Object.keys(v).length) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
      aoSalvar(v);
    });
    l.appendChild(b);
    return l;
  };
  // Os valores atuais saem do parâmetro serializado: {"Variante": {campo: v}} ou {"Variante": v}.
  const valoresDe = (parametro, campos) => {
    const v = parametro && typeof parametro === "object" ? Object.values(parametro)[0] : null;
    if (v !== null && typeof v === "object") return v;
    return campos.length ? { [campos[0][0]]: v } : {};
  };
  for (const [oi, campos] of Object.entries(ed.operacoes || {})) {
    const o = t.operacoes[Number(oi)];
    caixa.appendChild(linhaDeCampos(`Operação ${Number(oi) + 1} (${o.tipo || o.tipo_cru}):`, campos, valoresDe(o.parametro, campos),
      (valores) => salvar({ operacao: { indice: Number(oi), valores } }, "Operação salva no rascunho.")));
  }
  const noDe = (caminho) => { let n = t.condicao; for (const c of caminho) n = c === "e" ? n?.esquerda : n?.direita; return n; };
  for (const [caminho, campos] of Object.entries(ed.condicoes || {})) {
    const n = noDe(caminho);
    caixa.appendChild(linhaDeCampos(`Condição ${caminho || "raiz"} (${n?.tipo || n?.tipo_cru}):`, campos, valoresDe(n?.parametro, campos),
      (valores) => salvar({ condicao: { caminho, valores } }, "Condição salva no rascunho.")));
  }
  caixa.appendChild(criar("div", "texto-suave", "Grava o aipolicy.data do realm (só do servidor). Só parâmetros de tamanho fixo; reinicie o realm depois de publicar."));
  return caixa;
}

(function ligarComportamento() {
  elemento("estudio-vista-comportamento").addEventListener("click", () => mostrarVistaDoEstudio("comportamento"));
  let espera = null;
  elemento("comportamento-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(carregarComportamentos, 300); });
  elemento("estudio-rascunho").addEventListener("change", () => {
    const h = elemento("comportamento-ficha").querySelector("h3");
    const m = h && h.textContent.match(/^Política (\d+)/);
    if (!elemento("estudio-comportamento").hidden && m) abrirComportamento(Number(m[1]));
  });
})();
