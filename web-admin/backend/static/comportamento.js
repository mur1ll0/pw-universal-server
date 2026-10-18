/* Estúdio, fase S7 (B258): comportamento dos monstros (`aipolicy.data` do realm, só leitura). Os gatilhos, condições
   e operações são os do leitor do GS (`pw-data-loader/src/aipolicy.rs`, porte de `cgame/gs/ai/policy.cpp`), que o mundo
   interpreta desde o B127 (`pw_gs::politica`). Usa `api`, `criar`, `elemento` e `estudio`/`mostrarVistaDoEstudio`/
   `abrirRegistroDoEstudio`. */

function rotaDeComportamento(resto, q = {}) {
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
  for (const t of pol.triggers) {
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
    area.appendChild(fs);
  }
  area.appendChild(criar("p", "texto-suave", "Só leitura: editar o aipolicy.data exige um gravador do formato (próxima fase)."));
}

(function ligarComportamento() {
  elemento("estudio-vista-comportamento").addEventListener("click", () => mostrarVistaDoEstudio("comportamento"));
  let espera = null;
  elemento("comportamento-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(carregarComportamentos, 300); });
})();
