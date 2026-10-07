"""Integração real no schema test; ausência de banco é falha, nunca skip."""
import hashlib
import asyncio
import hmac
import json
import struct
import socket
import tempfile
import os
import sys
import unittest
import uuid
from contextlib import asynccontextmanager
from pathlib import Path
from unittest.mock import patch

import asyncpg
import httpx
import redis.asyncio as aioredis

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from painel.aplicacao import criar_aplicacao
from painel.seguranca import COOKIE, Seguranca


class BaseAdministrativa(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.sufixo = uuid.uuid4().hex[:12]
        self.prefixo = f"pw-admin:teste:{self.sufixo}:"
        self.pool = await asyncpg.create_pool(os.environ["TEST_DATABASE_URL"], min_size=1, max_size=2,
                                            server_settings={"search_path": "test"}, command_timeout=5)
        self.redis = aioredis.from_url(os.getenv("TEST_REDIS_URL", "redis://localhost:6379"),
                                      decode_responses=True, socket_timeout=2)
        await self.redis.ping()
        # Padrão já coberto pela limpeza central de sobras do pw-storage.
        self.usuario = f"at_{int(self.sufixo, 16)}"
        self.senha = "senha-do-teste"
        self.hash = hashlib.md5((self.usuario + self.senha).encode()).hexdigest()
        async with self.pool.acquire() as conexao:
            self.conta_id = await conexao.fetchval(
                "INSERT INTO accounts (username,password_hash,gm_privileges) VALUES($1,$2,1) RETURNING id",
                self.usuario, self.hash,
            )
        self.app = criar_aplicacao()
        self.app.state.seguranca = Seguranca(self.pool, self.redis, self.prefixo)
        self.cliente = httpx.AsyncClient(transport=httpx.ASGITransport(app=self.app), base_url="https://painel.test")
        self.realms_criados = []
        self.contas_criadas = []
        self.servidores = []
        self.pedidos_admin = []
        self.ambiente_canal = patch.dict(os.environ, {"ADMIN_DAEMONS": "{}", "ADMIN_CHANNEL_SECRETS": "{}"})
        self.ambiente_canal.start()

    async def asyncTearDown(self):
        for servidor in self.servidores:
            servidor.close()
            await servidor.wait_closed()
        self.ambiente_canal.stop()
        await self.cliente.aclose()
        async with self.pool.acquire() as conexao:
            await conexao.execute("DELETE FROM comandos_administrativos WHERE administrador_id=$1", self.conta_id)
            for conta in self.contas_criadas:
                await conexao.execute("DELETE FROM accounts WHERE id=$1", conta)
            await conexao.execute("DELETE FROM accounts WHERE id=$1", self.conta_id)
            for realm in self.realms_criados:
                await conexao.execute("DELETE FROM realms WHERE id=$1", realm)
        chaves = [chave async for chave in self.redis.scan_iter(match=f"{self.prefixo}*")]
        if chaves:
            await self.redis.delete(*chaves)
        await self.redis.aclose()
        await self.pool.close()

    async def entrar(self, senha=None):
        return await self.cliente.post("/api/sessao/entrar", json={"usuario": self.usuario.upper(),
                                                               "senha": senha or self.senha})

    async def modificar_conta(self, campo, valor):
        assert campo in ("gm_privileges", "is_banned", "password_hash")
        async with self.pool.acquire() as conexao:
            await conexao.execute(f"UPDATE accounts SET {campo}=$1 WHERE id=$2", valor, self.conta_id)

    async def criar_personagem(self):
        realm = f"admin_{self.sufixo}"
        async with self.pool.acquire() as conexao:
            await conexao.execute("INSERT INTO realms(id,name,version,host,port) VALUES($1,'Admin teste','1.2.6','127.0.0.1',1)", realm)
            personagem = await conexao.fetchval("INSERT INTO characters(account_id,realm_id,name,race,cls,gender) VALUES($1,$2,'PersonagemAdmin',0,0,0) RETURNING id", self.conta_id, realm)
        self.realms_criados.append(realm)
        await self.entrar()
        return realm, personagem

    async def ativar_daemon(self, realm, presenca="ausente", ficha=None, erro=None, mundos=None, acrescentar=False):
        chave = b"c" * 32
        desafio = b"d" * 32

        async def atender(leitor, escritor):
            async def enviar(corpo):
                escritor.write(struct.pack("!I", len(corpo)) + corpo)
                await escritor.drain()
            try:
                await enviar(json.dumps({"protocolo": 1, "desafio": desafio.hex()}).encode())
                tamanho = struct.unpack("!I", await leitor.readexactly(4))[0]
                corpo = await leitor.readexactly(tamanho)
                assinatura = await leitor.readexactly(32)
                self.assertTrue(hmac.compare_digest(assinatura, hmac.digest(chave, desafio + b"pedido" + corpo, "sha256")))
                pedido = json.loads(corpo)
                self.pedidos_admin.append(pedido)
                if erro == "timeout":
                    await leitor.read()
                    return
                consulta = pedido["consulta"]
                estado_resposta = "consultado"
                if consulta["tipo"] == "mundos":
                    dados = {"presenca": "observada", "mapas": mundos or [], "carregaveis": [1, 105]}
                elif consulta["tipo"] == "definir_mapa":
                    # Como o GS (B183): mapa que não serve só é aceito com `carregar`.
                    if consulta.get("carregar"):
                        estado_resposta = "aplicado"
                        dados = {"estado": "aplicado", "tipo": "definir_mapa", "mapa": consulta["mapa"],
                                 "ligado": consulta["ligado"], "carregando": consulta["ligado"], "desconectados": 0}
                    else:
                        estado_resposta = "falha"
                        dados = {"codigo": "mapa_nao_servido", "mapa": consulta["mapa"]}
                else:
                    dados = {"presenca": presenca, "ficha": ficha}
                resposta = json.dumps({"operacao_id": pedido["operacao_id"], "realm_id": "outro" if erro == "realm" else realm,
                                       "estado": estado_resposta, "dados": dados}).encode()
                await enviar(resposta)
                escritor.write(b"0" * 32 if erro == "assinatura" else hmac.digest(chave, desafio + b"resposta" + resposta, "sha256"))
                await escritor.drain()
            finally:
                escritor.close()
                await escritor.wait_closed()

        servidor = await asyncio.start_server(atender, "127.0.0.1", 0)
        self.servidores.append(servidor)
        alvos = json.loads(os.environ["ADMIN_DAEMONS"]) if acrescentar else {}
        alvos.setdefault(realm, []).append({"host": "127.0.0.1", "porta": servidor.sockets[0].getsockname()[1]})
        os.environ["ADMIN_DAEMONS"] = json.dumps(alvos)
        os.environ["ADMIN_CHANNEL_SECRETS"] = json.dumps({realm: chave.hex()})

    async def detalhe(self, realm, personagem):
        resposta = await self.cliente.get(f"/api/realms/{realm}/personagens/{personagem}")
        self.assertEqual(resposta.status_code, 200, resposta.text)
        return resposta.json()

    @asynccontextmanager
    async def gs_real(self, realm):
        with socket.socket() as porta:
            porta.bind(("127.0.0.1", 0))
            numero = porta.getsockname()[1]
        chave = os.urandom(32).hex()
        os.environ["ADMIN_DAEMONS"] = json.dumps({realm: [{"host": "127.0.0.1", "porta": numero}]})
        os.environ["ADMIN_CHANNEL_SECRETS"] = json.dumps({realm: chave})
        binario = os.getenv("PW_GS_TESTE", str(Path(__file__).resolve().parents[3] / "target/debug/pw-gs.exe"))
        if os.name != "nt" and not os.getenv("PW_GS_TESTE"):
            binario = binario.removesuffix(".exe")
        with tempfile.TemporaryDirectory(prefix="pw-admin-gs-") as dados:
            ambiente = dict(os.environ, DATABASE_URL=os.environ["TEST_DATABASE_URL"], REALM_ID=realm,
                            GAME_VERSION="1.2.6", WORLD_TAGS="1,161", CONFIG_DIR=dados,
                            ADMIN_SECRET=chave, ADMIN_LISTEN=f"127.0.0.1:{numero}",
                            BUS_LISTEN="127.0.0.1:0", RUST_LOG="error")
            processo = await asyncio.create_subprocess_exec(binario, env=ambiente,
                    stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL)
            try:
                for _ in range(100):
                    if processo.returncode is not None:
                        self.fail(f"GS encerrou com código {processo.returncode}")
                    try:
                        _, escritor = await asyncio.open_connection("127.0.0.1", numero)
                        escritor.close()
                        await escritor.wait_closed()
                        break
                    except OSError:
                        await asyncio.sleep(0.05)
                else:
                    self.fail("GS não abriu o canal administrativo")
                yield numero
            finally:
                if processo.returncode is None:
                    processo.terminate()
                await processo.wait()

    async def test_criacao_api_autorizacao_csrf_validacao_e_timeout(self):
        caminho = "/api/realms/realm_126/contas"
        pedido = {"operacao_id": uuid.uuid4().hex, "usuario": "NovaConta", "senha": "Nova!"}
        self.assertEqual((await self.cliente.post(caminho, json=pedido)).status_code, 401)
        await self.entrar()
        self.assertEqual((await self.cliente.post(caminho, json=pedido)).status_code, 403)
        cabecalhos = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        for nome in ("", "nome com espaço", "é", "名", "a" * 65, "nome\n", "admin-"):
            resposta = await self.cliente.post(caminho, json={**pedido, "usuario": nome}, headers=cabecalhos)
            self.assertEqual(resposta.status_code, 422, resposta.text)
            self.assertNotIn("Nova!", resposta.text)
        for senha in ("", "á", "a" * 65, "abc\n"):
            self.assertEqual((await self.cliente.post(caminho, json={**pedido, "senha": senha}, headers=cabecalhos)).status_code, 422)
        self.assertEqual((await self.cliente.post(caminho, json={**pedido, "gm": 32}, headers=cabecalhos)).status_code, 422)
        self.assertEqual((await self.cliente.post("/api/realms/inexistente/contas", json=pedido, headers=cabecalhos)).status_code, 404)
        # Canal sem chave: nada sai do painel -> falha definitiva, repetir é seguro (B174).
        nao_enviado = await self.cliente.post(caminho, json=pedido, headers=cabecalhos)
        self.assertEqual(nao_enviado.status_code, 503, nao_enviado.text)
        self.assertEqual(nao_enviado.json()["estado"], "falha")
        self.assertEqual(nao_enviado.json()["codigo"], "canal_nao_enviado")
        self.assertEqual(nao_enviado.json()["operacao_id"], pedido["operacao_id"])
        await self.modificar_conta("gm_privileges", 0)
        self.assertEqual((await self.cliente.post(caminho, json=pedido, headers=cabecalhos)).status_code, 401)

    async def test_criacao_real_concorrente_reinicio_relogin_e_outro_realm(self):
        realm, _ = await self.criar_personagem()
        nome = f"at_{int(self.sufixo, 16)}_2"
        pedido = {"operacao_id": uuid.uuid4().hex, "usuario": nome.upper(), "senha": "Criada!"}
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real(realm):
            a, b = await asyncio.gather(*[self.cliente.post(f"/api/realms/{realm}/contas", json=pedido,
                                            headers={"X-CSRF-Token": csrf}) for _ in range(2)])
            self.assertEqual(a.status_code, 200, a.text)
            self.assertEqual(a.json(), b.json())
            nova = a.json()["conta_id"]
            self.contas_criadas.append(nova)
            self.assertEqual(a.json()["usuario"], nome)
            async with self.pool.acquire() as c:
                conta = await c.fetchrow("SELECT * FROM accounts WHERE id=$1", nova)
                self.assertEqual((conta["gold_balance"], conta["silver_balance"], conta["gm_privileges"], conta["is_banned"]), (0, 0, 0, False))
                self.assertEqual(conta["password_hash"], hashlib.md5((nome + "Criada!").encode()).hexdigest())
            self.assertNotIn("Criada!", a.text)
        await self.entrar()
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real("realm_155"):
            recuperado = await self.cliente.get(f"/api/realms/realm_155/operacoes/{pedido['operacao_id']}")
            self.assertEqual(recuperado.json(), a.json())
            repetido = await self.cliente.post("/api/realms/realm_155/contas", json={**pedido, "usuario": nome}, headers={"X-CSRF-Token": csrf})
            self.assertEqual(repetido.json(), a.json())
            for alteracao in ({"senha": "Outra!"}, {"usuario": nome + "2"}):
                conflito = await self.cliente.post("/api/realms/realm_155/contas", json={**pedido, **alteracao}, headers={"X-CSRF-Token": csrf})
                self.assertEqual(conflito.status_code, 409, conflito.text)
                self.assertEqual(conflito.json()["codigo"], "operacao_em_conflito")
            duplicada = {**pedido, "operacao_id": uuid.uuid4().hex}
            duplicado = await self.cliente.post("/api/realms/realm_155/contas", json=duplicada, headers={"X-CSRF-Token": csrf})
            self.assertEqual(duplicado.status_code, 409, duplicado.text)
            self.assertEqual(duplicado.json()["codigo"], "usuario_existente")
            resultado = await self.cliente.get(f"/api/realms/realm_155/operacoes/{duplicada['operacao_id']}")
            self.assertEqual(resultado.json()["codigo"], "usuario_existente")
        # Conta comum não ganha acesso ao painel.
        self.assertEqual((await self.cliente.post("/api/sessao/entrar", json={"usuario": nome, "senha": "Criada!"})).status_code, 401)

    async def test_criacao_perde_resposta_apos_commit_e_recupera_sem_nova_conta(self):
        realm, _ = await self.criar_personagem()
        pedido = {"operacao_id": uuid.uuid4().hex, "usuario": f"at_{int(self.sufixo, 16)}_2", "senha": "Criada!"}
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real(realm) as porta_gs:
            async def cortar_resposta(leitor, escritor):
                gs_leitor, gs_escritor = await asyncio.open_connection("127.0.0.1", porta_gs)
                try:
                    tamanho = struct.unpack("!I", await gs_leitor.readexactly(4))[0]
                    escritor.write(struct.pack("!I", tamanho) + await gs_leitor.readexactly(tamanho))
                    await escritor.drain()
                    tamanho = struct.unpack("!I", await leitor.readexactly(4))[0]
                    gs_escritor.write(struct.pack("!I", tamanho) + await leitor.readexactly(tamanho + 32))
                    await gs_escritor.drain()
                    tamanho = struct.unpack("!I", await gs_leitor.readexactly(4))[0]
                    await gs_leitor.readexactly(tamanho + 32)
                finally:
                    gs_escritor.close()
                    escritor.close()
                    await gs_escritor.wait_closed()
                    await escritor.wait_closed()
            proxy = await asyncio.start_server(cortar_resposta, "127.0.0.1", 0)
            self.servidores.append(proxy)
            os.environ["ADMIN_DAEMONS"] = json.dumps({realm: [{"host": "127.0.0.1", "porta": proxy.sockets[0].getsockname()[1]}]})
            resposta = await self.cliente.post(f"/api/realms/{realm}/contas", json=pedido, headers={"X-CSRF-Token": csrf})
            self.assertEqual(resposta.status_code, 202, resposta.text)
            async with self.pool.acquire() as c:
                nova = await c.fetchval("SELECT id FROM accounts WHERE username=$1", pedido["usuario"])
                self.contas_criadas.append(nova)
        await self.entrar()
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real("realm_155"):
            recuperado = await self.cliente.get(f"/api/realms/realm_155/operacoes/{pedido['operacao_id']}")
            self.assertEqual(recuperado.json()["conta_id"], nova)
            self.assertEqual(recuperado.json()["estado"], "salvo")
            repetido = await self.cliente.post("/api/realms/realm_155/contas", json=pedido, headers={"X-CSRF-Token": csrf})
            self.assertEqual(repetido.json(), recuperado.json())
            async with self.pool.acquire() as c:
                self.assertEqual(await c.fetchval("SELECT count(*) FROM accounts WHERE lower(username)=lower($1)", pedido["usuario"]), 1)

    async def test_canal_recusa_resultados_salvos_de_outro_tipo_nome_ou_id(self):
        from painel.canal import comandar, CanalIndisponivel
        os.environ["ADMIN_DAEMONS"] = json.dumps({"realm_126": [{"host": "127.0.0.1", "porta": 1}]})
        os.environ["ADMIN_CHANNEL_SECRETS"] = json.dumps({"realm_126": "01" * 32})
        valido = {"tipo": "criar_conta", "conta_id": 12, "usuario": "nova", "alcance": "global"}
        for alteracao in ({"tipo": "trocar_senha"}, {"usuario": "outra"}, {"conta_id": True}, {"conta_id": 0}, {"alcance": "realm"}):
            with patch("painel.canal.enviar_alvo", return_value={"estado": "salvo", "dados": {**valido, **alteracao}}):
                with self.assertRaises(CanalIndisponivel):
                    await comandar("realm_126", self.conta_id, {"tipo": "criar_conta", "usuario": "nova", "senha": "Nova!"}, "teste")

    async def test_criacao_lock_timeout_desconhecido_libera_id_para_mesma_tentativa(self):
        realm, _ = await self.criar_personagem()
        pedido = {"operacao_id": uuid.uuid4().hex, "usuario": f"at_{int(self.sufixo, 16)}_2", "senha": "Criada!"}
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real(realm):
            async with self.pool.acquire() as c:
                bloqueio = c.transaction()
                await bloqueio.start()
                try:
                    await c.execute("SELECT id FROM accounts WHERE id=$1 FOR UPDATE", self.conta_id)
                    resposta = await self.cliente.post(f"/api/realms/{realm}/contas", json=pedido, headers={"X-CSRF-Token": csrf})
                    self.assertEqual(resposta.status_code, 202, resposta.text)
                    recuperado = await self.cliente.get(f"/api/realms/{realm}/operacoes/{pedido['operacao_id']}")
                    self.assertEqual(recuperado.json()["estado"], "desconhecido")
                    self.assertIsNone(await c.fetchval("SELECT id FROM accounts WHERE username=$1", pedido["usuario"]))
                finally:
                    await bloqueio.rollback()
            repetido = await self.cliente.post(f"/api/realms/{realm}/contas", json=pedido, headers={"X-CSRF-Token": csrf})
            self.assertEqual(repetido.status_code, 200, repetido.text)
            self.contas_criadas.append(repetido.json()["conta_id"])
            self.assertEqual(repetido.json()["operacao_id"], pedido["operacao_id"])

    async def test_migracao_recusa_duplicatas_sem_alterar_contas_ou_layout(self):
        # Tabelas temporárias sombreiam test; script real não toca contas existentes.
        script = (Path(__file__).resolve().parents[3] / "scripts/2026_10_05_criacao_contas.sql").read_text()
        async with self.pool.acquire() as c:
            await c.execute("CREATE TEMP TABLE accounts(username text); CREATE TEMP TABLE comandos_administrativos(conta_id integer NOT NULL)")
            try:
                await c.execute("INSERT INTO pg_temp.accounts VALUES('Nome'),('nome')")
                with self.assertRaises(asyncpg.UniqueViolationError):
                    await c.execute(script)
                await c.execute("ROLLBACK")
                self.assertEqual(await c.fetchval("SELECT count(*) FROM pg_temp.accounts"), 2)
                self.assertTrue(await c.fetchval("SELECT attnotnull FROM pg_attribute WHERE attrelid='pg_temp.comandos_administrativos'::regclass AND attname='conta_id'"))
            finally:
                await c.execute("DROP TABLE pg_temp.accounts,pg_temp.comandos_administrativos")

    async def test_contas_globais_sem_expor_credenciais(self):
        await self.entrar()
        resposta = await self.cliente.get("/api/contas", params={"busca": self.usuario})
        self.assertEqual(resposta.status_code, 200)
        self.assertEqual(resposta.json()["alcance"], "global")
        conta, = resposta.json()["contas"]
        self.assertEqual({c: conta[c] for c in ("id", "usuario", "gm", "banida", "gold", "personagens")},
                         {"id": self.conta_id, "usuario": self.usuario, "gm": 1, "banida": False,
                          "gold": "0", "personagens": 0})
        self.assertEqual((resposta.json()["total"], resposta.json()["pagina"]), (1, 1))
        self.assertNotIn(self.hash, resposta.text)
        # Paginação (B174) e "_" literal na busca, não curinga do ILIKE.
        pagina = await self.cliente.get("/api/contas", params={"por_pagina": 1, "pagina": 1})
        self.assertEqual(len(pagina.json()["contas"]), 1)
        self.assertGreaterEqual(pagina.json()["total"], 1)
        self.assertEqual((await self.cliente.get("/api/contas", params={"por_pagina": 49})).status_code, 422)
        curinga = await self.cliente.get("/api/contas", params={"busca": self.usuario.replace(self.usuario[1], "_", 1)})
        if "_" not in self.usuario:
            self.assertEqual(curinga.json()["total"], 0)

    async def test_senha_api_exige_csrf_valida_payload_e_realm_antes_do_envio(self):
        realm, _ = await self.criar_personagem()
        caminho = f"/api/realms/{realm}/contas/{self.conta_id}/senha"
        pedido = {"operacao_id": uuid.uuid4().hex, "senha": "Nova!"}
        self.assertEqual((await self.cliente.post(caminho, json=pedido)).status_code, 403)
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        cabecalhos = {"X-CSRF-Token": csrf}
        for invalido in ["", "não-ASCII", "\n", "x" * 65]:
            resposta = await self.cliente.post(caminho, json={**pedido, "senha": invalido}, headers=cabecalhos)
            self.assertEqual(resposta.status_code, 422)
            self.assertNotIn("senha", resposta.text)
        self.assertEqual((await self.cliente.post(caminho.replace(realm, "realm_inexistente"), json=pedido, headers=cabecalhos)).status_code, 404)
        self.assertEqual((await self.cliente.post(caminho.replace(str(self.conta_id), "2147483648"), json=pedido, headers=cabecalhos)).status_code, 422)
        resposta = await self.cliente.post(caminho, json=pedido, headers=cabecalhos)
        self.assertEqual(resposta.status_code, 503)  # canal sem chave: nada enviado (B174)
        self.assertEqual(resposta.json()["codigo"], "canal_nao_enviado")
        self.assertEqual(resposta.json()["operacao_id"], pedido["operacao_id"])
        async with self.pool.acquire() as conexao:
            self.assertEqual(await conexao.fetchval("SELECT password_hash FROM accounts WHERE id=$1", self.conta_id), self.hash)

    async def test_api_senha_real_recupera_apos_reinicio_revoga_sessao_e_recusa_senha_antiga(self):
        realm, _ = await self.criar_personagem()
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        id_operacao = uuid.uuid4().hex
        async with self.gs_real(realm):
            resposta = await self.cliente.post(f"/api/realms/{realm}/contas/{self.conta_id}/senha",
                json={"operacao_id": id_operacao, "senha": "NovaSenha!"}, headers={"X-CSRF-Token": csrf})
            self.assertEqual(resposta.status_code, 200, resposta.text)
            self.assertEqual(resposta.json()["estado"], "salvo")
            self.assertEqual(resposta.json()["alcance"], "global")
            self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)
        self.assertEqual((await self.entrar()).status_code, 401)
        self.assertEqual((await self.entrar("NovaSenha!")).status_code, 200)
        async with self.gs_real(realm):
            recuperado = await self.cliente.get(f"/api/realms/{realm}/operacoes/{id_operacao}")
            self.assertEqual(recuperado.json(), resposta.json())
            csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
            repetido = await self.cliente.post(f"/api/realms/{realm}/contas/{self.conta_id}/senha",
                json={"operacao_id": id_operacao, "senha": "NovaSenha!"}, headers={"X-CSRF-Token": csrf})
            self.assertEqual(repetido.json(), resposta.json())
            conflito = await self.cliente.post(f"/api/realms/{realm}/contas/{self.conta_id}/senha",
                json={"operacao_id": id_operacao, "senha": "OutraSenha!"}, headers={"X-CSRF-Token": csrf})
            self.assertEqual(conflito.status_code, 409)
            self.assertEqual(conflito.json()["codigo"], "operacao_em_conflito")

    async def test_perda_de_resposta_real_nao_repete_escrita_e_recupera_apos_relogin(self):
        realm, _ = await self.criar_personagem()
        id_operacao = uuid.uuid4().hex
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async with self.gs_real(realm) as porta_gs:
            async def cortar_resposta(leitor, escritor):
                gs_leitor, gs_escritor = await asyncio.open_connection("127.0.0.1", porta_gs)
                try:
                    tamanho = struct.unpack("!I", await gs_leitor.readexactly(4))[0]
                    escritor.write(struct.pack("!I", tamanho) + await gs_leitor.readexactly(tamanho))
                    await escritor.drain()
                    tamanho = struct.unpack("!I", await leitor.readexactly(4))[0]
                    gs_escritor.write(struct.pack("!I", tamanho) + await leitor.readexactly(tamanho + 32))
                    await gs_escritor.drain()
                    # Daemon confirmou commit, mas o painel perde quadro/MAC.
                    tamanho = struct.unpack("!I", await gs_leitor.readexactly(4))[0]
                    await gs_leitor.readexactly(tamanho + 32)
                finally:
                    gs_escritor.close()
                    escritor.close()
                    await gs_escritor.wait_closed()
                    await escritor.wait_closed()
            proxy = await asyncio.start_server(cortar_resposta, "127.0.0.1", 0)
            self.servidores.append(proxy)
            os.environ["ADMIN_DAEMONS"] = json.dumps({realm: [{"host": "127.0.0.1", "porta": proxy.sockets[0].getsockname()[1]}]})
            resposta = await self.cliente.post(f"/api/realms/{realm}/contas/{self.conta_id}/senha",
                json={"operacao_id": id_operacao, "senha": "RespostaPerdida!"}, headers={"X-CSRF-Token": csrf})
            self.assertEqual(resposta.status_code, 202, resposta.text)
            self.assertEqual(resposta.json()["estado"], "desconhecido")
        self.assertEqual((await self.entrar("RespostaPerdida!")).status_code, 200)
        async with self.gs_real(realm):
            recuperado = await self.cliente.get(f"/api/realms/{realm}/operacoes/{id_operacao}")
            self.assertEqual(recuperado.json()["estado"], "salvo")
            self.assertEqual(recuperado.json()["conta_id"], self.conta_id)

    async def test_consulta_sem_daemon_identifica_persistencia_e_presenca_desconhecida(self):
        realm, personagem = await self.criar_personagem()
        dados = await self.detalhe(realm, personagem)
        self.assertEqual(dados["origem"], "persistida")
        self.assertEqual(dados["presenca"], "desconhecida")
        self.assertFalse(dados["edicao_disponivel"])

    async def test_personagem_de_outro_realm_nao_chega_ao_daemon(self):
        realm, personagem = await self.criar_personagem()
        outro = f"admin_outro_{self.sufixo}"
        async with self.pool.acquire() as conexao:
            await conexao.execute("INSERT INTO realms(id,name,version,host,port) VALUES($1,'Outro','1.2.6','127.0.0.1',1)", outro)
        self.realms_criados.append(outro)
        await self.ativar_daemon(outro)
        resposta = await self.cliente.get(f"/api/realms/{outro}/personagens/{personagem}")
        self.assertEqual(resposta.status_code, 404)
        self.assertEqual(self.pedidos_admin, [])

    async def test_id_personagem_fora_do_tipo_sql_recusado(self):
        realm, _ = await self.criar_personagem()
        for identificador in (0, 2_147_483_648):
            resposta = await self.cliente.get(f"/api/realms/{realm}/personagens/{identificador}")
            self.assertEqual(resposta.status_code, 422)

    async def test_consulta_viva_substitui_ficha_persistida_sem_gravar(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "online", {"id": personagem, "nome": "Vivo", "nivel": 44, "exp": "9007199254740993"})
        dados = await self.detalhe(realm, personagem)
        self.assertEqual(dados["origem"], "viva")
        self.assertEqual(dados["ficha"]["nivel"], 44)
        self.assertEqual(dados["ficha"]["exp"], "9007199254740993")
        self.assertEqual(self.pedidos_admin[0]["administrador_id"], self.conta_id)
        async with self.pool.acquire() as conexao:
            self.assertEqual(await conexao.fetchval("SELECT level FROM characters WHERE id=$1", personagem), 1)

    async def test_ausencia_observada_nao_habilita_edicao_offline(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm)
        dados = await self.detalhe(realm, personagem)
        self.assertEqual(dados["presenca"], "ausente_nos_daemons")
        self.assertEqual(dados["origem"], "persistida")
        self.assertFalse(dados["edicao_disponivel"])

    async def test_transicao_nao_parece_offline(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "em_transicao")
        self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "em_transicao")

    async def test_entidade_residual_nao_parece_offline(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "inconsistente")
        dados = await self.detalhe(realm, personagem)
        self.assertEqual(dados["presenca"], "desconhecida")
        self.assertFalse(dados["edicao_disponivel"])

    async def test_assinatura_incorreta_daemon_nao_produz_dado_vivo(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "online", {"id": personagem}, erro="assinatura")
        dados = await self.detalhe(realm, personagem)
        self.assertEqual(dados["presenca"], "desconhecida")
        self.assertEqual(dados["origem"], "persistida")

    async def test_resposta_outro_realm_recusada(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "online", {"id": personagem}, erro="realm")
        self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "desconhecida")

    async def test_resposta_outro_personagem_recusada(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "online", {"id": personagem + 1})
        self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "desconhecida")

    async def test_presenca_duplicada_em_dois_daemons_recusada(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, "online", {"id": personagem})
        await self.ativar_daemon(realm, "online", {"id": personagem}, acrescentar=True)
        self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "desconhecida")

    async def test_timeout_nao_parece_daemon_vazio(self):
        realm, personagem = await self.criar_personagem()
        await self.ativar_daemon(realm, erro="timeout")
        self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "desconhecida")

    async def test_mapas_e_online_vem_do_daemon(self):
        realm, _ = await self.criar_personagem()
        await self.ativar_daemon(realm, mundos=[{"mapa": 1, "jogadores_online": 3}, {"mapa": 161, "jogadores_online": 2}])
        resposta = await self.cliente.get(f"/api/realms/{realm}/estado")
        self.assertEqual(resposta.status_code, 200)
        self.assertEqual(resposta.json()["jogadores_online"], 5)
        self.assertEqual(resposta.json()["origem"], "viva")

    async def test_busca_personagens_filtra_realm(self):
        realm, personagem = await self.criar_personagem()
        resposta = await self.cliente.get(f"/api/realms/{realm}/personagens?busca=Personagem")
        self.assertEqual(resposta.status_code, 200)
        self.assertEqual([p["id"] for p in resposta.json()["personagens"]], [personagem])

    async def test_api_python_consulta_binario_gs_real_com_banco_isolado(self):
        realm, personagem = await self.criar_personagem()
        with socket.socket() as porta:
            porta.bind(("127.0.0.1", 0))
            numero = porta.getsockname()[1]
        chave = os.urandom(32).hex()
        os.environ["ADMIN_DAEMONS"] = json.dumps({realm: [{"host": "127.0.0.1", "porta": numero}]})
        os.environ["ADMIN_CHANNEL_SECRETS"] = json.dumps({realm: chave})
        binario = os.getenv("PW_GS_TESTE", str(Path(__file__).resolve().parents[3] / "target/debug/pw-gs.exe"))
        if os.name != "nt" and not os.getenv("PW_GS_TESTE"):
            binario = binario.removesuffix(".exe")
        with tempfile.TemporaryDirectory(prefix="pw-admin-gs-") as dados:
            ambiente = dict(os.environ, DATABASE_URL=os.environ["TEST_DATABASE_URL"], REALM_ID=realm,
                            GAME_VERSION="1.2.6", WORLD_TAGS="1,161", CONFIG_DIR=dados,
                            ADMIN_SECRET=chave, ADMIN_LISTEN=f"127.0.0.1:{numero}",
                            BUS_LISTEN="127.0.0.1:0", RUST_LOG="error")
            processo = await asyncio.create_subprocess_exec(binario, env=ambiente,
                                                            stdout=asyncio.subprocess.DEVNULL,
                                                            stderr=asyncio.subprocess.DEVNULL)
            try:
                conectado = False
                for _ in range(100):
                    if processo.returncode is not None:
                        self.fail(f"GS encerrou com código {processo.returncode}")
                    try:
                        _, escritor = await asyncio.open_connection("127.0.0.1", numero)
                        escritor.close()
                        await escritor.wait_closed()
                        conectado = True
                        break
                    except OSError:
                        await asyncio.sleep(0.05)
                self.assertTrue(conectado, "GS não abriu o canal administrativo")
                resposta = await self.cliente.get(f"/api/realms/{realm}/estado")
                self.assertEqual(resposta.status_code, 200, resposta.text)
                self.assertEqual(resposta.json()["estado"], "consultado")
                self.assertEqual(resposta.json()["mapas"], [{"mapa": 1, "jogadores_online": 0, "ligado": True}, {"mapa": 161, "jogadores_online": 0, "ligado": True}])
                self.assertEqual((await self.detalhe(realm, personagem))["presenca"], "ausente_nos_daemons")
            finally:
                if processo.returncode is None:
                    processo.terminate()
                await processo.wait()

    async def test_api_recusa_anonimo_inclusive_rotas_antigas(self):
        for caminho in ("/api/realms", "/api/accounts/list", "/api/characters/1", "/api/elements/search-items"):
            self.assertEqual((await self.cliente.get(caminho)).status_code, 401)
        self.assertEqual((await self.cliente.post("/api/accounts/grant-gold", json={})).status_code, 401)

    async def test_login_gm_reutiliza_verificador_e_cookie_protegido(self):
        resposta = await self.entrar()
        self.assertEqual(resposta.status_code, 200, resposta.text)
        cookie = resposta.headers["set-cookie"]
        for atributo in ("HttpOnly", "Secure", "SameSite=strict"):
            self.assertIn(atributo, cookie)
        sessao = (await self.cliente.get("/api/sessao")).json()
        self.assertEqual(sessao["usuario"], self.usuario)
        self.assertNotIn("senha", sessao)
        self.assertNotIn("hash", sessao)

    async def test_senha_incorreta(self):
        self.assertEqual((await self.entrar("errada")).status_code, 401)
        self.assertNotIn(COOKIE, self.cliente.cookies)

    async def test_token_falso_recusado(self):
        self.cliente.cookies.set(COOKIE, "inventado")
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)

    async def test_sessao_expirada(self):
        await self.entrar()
        chaves = [chave async for chave in self.redis.scan_iter(match=f"{self.prefixo}sessoes:*")]
        await self.redis.expire(chaves[0], 0)
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)

    async def test_login_excessivo_nao_ecoado(self):
        resposta = await self.cliente.post("/api/sessao/entrar", content=b"S" * 8193,
                                          headers={"Content-Type": "application/json"})
        self.assertEqual(resposta.status_code, 413)

    async def test_login_em_chunks_limitado_sem_content_length(self):
        async def corpo():
            yield b"S" * 4096
            yield b"S" * 4097
            raise AssertionError("O servidor leu além do limite.")
        resposta = await self.cliente.post("/api/sessao/entrar", content=corpo(),
                                          headers={"Content-Type": "application/json"})
        self.assertEqual(resposta.status_code, 413)

    async def test_sem_gm(self):
        await self.modificar_conta("gm_privileges", 0)
        self.assertEqual((await self.entrar()).status_code, 401)

    async def test_conta_banida(self):
        await self.modificar_conta("is_banned", True)
        self.assertEqual((await self.entrar()).status_code, 401)

    async def test_gm_revogado_encerra_sessao(self):
        await self.entrar()
        await self.modificar_conta("gm_privileges", 0)
        self.assertEqual((await self.cliente.get("/api/realms")).status_code, 401)

    async def test_banimento_encerra_sessao(self):
        await self.entrar()
        await self.modificar_conta("is_banned", True)
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)

    async def test_troca_senha_encerra_sessao(self):
        await self.entrar()
        await self.modificar_conta("password_hash", "outro-hash")
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)

    async def test_csrf_logout_e_replay(self):
        await self.entrar()
        token = self.cliente.cookies.get(COOKIE)
        self.assertEqual((await self.cliente.post("/api/sessao/sair")).status_code, 403)
        sessao = (await self.cliente.get("/api/sessao")).json()
        resposta = await self.cliente.post("/api/sessao/sair", headers={"X-CSRF-Token": sessao["csrf"]})
        self.assertEqual(resposta.status_code, 204)
        self.cliente.cookies.set(COOKIE, token)
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 401)

    async def test_novo_login_invalida_token_anterior(self):
        await self.entrar()
        antigo = self.cliente.cookies.get(COOKIE)
        await self.entrar()
        async with httpx.AsyncClient(transport=httpx.ASGITransport(app=self.app), base_url="https://painel.test",
                                     cookies={COOKIE: antigo}) as outro:
            self.assertEqual((await outro.get("/api/sessao")).status_code, 401)

    async def test_origem_cruzada_recusada(self):
        resposta = await self.cliente.post("/api/sessao/entrar", headers={"Origin": "https://outro.test"},
                                          json={"usuario": self.usuario, "senha": self.senha})
        self.assertEqual(resposta.status_code, 403)
        await self.entrar()
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        resposta = await self.cliente.post("/api/sessao/sair", headers={"Origin": "https://outro.test", "X-CSRF-Token": csrf})
        self.assertEqual(resposta.status_code, 403)

    async def test_escrita_antiga_nao_aplica_nem_informa_sucesso(self):
        await self.entrar()
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        resposta = await self.cliente.post("/api/accounts/grant-gold", headers={"X-CSRF-Token": csrf},
                                          json={"account_id": self.conta_id, "amount": 100})
        self.assertEqual(resposta.status_code, 501)
        async with self.pool.acquire() as conexao:
            saldo = await conexao.fetchval("SELECT gold_balance FROM accounts WHERE id=$1", self.conta_id)
        self.assertEqual(saldo, 0)

    async def test_limite_tentativas(self):
        for _ in range(10):
            self.assertEqual((await self.entrar("errada")).status_code, 401)
        self.assertEqual((await self.entrar()).status_code, 429)

    async def test_verificador_indisponivel(self):
        with patch.dict(os.environ, {"PW_VALIDADOR_CREDENCIAIS": "arquivo-inexistente-do-teste"}):
            self.assertEqual((await self.entrar()).status_code, 503)

    async def test_redis_indisponivel_recusa_sem_sucesso(self):
        redis = aioredis.from_url("redis://127.0.0.1:1", socket_connect_timeout=0.1)
        self.app.state.seguranca.redis = redis
        try:
            self.assertEqual((await self.entrar()).status_code, 503)
            self.assertNotIn(COOKIE, self.cliente.cookies)
        finally:
            self.app.state.seguranca.redis = self.redis
            await redis.aclose()

    async def test_validacao_nao_devolve_senha(self):
        resposta = await self.cliente.post("/api/sessao/entrar", json={"usuario": "", "senha": "SEGREDO"})
        self.assertEqual(resposta.status_code, 422)
        self.assertNotIn("SEGREDO", resposta.text)

    async def test_realms_alvo_correto_versao_desconhecida_sem_presenca_falsa(self):
        for indice, versao in enumerate(("1.2.6", "1.5.5", "9.9.9")):
            realm = f"admin_{self.sufixo}_{indice}"
            async with self.pool.acquire() as conexao:
                await conexao.execute("INSERT INTO realms(id,name,version,host,port) VALUES($1,$2,$3,'127.0.0.1',1)",
                                     realm, f"Realm {indice}", versao)
            self.realms_criados.append(realm)
        await self.entrar()
        resposta = await self.cliente.get("/api/realms")
        self.assertEqual(resposta.status_code, 200, resposta.text)
        realms = {realm["id"]: realm for realm in resposta.json()["realms"]}
        for indice, identificador in enumerate(self.realms_criados):
            realm = realms[identificador]
            self.assertEqual(realm["nome"], f"Realm {indice}")
            self.assertIsNone(realm["jogadores_online"])
            self.assertEqual(realm["mundo"], "desconhecido")
            self.assertEqual(realm["gateway"], "inacessivel")
            protocolo = next(cap for cap in realm["capacidades"] if cap["id"] == "protocolo")
            self.assertEqual(protocolo["estado"], "nao_validado" if indice == 2 else "implementado")

    async def test_interface_unica_com_csp_e_sem_cdns(self):
        resposta = await self.cliente.get("/")
        self.assertEqual(resposta.status_code, 200)
        self.assertIn("frame-ancestors 'none'", resposta.headers["content-security-policy"])
        self.assertNotIn("https://", resposta.text)
        self.assertEqual((await self.cliente.get("/docs")).status_code, 404)


    async def test_gm_api_exige_csrf_booleano_estrito_e_alvo_correto(self):
        realm, _ = await self.criar_personagem()
        caminho = f"/api/realms/{realm}/contas/{self.conta_id}/gm"
        pedido = {"operacao_id": uuid.uuid4().hex, "habilitado": True}
        self.assertEqual((await self.cliente.post(caminho, json=pedido)).status_code, 403)
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        for valor in (1, "true", None):
            self.assertEqual((await self.cliente.post(caminho, json={**pedido, "habilitado": valor}, headers={"X-CSRF-Token": csrf})).status_code, 422)
        resposta = await self.cliente.post(caminho, json=pedido, headers={"X-CSRF-Token": csrf})
        self.assertEqual(resposta.status_code, 503)  # canal sem chave: nada enviado (B174)
        self.assertEqual(resposta.json()["estado"], "falha")
        await self.modificar_conta("gm_privileges", 0)
        self.assertEqual((await self.cliente.post(caminho, json=pedido, headers={"X-CSRF-Token": csrf})).status_code, 401)

    async def test_gold_ban_e_desconectar_validam_antes_do_envio(self):
        """E4 (B175): CSRF, valores estritos e canal ausente = falha definitiva (nada enviado)."""
        realm, _ = await self.criar_personagem()
        base = f"/api/realms/{realm}/contas/{self.conta_id}"
        gold = {"operacao_id": uuid.uuid4().hex, "delta": 500}
        self.assertEqual((await self.cliente.post(f"{base}/gold", json=gold)).status_code, 403)
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        for delta in (0, -500, "500", 1.5, 2_147_483_648):
            self.assertEqual((await self.cliente.post(f"{base}/gold", json={**gold, "delta": delta}, headers=csrf)).status_code, 422)
        resposta = await self.cliente.post(f"{base}/gold", json=gold, headers=csrf)
        self.assertEqual((resposta.status_code, resposta.json()["codigo"]), (503, "canal_nao_enviado"))
        ban = {"operacao_id": uuid.uuid4().hex, "banida": True, "motivo": "teste"}
        for invalido in ({"banida": "sim"}, {"motivo": "a" * 121}, {"motivo": "linha" + chr(10) + "quebrada"}):
            self.assertEqual((await self.cliente.post(f"{base}/ban", json={**ban, **invalido}, headers=csrf)).status_code, 422)
        resposta = await self.cliente.post(f"{base}/ban", json=ban, headers=csrf)
        self.assertEqual((resposta.status_code, resposta.json()["codigo"]), (503, "canal_nao_enviado"))
        caminho = f"/api/contas/{self.conta_id}/desconectar"
        self.assertEqual((await self.cliente.post(caminho)).status_code, 403)
        self.assertEqual((await self.cliente.post(caminho, headers=csrf)).status_code, 503)
        # Nada foi gravado: a conta segue sem ban e com o mesmo saldo.
        async with self.pool.acquire() as conexao:
            banida, saldo = await conexao.fetchrow("SELECT is_banned, gold_balance FROM accounts WHERE id=$1", self.conta_id)
        self.assertEqual((banida, saldo), (False, 0))

    async def test_rates_validam_limites_e_sem_canal_nada_grava(self):
        """E7 (B176): NUMERIC(3,1) = 0,1 a 99,9; sem canal, falha sem tocar em realms."""
        realm, _ = await self.criar_personagem()
        caminho = f"/api/realms/{realm}/rates"
        pedido = {"exp": 2.0, "sp": 1.5, "drop": 1.5, "moedas": 3.0}
        self.assertEqual((await self.cliente.post(caminho, json=pedido)).status_code, 403)
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        for invalido in ({"exp": 0}, {"sp": 100}, {"drop": "x"}, {"outra": 1.0}):
            self.assertEqual((await self.cliente.post(caminho, json={**pedido, **invalido}, headers=csrf)).status_code, 422)
        resposta = await self.cliente.post(caminho, json=pedido, headers=csrf)
        self.assertEqual((resposta.status_code, resposta.json()["estado"]), (503, "falha"))
        async with self.pool.acquire() as conexao:
            exp = await conexao.fetchval("SELECT double_exp_multiplier FROM realms WHERE id=$1", realm)
        self.assertEqual(float(exp), 1.0)

    async def test_icone_recortado_do_atlas_do_cliente(self):
        """B188: PNG 32×32 do atlas (data/icones); hexadecimal do nome GBK; 404 fora do atlas."""
        await self.criar_personagem()
        titulo = "钢刀.dds".encode("gbk").hex()
        resposta = await self.cliente.get(f"/api/icones/m/{titulo}.png")
        if resposta.status_code == 503:
            self.skipTest("atlas de ícones ausente em data/icones")
        self.assertEqual((resposta.status_code, resposta.headers["content-type"]), (200, "image/png"), resposta.text)
        self.assertEqual(resposta.content[:8], bytes.fromhex("89504e470d0a1a0a"))
        self.assertEqual(int.from_bytes(resposta.content[16:20], "big"), 32)
        self.assertEqual((await self.cliente.get(f"/api/icones/m/{'nao-existe'.encode().hex()}.png")).status_code, 404)
        self.assertEqual((await self.cliente.get(f"/api/icones/x/{titulo}.png")).status_code, 422)
        self.assertEqual((await self.cliente.get("/api/icones/m/zz.png")).status_code, 422)
        self.cliente.cookies.clear()
        self.assertEqual((await self.cliente.get(f"/api/icones/m/{titulo}.png")).status_code, 401)

    async def test_inventario_e_busca_de_itens_exigem_canal_e_parametros(self):
        """B186: leitura pelo primeiro GS; sem canal 503; busca exige 1–64 caracteres."""
        realm, personagem = await self.criar_personagem()
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/itens")).status_code, 422)
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/itens?busca=" + "x" * 65)).status_code, 422)
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/personagens/0/inventario")).status_code, 422)
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/itens?busca=po")).status_code, 503)
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/personagens/{personagem}/inventario")).status_code, 503)
        self.assertEqual((await self.cliente.get(f"/api/realms/inexistente/itens?busca=po")).status_code, 404)

    async def test_catalogo_de_mapas_por_versao(self):
        """B183: todos os mapas da versão do realm, do gs.conf original (126: 43; 155: 79)."""
        realm, _ = await self.criar_personagem()
        dados = (await self.cliente.get(f"/api/realms/{realm}/catalogo-mapas")).json()
        self.assertEqual((dados["versao"], len(dados["mapas"])), ("1.2.6", 43))
        self.assertEqual(dados["mapas"][0], {"mapa": 1, "chave": "gs01", "pasta": "world", "instancia": False, "nome": "Mundo"})
        async with self.pool.acquire() as conexao:
            await conexao.execute("UPDATE realms SET version='1.5.5' WHERE id=$1", realm)
        dados = (await self.cliente.get(f"/api/realms/{realm}/catalogo-mapas")).json()
        self.assertEqual(len(dados["mapas"]), 79)
        self.assertIn({"mapa": 161, "chave": "is61", "pasta": "a61", "instancia": True, "nome": "Vale Celestial (Inicial 1.5.3)"}, dados["mapas"])
        self.assertEqual((await self.cliente.get("/api/realms/inexistente/catalogo-mapas")).status_code, 404)

    async def test_ligar_mapa_nao_servido_vai_ao_primeiro_daemon_com_carregar(self):
        """B183: nenhum daemon serve o mapa → o primeiro recebe `carregar` e monta; o estado
        traz os mapas que algum daemon sabe montar."""
        realm, _ = await self.criar_personagem()
        await self.ativar_daemon(realm, mundos=[{"mapa": 1, "jogadores_online": 0, "ligado": True}])
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        resposta = await self.cliente.post(f"/api/realms/{realm}/mapas/105", json={"ligado": True}, headers=csrf)
        self.assertEqual(resposta.status_code, 200, resposta.text)
        self.assertEqual((resposta.json()["carregando"], resposta.json()["mapa"]), (True, 105))
        definir = [p["consulta"] for p in self.pedidos_admin if p["consulta"]["tipo"] == "definir_mapa"]
        self.assertEqual([c.get("carregar", False) for c in definir], [False, True])
        estado = (await self.cliente.get(f"/api/realms/{realm}/estado")).json()
        self.assertEqual(estado["carregaveis"], [1, 105])

    async def test_mapas_exigem_csrf_booleano_e_canal(self):
        """E7 (B177): ligar/desligar mapa — CSRF, booleano estrito, sem canal não muda nada."""
        realm, _ = await self.criar_personagem()
        caminho = f"/api/realms/{realm}/mapas/1"
        self.assertEqual((await self.cliente.post(caminho, json={"ligado": False})).status_code, 403)
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        for invalido in ({"ligado": "false"}, {"ligado": 0}, {}, {"ligado": False, "extra": 1}):
            self.assertEqual((await self.cliente.post(caminho, json=invalido, headers=csrf)).status_code, 422)
        self.assertEqual((await self.cliente.post(f"/api/realms/{realm}/mapas/0", json={"ligado": False}, headers=csrf)).status_code, 422)
        resposta = await self.cliente.post(caminho, json={"ligado": False}, headers=csrf)
        self.assertEqual((resposta.status_code, resposta.json()["estado"]), (503, "falha"))
        async with self.pool.acquire() as conexao:
            config = await conexao.fetchval("SELECT config->'mapas_desligados' FROM realms WHERE id=$1", realm)
        self.assertIsNone(config)

    async def test_editar_personagem_valida_antes_do_envio(self):
        """E5 (B179, B182): um só de dinheiro, exp/sp, pontos, nível ou cultivo; limites; CSRF; sem canal nada muda."""
        realm, personagem = await self.criar_personagem()
        caminho = f"/api/realms/{realm}/personagens/{personagem}/editar"
        base = {"operacao_id": uuid.uuid4().hex}
        self.assertEqual((await self.cliente.post(caminho, json={**base, "dinheiro": 10})).status_code, 403)
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        for invalido in ({}, {"dinheiro": 0}, {"dinheiro": -10}, {"dinheiro": 5, "exp": 1}, {"exp": -1},
                         {"dinheiro": 2_000_000_001}, {"dinheiro": "10"}, {"sp": 1.5},
                         # B182: um só tipo por operação; pontos 1–10 000, nível ≥ 2, cultivo 0–255.
                         {"pontos": 0}, {"pontos": 10_001}, {"nivel": 1}, {"cultivo": -1}, {"cultivo": 256},
                         {"pontos": 1, "nivel": 5}, {"cultivo": 3, "exp": 1}, {"dinheiro": 1, "pontos": 1},
                         # B184: quatro atributos 0–100 000; redistribuir só `true`; um tipo só.
                         {"atributos": [1, 2, 3]}, {"atributos": [1, 2, 3, 4, 5]}, {"atributos": [1, 2, 3, -1]},
                         {"atributos": [1, 2, 3, 100_001]}, {"redistribuir": False}, {"redistribuir": True, "pontos": 1},
                         {"atributos": [5, 5, 5, 5], "redistribuir": True},
                         # B185: posição com mapa ≥ 1, x/z obrigatórios, ±100 000, sem campo extra.
                         {"posicao": {"mapa": 0, "x": 1.0, "z": 1.0}}, {"posicao": {"mapa": 1, "x": 1.0}},
                         {"posicao": {"mapa": 1, "x": 200_000.0, "z": 1.0}}, {"posicao": {"mapa": 1, "x": 1.0, "z": 1.0, "w": 1}},
                         {"posicao": {"mapa": 1, "x": "1", "z": 1.0}}, {"posicao": {"mapa": 1, "x": 1.0, "z": 1.0}, "pontos": 1},
                         # B186: item com id ≥ 1 e quantidade 1–100 000, sem campo extra.
                         {"item": {"id": 0, "quantidade": 1}}, {"item": {"id": 1, "quantidade": 0}},
                         {"item": {"id": 1, "quantidade": 100_001}}, {"item": {"id": 1}},
                         {"item": {"id": 1, "quantidade": 1, "slot": 2}}, {"item": {"id": 1, "quantidade": 1}, "pontos": 1},
                         # B187: remover com recipiente conhecido, slot 0–255, id ≥ 1.
                         {"remover_item": {"recipiente": "bau", "slot": 0, "id": 1}},
                         {"remover_item": {"recipiente": "bolsa", "slot": 256, "id": 1}},
                         {"remover_item": {"recipiente": "bolsa", "slot": 0, "id": 0}},
                         {"remover_item": {"recipiente": "bolsa", "slot": 0, "id": 1, "quantidade": 0}},
                         {"remover_item": {"recipiente": "bolsa", "slot": 0}}):
            self.assertEqual((await self.cliente.post(caminho, json={**base, **invalido}, headers=csrf)).status_code, 422, invalido)
        for valido in ({"dinheiro": 10}, {"pontos": 5}, {"nivel": 30}, {"cultivo": 0},
                       {"atributos": [5, 5, 5, 5]}, {"redistribuir": True},
                       {"posicao": {"mapa": 1, "x": -319.5, "z": -900.0}}, {"posicao": {"mapa": 161, "x": 1, "y": 2.5, "z": 3}},
                       {"item": {"id": 3001, "quantidade": 150}},
                       {"remover_item": {"recipiente": "armazem", "slot": 4, "id": 3001}},
                       {"remover_item": {"recipiente": "bolsa", "slot": 0, "id": 3001, "quantidade": 5}}):
            resposta = await self.cliente.post(caminho, json={**base, **valido}, headers=csrf)
            self.assertEqual((resposta.status_code, resposta.json()["codigo"]), (503, "canal_nao_enviado"), valido)

    @staticmethod
    def inteiro_gnet(valor):
        if valor < 0x80: return bytes([valor])
        if valor < 0x4000: return struct.pack("!H", valor | 0x8000)
        if valor < 0x20000000: return struct.pack("!I", valor | 0xc0000000)
        return b"\xe0" + struct.pack("!I", valor)

    async def ler_inteiro_gnet(self, leitor):
        primeiro = (await leitor.readexactly(1))[0]
        if primeiro < 0x80: return primeiro
        if primeiro < 0xc0: return ((primeiro & 0x3f) << 8) | (await leitor.readexactly(1))[0]
        if primeiro < 0xe0: return int.from_bytes(bytes([primeiro & 0x1f]) + await leitor.readexactly(3), "big")
        return int.from_bytes(await leitor.readexactly(4), "big")

    async def pacote_jogo(self, escritor, opcode, corpo):
        escritor.write(self.inteiro_gnet(opcode) + self.inteiro_gnet(len(corpo)) + corpo)
        await escritor.drain()

    async def esperar_pacote_jogo(self, leitor, opcode):
        async def ler():
            for _ in range(400):
                op = await self.ler_inteiro_gnet(leitor)
                tamanho = await self.ler_inteiro_gnet(leitor)
                self.assertLess(tamanho, 1_048_576)
                corpo = await leitor.readexactly(tamanho)
                if op == opcode: return corpo
            self.fail("Pacote de jogo não chegou")
        return await asyncio.wait_for(ler(), timeout=5)

    @staticmethod
    def porta_descartavel():
        with socket.socket() as p:
            p.bind(("127.0.0.1", 0)); return p.getsockname()[1]

    @asynccontextmanager
    async def daemon_gm_real(self, tipo, realm, versao, identidade, alvos, barramento=None):
        porta = self.porta_descartavel()
        bus = self.porta_descartavel()
        raiz = Path(__file__).resolve().parents[3]
        binario = raiz / "target/debug" / (f"pw-{tipo}.exe" if os.name == "nt" else f"pw-{tipo}")
        with tempfile.TemporaryDirectory(prefix="pw-admin-gm-") as dados:
            Path(dados, "config").mkdir()
            ambiente = dict(os.environ, DATABASE_URL=os.environ["TEST_DATABASE_URL"],
                REDIS_URL=os.environ["TEST_REDIS_URL"], REALM_ID=realm, GAME_VERSION=versao,
                ADMIN_COORDENACAO_ID=identidade, ADMIN_COORDENACAO_ALVOS=",".join(alvos),
                CONFIG_DIR=dados, WORLD_TAGS="1,161", RUST_LOG="error")
            if tipo == "gs":
                chave = os.urandom(32).hex()
                ambiente.update(ADMIN_SECRET=chave, ADMIN_LISTEN=f"127.0.0.1:{porta}", BUS_LISTEN=f"127.0.0.1:{bus}")
                daemons = json.loads(os.environ.get("ADMIN_DAEMONS", "{}")); daemons[realm] = [{"host":"127.0.0.1", "porta":porta}]
                chaves = json.loads(os.environ.get("ADMIN_CHANNEL_SECRETS", "{}")); chaves[realm] = chave
                os.environ["ADMIN_DAEMONS"] = json.dumps(daemons); os.environ["ADMIN_CHANNEL_SECRETS"] = json.dumps(chaves)
            else:
                ambiente.update(GATEWAY_PORT=str(porta), GS_BUS=f"127.0.0.1:{barramento}")
            processo = await asyncio.create_subprocess_exec(str(binario), env=ambiente, cwd=dados,
                stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL)
            try:
                for _ in range(200):
                    if processo.returncode is not None: self.fail(f"{tipo} encerrou: {processo.returncode}")
                    try:
                        _, escritor = await asyncio.open_connection("127.0.0.1", porta)
                        escritor.close(); await escritor.wait_closed(); break
                    except OSError: await asyncio.sleep(0.025)
                else: self.fail(f"{tipo} não abriu a porta")
                yield porta, bus, processo
            finally:
                if processo.returncode is None: processo.terminate()
                await processo.wait()

    async def test_gm_global_api_links_gs_reais_126_155_sessoes_abertas_e_reinicio(self):
        from contextlib import AsyncExitStack
        realm1, _ = await self.criar_personagem()
        realm2 = f"admin_{self.sufixo}_155"
        nome = self.usuario + "_2"
        senha = "Credencial!"
        async with self.pool.acquire() as c:
            await c.execute("INSERT INTO realms(id,name,version,host,port) VALUES($1,'GM155','1.5.5','127.0.0.1',1)", realm2)
            alvo = await c.fetchval("INSERT INTO accounts(username,password_hash) VALUES($1,$2) RETURNING id", nome, hashlib.md5((nome+senha).encode()).hexdigest())
            personagens = []
            for realm in (realm1, realm2):
                personagens.append(await c.fetchval("INSERT INTO characters(account_id,realm_id,name,race,cls,gender) VALUES($1,$2,'GMCoordenado',0,0,0) RETURNING id", alvo, realm))
        self.realms_criados.append(realm2); self.contas_criadas.append(alvo)
        alvos = [f"teste-{t}-{v}-{self.sufixo}" for v in (126,155) for t in ("link","gs")]
        csrf = (await self.cliente.get("/api/sessao")).json()["csrf"]
        async def comandar(realm, habilitado, id_op):
            return await self.cliente.post(f"/api/realms/{realm}/contas/{alvo}/gm", json={"operacao_id":id_op,"habilitado":habilitado}, headers={"X-CSRF-Token":csrf})
        async def aplicado(realm, id_op):
            for _ in range(200):
                resposta = await self.cliente.get(f"/api/realms/{realm}/operacoes/{id_op}")
                if resposta.json()["estado"]=="aplicado": return resposta.json()
                await asyncio.sleep(0.025)
            self.fail(f"GM não foi reconciliado: {resposta.json()}")
        async def ficha(realm, personagem):
            return (await self.detalhe(realm,personagem))["ficha"]
        async def estado_gm(realm, personagem, campo, esperado):
            for _ in range(100):
                f = await ficha(realm,personagem)
                if f.get(campo)==esperado: return f
                await asyncio.sleep(0.025)
            self.fail(f"{campo} não mudou: {f}")
        try:
            async with AsyncExitStack() as pilha:
                gs1 = await pilha.enter_async_context(self.daemon_gm_real("gs",realm1,"1.2.6",alvos[1],alvos))
                gs2 = await pilha.enter_async_context(self.daemon_gm_real("gs",realm2,"1.5.5",alvos[3],alvos))
                l1 = await pilha.enter_async_context(self.daemon_gm_real("link",realm1,"1.2.6",alvos[0],alvos,gs1[1]))
                l2 = await pilha.enter_async_context(self.daemon_gm_real("link",realm2,"1.5.5",alvos[2],alvos,gs2[1]))
                sessoes=[]
                for realm, versao, link, personagem in ((realm1,"1.2.6",l1,personagens[0]),(realm2,"1.5.5",l2,personagens[1])):
                    leitor, escritor = await asyncio.open_connection("127.0.0.1",link[0]); sessoes.append((realm,personagem,leitor,escritor))
                    desafio = await self.esperar_pacote_jogo(leitor,1); nonce = desafio[1:17]
                    prova = hmac.digest(hashlib.md5((nome+senha).encode()).digest(),nonce,"md5")
                    await self.pacote_jogo(escritor,2 if versao=="1.2.6" else 3,bytes([len(nome)])+nome.encode()+bytes([16])+prova+b"\0\0")
                    self.assertEqual(struct.unpack("!i",(await self.esperar_pacote_jogo(leitor,4))[:4])[0],alvo)
                    await self.pacote_jogo(escritor,70,struct.pack("!ib",personagem,0))
                    auth=await self.esperar_pacote_jogo(leitor,71); self.assertEqual(auth[4],0)
                    await self.pacote_jogo(escritor,72,struct.pack("!iiiiiI",personagem,1,0,60,0,77))
                    await estado_gm(realm,personagem,"gm",0)
                async def login_concorrente(link, versao):
                    leitor, escritor = await asyncio.open_connection("127.0.0.1",link[0])
                    desafio = await self.esperar_pacote_jogo(leitor,1)
                    prova = hmac.digest(hashlib.md5((nome+senha).encode()).digest(),desafio[1:17],"md5")
                    await self.pacote_jogo(escritor,2 if versao=="1.2.6" else 3,bytes([len(nome)])+nome.encode()+bytes([16])+prova+b"\0\0")
                    self.assertEqual(struct.unpack("!i",(await self.esperar_pacote_jogo(leitor,4))[:4])[0],alvo)
                    # Fechamento concorrente de sessão autenticada sem personagem.
                    escritor.close(); await escritor.wait_closed()
                id1=uuid.uuid4().hex
                with patch.dict(os.environ,{"ADMIN_COORDENACAO_ALVOS":",".join(alvos)}):
                    # O contrato está no daemon, não no ambiente do cliente API.
                    resposta, *_ = await asyncio.gather(comandar(realm1,True,id1), login_concorrente(l1,"1.2.6"), login_concorrente(l2,"1.5.5"))
                self.assertEqual(resposta.status_code,202); self.assertEqual(resposta.json()["persistencia"],"salva")
                await aplicado(realm2,id1)
                for realm,personagem,leitor,escritor in sessoes:
                    await estado_gm(realm,personagem,"gm",1)
                    await self.pacote_jogo(escritor,70,struct.pack("!ib",personagem,0))
                    self.assertEqual((await self.esperar_pacote_jogo(leitor,71))[4:],b"\x20"+b"\xff"*32,"cache GM vivo do link não consumido")
                    data=struct.pack("<H",205); await self.pacote_jogo(escritor,34,bytes([len(data)])+data)
                    await estado_gm(realm,personagem,"gm_invencivel",True)
                repetido=await comandar(realm2,True,id1)
                self.assertEqual(repetido.json()["revisao"],resposta.json()["revisao"])
                self.assertEqual((await comandar(realm2,False,id1)).status_code,409)
                id2=uuid.uuid4().hex
                self.assertEqual((await comandar(realm2,False,id2)).status_code,202)
                await aplicado(realm1,id2)
                for realm,personagem,leitor,escritor in sessoes:
                    f=await estado_gm(realm,personagem,"gm",0); self.assertFalse(f["gm_invencivel"])
                    await self.pacote_jogo(escritor,70,struct.pack("!ib",personagem,0))
                    self.assertEqual((await self.esperar_pacote_jogo(leitor,71))[4:],b"\0")
                    data=struct.pack("<H",205); await self.pacote_jogo(escritor,34,bytes([len(data)])+data)
                    self.assertFalse((await ficha(realm,personagem))["gm_invencivel"])
                # GS indisponível: persistência confirmada, efeito global pendente.
                gs2[2].terminate(); await gs2[2].wait()
                id3=uuid.uuid4().hex
                self.assertEqual((await comandar(realm1,True,id3)).json()["estado"],"pendente")
                await asyncio.sleep(1.2)
                self.assertEqual((await self.cliente.get(f"/api/realms/{realm1}/operacoes/{id3}")).json()["estado"],"pendente")
                await pilha.enter_async_context(self.daemon_gm_real("gs",realm2,"1.5.5",alvos[3],alvos))
                await aplicado(realm1,id3)
                for _,_,_,escritor in sessoes: escritor.close(); await escritor.wait_closed()
        finally:
            async with self.pool.acquire() as c:
                await c.execute("DELETE FROM coordenacao_gm_processos WHERE processo=ANY($1)",alvos)


if __name__ == "__main__":
    unittest.main()
