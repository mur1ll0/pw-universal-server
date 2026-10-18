/* Estúdio, fase S7 parte 2 (B255): navegador de missões (`tasks.data` do realm, só leitura). Os campos são os do
   `TaskTemplate` do leitor do GS (`pw-data-loader/src/tasks.rs`); NPCs, itens e monstros citados vêm com o nome do
   elements e link para a ficha. Usa `api`, `criar`, `elemento` do painel.js e `estudio`, `mostrarVistaDoEstudio`,
   `abrirRegistroDoEstudio`. */

const missoes = { pagina: 0, por_pagina: 100, total: 0, aberta: null };

function rotaDeMissoes(resto, q = {}) {
  const p = new URLSearchParams(q).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/missoes${resto}${p ? `?${p}` : ""}`;
}

async function carregarMissoes() {
  if (!estudio.realm) return;
  const busca = elemento("missoes-busca").value.trim();
  elemento("missoes-situacao").textContent = "Carregando (a primeira vez lê o tasks.data inteiro)…";
  let r;
  try {
    r = await api(rotaDeMissoes("", { busca, topo: elemento("missoes-topo").checked ? "true" : "false", pagina: missoes.pagina, por_pagina: missoes.por_pagina }));
  } catch (erro) { elemento("missoes-situacao").textContent = erro.message; return; }
  missoes.total = r.total;
  elemento("missoes-situacao").textContent = `${r.missoes} missões (${r.de_topo} de topo) · versão ${r.versao_do_arquivo} · ${r.total} na lista · página ${r.pagina + 1} de ${Math.max(1, Math.ceil(r.total / missoes.por_pagina))}`;
  const area = elemento("missoes-lista");
  area.replaceChildren();
  for (const [id, nome, tipo, nmin, nmax, mae, filhas] of r.linhas) {
    const b = criar("button", "", `${id} · ${nome || "(sem nome)"} · nível ${nmin}–${nmax}${filhas ? ` · ${filhas} filhas` : ""}${mae ? ` · mãe ${mae}` : ""}`);
    b.type = "button";
    b.setAttribute("aria-current", missoes.aberta === id ? "true" : "false");
    b.addEventListener("click", () => abrirMissao(id));
    area.appendChild(b);
  }
  if (!r.linhas.length) area.appendChild(criar("div", "texto-suave", "Nenhuma missão."));
}

function linkDeMissao(m) {
  const a = criar("a", "", `${m.id} ${m.nome || "(não existe)"}`);
  a.href = "#";
  a.addEventListener("click", (e) => { e.preventDefault(); abrirMissao(m.id); });
  return a;
}

function valorComNome(v, nomes) {
  const n = nomes[String(v)];
  if (!n) return criar("span", "", String(v));
  const a = criar("a", "", `${v} (${n.nome})`);
  a.href = "#";
  a.title = `${n.tabela} #${n.indice}`;
  a.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio(n.tabela, n.indice); });
  return a;
}

/* Desenha um valor da ficha: objetos como lista de chave/valor, listas de objetos como tabela, ids com nome. */
function desenharValor(v, chave, nomes) {
  if (v === null || v === undefined) return criar("span", "texto-suave", "—");
  if (Array.isArray(v)) {
    if (!v.length) return criar("span", "texto-suave", "(vazio)");
    if (typeof v[0] === "object" && v[0] !== null && !Array.isArray(v[0])) {
      const t = criar("table", "tabela-simples");
      const cab = criar("tr");
      Object.keys(v[0]).forEach((k) => cab.appendChild(criar("th", "", k)));
      t.appendChild(cab);
      for (const linha of v) {
        const tr = criar("tr");
        Object.entries(linha).forEach(([k, x]) => { const td = criar("td"); td.appendChild(desenharValor(x, k, nomes)); tr.appendChild(td); });
        t.appendChild(tr);
      }
      return t;
    }
    const s = criar("span");
    v.forEach((x, i) => { if (i) s.append(", "); s.appendChild(desenharValor(x, chave, nomes)); });
    return s;
  }
  if (typeof v === "object") {
    const d = criar("div", "missao-objeto");
    for (const [k, x] of Object.entries(v)) {
      const linha = criar("div");
      linha.append(criar("strong", "", `${k}: `), desenharValor(x, k, nomes));
      d.appendChild(linha);
    }
    return d;
  }
  const k = (chave || "").toLowerCase();
  if (typeof v === "number" && v > 0 && (k === "id" || /npc|item|monstro|brinde/.test(k)) && !/quantidade/.test(k)) return valorComNome(v, nomes);
  return criar("span", "", typeof v === "boolean" ? (v ? "sim" : "não") : String(v));
}

async function abrirMissao(id) {
  missoes.aberta = id;
  const area = elemento("missoes-ficha");
  area.textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeMissoes(`/${id}`)); } catch (erro) { area.textContent = erro.message; return; }
  area.replaceChildren();
  const f = r.ficha;
  area.appendChild(criar("h3", "", `${f.id} · ${f.name || "(sem nome)"}`));
  const rel = criar("div", "texto-suave");
  if (r.mae) { rel.append("Mãe: "); rel.appendChild(linkDeMissao(r.mae)); rel.append(" · "); }
  if (r.filhas.length) { rel.append("Filhas: "); r.filhas.forEach((m, i) => { if (i) rel.append(", "); rel.appendChild(linkDeMissao(m)); }); rel.append(" · "); }
  if (r.pre_missoes.length) { rel.append("Pré-missões: "); r.pre_missoes.forEach((m, i) => { if (i) rel.append(", "); rel.appendChild(linkDeMissao(m)); }); rel.append(" · "); }
  if (r.nova_missao) { rel.append("Ao concluir, dá: "); rel.appendChild(linkDeMissao(r.nova_missao)); }
  area.appendChild(rel);
  if (f.descricao) area.appendChild(criar("p", "", f.descricao));
  const ordem = [
    ["Quem dá e quem premia", ["npc_que_entrega", "npc_que_premia", "entrega_automatica", "tipo", "metodo", "tipo_de_conclusao"]],
    ["Requisitos", ["min_level", "max_level", "req_classes", "genero", "pre_tasks", "missoes_exclusivas", "itens_exigidos", "dinheiro_pedido", "frequencia", "limite_por_periodo", "limite_de_tempo", "espera", "max_receptores"]],
    ["Objetivo", ["monster_kills", "item_collections", "nivel_a_alcancar", "mundo_a_alcancar"]],
    ["Ao receber", ["itens_entregues"]],
    ["Prêmio", ["rewards"]],
    ["Prêmio de falha", ["premio_de_falha"]],
  ];
  const usados = new Set(["id", "name", "descricao", "parent", "sub_tasks"]);
  for (const [titulo, chaves] of ordem) {
    const fs = criar("fieldset");
    fs.appendChild(criar("legend", "", titulo));
    for (const k of chaves) {
      if (!(k in f)) continue;
      usados.add(k);
      const linha = criar("div");
      linha.append(criar("strong", "", `${k}: `), desenharValor(f[k], k, r.nomes));
      fs.appendChild(linha);
    }
    area.appendChild(fs);
  }
  const resto = criar("details");
  resto.appendChild(criar("summary", "", "Demais campos"));
  for (const [k, v] of Object.entries(f)) {
    if (usados.has(k)) continue;
    const linha = criar("div");
    linha.append(criar("strong", "", `${k}: `), desenharValor(v, k, r.nomes));
    resto.appendChild(linha);
  }
  area.appendChild(resto);
  area.appendChild(criar("p", "texto-suave", "Só leitura nesta fase: a edição de missões precisa de um gravador do tasks.data com ida e volta idêntica."));
  document.querySelectorAll("#missoes-lista button").forEach((b) => b.setAttribute("aria-current", b.textContent.startsWith(`${id} `) ? "true" : "false"));
}

(function ligarMissoes() {
  elemento("estudio-vista-missoes").addEventListener("click", () => mostrarVistaDoEstudio("missoes"));
  let espera = null;
  elemento("missoes-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(() => { missoes.pagina = 0; carregarMissoes(); }, 300); });
  elemento("missoes-topo").addEventListener("change", () => { missoes.pagina = 0; carregarMissoes(); });
  elemento("missoes-anterior").addEventListener("click", () => { if (missoes.pagina > 0) { missoes.pagina--; carregarMissoes(); } });
  elemento("missoes-proxima").addEventListener("click", () => { if ((missoes.pagina + 1) * missoes.por_pagina < missoes.total) { missoes.pagina++; carregarMissoes(); } });
})();
