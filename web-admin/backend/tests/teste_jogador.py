"""B387: painel do jogador — conta comum entra, só alcança a lista fechada e só os próprios personagens."""
import hashlib
import json
import os
import unittest
from unittest.mock import patch

import teste_base


class TesteJogador(teste_base.BaseAdministrativa):
    async def virar_jogador(self):
        await self.modificar_conta("gm_privileges", 0)

    async def personagem_de_outra_conta(self, realm):
        async with self.pool.acquire() as conexao:
            outra = await conexao.fetchval(
                "INSERT INTO accounts (username,password_hash) VALUES($1,'x') RETURNING id", f"ou_{self.sufixo}")
            personagem = await conexao.fetchval(
                "INSERT INTO characters(account_id,realm_id,name,race,cls,gender) VALUES($1,$2,'DeOutraConta',0,0,0) "
                "RETURNING id", outra, realm)
        self.contas_criadas.append(outra)
        return personagem

    async def asyncTearDown(self):
        async with self.pool.acquire() as conexao:
            for conta in self.contas_criadas:
                await conexao.execute("DELETE FROM characters WHERE account_id=$1", conta)
            await conexao.execute("DELETE FROM characters WHERE account_id=$1", self.conta_id)
        await super().asyncTearDown()

    async def test_jogador_so_alcanca_a_lista_fechada(self):
        realm, _ = await self.criar_personagem()
        await self.virar_jogador()
        sessao = (await self.cliente.get("/api/sessao")).json()
        self.assertEqual(sessao["papel"], "jogador")
        for caminho in ("/api/sessao", "/api/realms", "/api/classes", "/api/jogador/conta",
                        f"/api/realms/{realm}/personagens"):
            self.assertEqual((await self.cliente.get(caminho)).status_code, 200, caminho)
        cabecalhos = {"X-CSRF-Token": sessao["csrf"]}
        negadas = [("GET", "/api/contas"), ("GET", "/api/contas/"), ("GET", f"/api/realms/{realm}/templates"),
                   ("GET", f"/api/realms/{realm}/itens?busca=a"), ("GET", f"/api/realms/{realm}/estudio"),
                   ("GET", "/api/recursos/versoes"), ("GET", "/api/recursos/1.5.5/alteracoes"),
                   ("GET", "/api/habilidades?busca=a"), ("GET", "/api/icones/m?busca=a"),
                   ("GET", "/api/rota/que/nao/existe"), ("GET", f"/api/realms/{realm}/servidor"),
                   ("POST", f"/api/realms/{realm}/rates"), ("POST", f"/api/realms/{realm}/reiniciar"),
                   ("POST", f"/api/realms/{realm}/contas"), ("POST", f"/api/realms/{realm}/contas/{self.conta_id}/senha"),
                   ("POST", f"/api/realms/{realm}/contas/{self.conta_id}/gold"),
                   ("POST", f"/api/realms/{realm}/personagens/1/editar"),
                   ("POST", f"/api/realms/{realm}/mapas/1"), ("DELETE", "/api/recursos/envios/1"),
                   ("PUT", f"/api/realms/{realm}/estudio/tabelas/1/1")]
        for metodo, caminho in negadas:
            resposta = await self.cliente.request(metodo, caminho, json={}, headers=cabecalhos)
            self.assertEqual(resposta.status_code, 403, f"{metodo} {caminho}")
        # O GM continua com tudo.
        await self.modificar_conta("gm_privileges", 1)
        self.assertEqual((await self.cliente.get("/api/contas")).status_code, 200)

    async def test_jogador_ve_so_os_proprios_personagens(self):
        realm, meu = await self.criar_personagem()
        alheio = await self.personagem_de_outra_conta(realm)
        await self.virar_jogador()
        lista = (await self.cliente.get(f"/api/realms/{realm}/personagens")).json()
        self.assertEqual([p["id"] for p in lista["personagens"]], [meu])
        self.assertEqual(lista["total"], 1)
        # Buscar pelo nome da outra conta não traz nada.
        busca = (await self.cliente.get(f"/api/realms/{realm}/personagens?busca=ou_")).json()
        self.assertEqual(busca["personagens"], [])
        self.assertEqual((await self.cliente.get(f"/api/realms/{realm}/personagens/{meu}")).status_code, 200)
        for sufixo in ("", "/inventario", "/habilidades", "/mascotes", "/missoes", "/daimon", "/itens/bolsa/0/dica"):
            resposta = await self.cliente.get(f"/api/realms/{realm}/personagens/{alheio}{sufixo}")
            self.assertEqual(resposta.status_code, 404, sufixo)
        # O próprio, pela URL de outro realm, também não.
        self.assertEqual((await self.cliente.get(f"/api/realms/outro_realm/personagens/{meu}")).status_code, 404)
        # GM vê os dois.
        await self.modificar_conta("gm_privileges", 1)
        todos = (await self.cliente.get(f"/api/realms/{realm}/personagens")).json()
        self.assertEqual(sorted(p["id"] for p in todos["personagens"]), sorted([meu, alheio]))

    async def test_minha_conta_conta_personagens_por_realm(self):
        realm, _ = await self.criar_personagem()
        await self.virar_jogador()
        conta = (await self.cliente.get("/api/jogador/conta")).json()
        self.assertEqual(conta["id"], self.conta_id)
        self.assertFalse(conta["gm"])
        self.assertEqual(conta["personagens"], 1)
        self.assertEqual({r["realm_id"]: r["personagens"] for r in conta["por_realm"]}[realm], 1)

    def falso_gs(self, chamadas):
        """Faz o papel do GS: cadastra/troca a senha no banco como o `pw-gs` faria."""
        pool = self.pool

        async def comandar(realm, administrador, consulta, operacao):
            chamadas.append((administrador, consulta))
            async with pool.acquire() as conexao:
                if consulta["tipo"] == "cadastrar_conta":
                    usuario = consulta["usuario"]
                    conta = await conexao.fetchval(
                        "INSERT INTO accounts(username,password_hash) VALUES($1,$2) ON CONFLICT DO NOTHING RETURNING id",
                        usuario, hashlib.md5((usuario + consulta["senha"]).encode()).hexdigest())
                    if conta is None:
                        return {"estado": "falha", "dados": {"codigo": "usuario_existente"}}
                    self.contas_criadas.append(conta)
                    return {"estado": "salvo", "dados": {"tipo": "criar_conta", "conta_id": conta, "usuario": usuario}}
                usuario = await conexao.fetchval("SELECT username FROM accounts WHERE id=$1", consulta["conta_id"])
                await conexao.execute("UPDATE accounts SET password_hash=$1 WHERE id=$2",
                                      hashlib.md5((usuario + consulta["senha"]).encode()).hexdigest(), consulta["conta_id"])
                return {"estado": "salvo", "dados": {"tipo": "trocar_senha", "conta_id": consulta["conta_id"]}}
        return comandar

    async def test_cadastro_cria_conta_comum_e_ja_entra(self):
        await self.criar_personagem()  # garante um realm
        await self.cliente.post("/api/sessao/sair",
                                headers={"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]})
        chamadas = []
        nome = f"Cad_{self.sufixo}"
        with patch("painel.jogador.configuracao", lambda realm: None), \
                patch("painel.jogador.comandar", self.falso_gs(chamadas)):
            self.assertTrue((await self.cliente.get("/api/jogador/cadastro")).json()["aberto"])
            for pedido in ({"usuario": "a b", "senha": "123456"}, {"usuario": nome, "senha": "123"},
                           {"usuario": nome, "senha": "123456", "gm": True}):
                self.assertEqual((await self.cliente.post("/api/jogador/cadastro", json=pedido)).status_code, 422)
            criada = await self.cliente.post("/api/jogador/cadastro", json={"usuario": nome, "senha": "segredo1"})
            self.assertEqual(criada.status_code, 201, criada.text)
            self.assertEqual(chamadas[-1], (0, {"tipo": "cadastrar_conta", "usuario": nome.lower(), "senha": "segredo1"}))
            sessao = (await self.cliente.get("/api/sessao")).json()
            self.assertEqual((sessao["usuario"], sessao["papel"]), (nome.lower(), "jogador"))
            repetida = await self.cliente.post("/api/jogador/cadastro", json={"usuario": nome, "senha": "segredo1"})
            self.assertEqual(repetida.status_code, 409)
            for i in range(3):
                await self.cliente.post("/api/jogador/cadastro", json={"usuario": f"{nome}{i}", "senha": "segredo1"})
            self.assertEqual((await self.cliente.post("/api/jogador/cadastro",
                                                      json={"usuario": f"{nome}x", "senha": "segredo1"})).status_code, 429)
            with patch.dict(os.environ, {"PAINEL_CADASTRO": "fechado"}):
                self.assertEqual((await self.cliente.post("/api/jogador/cadastro",
                                                          json={"usuario": f"{nome}y", "senha": "segredo1"})).status_code, 403)
        # Origem cruzada recusada.
        self.assertEqual((await self.cliente.post("/api/jogador/cadastro", json={"usuario": f"{nome}z", "senha": "segredo1"},
                                                  headers={"Origin": "https://outro.test"})).status_code, 403)

    async def test_jogador_troca_a_propria_senha_e_continua_logado(self):
        await self.criar_personagem()
        await self.virar_jogador()
        chamadas = []
        csrf = {"X-CSRF-Token": (await self.cliente.get("/api/sessao")).json()["csrf"]}
        with patch("painel.jogador.configuracao", lambda realm: None), \
                patch("painel.jogador.comandar", self.falso_gs(chamadas)):
            errada = await self.cliente.post("/api/jogador/senha", json={"senha_atual": "nao-e", "senha": "nova-senha"},
                                             headers=csrf)
            self.assertEqual(errada.status_code, 400)
            self.assertEqual(chamadas, [])
            sem_csrf = await self.cliente.post("/api/jogador/senha", json={"senha_atual": self.senha, "senha": "nova-senha"})
            self.assertEqual(sem_csrf.status_code, 403)
            certa = await self.cliente.post("/api/jogador/senha",
                                            json={"senha_atual": self.senha, "senha": "nova-senha"}, headers=csrf)
            self.assertEqual(certa.status_code, 200, certa.text)
        self.assertEqual(chamadas, [(self.conta_id, {"tipo": "trocar_senha", "conta_id": self.conta_id, "senha": "nova-senha"})])
        # A sessão nova vale; a senha antiga não entra mais e a nova entra.
        self.assertEqual((await self.cliente.get("/api/sessao")).status_code, 200)
        self.assertEqual((await self.entrar()).status_code, 401)
        self.assertEqual((await self.entrar("nova-senha")).status_code, 200)

    async def test_historico_de_versoes_so_o_publicado_sem_autor(self):
        await self.criar_personagem()
        await self.virar_jogador()
        alteracoes = [{"id": 2, "estado": "rascunho", "descricao": "segredo", "autor_nome": "gm", "lista": []},
                      {"id": 1, "estado": "publicada", "descricao": "Loja nova", "autor_nome": "gm", "versao": 3,
                       "criada_em": "2026-10-25T00:00:00Z", "lista": [{"caminho": "gshop.data"}]}]

        def pedir(metodo, caminho, consulta=None, corpo=None, tipo=None):
            conteudo = [{"versao": "1.5.5", "atual": 3, "minima": 0}] if caminho == "/admin/versoes" else alteracoes
            return 200, {}, json.dumps(conteudo).encode()
        with patch("painel.jogador._pedir", pedir):
            self.assertEqual((await self.cliente.get("/api/jogador/versoes")).json(),
                             {"versoes": [{"versao": "1.5.5", "atual": 3}]})
            historico = (await self.cliente.get("/api/jogador/versoes/1.5.5/historico")).json()
        self.assertEqual(historico["alteracoes"], [{"versao": 3, "descricao": "Loja nova", "criada_em": "2026-10-25T00:00:00Z",
                                                    "arquivos": ["gshop.data"], "reverte": None}])


def load_tests(loader, testes, padrao):
    """Só os testes desta classe (a base também tem `test_*`, que rodam no `teste_base`)."""
    return unittest.TestSuite(TesteJogador(nome) for nome in vars(TesteJogador) if nome.startswith("test_"))


if __name__ == "__main__":
    unittest.main()
