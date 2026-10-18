/* Estúdio, fase S7 (B260): tabelas de texto do cliente (`configs\fixed_msg.txt`, `item_desc.txt`, `item_ext_desc.txt`).
   Leitor e gravador: `pw-recursos/src/textos.rs` (porte de `CECStringTab::LoadWideStrings`). Item "cliente": vai no
   manifesto do cliente, não no realm. Usa `api`, `criar`, `elemento`, `alerta` e `estudio`. */

const textos = { pagina: 0, total: 0, porPagina: 100 };

function rotaDeTextos(resto, q = {}) {
  if (estudio.rascunho) q = { ...q, rascunho: estudio.rascunho };
  const p = new URLSearchParams(q).toString();
  const arquivo = elemento("textos-arquivo").value;
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/textos/${arquivo}${resto}${p ? `?${p}` : ""}`;
}

async function salvarTexto(metodo, resto, corpo, msg) {
  try {
    const r = await api(rotaDeTextos(resto), { method: metodo, body: JSON.stringify(corpo) });
    alerta("ok", "Estúdio", msg + (r.igual_ao_publicado ? " Ficou igual ao publicado: o item saiu do rascunho." : ""), { toast: true });
    await carregarTextos();
    if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
  } catch (erro) { alerta("erro", "Estúdio", erro.message); }
}

async function carregarTextos() {
  if (!estudio.realm) return;
  elemento("textos-situacao").textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeTextos("", { busca: elemento("textos-busca").value.trim(), pagina: textos.pagina })); }
  catch (erro) { elemento("textos-situacao").textContent = erro.message; return; }
  textos.total = r.total;
  const paginas = Math.max(1, Math.ceil(r.total / textos.porPagina));
  elemento("textos-situacao").textContent = `${r.entradas} entradas · ${r.indexada ? "indexada (id)" : "sequencial (posição)"} · ${r.total} na busca · página ${r.pagina + 1}/${paginas}${r.no_rascunho ? " · com alterações no rascunho" : ""}`;
  const novo = elemento("textos-novo");
  novo.replaceChildren();
  if (r.indexada && estudio.rascunho) {
    const id = criar("input", "campo");
    id.type = "number";
    id.placeholder = "id novo";
    id.style.maxWidth = "110px";
    const t = criar("input", "campo");
    t.placeholder = "texto da entrada nova";
    const b = criar("button", "botao botao-pequeno botao-secundario", "Acrescentar");
    b.type = "button";
    b.addEventListener("click", () => {
      if (!id.value || !t.value) { alerta("aviso", "Estúdio", "Informe o id e o texto.", { toast: true }); return; }
      salvarTexto("POST", "", { id: Math.trunc(Number(id.value)), texto: t.value }, "Entrada acrescentada no rascunho.");
    });
    novo.append(id, t, b);
  }
  const lista = elemento("textos-lista");
  lista.replaceChildren();
  for (const [chave, texto] of r.linhas) {
    const l = criar("div", "recursos-linha");
    l.appendChild(criar("strong", "", String(chave)));
    const i = criar("input", "campo");
    i.value = texto;
    i.style.flex = "1";
    i.disabled = !estudio.rascunho;
    i.title = estudio.rascunho ? "Enter salva no rascunho" : "Escolha um rascunho no topo do Estúdio para editar";
    i.addEventListener("keydown", (e) => {
      if (e.key !== "Enter" || i.value === texto) return;
      salvarTexto("PUT", `/${chave}`, { texto: i.value }, `Entrada ${chave} salva no rascunho.`);
    });
    l.appendChild(i);
    lista.appendChild(l);
  }
  if (!r.linhas.length) lista.appendChild(criar("div", "texto-suave", "Nada encontrado."));
  lista.appendChild(criar("div", "texto-suave", "Códigos de cor (^ffcb4a) e \\r ficam no texto como estão. Aspas e quebra de linha são recusadas: o leitor do cliente não tem escape."));
}

(function ligarTextos() {
  elemento("estudio-vista-textos").addEventListener("click", () => mostrarVistaDoEstudio("textos"));
  let espera = null;
  elemento("textos-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(() => { textos.pagina = 0; carregarTextos(); }, 300); });
  elemento("textos-arquivo").addEventListener("change", () => { textos.pagina = 0; carregarTextos(); });
  elemento("textos-anterior").addEventListener("click", () => { if (textos.pagina > 0) { textos.pagina--; carregarTextos(); } });
  elemento("textos-proxima").addEventListener("click", () => {
    if ((textos.pagina + 1) * textos.porPagina < textos.total) { textos.pagina++; carregarTextos(); }
  });
  elemento("estudio-rascunho").addEventListener("change", () => { if (!elemento("estudio-textos").hidden) carregarTextos(); });
})();
