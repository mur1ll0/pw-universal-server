/* Estúdio, fase S7 parte 1 (B254): a Loja Gold (`data\gshop*.data`). O mesmo arquivo serve o cliente e o realm
   (o índice da oferta é o `goods_index` do MALL_SHOPPING), então a edição vira um item "ambos" no rascunho e o
   carimbo não muda. Campos de GSHOP_ITEM (`globaldataman.h:97-138`) pelos nomes do fonte.
   Usa `api`, `criar`, `elemento`, `alerta` do painel.js, `estudio` do estudio.js e `mostrarVistaDoEstudio`/
   `abrirRegistroDoEstudio`. */

const loja = { dados: null, filtro: "", categoria: "", aberta: null };

const ROTULOS_DA_LOJA = {
  local_id: "Id local", main_type: "Categoria", sub_type: "Subcategoria", icon: "Ícone (arquivo)", id: "Item (id)",
  num: "Quantidade", desc: "Descrição", szName: "Nome", status: "Situação", idGift: "Brinde (id)", iGiftNum: "Brinde: quantidade",
  iGiftTime: "Brinde: duração (s)", iLogPrice: "Preço no log", buy_times_limit: "Limite de compras", buy_times_limit_mode: "Período do limite",
};
const ROTULOS_DA_COMPRA = {
  price: "Preço", end_time: "Fim da venda", time: "Duração do item (s, 0 = sem)", start_time: "Início da venda",
  type: "Período (0 sem, 1 semana, 2 mês, -1 inválido)", day: "Dias (bits)", status: "Situação (0 normal, 1 novo, 2 promoção, 3 recomendado, 4-12 desconto, 13 relâmpago)",
  flag: "Controle", min_vip_level: "VIP mínimo",
};

function rotaDaLoja(resto = "", rascunho = estudio.rascunho) {
  const q = rascunho ? `?rascunho=${rascunho}` : "";
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/loja/${encodeURIComponent(elemento("loja-arquivo").value)}${resto}${q}`;
}

async function carregarLoja() {
  if (!estudio.realm) return;
  elemento("loja-situacao").textContent = "Carregando…";
  try { loja.dados = await api(rotaDaLoja()); }
  catch (erro) { loja.dados = null; elemento("loja-situacao").textContent = erro.message; elemento("loja-lista").replaceChildren(); return; }
  const d = loja.dados;
  elemento("loja-situacao").textContent = `${d.ofertas.length} ofertas · layout ${d.formato} · carimbo ${d.carimbo}${d.no_rascunho ? ` · com as mudanças do rascunho #${estudio.rascunho}` : ""}`;
  const sel = elemento("loja-categoria");
  const atual = sel.value;
  sel.replaceChildren(criar("option", "", "Todas as categorias"));
  sel.firstChild.value = "";
  d.categorias.forEach((c, i) => {
    const o = criar("option", "", c.nome || `Categoria ${i}`); o.value = `${i}`; sel.appendChild(o);
    c.subcategorias.forEach((s, k) => { const so = criar("option", "", `  · ${s || `sub ${k}`}`); so.value = `${i}:${k}`; sel.appendChild(so); });
  });
  sel.value = [...sel.options].some((o) => o.value === atual) ? atual : "";
  desenharListaDaLoja();
  if (loja.aberta !== null && loja.aberta < d.ofertas.length) abrirOferta(loja.aberta);
}

function desenharListaDaLoja() {
  const area = elemento("loja-lista");
  area.replaceChildren();
  if (!loja.dados) return;
  const t = elemento("loja-busca").value.trim().toLowerCase();
  const [cat, sub] = elemento("loja-categoria").value.split(":");
  let n = 0;
  for (const o of loja.dados.ofertas) {
    const [i, principal, secundaria, id, num, nome, preco, item] = o;
    if (cat !== "" && cat !== undefined && String(principal) !== cat) continue;
    if (sub !== undefined && String(secundaria) !== sub) continue;
    if (t && !(String(i) === t || String(id) === t || (nome || "").toLowerCase().includes(t) || (item || "").toLowerCase().includes(t))) continue;
    if (++n > 400) { area.appendChild(criar("div", "texto-suave", "… refine a busca para ver mais.")); break; }
    const b = criar("button", "", `#${i} ${nome || item || `item ${id}`} · ×${num} · ${preco}`);
    b.type = "button";
    b.setAttribute("aria-current", loja.aberta === i ? "true" : "false");
    b.addEventListener("click", () => abrirOferta(i));
    area.appendChild(b);
  }
  if (!n) area.appendChild(criar("div", "texto-suave", "Nenhuma oferta."));
}

async function abrirOferta(indice) {
  loja.aberta = indice;
  desenharListaDaLoja();
  const area = elemento("loja-oferta");
  let o;
  try { o = await api(rotaDaLoja(`/${indice}`)); }
  catch (erro) { area.textContent = erro.message; return; }
  area.replaceChildren();
  const editavel = !!estudio.rascunho;
  const titulo = criar("div", "recursos-linha");
  titulo.append(criar("strong", "", `Oferta #${indice}: ${o.campos.szName || "(sem nome)"}`),
    criar("span", "texto-suave", editavel ? ` · rascunho #${estudio.rascunho}` : " · só leitura: escolha ou crie um rascunho no topo do Estúdio"));
  area.appendChild(titulo);
  if (o.item) {
    const link = criar("a", "", `Item: ${o.item.nome} (abrir a ficha)`);
    link.href = "#";
    link.addEventListener("click", (e) => { e.preventDefault(); mostrarVistaDoEstudio("tabelas"); abrirRegistroDoEstudio(o.item.tabela, o.item.indice); });
    area.appendChild(link);
  } else {
    area.appendChild(criar("div", "texto-suave", `O item ${o.campos.id} não está no elements desta versão.`));
  }
  const tipos = Object.fromEntries(loja.dados.campos);
  const entradas = {};
  const grupo = (titulo, nomes) => {
    const f = criar("fieldset");
    f.appendChild(criar("legend", "", titulo));
    const g = criar("div", "mapa-campos");
    for (const nome of nomes) {
      if (!(nome in o.campos)) continue;
      const tipo = tipos[nome];
      const label = criar("label");
      const ehTexto = typeof tipo === "object";
      const campo = ehTexto && nome === "desc" ? criar("textarea", "campo") : criar("input", "campo");
      if (!ehTexto) { campo.type = "number"; campo.step = "1"; if (tipo === "u32") campo.min = "0"; }
      else campo.maxLength = (tipo.utf16 || tipo.gbk) - 1;
      campo.value = o.campos[nome];
      campo.disabled = !editavel;
      const m = nome.match(/^buy_(\d)_(.+)$/);
      label.append(criar("span", "", m ? ROTULOS_DA_COMPRA[m[2]] || m[2] : ROTULOS_DA_LOJA[nome] || nome), campo);
      label.title = nome;
      g.appendChild(label);
      entradas[nome] = { campo, ehTexto };
    }
    f.appendChild(g);
    area.appendChild(f);
  };
  grupo("Identificação", ["szName", "id", "num", "main_type", "sub_type", "local_id", "icon", "status"]);
  for (let k = 0; k < 4; k++) grupo(`Compra ${k + 1}`, Object.keys(o.campos).filter((n) => n.startsWith(`buy_${k}_`)));
  grupo("Descrição", ["desc"]);
  grupo("Brinde, donos e limite", Object.keys(o.campos).filter((n) => /^(idGift|iGift|iLogPrice|owner_npcs_|buy_times_limit)/.test(n)));
  if (!editavel) return;
  const botoes = criar("div", "recursos-linha");
  const botao = (texto, classe, acao) => { const b = criar("button", `botao botao-pequeno ${classe}`, texto); b.type = "button"; b.addEventListener("click", acao); botoes.appendChild(b); };
  const depois = async (msg, abrir) => {
    alerta("ok", "Estúdio", msg, { toast: true });
    loja.aberta = abrir;
    await carregarLoja();
    if (abrir === null) area.textContent = "Escolha uma oferta.";
    if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
  };
  botao("Salvar no rascunho", "botao-primario", async () => {
    const campos = {};
    for (const [nome, { campo, ehTexto }] of Object.entries(entradas)) {
      const v = ehTexto ? campo.value : Math.trunc(Number(campo.value));
      if (v !== o.campos[nome]) campos[nome] = v;
    }
    if (!Object.keys(campos).length) { alerta("aviso", "Estúdio", "Nada mudou.", { toast: true }); return; }
    try {
      const r = await api(rotaDaLoja(`/${indice}`), { method: "PUT", body: JSON.stringify({ campos }) });
      await depois("Oferta salva no rascunho." + (r.igual_ao_publicado ? " Ficou igual ao publicado: o item saiu do rascunho." : ""), indice);
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  botao("Clonar (no fim da loja)", "botao-secundario", async () => {
    try { const r = await api(rotaDaLoja(""), { method: "POST", body: JSON.stringify({ base: indice }) }); await depois(`Oferta nova #${r.resultado.indice}.`, r.resultado.indice); }
    catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  botao("Remover", "botao-secundario texto-perigo", async () => {
    if (!confirm(`Remover a oferta #${indice}? As seguintes sobem um número (no cliente e no realm, juntos).`)) return;
    try { await api(rotaDaLoja(`/${indice}`), { method: "DELETE" }); await depois("Oferta removida no rascunho.", null); }
    catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  area.appendChild(botoes);
}

(function ligarLoja() {
  elemento("estudio-vista-loja").addEventListener("click", () => mostrarVistaDoEstudio("loja"));
  elemento("loja-arquivo").addEventListener("change", () => { loja.aberta = null; elemento("loja-oferta").textContent = "Escolha uma oferta."; carregarLoja(); });
  elemento("loja-categoria").addEventListener("change", desenharListaDaLoja);
  elemento("loja-busca").addEventListener("input", desenharListaDaLoja);
  elemento("estudio-rascunho").addEventListener("change", () => { if (!elemento("estudio-loja").hidden) carregarLoja(); });
})();
