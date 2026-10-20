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
  const porNivel = Object.keys(s).filter((k) => Array.isArray(s[k]) && s[k].length === n && n > 0 && !gerais.includes(k) && s[k].every((x) => x === null || typeof x !== "object"));
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
        if (editaveis.has(k) && s[k][i] !== null) {
          const e = criar("input", "campo");
          e.type = "number";
          e.value = String(s[k][i]);
          e.style.width = "7em";
          (entradas[k] ||= []).push(e);
          td.appendChild(e);
        } else td.textContent = s[k][i] === null ? "—" : String(s[k][i]);
        tr.appendChild(td);
      }
      tn.appendChild(tr);
    }
    ficha.appendChild(tn);
    if (Object.keys(entradas).length) {
      ficha.appendChild(criar("div", "texto-suave", `Números de: ${f.origem_dos_numeros}. ✎ = editável; passe o mouse no cabeçalho para ver quem confere. Recarga e ódio são só do servidor; mana, nível, SP e dinheiro o cliente confere pelo elementskill.dll dele (mudar só aqui desalinha). Vale no jogo depois de publicar e reiniciar o mundo.`));
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
