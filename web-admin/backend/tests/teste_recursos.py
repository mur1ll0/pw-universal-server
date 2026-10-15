"""Recursos do cliente no painel (B235): sessão, CSRF, autor, segredo e o limite de envio.

O pw-recursos é substituído por um servidor HTTP de mentira que registra o que recebeu; o
motor de verdade é testado em `crates/pw-recursos/tests/governanca.rs`.
"""
import json
import os
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

import teste_base as base

SEGREDO = "segredo-do-painel-0123456789"


class RecursosFalso(BaseHTTPRequestHandler):
    pedidos = []

    def _responder(self):
        tamanho = int(self.headers.get("content-length") or 0)
        corpo = self.rfile.read(tamanho) if tamanho else b""
        partes = urlsplit(self.path)
        RecursosFalso.pedidos.append({
            "metodo": self.command, "caminho": partes.path, "consulta": parse_qs(partes.query),
            "segredo": self.headers.get("x-recursos-segredo"), "corpo": corpo,
        })
        if "/pecas/" in partes.path:
            self.send_response(200)
            self.send_header("content-type", "image/png")
            self.end_headers()
            self.wfile.write(b"PNGFALSO")
            return
        if partes.path.endswith("/arquivo") and self.command == "GET":
            self.send_response(200)
            self.send_header("content-type", "application/octet-stream")
            self.send_header("content-disposition", "attachment; filename*=UTF-8''a.txt")
            self.end_headers()
            self.wfile.write(b"conteudo")
            return
        resposta = json.dumps({"ok": True}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(resposta)))
        self.end_headers()
        self.wfile.write(resposta)

    do_GET = do_POST = do_PUT = do_DELETE = _responder

    def log_message(self, *args):
        pass


class TesteRecursos(unittest.IsolatedAsyncioTestCase):
    """Reusa a preparação da `BaseAdministrativa` (banco `test`, Redis isolado, GM de teste)
    sem herdar os testes dela."""
    entrar = base.BaseAdministrativa.entrar

    async def asyncSetUp(self):
        await base.BaseAdministrativa.asyncSetUp(self)
        RecursosFalso.pedidos = []
        self.falso = ThreadingHTTPServer(("127.0.0.1", 0), RecursosFalso)
        threading.Thread(target=self.falso.serve_forever, daemon=True).start()
        self.ambiente = patch.dict(os.environ, {
            "RECURSOS_URL": f"http://127.0.0.1:{self.falso.server_address[1]}", "RECURSOS_SEGREDO": SEGREDO})
        self.ambiente.start()

    async def asyncTearDown(self):
        self.ambiente.stop()
        self.falso.shutdown()
        self.falso.server_close()
        await base.BaseAdministrativa.asyncTearDown(self)

    async def csrf(self):
        return {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}

    async def test_sem_sessao_nada_chega_ao_pw_recursos(self):
        r = await self.cliente.get("/api/recursos/versoes")
        self.assertEqual(r.status_code, 401)
        self.assertEqual(RecursosFalso.pedidos, [])

    async def test_rascunho_leva_autor_segredo_e_exige_csrf(self):
        self.assertEqual((await self.entrar()).status_code, 200)
        pedido = {"descricao": "troca o a.txt", "operacao_id": "op-teste-123"}
        r = await self.cliente.post("/api/recursos/1.5.5/rascunhos", json=pedido)
        self.assertEqual(r.status_code, 403)
        r = await self.cliente.post("/api/recursos/1.5.5/rascunhos", json=pedido, headers=await self.csrf())
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"], p["segredo"]), ("POST", "/admin/1.5.5/rascunhos", SEGREDO))
        self.assertEqual(json.loads(p["corpo"]), {"autor": self.conta_id, "descricao": "troca o a.txt",
                                                 "operacao_id": "op-teste-123"})

    async def test_envio_de_arquivo_passa_de_8_kib_so_na_rota_de_envio(self):
        await self.entrar()
        csrf = await self.csrf()
        conteudo = os.urandom(100_000)
        r = await self.cliente.put("/api/recursos/1.5.5/rascunhos/7/arquivo",
                                   params={"caminho": "configs\\a.txt"}, content=conteudo,
                                   headers={**csrf, "content-type": "application/octet-stream"})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"], p["corpo"]), ("PUT", "/admin/1.5.5/rascunhos/7/arquivo", conteudo))
        self.assertEqual(p["consulta"], {"caminho": ["configs\\a.txt"], "destino": ["cliente"]})
        # Em qualquer outra rota o limite continua 8 KiB.
        r = await self.cliente.post("/api/recursos/1.5.5/rascunhos", headers=csrf,
                                    json={"descricao": "x" * 10_000, "operacao_id": "op-teste-456"})
        self.assertEqual(r.status_code, 413)

    async def test_publicar_e_baixar(self):
        await self.entrar()
        csrf = await self.csrf()
        r = await self.cliente.post("/api/recursos/1.5.5/publicar", headers=csrf, json={"rascunhos": [3, 4], "minima": 2})
        self.assertEqual(r.status_code, 200)
        self.assertEqual(json.loads(RecursosFalso.pedidos[-1]["corpo"]),
                         {"rascunhos": [3, 4], "autor": self.conta_id, "minima": 2})
        r = await self.cliente.post("/api/recursos/1.5.5/publicar", headers=csrf, json={"rascunhos": []})
        self.assertEqual(r.status_code, 422)
        r = await self.cliente.get("/api/recursos/1.5.5/arquivo", params={"caminho": "configs\\a.txt"})
        self.assertEqual((r.status_code, r.content), (200, b"conteudo"))
        self.assertIn("attachment", r.headers["content-disposition"])

    async def test_estudio_usa_a_versao_do_realm(self):
        """B237: /api/realms/{realm}/estudio/... vai para /admin/<versão do realm>/estudio/..."""
        await self.entrar()
        csrf = await self.csrf()
        r = await self.cliente.get("/api/realms/realm_126/estudio")
        self.assertEqual(r.json(), {"versao": "1.2.6"})
        r = await self.cliente.get("/api/realms/realm_126/estudio/tabelas/WEAPON_ESSENCE",
                                   params={"busca": "espada", "rascunho": 9})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual(p["caminho"], "/admin/1.2.6/estudio/tabelas/WEAPON_ESSENCE")
        self.assertEqual((p["consulta"]["busca"], p["consulta"]["rascunho"]), (["espada"], ["9"]))
        r = await self.cliente.put("/api/realms/realm_155/estudio/tabelas/WEAPON_ESSENCE/3", params={"rascunho": 9},
                                   headers=csrf, json={"campos": {"price": 10}})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"]), ("PUT", "/admin/1.5.5/estudio/tabelas/WEAPON_ESSENCE/3"))
        self.assertEqual(json.loads(p["corpo"]), {"campos": {"price": 10}})
        r = await self.cliente.get("/api/realms/realm_126/estudio/espacos/ESSENCE", params={"busca": "espada"})
        self.assertEqual(r.status_code, 200)
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.2.6/estudio/espacos/ESSENCE")
        # Tabela fora do padrão, realm inexistente e edição sem campos não chegam ao pw-recursos.
        antes = len(RecursosFalso.pedidos)
        self.assertEqual((await self.cliente.get("/api/realms/realm_126/estudio/espacos/XYZ")).status_code, 404)
        self.assertEqual((await self.cliente.get("/api/realms/realm_126/estudio/tabelas/x;y")).status_code, 404)
        self.assertEqual((await self.cliente.get("/api/realms/nao_existe/estudio/tabelas")).status_code, 404)
        r = await self.cliente.put("/api/realms/realm_126/estudio/tabelas/WEAPON_ESSENCE/3", params={"rascunho": 9},
                                   headers=csrf, json={"campos": {}})
        self.assertEqual(r.status_code, 422)
        self.assertEqual(len(RecursosFalso.pedidos), antes)

    async def test_conversa_grande_passa_so_na_rota_da_conversa(self):
        """B240: salvar conversa manda a árvore inteira; o limite de 8 KiB vale para o resto."""
        await self.entrar()
        csrf = await self.csrf()
        janelas = [{"id": i, "id_pai": -1, "texto": "x" * 400, "opcoes": []} for i in range(1, 50)]
        r = await self.cliente.put("/api/realms/realm_126/estudio/conversas/7", params={"rascunho": 3}, headers=csrf,
                                   json={"conversa": {"id": 9, "texto": "t", "janelas": janelas}})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"]), ("PUT", "/admin/1.2.6/estudio/conversas/7"))
        self.assertEqual(len(json.loads(p["corpo"])["conversa"]["janelas"]), 49)
        r = await self.cliente.put("/api/realms/realm_126/estudio/tabelas/WEAPON_ESSENCE/1", params={"rascunho": 3},
                                   headers=csrf, json={"campos": {"Name": "x" * 20_000}})
        self.assertEqual(r.status_code, 413)
        r = await self.cliente.get("/api/realms/realm_155/estudio/funcoes")
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.5.5/estudio/funcoes")

    async def test_mapas_repassam_png_e_recusam_caminho_ruim(self):
        """B242: a peça do minimapa sai como image/png; caminho e nome de peça fora do padrão não saem do painel."""
        await self.entrar()
        r = await self.cliente.get("/api/realms/realm_155/estudio/mapas/world/pecas/0102.png")
        self.assertEqual((r.status_code, r.headers["content-type"], r.content), (200, "image/png", b"PNGFALSO"))
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.5.5/estudio/mapas/world/pecas/0102.png")
        antes = len(RecursosFalso.pedidos)
        self.assertEqual((await self.cliente.get("/api/realms/realm_155/estudio/mapas/world/pecas/12.png")).status_code, 404)
        self.assertEqual((await self.cliente.get("/api/realms/realm_155/estudio/mapas/a.b/criaturas")).status_code, 404)
        self.assertEqual(len(RecursosFalso.pedidos), antes)
        r = await self.cliente.get("/api/realms/realm_126/estudio/mapas/a01/criaturas")
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.2.6/estudio/mapas/a01/criaturas")

    async def test_area_do_npcgen_repassa_e_recusa_o_que_nao_e_area(self):
        """B243: salvar área do npcgen passa de 8 KiB (geradores) só na rota dela; tipo fora do padrão fica no painel."""
        await self.entrar()
        csrf = await self.csrf()
        filhos = [{"dwID": 1000 + i, "dwNum": 2, "iRefresh": 30, "fOffsetTrn": 0.5, "bNeedHelp": False} for i in range(64)]
        r = await self.cliente.put("/api/realms/realm_126/estudio/mapas/world/npcgen/areas/12", params={"rascunho": 4},
                                   headers=csrf, json={"campos": {"vPos_x": 10.5}, "filhos": filhos})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"]), ("PUT", "/admin/1.2.6/estudio/mapas/world/npcgen/areas/12"))
        self.assertEqual(p["consulta"], {"rascunho": ["4"]})
        self.assertEqual(len(json.loads(p["corpo"])["filhos"]), 64)
        r = await self.cliente.post("/api/realms/realm_155/estudio/mapas/a01/npcgen/recursos/3/clonar",
                                    params={"rascunho": 4}, headers=csrf)
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.5.5/estudio/mapas/a01/npcgen/recursos/3/clonar")
        await self.cliente.get("/api/realms/realm_155/estudio/mapas/a01/npcgen", params={"rascunho": 4})
        self.assertEqual(RecursosFalso.pedidos[-1]["consulta"], {"rascunho": ["4"]})
        antes = len(RecursosFalso.pedidos)
        r = await self.cliente.put("/api/realms/realm_126/estudio/mapas/world/npcgen/xyz/1", params={"rascunho": 4},
                                   headers=csrf, json={"campos": {"vPos_x": 1}})
        self.assertEqual(r.status_code, 404)
        r = await self.cliente.put("/api/realms/realm_126/estudio/mapas/world/npcgen/areas/1", params={"rascunho": 4},
                                   headers=csrf, json={"campos": {}})
        self.assertEqual(r.status_code, 422)
        self.assertEqual(len(RecursosFalso.pedidos), antes)

    async def test_distritos_e_portais(self):
        """B244: distritos/portais repassam com o rascunho; outro nome de arquivo e edição vazia ficam no painel."""
        await self.entrar()
        csrf = await self.csrf()
        await self.cliente.get("/api/realms/realm_155/estudio/mapas/world/distritos", params={"rascunho": 5})
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["caminho"], p["consulta"]), ("/admin/1.5.5/estudio/mapas/world/distritos", {"rascunho": ["5"]}))
        r = await self.cliente.put("/api/realms/realm_126/estudio/mapas/a01/portais/2", params={"rascunho": 5},
                                   headers=csrf, json={"campos": {"alvo": [1, 2, 3]}})
        self.assertEqual(r.status_code, 200)
        p = RecursosFalso.pedidos[-1]
        self.assertEqual((p["metodo"], p["caminho"]), ("PUT", "/admin/1.2.6/estudio/mapas/a01/portais/2"))
        self.assertEqual(json.loads(p["corpo"]), {"campos": {"alvo": [1, 2, 3]}})
        await self.cliente.get("/api/realms/realm_155/estudio/mapas/world/criaturas")
        self.assertEqual(RecursosFalso.pedidos[-1]["caminho"], "/admin/1.5.5/estudio/mapas/world/criaturas")
        antes = len(RecursosFalso.pedidos)
        self.assertEqual((await self.cliente.get("/api/realms/realm_155/estudio/mapas/world/tasks")).status_code, 404)
        r = await self.cliente.put("/api/realms/realm_155/estudio/mapas/world/distritos/0", params={"rascunho": 5},
                                   headers=csrf, json={"campos": {}})
        self.assertEqual(r.status_code, 422)
        self.assertEqual(len(RecursosFalso.pedidos), antes)

    async def test_versao_invalida_e_servico_sem_segredo(self):
        await self.entrar()
        r = await self.cliente.get("/api/recursos/1$5/alteracoes")
        self.assertEqual(r.status_code, 404)
        with patch.dict(os.environ, {"RECURSOS_SEGREDO": ""}):
            r = await self.cliente.get("/api/recursos/versoes")
        self.assertEqual(r.status_code, 503)


if __name__ == "__main__":
    unittest.main()
