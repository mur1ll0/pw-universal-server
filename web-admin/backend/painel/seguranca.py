"""Sessões administrativas independentes dos tickets e da presença do jogo."""
import asyncio
import hashlib
import hmac
import json
import logging
import os
import secrets
from urllib.parse import urlsplit

from fastapi import HTTPException
from fastapi.responses import JSONResponse

REGISTRO = logging.getLogger("pw_admin")
COOKIE = "pw_admin_sessao"
TEMPO_SESSAO = 8 * 60 * 60


LIMITE_ARQUIVO_DE_RECURSO = 512 << 20


def eh_envio_de_recurso(escopo):
    partes = escopo["path"].strip("/").split("/")
    # api / recursos / <versao> / rascunhos / <id> / arquivo
    return (escopo["method"] == "PUT" and len(partes) == 6 and partes[:2] == ["api", "recursos"]
            and partes[3] == "rascunhos" and partes[5] == "arquivo")


LIMITE_CONVERSA = 256 << 10


def eh_conversa_do_estudio(escopo):
    """B240: salvar uma conversa manda a árvore inteira (janelas e opções) em JSON."""
    partes = escopo["path"].strip("/").split("/")
    # api / realms / <realm> / estudio / conversas / <indice>
    return (escopo["method"] == "PUT" and len(partes) == 6 and partes[:2] == ["api", "realms"]
            and partes[3:5] == ["estudio", "conversas"])


LIMITE_AREA_DO_NPCGEN = 64 << 10


def eh_area_do_npcgen(escopo):
    """B243: salvar uma área do npcgen manda os campos dela e de todos os geradores (até 64)."""
    partes = escopo["path"].strip("/").split("/")
    # api / realms / <realm> / estudio / mapas / <caminho> / npcgen / <tipo> / <indice>
    if (escopo["method"] == "PUT" and len(partes) == 9 and partes[:2] == ["api", "realms"]
            and partes[3:5] == ["estudio", "mapas"] and partes[6] == "npcgen"):
        return True
    # B251: polígono de distrito (até 512 vértices) ao editar ou criar.
    return (escopo["method"] in ("PUT", "POST") and len(partes) in (7, 8) and partes[:2] == ["api", "realms"]
            and partes[3:5] == ["estudio", "mapas"] and partes[6] == "distritos")


LIMITE_IMPORTACAO_DE_MODELO = 64 << 20


def eh_importacao_de_modelo(escopo):
    """B290: importar um glTF (GLB) como pele de um modelo do Estúdio."""
    partes = escopo["path"].strip("/").split("/")
    # api / realms / <realm> / estudio / modelos / importar
    return (escopo["method"] == "POST" and len(partes) == 6 and partes[:2] == ["api", "realms"]
            and partes[3:6] == ["estudio", "modelos", "importar"])


class LimitarEntrada:
    """Limita antes de acumular o corpo, inclusive sem Content-Length."""
    def __init__(self, app):
        self.app = app

    async def __call__(self, escopo, receber, enviar):
        if not (escopo["type"] == "http" and escopo["path"].startswith("/api/")
                and escopo["method"] in ("POST", "PUT", "PATCH", "DELETE")):
            await self.app(escopo, receber, enviar)
            return
        corpo = bytearray()
        # Só o envio de arquivo para um rascunho dos recursos do cliente passa de 8 KiB (B235):
        # o mesmo teto do pw-recursos (admin::MAIOR_ARQUIVO).
        limite = (LIMITE_ARQUIVO_DE_RECURSO if eh_envio_de_recurso(escopo)
                  else LIMITE_IMPORTACAO_DE_MODELO if eh_importacao_de_modelo(escopo)
                  else LIMITE_CONVERSA if eh_conversa_do_estudio(escopo)
                  else LIMITE_AREA_DO_NPCGEN if eh_area_do_npcgen(escopo) else 8192)
        while True:
            mensagem = await receber()
            if mensagem["type"] == "http.disconnect":
                return
            trecho = mensagem.get("body", b"")
            if len(corpo) + len(trecho) > limite:
                await JSONResponse({"detail": "Dados excedem o tamanho permitido."},
                                   status_code=413, headers={"X-Content-Type-Options": "nosniff",
                                                            "Cache-Control": "no-store"})(escopo, receber, enviar)
                return
            corpo.extend(trecho)
            if not mensagem.get("more_body", False):
                break
        entregue = False

        async def receber_limitado():
            nonlocal entregue
            if not entregue:
                entregue = True
                return {"type": "http.request", "body": bytes(corpo), "more_body": False}
            return await receber()

        await self.app(escopo, receber_limitado, enviar)


def impressao_senha(hash_senha):
    return hashlib.sha256(hash_senha.encode()).hexdigest()


def chave_sessao(token):
    return hashlib.sha256(token.encode()).hexdigest()


def verificar_origem(requisicao):
    origem = requisicao.headers.get("origin")
    if origem:
        try:
            partes = urlsplit(origem)
            mesma_origem = (partes.scheme == requisicao.url.scheme
                            and partes.netloc == requisicao.url.netloc
                            and not partes.path and not partes.query and not partes.fragment)
        except ValueError:
            mesma_origem = False
        if not mesma_origem:
            raise HTTPException(403, "Origem da requisição recusada.")
    if requisicao.headers.get("sec-fetch-site") == "cross-site":
        raise HTTPException(403, "Origem da requisição recusada.")


class Seguranca:
    def __init__(self, pool, redis, prefixo):
        self.pool = pool
        self.redis = redis
        self.prefixo = prefixo
        self.verificacoes = 0

    async def limitar_login(self, requisicao, usuario):
        ip = requisicao.client.host if requisicao.client else "desconhecido"
        # Política do painel; não representa uma regra do jogo.
        for identidade, limite in ((f"ip:{ip}", 20), (f"conta:{usuario.lower()}", 10)):
            identificador = hashlib.sha256(identidade.encode()).hexdigest()
            chave = f"{self.prefixo}tentativas:{identificador}"
            quantidade = await self.redis.eval(
                "local n=redis.call('INCR',KEYS[1]); "
                "if n==1 then redis.call('EXPIRE',KEYS[1],60) end; return n", 1, chave,
            )
            if quantidade > limite:
                raise HTTPException(429, "Muitas tentativas. Aguarde um minuto.")

    async def validar_senha(self, usuario, senha, hash_senha):
        if self.verificacoes >= 4:
            raise HTTPException(503, "Autenticação ocupada. Tente novamente.")
        self.verificacoes += 1
        processo = None
        try:
            processo = await asyncio.create_subprocess_exec(
                os.getenv("PW_VALIDADOR_CREDENCIAIS", "pw-validar-credenciais"),
                stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL,
            )
            entrada = json.dumps({"usuario": usuario, "senha": senha, "hash": hash_senha}).encode()
            saida, _ = await asyncio.wait_for(processo.communicate(entrada), timeout=5)
            if processo.returncode != 0 or saida.strip() not in (b"true", b"false"):
                REGISTRO.error("verificador_admin_falhou codigo=%s", processo.returncode)
                raise HTTPException(503, "Verificador de credenciais indisponível.")
            return saida.strip() == b"true"
        except (OSError, asyncio.TimeoutError) as erro:
            REGISTRO.error("verificador_admin_indisponivel tipo=%s", type(erro).__name__)
            raise HTTPException(503, "Verificador de credenciais indisponível.")
        finally:
            if processo is not None and processo.returncode is None:
                try:
                    processo.kill()
                except ProcessLookupError:
                    pass
                await processo.wait()
            self.verificacoes -= 1

    async def entrar(self, requisicao, usuario, senha):
        verificar_origem(requisicao)
        await self.limitar_login(requisicao, usuario)
        async with self.pool.acquire() as conexao:
            conta = await conexao.fetchrow(
                "SELECT id, username, password_hash, gm_privileges, is_banned "
                "FROM accounts WHERE LOWER(username) = LOWER($1)", usuario,
            )
        if conta is None or conta["is_banned"] or conta["gm_privileges"] <= 0:
            raise HTTPException(401, "Conta ou senha inválida, ou acesso GM indisponível.")
        if not await self.validar_senha(conta["username"], senha, conta["password_hash"]):
            raise HTTPException(401, "Conta ou senha inválida, ou acesso GM indisponível.")
        token = secrets.token_urlsafe(32)
        sessao = {"conta_id": conta["id"], "csrf": secrets.token_urlsafe(32),
                  "impressao": impressao_senha(conta["password_hash"])}
        await self.redis.set(f"{self.prefixo}sessoes:{chave_sessao(token)}", json.dumps(sessao), ex=TEMPO_SESSAO)
        anterior = requisicao.cookies.get(COOKIE)
        if anterior:
            await self.sair(anterior)
        REGISTRO.info("sessao_admin_criada conta=%s", conta["id"])
        return token

    async def autorizar(self, requisicao):
        token = requisicao.cookies.get(COOKIE, "")
        if not token or len(token) > 128:
            raise HTTPException(401, "Entre com sua conta PW com acesso GM.")
        chave = f"{self.prefixo}sessoes:{chave_sessao(token)}"
        registro = await self.redis.get(chave)
        if not registro:
            raise HTTPException(401, "Sessão encerrada. Entre novamente.")
        sessao = json.loads(registro)
        async with self.pool.acquire() as conexao:
            conta = await conexao.fetchrow(
                "SELECT id, username, gm_privileges, is_banned, password_hash "
                "FROM accounts WHERE id = $1", sessao["conta_id"],
            )
        if (conta is None or conta["is_banned"] or conta["gm_privileges"] <= 0
                or not hmac.compare_digest(sessao["impressao"], impressao_senha(conta["password_hash"]))):
            await self.redis.delete(chave)
            raise HTTPException(401, "Acesso GM revogado. Entre novamente.")
        if requisicao.method not in ("GET", "HEAD", "OPTIONS"):
            verificar_origem(requisicao)
            if not hmac.compare_digest(requisicao.headers.get("x-csrf-token", ""), sessao["csrf"]):
                raise HTTPException(403, "Recarregue a sessão antes de realizar esta operação.")
        return {"conta_id": conta["id"], "usuario": conta["username"], "csrf": sessao["csrf"]}

    async def sair(self, token):
        await self.redis.delete(f"{self.prefixo}sessoes:{chave_sessao(token)}")
