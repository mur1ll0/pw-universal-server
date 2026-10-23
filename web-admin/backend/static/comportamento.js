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
  if (estudio.rascunho) area.appendChild(criarGatilhoDoZero(pol, r.tipos || { condicoes: [], operacoes: [] }));
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
    fs.appendChild(editorDoGatilho(pol, t, ti, (r.editaveis || [])[ti] || { condicoes: {}, operacoes: {} }, r.tipos || { condicoes: [], operacoes: [] }));
    area.appendChild(fs);
  });
}

/* Edição no lugar (B259): flags do gatilho e os parâmetros de tamanho fixo de cada operação e condição. */
function editorDoGatilho(pol, t, ti, ed, tipos) {
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
  // B365: nome do gatilho (rótulo do editor, GBK; o GS não usa).
  const ln = criar("div", "recursos-linha");
  const inome = criar("input", "campo");
  inome.value = t.nome || "";
  inome.maxLength = 63;
  const bnome = criar("button", "botao botao-pequeno botao-secundario", "Renomear");
  bnome.type = "button";
  bnome.addEventListener("click", () => {
    const v = inome.value.trim();
    if (!v || v === (t.nome || "")) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
    salvar({ nome: v }, "Nome do gatilho salvo no rascunho.");
  });
  ln.append("Nome: ", inome, bnome);
  caixa.appendChild(ln);
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
  // Falas (B261): o texto muda de tamanho; o gravador mantém o NUL final do original.
  t.operacoes.forEach((o, oi) => {
    const p = o.parametro && typeof o.parametro === "object" ? o.parametro : {};
    const fala = p.Fala || p.Fala2 || p.SkillComFala;
    if (!fala) return;
    const l = criar("div", "recursos-linha");
    l.appendChild(criar("span", "", `Fala da operação ${oi + 1}:`));
    const i = criar("input", "campo");
    i.value = fala.texto || "";
    i.style.flex = "1";
    const b = criar("button", "botao botao-pequeno botao-secundario", "Salvar fala");
    b.type = "button";
    b.addEventListener("click", () => {
      if (i.value === fala.texto) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
      salvar({ fala: { indice: oi, texto: i.value } }, "Fala salva no rascunho.");
    });
    l.append(i, b);
    caixa.appendChild(l);
  });
  // Tipos (B271): trocar o tipo de uma operação ou de uma folha de condição (mesmo papel), acrescentar e remover
  // operação. Só os tipos que este arquivo já usa; rodar/parar/ativar gatilho pedem o id de um gatilho da política.
  const seletor = (lista, atual) => {
    const s = criar("select", "campo");
    for (const [n, nome] of lista) { const o = new Option(`${nome} (${n})`, String(n)); o.selected = n === atual; s.appendChild(o); }
    return s;
  };
  const pedirGatilho = (tipo) => {
    if (tipo < 4 || tipo > 6) return undefined;
    const v = prompt(`Id do gatilho (desta política: ${pol.triggers.map((x) => x.id).join(", ")}):`);
    return v ? Math.trunc(Number(v)) : null;
  };
  t.operacoes.forEach((o, oi) => {
    const l = criar("div", "recursos-linha");
    const s = seletor(tipos.operacoes, o.tipo_cru);
    const b = criar("button", "botao botao-pequeno botao-secundario", "Trocar tipo");
    b.type = "button";
    b.addEventListener("click", () => {
      const tipo = Number(s.value); const g = pedirGatilho(tipo); if (g === null) return;
      salvar({ tipo_da_operacao: { indice: oi, tipo, id_gatilho: g } }, "Tipo da operação trocado no rascunho (parâmetro zerado: ajuste os campos).");
    });
    const rm = criar("button", "botao botao-pequeno botao-perigo", "Remover");
    rm.type = "button";
    rm.disabled = t.operacoes.length <= 1;
    rm.addEventListener("click", () => { if (confirm("Remover esta operação?")) salvar({ remover_operacao: oi }, "Operação removida no rascunho."); });
    l.append(`Operação ${oi + 1}: `, s, b, rm);
    caixa.appendChild(l);
  });
  const nova = criar("div", "recursos-linha");
  const sn = seletor(tipos.operacoes, -1);
  const bn = criar("button", "botao botao-pequeno botao-secundario", "+ Operação");
  bn.type = "button";
  bn.addEventListener("click", () => {
    const tipo = Number(sn.value); const g = pedirGatilho(tipo); if (g === null) return;
    salvar({ operacao_nova: { tipo, id_gatilho: g } }, "Operação acrescentada no rascunho.");
  });
  nova.append("Nova operação: ", sn, bn);
  caixa.appendChild(nova);
  const folhas = [];
  const andar = (n, caminho, pai) => {
    if (!n) return;
    if (!n.esquerda && !n.direita) folhas.push([caminho, n, pai !== null && pai >= 9 && pai <= 15]);
    andar(n.esquerda, caminho + "e", n.tipo_cru); andar(n.direita, caminho + "d", n.tipo_cru);
  };
  andar(t.condicao, "", null);
  for (const [caminho, n, valor] of folhas) {
    if ((n.tipo_cru >= 5 && n.tipo_cru <= 7) || (n.tipo_cru >= 9 && n.tipo_cru <= 15)) continue;
    const l = criar("div", "recursos-linha");
    const s = seletor(tipos.condicoes.filter(([, , v]) => v === valor).map(([x, nome]) => [x, nome]), n.tipo_cru);
    const b = criar("button", "botao botao-pequeno botao-secundario", "Trocar tipo");
    b.type = "button";
    b.addEventListener("click", () => salvar({ tipo_da_condicao: { caminho, tipo: Number(s.value) } }, "Tipo da condição trocado no rascunho (parâmetro zerado: ajuste os campos)."));
    l.append(`Condição ${caminho || "raiz"} (${valor ? "valor" : "condição"}): `, s, b);
    caixa.appendChild(l);
  }
  // Árvore (B365): pôr Não/E/Ou em volta de um nó de condição e tirar nó (Não sai deixando o filho; filho de E/Ou
  // sai e o operador vira o irmão). Gravado como o SaveConditonTree do original.
  const nos = [];
  const todos = (n, caminho, debaixoDeValor) => {
    if (!n) return;
    nos.push([caminho, n, debaixoDeValor]);
    const v = debaixoDeValor || (n.tipo_cru >= 9 && n.tipo_cru <= 15);
    todos(n.esquerda, caminho + "e", v); todos(n.direita, caminho + "d", v);
  };
  todos(t.condicao, "", false);
  const condicoesDeFolha = tipos.condicoes.filter(([, , v]) => !v).map(([x, nome]) => [x, nome]);
  for (const [caminho, n, valor] of nos) {
    if (valor) continue;
    const l = criar("div", "recursos-linha");
    const sop = criar("select", "campo");
    for (const [v, nome] of [[5, "Não"], [6, "Ou"], [7, "E"]]) sop.appendChild(new Option(nome, String(v)));
    const sfolha = seletor(condicoesDeFolha, -1);
    const env = criar("button", "botao botao-pequeno botao-secundario", "Envolver");
    env.type = "button";
    env.title = "Não: o nó vira o operando. E/Ou: o nó fica à esquerda e a condição escolhida à direita.";
    env.addEventListener("click", () => {
      const operador = Number(sop.value);
      salvar({ envolver: { caminho, operador, folha: operador === 5 ? undefined : Number(sfolha.value) } }, "Condição envolvida no rascunho.");
    });
    const tirar = criar("button", "botao botao-pequeno botao-perigo", n.tipo_cru === 5 ? "Tirar o Não" : "Tirar nó");
    tirar.type = "button";
    const pai = caminho ? nos.find(([c]) => c === caminho.slice(0, -1))?.[1] : null;
    tirar.disabled = !(n.tipo_cru === 5 || (pai && (pai.tipo_cru === 6 || pai.tipo_cru === 7)));
    tirar.addEventListener("click", () => { if (confirm(`Tirar a condição ${caminho || "raiz"}?`)) salvar({ remover_condicao: caminho }, "Condição tirada no rascunho."); });
    l.append(`Nó ${caminho || "raiz"} (${n.tipo || n.tipo_cru}): `, sop, " nova condição (E/Ou): ", sfolha, env, tirar);
    caixa.appendChild(l);
  }
  // Estrutura (B261): duplicar (cópia logo depois, id novo) e remover (recusado se outro gatilho o chama).
  const est = criar("div", "recursos-linha");
  const dup = criar("button", "botao botao-pequeno botao-secundario", "Duplicar gatilho");
  dup.type = "button";
  dup.addEventListener("click", () => salvar({ duplicar: {} }, "Gatilho duplicado no rascunho."));
  const rem = criar("button", "botao botao-pequeno botao-perigo", "Remover gatilho");
  rem.type = "button";
  rem.addEventListener("click", () => {
    if (!confirm(`Remover o gatilho ${t.id} desta política (no rascunho)?`)) return;
    salvar({ remover: true }, "Gatilho removido no rascunho.");
  });
  est.append(dup, rem);
  caixa.appendChild(est);
  caixa.appendChild(criar("div", "texto-suave", "Grava o aipolicy.data do realm (só do servidor); reinicie o realm depois de publicar."));
  return caixa;
}

/* Gatilho do zero (B365): nome, uma condição e uma operação (alvo 0); o resto se edita depois, como qualquer gatilho. */
function criarGatilhoDoZero(pol, tipos) {
  const caixa = criar("details");
  caixa.appendChild(criar("summary", "", "Novo gatilho (do zero)"));
  const l = criar("div", "recursos-linha");
  const nome = criar("input", "campo");
  nome.placeholder = "Nome";
  nome.maxLength = 63;
  const seletor = (lista) => { const s = criar("select", "campo"); for (const [n, x] of lista) s.appendChild(new Option(`${x} (${n})`, String(n))); return s; };
  const sc = seletor(tipos.condicoes.filter(([, , v]) => !v).map(([x, n]) => [x, n]));
  const so = seletor(tipos.operacoes);
  const b = criar("button", "botao botao-pequeno botao-primario", "Criar");
  b.type = "button";
  b.addEventListener("click", async () => {
    if (!nome.value.trim()) { alerta("aviso", "Estúdio", "Dê um nome ao gatilho.", { toast: true }); return; }
    const operacao = Number(so.value);
    let id_gatilho;
    if (operacao >= 4 && operacao <= 6) {
      const v = prompt(`Id do gatilho que a operação chama (desta política: ${pol.triggers.map((x) => x.id).join(", ")}):`);
      if (!v) return;
      id_gatilho = Math.trunc(Number(v));
    }
    try {
      const r = await api(rotaDeComportamento(`/${pol.id}`), { method: "PUT",
        body: JSON.stringify({ novo_gatilho: { nome: nome.value.trim(), condicao: Number(sc.value), operacao, id_gatilho } }) });
      alerta("ok", "Estúdio", `Gatilho ${r.resultado?.novo_gatilho ?? ""} criado no rascunho (ligado, só em combate; parâmetros zerados: ajuste).`, { toast: true });
      await abrirComportamento(pol.id);
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  l.append(nome, " quando ", sc, " faz ", so, b);
  caixa.appendChild(l);
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
