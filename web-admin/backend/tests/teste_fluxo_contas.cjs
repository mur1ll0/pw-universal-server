// Teste de estado da UI sem navegador: perda, conflito e recuperação do mesmo comando.
// Não substitui renderização/teclado/responsividade.
const { test } = require("node:test");
const assert = require("node:assert/strict");
const vm = require("node:vm");
const fs = require("node:fs");
const path = require("node:path");
const { webcrypto } = require("node:crypto");

async function montar() {
  const elementos = new Map();
  const armazenamento = new Map();
  const pedidos = [];
  const documento = {
    getElementById(id) {
      if (!elementos.has(id)) elementos.set(id, { value: "", textContent: "", hidden: false,
        disabled: false, addEventListener() {}, replaceChildren() {}, append() {},
        classList: { toggle() {} }, setAttribute() {} });
      return elementos.get(id);
    },
    querySelectorAll() { return []; },
  };
  let proxima = { status: 202, dados: { estado: "desconhecido" } };
  const contexto = vm.createContext({ document: documento, crypto: webcrypto, AbortSignal,
    localStorage: { getItem: (c) => armazenamento.get(c) || null,
      setItem: (c,v) => armazenamento.set(c,v), removeItem: (c) => armazenamento.delete(c) },
    fetch: async (caminho, opcoes) => {
      if (caminho === "/api/sessao") return { ok: false, status: 401, json: async () => ({ detail: "Teste" }) };
      pedidos.push({ caminho, opcoes });
      return { ok: proxima.status < 400, status: proxima.status, json: async () => proxima.dados };
    },
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, "../static/painel.js"), "utf8"), contexto);
  await new Promise((resolver) => setImmediate(resolver));
  vm.runInContext("estado.sessao={conta_id:7,csrf:'teste'}; estado.conta={id:9,usuario:'alvo'}; estado.selecionado={id:'realm_126'}", contexto);
  return { contexto, elementos, armazenamento, pedidos,
    avaliar: (codigo) => vm.runInContext(codigo, contexto),
    resposta: (status, dados) => { proxima = { status, dados }; } };
}

test("timeout e conflito preservam identificador/alvo e não armazenam senha", async () => {
  const c = await montar();
  c.elementos.get("nova-senha").value = "Segredo!";
  await c.avaliar("executarComando(false)");
  const primeiro = JSON.parse(c.pedidos[0].opcoes.body);
  assert.equal(c.elementos.get("nova-senha").value, "");
  assert.equal(c.armazenamento.size, 1);
  assert.ok(![...c.armazenamento.values()][0].includes("Segredo"));
  c.resposta(409, { estado: "falha", codigo: "operacao_em_conflito" });
  c.avaliar("estado.selecionado={id:'realm_155'}");
  c.elementos.get("nova-senha").value = "Diferente!";
  await c.avaliar("executarComando(false)");
  assert.equal(JSON.parse(c.pedidos[1].opcoes.body).operacao_id, primeiro.operacao_id);
  assert.ok(c.pedidos[1].caminho.includes("realm_155/contas/9/senha"));
  assert.equal(c.armazenamento.size, 1, "conflito não esquece o resultado original");
  assert.equal(c.avaliar("estado.comando.id"), primeiro.operacao_id);
  assert.match(c.elementos.get("resultado-comando").textContent, /parâmetros.*diferentes/);
  c.resposta(200, { estado: "salvo", conta_id: 9 });
  await c.avaliar("executarComando(true)");
  assert.ok(c.pedidos[2].caminho.endsWith(`/operacoes/${primeiro.operacao_id}`));
  assert.equal(c.armazenamento.size, 0);
  assert.equal(c.avaliar("estado.comando"), null);
});

test("relogin recupera metadados somente do mesmo administrador", async () => {
  const c = await montar();
  c.elementos.get("nova-senha").value = "Segredo!";
  await c.avaliar("executarComando(false)");
  const id = c.avaliar("estado.comando.id");
  c.avaliar("mostrarEntrada(); estado.sessao={conta_id:8}; estado.comando=lerComando()");
  assert.equal(c.avaliar("estado.comando"), null);
  c.avaliar("estado.sessao={conta_id:7}; estado.comando=lerComando()");
  assert.equal(c.avaliar("estado.comando.id"), id);
});

test("criação preserva ID sem alvo numérico em timeout, reload, conflito e recuperação", async () => {
  const c = await montar();
  c.elementos.get("usuario-novo").value = "Nova_Conta";
  c.elementos.get("senha-criacao").value = "Segredo!";
  await c.avaliar("executarComando(false, 'criar_conta')");
  const primeiro = JSON.parse(c.pedidos[0].opcoes.body);
  assert.equal(primeiro.usuario, "nova_conta");
  assert.ok(c.pedidos[0].caminho.endsWith("realm_126/contas"));
  assert.equal(c.elementos.get("senha-criacao").value, "");
  assert.ok(![...c.armazenamento.values()][0].includes("Segredo"));
  assert.equal(c.avaliar("estado.comando.conta_id"), undefined);
  c.avaliar("mostrarEntrada(); estado.sessao={conta_id:7,csrf:'teste'}; estado.comando=lerComando(); estado.selecionado={id:'realm_155'}; atualizarComando()");
  assert.equal(c.avaliar("estado.comando.id"), primeiro.operacao_id);
  assert.equal(c.elementos.get("usuario-novo").value, "nova_conta");
  assert.equal(c.elementos.get("aplicar-senha").disabled, true);
  c.resposta(403, { estado: "falha", codigo: "administrador_recusado" });
  await c.avaliar("executarComando(true)");
  assert.equal(c.armazenamento.size, 1, "recusa de autorização não afirma resultado durável");
  c.resposta(409, { estado: "falha", codigo: "operacao_em_conflito" });
  c.elementos.get("senha-criacao").value = "Outra!";
  await c.avaliar("executarComando(false, 'criar_conta')");
  assert.equal(JSON.parse(c.pedidos[2].opcoes.body).operacao_id, primeiro.operacao_id);
  assert.equal(c.armazenamento.size, 1);
  // Resultado assinado de outro alvo não pode apagar o ID pendente.
  c.resposta(200, { estado: "salvo", tipo: "criar_conta", conta_id: 10, usuario: "outro" });
  await c.avaliar("executarComando(true)");
  assert.equal(c.armazenamento.size, 1);
  c.resposta(200, { estado: "salvo", tipo: "criar_conta", conta_id: 10, usuario: "nova_conta" });
  await c.avaliar("executarComando(true)");
  assert.equal(c.armazenamento.size, 0);
  assert.match(c.elementos.get("resultado-comando").textContent, /conta global nova_conta criada · ID 10/);
});

test("operação pendente impede iniciar escrita de outro tipo", async () => {
  const c = await montar();
  c.elementos.get("nova-senha").value = "Segredo!";
  await c.avaliar("executarComando(false)");
  c.elementos.get("usuario-novo").value = "nova";
  c.elementos.get("senha-criacao").value = "Segredo!";
  await c.avaliar("executarComando(false, 'criar_conta')");
  assert.equal(c.pedidos.length, 1);
  assert.equal(c.armazenamento.size, 1);
});

test("GM pendente persiste alvo/parâmetros entre reload, conflito e aplicação parcial", async () => {
  const c = await montar();
  c.avaliar('elemento("gm-habilitado").value = "true"');
  await c.avaliar('executarComando(false, "definir_gm")');
  const primeiro = JSON.parse(c.pedidos[0].opcoes.body);
  assert.equal(primeiro.habilitado, true);
  assert.equal(primeiro.senha, undefined);
  assert.match(c.pedidos[0].caminho, /\/gm$/);
  c.avaliar('estado.comando = lerComando(); estado.selecionado={id:"realm_155"}');
  c.resposta(202, { estado:"pendente", tipo:"definir_gm", conta_id:9, habilitado:true, persistencia:"salva", processos_pendentes:["gs-126"] });
  await c.avaliar('executarComando(true)');
  assert.equal(c.armazenamento.size,1);
  assert.match(c.elementos.get("resultado-comando").textContent,/salvo; sessões pendentes/);
  c.resposta(409,{estado:"falha",codigo:"operacao_em_conflito"});
  await c.avaliar('executarComando(false,"definir_gm")');
  assert.equal(JSON.parse(c.pedidos.at(-1).opcoes.body).operacao_id,primeiro.operacao_id);
  c.resposta(200,{estado:"aplicado",tipo:"definir_gm",conta_id:9,habilitado:false});
  await c.avaliar('executarComando(true)');
  assert.equal(c.armazenamento.size,1,"parâmetro diferente não encerra recuperação");
  c.resposta(200,{estado:"aplicado",tipo:"definir_gm",conta_id:9,habilitado:true});
  await c.avaliar('executarComando(true)');
  assert.equal(c.armazenamento.size,0);
  assert.match(c.elementos.get("resultado-comando").textContent,/Entre novamente/);
});

test("canal sem envio é falha definitiva: libera a operação e diz que nada mudou (B174)", async () => {
  const c = await montar();
  c.resposta(503, { estado: "falha", codigo: "canal_nao_enviado", mensagem: "Canal administrativo não configurado para este realm." });
  c.avaliar('elemento("gm-habilitado").value = "true"');
  await c.avaliar('executarComando(false, "definir_gm")');
  assert.equal(c.armazenamento.size, 0, "nada foi enviado: não fica pendente");
  assert.equal(c.avaliar("estado.comando"), null);
  assert.match(c.elementos.get("resultado-comando").textContent, /Nada foi alterado/);
});

test("gold e ban (B175): corpo certo, recuperação e resultado de outro alvo não encerra", async () => {
  const c = await montar();
  c.avaliar('elemento("gold-valor").value = "12.5"');
  await c.avaliar('executarComando(false, "ajustar_gold")');
  const corpo = JSON.parse(c.pedidos[0].opcoes.body);
  assert.equal(corpo.delta, 1250, "12,5 gold = 1250 unidades do cash");
  assert.match(c.pedidos[0].caminho, /realm_126\/contas\/9\/gold$/);
  c.resposta(200, { estado: "salvo", tipo: "ajustar_gold", conta_id: 10, saldo: "1250" });
  await c.avaliar("executarComando(true)");
  assert.equal(c.armazenamento.size, 1, "resultado de outra conta não encerra");
  c.resposta(200, { estado: "salvo", tipo: "ajustar_gold", conta_id: 9, saldo: "1250" });
  await c.avaliar("executarComando(true)");
  assert.equal(c.armazenamento.size, 0);
  assert.match(c.elementos.get("resultado-comando").textContent, /12,50 gold/);
  c.avaliar("estado.conta={id:9,usuario:'alvo',banida:false}");
  c.avaliar('elemento("ban-motivo").value = "  trapaça "');
  c.resposta(202, { estado: "desconhecido" });
  await c.avaliar('executarComando(false, "definir_ban")');
  const ban = JSON.parse(c.pedidos.at(-1).opcoes.body);
  assert.deepEqual([ban.banida, ban.motivo], [true, "trapaça"]);
  assert.equal(c.avaliar("lerComando().motivo"), "trapaça", "motivo guardado para repetir igual");
});
