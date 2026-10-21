/* Estúdio, fase S8 (B301): habilidades — os números do servidor (`habilidades.json`, extraído dos stubs do `gs`) e o
   nome e a descrição do cliente (`Configs\skillstr.txt`: nome em N×10, descrição em N×10+1, com os `%d`/`%.1f` que o
   `elementskill.dll` preenche). Nome e descrição se editam no rascunho; os números, por enquanto, só se leem.
   Usa `api`, `criar`, `elemento`, `alerta`, `estudio` e `mostrarVistaDoEstudio`. */

function rotaDeHabilidades(resto, q = {}) {
  const p = new URLSearchParams(Object.fromEntries(Object.entries(q).filter(([, v]) => v !== undefined && v !== null && v !== ""))).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/habilidades${resto}${p ? `?${p}` : ""}`;
}

async function carregarHabilidades() {
  if (!estudio.realm) return;
  elemento("habilidades-situacao").textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeHabilidades("", { busca: elemento("habilidades-busca").value.trim(), rascunho: estudio.rascunho })); }
  catch (erro) { elemento("habilidades-situacao").textContent = erro.message; return; }
  elemento("habilidades-situacao").textContent = `${r.total} habilidade(s)${r.total > r.habilidades.length ? ` (mostrando ${r.habilidades.length})` : ""}${r.skillstr ? "" : " — skillstr.txt não lido: sem nomes"}`;
  const area = elemento("habilidades-lista");
  area.replaceChildren();
  for (const h of r.habilidades) {
    const b = criar("button", "", `${h.id} · ${h.nome || "(sem nome)"}`);
    b.type = "button";
    b.title = `classe ${h.cls} · até nível ${h.max_level}`;
    b.addEventListener("click", () => abrirHabilidade(h.id));
    area.appendChild(b);
  }
  if (!r.habilidades.length) area.appendChild(criar("div", "texto-suave", "Nenhuma habilidade."));
}

async function abrirHabilidade(id) {
  const ficha = elemento("habilidades-ficha");
  ficha.textContent = "Carregando…";
  let f;
  try { f = await api(rotaDeHabilidades(`/${id}`, { rascunho: estudio.rascunho })); } catch (erro) { ficha.textContent = erro.message; return; }
  ficha.replaceChildren();
  const s = f.servidor || {};
  ficha.appendChild(criar("h3", "", `${f.id} · ${f.nome || "(sem nome)"}`));
  // Nome e descrição (cliente).
  const nome = criar("input", "campo");
  nome.value = f.nome ?? "";
  nome.size = 40;
  const desc = criar("textarea", "campo");
  desc.value = (f.descricao ?? "").replace(/\r\n/g, "\n");
  desc.rows = 10;
  desc.style.width = "100%";
  const salvar = criar("button", "botao botao-pequeno", "Salvar texto no rascunho");
  salvar.type = "button";
  salvar.addEventListener("click", async () => {
    if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio.", { toast: true }); return; }
    try {
      // O cliente lê o texto como está; as quebras do arquivo são CRLF.
      await api(rotaDeHabilidades(`/${f.id}/texto`, { rascunho: estudio.rascunho }), { method: "PUT", body: JSON.stringify({ nome: nome.value, descricao: desc.value.replace(/\r?\n/g, "\r\n") }) });
      alerta("ok", "Estúdio", `Texto da habilidade ${f.id} no rascunho.`, { toast: true });
      if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
    } catch (erro) { alerta("erro", "Estúdio", erro.message); }
  });
  const caixa = criar("div");
  caixa.append(criar("div", "texto-suave", `Nome (${f.skillstr}, ficha ${f.id * 10}):`), nome,
    criar("div", "texto-suave", `Descrição (ficha ${f.id * 10 + 1}; os %d/%.1f o cliente preenche pelo elementskill.dll; sem aspas; até 2.047 caracteres):`), desc, salvar);
  ficha.appendChild(caixa);
  // Campos gerais (servidor).
  const gerais = ["cls", "max_level", "type", "rank", "attr", "apcost", "apgain", "arrowcost", "is_movingcast", "commoncooldown", "commoncooldowntime", "allow_forms", "eventflag", "pre_skills", "restrict_weapons", "time_type", "tipo_de_area", "doenchant", "dobless"];
  const tg = criar("table", "tabela-simples");
  tg.appendChild(criar("caption", "", "Servidor (stubs do gs, habilidades.json)"));
  for (const k of gerais) {
    if (s[k] === undefined) continue;
    const tr = criar("tr");
    tr.append(criar("td", "texto-suave", k), criar("td", "", typeof s[k] === "object" ? JSON.stringify(s[k]) : String(s[k])));
    tg.appendChild(tr);
  }
  ficha.appendChild(tg);
  // B303: campos que o catálogo tomou do elementskill.dll do cliente (o binário é o juiz); o valor do fonte fica em do_fonte.
  if (s.do_fonte && Object.keys(s.do_fonte).length) {
    const aviso = criar("div", "texto-suave", `Do binário do cliente (elementskill.dll v156), não do fonte: ${Object.keys(s.do_fonte).join(", ")}. O fonte dizia: ${JSON.stringify(s.do_fonte)}`);
    ficha.appendChild(aviso);
  }
  // Por nível: as listas com um valor por nível. B302: os campos editáveis viram entradas; o cabeçalho diz quem confere
  // (o cliente decide sozinho mana e requisitos pelo elementskill.dll dele — mudar só o servidor desalinha).
  const n = Number(s.max_level) || 0;
  const editaveis = new Map((f.editaveis || []).map((x) => [x.campo, x.quem_confere]));
  const entradas = {};
  // B307: o dano por nível (dentro de `dano`) entra como colunas `dano.ratio`/`dano.plus`.
  const sn = { ...s };
  if (s.dano && typeof s.dano === "object") for (const k of ["ratio", "plus"]) if (Array.isArray(s.dano[k])) sn[`dano.${k}`] = s.dano[k];
  const porNivel = Object.keys(sn).filter((k) => Array.isArray(sn[k]) && sn[k].length === n && n > 0 && !gerais.includes(k) && sn[k].every((x) => x === null || typeof x !== "object"));
  if (porNivel.length) {
    const tn = criar("table", "tabela-simples");
    tn.appendChild(criar("caption", "", "Por nível (null = depende de mais que o nível)"));
    const cab = criar("tr");
    cab.appendChild(criar("th", "", "nível"));
    for (const k of porNivel) {
      const th = criar("th", "", editaveis.has(k) ? `${k} ✎` : k);
      if (editaveis.has(k)) th.title = editaveis.get(k);
      cab.appendChild(th);
    }
    tn.appendChild(cab);
    for (let i = 0; i < n; i++) {
      const tr = criar("tr");
      tr.appendChild(criar("td", "", String(i + 1)));
      for (const k of porNivel) {
        const td = criar("td");
        if (editaveis.has(k) && sn[k][i] !== null) {
          const e = criar("input", "campo");
          e.type = "number";
          e.value = String(sn[k][i]);
          e.style.width = "7em";
          (entradas[k] ||= []).push(e);
          td.appendChild(e);
        } else td.textContent = sn[k][i] === null ? "—" : String(sn[k][i]);
        tr.appendChild(td);
      }
      tn.appendChild(tr);
    }
    ficha.appendChild(tn);
    if (Object.keys(entradas).length) {
      ficha.appendChild(criar("div", "texto-suave", `Números de: ${f.origem_dos_numeros}. ✎ = editável; passe o mouse no cabeçalho para ver quem confere. Recarga, ódio, dano e precisão são só do servidor (a descrição do cliente, com os %d do elementskill.dll, não muda); mana, nível, SP e dinheiro o cliente confere pelo elementskill.dll dele (mudar só aqui desalinha). Vale no jogo depois de publicar e reiniciar o mundo.`));
      const salvarN = criar("button", "botao botao-pequeno", "Salvar números no rascunho");
      salvarN.type = "button";
      salvarN.addEventListener("click", async () => {
        if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio.", { toast: true }); return; }
        const corpo = {};
        for (const [k, es] of Object.entries(entradas)) {
          if (es.length !== n) continue; // campo com algum nível sem número fica de fora
          corpo[k] = es.map((e) => Number(e.value));
        }
        try {
          await api(rotaDeHabilidades(`/${f.id}/numeros`, { rascunho: estudio.rascunho }), { method: "PUT", body: JSON.stringify(corpo) });
          alerta("ok", "Estúdio", `Números da habilidade ${f.id} no rascunho (catalogo\\habilidades.json).`, { toast: true });
          if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
          abrirHabilidade(f.id);
        } catch (erro) { alerta("erro", "Estúdio", erro.message); }
      });
      ficha.appendChild(salvarN);
    }
  }
  // B308: os roteiros de efeito (no_alvo = StateAttack no alvo, em_si = BlessMe em quem lança) — a expressão de cada
  // passo se edita; quem e setter ficam. O servidor confere com o avaliador do GS e as variáveis do original.
  for (const [r, titulo] of [["no_alvo", "Efeito no alvo (no_alvo)"], ["em_si", "Efeito em quem lança (em_si)"]]) {
    const flag = r === "no_alvo" ? "doenchant" : "dobless";
    // B310: as flags que fazem o GS rodar o roteiro (só do servidor).
    const caixaFlag = criar("label", "texto-suave");
    const chk = criar("input");
    chk.type = "checkbox";
    chk.checked = !!s[flag];
    chk.addEventListener("change", async () => {
      if (!estudio.rascunho) { chk.checked = !chk.checked; alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio.", { toast: true }); return; }
      try {
        await api(rotaDeHabilidades(`/${f.id}/roteiro`, { rascunho: estudio.rascunho }), { method: "PUT", body: JSON.stringify({ [flag]: chk.checked }) });
        alerta("ok", "Estúdio", `${flag} = ${chk.checked} no rascunho.`, { toast: true });
        if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
      } catch (erro) { chk.checked = !chk.checked; alerta("erro", "Estúdio", erro.message); }
    });
    caixaFlag.append(chk, ` ${flag} — o GS roda ${r === "no_alvo" ? "o efeito no alvo (ou se a habilidade é de bênção/maldição)" : "o efeito em quem lança"} só com esta flag`);
    if (!Array.isArray(s[r]) || !s[r].length) {
      // B310: criar o roteiro (liga a flag ao salvar).
      const criarR = criar("button", "botao botao-pequeno", `Criar ${titulo.toLowerCase()}`);
      criarR.type = "button";
      criarR.addEventListener("click", () => { s[r] = [["V", "Probability", "100"]]; abrirHabilidadeComRoteiro(); });
      const abrirHabilidadeComRoteiro = () => { criarR.remove(); desenharRoteiro(r, titulo, caixaFlag); };
      ficha.append(criarR, caixaFlag);
      continue;
    }
    desenharRoteiro(r, titulo, caixaFlag);
  }
  function desenharRoteiro(r, titulo, caixaFlag) {
    // B309: o roteiro inteiro se edita — passos acrescentados (quem = V, setter da lista do GS), removidos, reordenados.
    const passos = s[r].map((p) => ({ quem: p[0], setter: p[1], expr: String(p[2]) }));
    const caixa = criar("div");
    const desenhar = () => {
      caixa.replaceChildren();
      const t = criar("table", "tabela-simples");
      t.appendChild(criar("caption", "", `${titulo} — só o servidor (o GS roda em ordem: os acumuladores valem para o efeito seguinte)`));
      const cab = criar("tr");
      cab.append(criar("th", "", "#"), criar("th", "", "quem"), criar("th", "", "setter"), criar("th", "", "expressão ✎"), criar("th", "", ""));
      t.appendChild(cab);
      passos.forEach((p, i) => {
        const tr = criar("tr");
        const e = criar("input", "campo");
        e.value = p.expr;
        e.style.width = "100%";
        e.addEventListener("input", () => { p.expr = e.value; });
        const td = criar("td");
        td.appendChild(e);
        const acoes = criar("td");
        for (const [rot, fazer] of [["↑", () => i > 0 && passos.splice(i - 1, 0, ...passos.splice(i, 1))], ["↓", () => i < passos.length - 1 && passos.splice(i + 1, 0, ...passos.splice(i, 1))], ["✕", () => passos.splice(i, 1)]]) {
          const b = criar("button", "botao botao-pequeno", rot);
          b.type = "button";
          b.addEventListener("click", () => { fazer(); desenhar(); });
          acoes.appendChild(b);
        }
        tr.append(criar("td", "", String(i + 1)), criar("td", "", p.quem), criar("td", "", p.setter), td, acoes);
        t.appendChild(tr);
      });
      caixa.appendChild(t);
      const sel = criar("select", "campo");
      for (const n of f.setters_do_gs || []) { const o = criar("option", "", n); o.value = n; sel.appendChild(o); }
      const mais = criar("button", "botao botao-pequeno", "Acrescentar passo");
      mais.type = "button";
      mais.addEventListener("click", () => { passos.push({ quem: "V", setter: sel.value, expr: "1" }); desenhar(); });
      const salvarR = criar("button", "botao botao-pequeno", `Salvar ${r} no rascunho`);
      salvarR.type = "button";
      salvarR.addEventListener("click", async () => {
        if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio.", { toast: true }); return; }
        try {
          await api(rotaDeHabilidades(`/${f.id}/roteiro`, { rascunho: estudio.rascunho }), { method: "PUT", body: JSON.stringify({ [r]: passos.map((p) => [p.quem, p.setter, p.expr]) }) });
          alerta("ok", "Estúdio", `Roteiro ${r} da habilidade ${f.id} no rascunho (catalogo\\habilidades.json).`, { toast: true });
          if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
          abrirHabilidade(f.id);
        } catch (erro) { alerta("erro", "Estúdio", erro.message); }
      });
      caixa.append(sel, mais, salvarR, criar("div", "texto-suave", "Variáveis: L (nível), P_* (quem lança), V_* (alvo), S_* (golpe da habilidade) — só as que algum roteiro do original usa; números, + − × ÷, comparações, && ||, ?:, INT(...). Passo novo: quem = V e um setter que o GS porta; Probability (0–100), Time (ms), Ratio, Value, Amount e Showicon valem para o efeito seguinte."));
    };
    desenhar();
    ficha.append(caixa, caixaFlag);
  }
  // B305: os números do cliente (elementskill.dll) — só as tabelas medidas no binário se editam; o resto vem do código.
  const c = f.cliente || {};
  if (!c.mapa) {
    ficha.appendChild(criar("div", "texto-suave", `Cliente: ${c.motivo || "sem mapa do elementskill.dll"}.`));
  } else {
    const nomes = Object.keys(c.campos || {});
    if (!nomes.length) {
      ficha.appendChild(criar("div", "texto-suave", "Cliente (elementskill.dll): nenhum número desta habilidade está numa tabela editável — vêm do código do DLL (constante ou fórmula)."));
    } else {
      const tc = criar("table", "tabela-simples");
      tc.appendChild(criar("caption", "", `Cliente (${c.arquivo}) — o que o jogo do jogador confere`));
      const cab = criar("tr");
      cab.appendChild(criar("th", "", "nível"));
      for (const k of nomes) {
        const th = criar("th", "", `${k} ✎`);
        th.title = c.campos[k].quem_confere;
        cab.appendChild(th);
      }
      tc.appendChild(cab);
      const ents = {};
      const linhas = Math.max(...nomes.map((k) => c.campos[k].valores.length));
      for (let i = 0; i < linhas; i++) {
        const tr = criar("tr");
        tr.appendChild(criar("td", "", String(i + 1)));
        for (const k of nomes) {
          const td = criar("td");
          const v = c.campos[k].valores[i];
          if (v === undefined) td.textContent = "—";
          else {
            const e = criar("input", "campo");
            e.type = "number";
            e.value = String(v);
            e.style.width = "7em";
            // Diferente do servidor: o cliente mostra/confere um número e o servidor cobra outro.
            const sv = Array.isArray(s[k]) ? s[k][i] : undefined;
            if (sv !== undefined && sv !== null && Number(sv) !== v) { e.style.outline = "2px solid var(--aviso, orange)"; e.title = `servidor: ${sv}`; }
            (ents[k] ||= []).push(e);
            td.appendChild(e);
          }
          tr.appendChild(td);
        }
        tc.appendChild(tr);
      }
      ficha.appendChild(tc);
      const alem = nomes.filter((k) => c.campos[k].le_alem).map((k) => `${k}: os ${c.campos[k].le_alem} últimos níveis leem a tabela seguinte no DLL original (não se editam)`);
      ficha.appendChild(criar("div", "texto-suave", `Contorno = difere do servidor (passe o mouse para ver). Mude os dois lados juntos, ou o cliente mostra um preço e o servidor cobra outro.${alem.length ? " " + alem.join("; ") + "." : ""} Vale para o jogador depois de publicar e ele atualizar pelo launcher.`));
      const salvarC = criar("button", "botao botao-pequeno", "Salvar números do cliente no rascunho");
      salvarC.type = "button";
      salvarC.addEventListener("click", async () => {
        if (!estudio.rascunho) { alerta("aviso", "Estúdio", "Escolha um rascunho no alto do Estúdio.", { toast: true }); return; }
        const corpo = Object.fromEntries(Object.entries(ents).map(([k, es]) => [k, es.map((e) => Number(e.value))]));
        try {
          await api(rotaDeHabilidades(`/${f.id}/numeros_do_cliente`, { rascunho: estudio.rascunho }), { method: "PUT", body: JSON.stringify(corpo) });
          alerta("ok", "Estúdio", `Números do cliente da habilidade ${f.id} no rascunho (elementskill.dll).`, { toast: true });
          if (typeof carregarRascunhosDoEstudio === "function") carregarRascunhosDoEstudio();
          abrirHabilidade(f.id);
        } catch (erro) { alerta("erro", "Estúdio", erro.message); }
      });
      ficha.appendChild(salvarC);
    }
  }
  const bruto = criar("details");
  bruto.appendChild(criar("summary", "", "Todos os campos do servidor (JSON)"));
  const pre = criar("pre");
  pre.textContent = JSON.stringify(s, null, 1);
  pre.style.maxHeight = "40vh";
  pre.style.overflow = "auto";
  bruto.appendChild(pre);
  ficha.appendChild(bruto);
}

(function ligarHabilidades() {
  elemento("estudio-vista-habilidades").addEventListener("click", () => mostrarVistaDoEstudio("habilidades"));
  let espera = null;
  elemento("habilidades-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(carregarHabilidades, 300); });
})();
