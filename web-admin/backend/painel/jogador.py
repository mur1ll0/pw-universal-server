"""Painel do jogador (B387): cadastro público, a própria conta e leitura dos próprios personagens.

Conta comum (sem `gm_privileges`) entra no mesmo painel, mas o middleware só a deixa alcançar as rotas
desta lista fechada — **tudo o mais é 403**, inclusive rota que ainda nem existe. Rota de personagem
ainda confere, no banco, que o personagem é da conta e do realm da URL (senão 404, sem dizer se existe).
O GS confere o mesmo do lado dele (`ServidorAdministrativo::liberado_a_conta_comum`).
"""
import asyncio
import hashlib
import json
import logging
import os
import re
import uuid

from fastapi import HTTPException, Path, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel, ConfigDict, Field

from .canal import CanalIndisponivel, CanalNaoEnviado, comandar, configuracao
from .recursos import _pedir, _versao
from .seguranca import COOKIE, TEMPO_SESSAO, verificar_origem

REGISTRO = logging.getLogger("pw_admin")

_REALM = r"[A-Za-z0-9_.-]{1,64}"
# Rotas que a conta comum alcança (método, caminho inteiro). HEAD vale como GET.
ROTAS_DO_JOGADOR = [(metodo, re.compile(caminho)) for metodo, caminho in (
    ("GET", r"^/api/sessao$"),
    ("POST", r"^/api/sessao/sair$"),
    ("GET", r"^/api/realms$"),
    ("GET", rf"^/api/realms/{_REALM}/estado$"),
    ("GET", rf"^/api/realms/{_REALM}/catalogo-mapas$"),
    # A lista sai filtrada pela conta da sessão (`consultas.buscar_personagens`).
    ("GET", rf"^/api/realms/{_REALM}/personagens$"),
    ("GET", r"^/api/icones/(m|f|habilidade|mascote)/[^/]{1,200}\.png$"),
    ("GET", r"^/api/classes$"),
    ("GET", r"^/api/jogador/conta$"),
    ("GET", r"^/api/jogador/versoes$"),
    ("GET", r"^/api/jogador/versoes/[^/]{1,32}/historico$"),
    ("POST", r"^/api/jogador/senha$"),
)]
# Leituras de um personagem: o dono é conferido antes de chegar à rota.
ROTA_DO_PERSONAGEM = re.compile(
    rf"^/api/realms/(?P<realm>{_REALM})/personagens/(?P<pid>[0-9]{{1,10}})"
    r"(|/inventario|/habilidades|/daimon|/mascotes|/missoes|/itens/[a-z_]{1,16}/[0-9]{1,3}/dica)$")
# Sem sessão: só o cadastro (o login fica em `aplicacao.py`).
ROTAS_PUBLICAS = {("POST", "/api/jogador/cadastro"), ("GET", "/api/jogador/cadastro")}


async def exigir_rota_do_jogador(requisicao, administrador, pool):
    """Barra do middleware: GM passa direto; conta comum só nas rotas acima, e só nos seus personagens."""
    if administrador["papel"] == "gm":
        return
    metodo = "GET" if requisicao.method == "HEAD" else requisicao.method
    caminho = requisicao.url.path
    if any(m == metodo and padrao.match(caminho) for m, padrao in ROTAS_DO_JOGADOR):
        return
    achado = ROTA_DO_PERSONAGEM.match(caminho) if metodo == "GET" else None
    if achado is None:
        raise HTTPException(403, "Recurso exclusivo de GM.")
    personagem = int(achado["pid"])
    if not 1 <= personagem <= 2_147_483_647:
        raise HTTPException(404, "Personagem não encontrado neste realm.")
    async with pool.acquire() as conexao:
        dono = await conexao.fetchval(
            "SELECT account_id FROM characters WHERE id=$1 AND realm_id=$2 AND NOT is_deleted",
            personagem, achado["realm"])
    if dono != administrador["conta_id"]:
        raise HTTPException(404, "Personagem não encontrado neste realm.")


class Cadastro(BaseModel):
    model_config = ConfigDict(extra="forbid")
    # Mesmos limites da criação pelo GM (`contas.CriacaoConta`) e do GS (`usuario_valido`/`senha_valida`).
    usuario: str = Field(min_length=3, max_length=32, pattern=r"^[A-Za-z0-9_]+$")
    senha: str = Field(min_length=6, max_length=64, pattern=r"^[\x20-\x7e]+$")


class TrocaDaPropriaSenha(BaseModel):
    model_config = ConfigDict(extra="forbid")
    senha_atual: str = Field(min_length=1, max_length=64)
    senha: str = Field(min_length=6, max_length=64, pattern=r"^[\x20-\x7e]+$")


def cadastro_aberto():
    return os.getenv("PAINEL_CADASTRO", "aberto").lower() != "fechado"


def _com_cookie(corpo, token, status=200):
    resposta = JSONResponse(corpo, status_code=status)
    resposta.set_cookie(COOKIE, token, max_age=TEMPO_SESSAO, httponly=True, samesite="strict",
                        secure=os.getenv("ADMIN_COOKIE_SECURE", "true").lower() == "true", path="/")
    return resposta


def registrar_jogador(app):
    async def limitar(requisicao, nome, limite, segundos):
        """Política do painel (não do jogo): tantas tentativas por IP na janela."""
        ip = requisicao.client.host if requisicao.client else "desconhecido"
        chave = f"{app.state.seguranca.prefixo}{nome}:{hashlib.sha256(ip.encode()).hexdigest()}"
        quantidade = await app.state.seguranca.redis.eval(
            "local n=redis.call('INCR',KEYS[1]); "
            "if n==1 then redis.call('EXPIRE',KEYS[1],ARGV[1]) end; return n", 1, chave, segundos)
        if quantidade > limite:
            raise HTTPException(429, "Muitas tentativas. Aguarde um pouco.")

    async def comandar_em_algum_realm(administrador, consulta):
        """Contas são globais: qualquer GS com canal configurado grava. Mesmo ID em todas as tentativas."""
        async with app.state.seguranca.pool.acquire() as conexao:
            realms = [r["id"] for r in await conexao.fetch("SELECT id FROM realms ORDER BY id")]
        operacao = uuid.uuid4().hex
        for realm in realms:
            try:
                configuracao(realm)
            except CanalNaoEnviado:
                continue
            try:
                return await comandar(realm, administrador, consulta, operacao)
            except CanalNaoEnviado:
                continue
            except CanalIndisponivel:
                return {"estado": "desconhecido", "dados": {"codigo": "canal_indisponivel"}}
        return None

    @app.get("/api/jogador/cadastro")
    async def estado_do_cadastro():
        return {"aberto": cadastro_aberto()}

    @app.post("/api/jogador/cadastro")
    async def cadastrar(pedido: Cadastro, requisicao: Request):
        verificar_origem(requisicao)
        if not cadastro_aberto():
            raise HTTPException(403, "O cadastro está fechado neste servidor.")
        await limitar(requisicao, "cadastros", 5, 3600)
        resposta = await comandar_em_algum_realm(
            0, {"tipo": "cadastrar_conta", "usuario": pedido.usuario.lower(), "senha": pedido.senha})
        if resposta is None:
            raise HTTPException(503, "Nenhum servidor disponível para o cadastro. Tente mais tarde.")
        dados = resposta["dados"]
        if resposta["estado"] != "salvo":
            if dados.get("codigo") == "usuario_existente":
                raise HTTPException(409, "Esse nome de conta já existe.")
            if resposta["estado"] == "desconhecido":
                return JSONResponse({"estado": "desconhecido",
                                     "detail": "Não deu para confirmar o cadastro. Tente entrar com a conta."},
                                    status_code=202)
            raise HTTPException(400, "Cadastro recusado. Confira o nome e a senha.")
        async with app.state.seguranca.pool.acquire() as conexao:
            conta = await conexao.fetchrow("SELECT id,password_hash FROM accounts WHERE id=$1", dados["conta_id"])
        REGISTRO.info("cadastro_publico conta=%s", conta["id"])
        token = await app.state.seguranca.criar_sessao(requisicao, conta["id"], conta["password_hash"])
        return _com_cookie({"estado": "autenticado", "conta_id": conta["id"], "usuario": dados["usuario"]},
                           token, 201)

    @app.get("/api/jogador/conta")
    async def minha_conta(requisicao: Request):
        conta_id = requisicao.state.administrador["conta_id"]
        async with app.state.seguranca.pool.acquire() as conexao:
            conta = await conexao.fetchrow(
                "SELECT id,username,gold_balance,gm_privileges,created_at,last_login_at FROM accounts WHERE id=$1",
                conta_id)
            por_realm = await conexao.fetch(
                "SELECT r.id,r.name,r.version,count(c.id) AS personagens FROM realms r "
                "LEFT JOIN characters c ON c.realm_id=r.id AND c.account_id=$1 AND NOT c.is_deleted "
                "GROUP BY r.id,r.name,r.version ORDER BY r.id", conta_id)
        return {"id": conta["id"], "usuario": conta["username"],
                # Unidade do PLAYER_CASH (o cliente mostra ÷100), como em `/api/contas`.
                "gold": str(conta["gold_balance"]), "gm": conta["gm_privileges"] > 0,
                "criada_em": conta["created_at"].isoformat() if conta["created_at"] else None,
                "ultimo_login": conta["last_login_at"].isoformat() if conta["last_login_at"] else None,
                "personagens": sum(r["personagens"] for r in por_realm),
                "por_realm": [{"realm_id": r["id"], "nome": r["name"], "versao": r["version"],
                               "personagens": r["personagens"]} for r in por_realm]}

    @app.post("/api/jogador/senha")
    async def trocar_propria_senha(pedido: TrocaDaPropriaSenha, requisicao: Request):
        administrador = requisicao.state.administrador
        seguranca = app.state.seguranca
        await seguranca.limitar_login(requisicao, administrador["usuario"])
        async with seguranca.pool.acquire() as conexao:
            hash_atual = await conexao.fetchval("SELECT password_hash FROM accounts WHERE id=$1",
                                                administrador["conta_id"])
        if not await seguranca.validar_senha(administrador["usuario"], pedido.senha_atual, hash_atual):
            raise HTTPException(400, "A senha atual não confere.")
        resposta = await comandar_em_algum_realm(
            administrador["conta_id"],
            {"tipo": "trocar_senha", "conta_id": administrador["conta_id"], "senha": pedido.senha})
        if resposta is None:
            raise HTTPException(503, "Nenhum servidor disponível. Tente mais tarde.")
        if resposta["estado"] != "salvo":
            if resposta["estado"] == "desconhecido":
                return JSONResponse({"estado": "desconhecido",
                                     "detail": "Não deu para confirmar a troca. Entre de novo para conferir."},
                                    status_code=202)
            raise HTTPException(400, "Troca de senha recusada.")
        # A sessão antiga cai com o hash novo; esta já nasce com ele.
        async with seguranca.pool.acquire() as conexao:
            hash_novo = await conexao.fetchval("SELECT password_hash FROM accounts WHERE id=$1",
                                               administrador["conta_id"])
        token = await seguranca.criar_sessao(requisicao, administrador["conta_id"], hash_novo)
        return _com_cookie({"estado": "salvo", "efeito": "novos_logins"}, token)

    @app.get("/api/jogador/versoes")
    async def versoes():
        status, _, conteudo = await asyncio.to_thread(_pedir, "GET", "/admin/versoes")
        if status != 200:
            raise HTTPException(502, "Histórico de versões indisponível.")
        return {"versoes": [{"versao": v.get("versao"), "atual": v.get("atual")} for v in json.loads(conteudo)]}

    @app.get("/api/jogador/versoes/{versao}/historico")
    async def historico(versao: str = Path(..., max_length=32)):
        """Só o que foi publicado, sem autor nem rascunho."""
        status, _, conteudo = await asyncio.to_thread(
            _pedir, "GET", f"/admin/{_versao(versao)}/alteracoes", {"limite": 200})
        if status != 200:
            raise HTTPException(502, "Histórico de versões indisponível.")
        return {"versao": versao, "alteracoes": [
            {"versao": a.get("versao"), "descricao": a.get("descricao"), "criada_em": a.get("criada_em"),
             "arquivos": [i.get("caminho") for i in a.get("lista") or []], "reverte": a.get("reverte")}
            for a in json.loads(conteudo) if a.get("estado") == "publicada"]}
