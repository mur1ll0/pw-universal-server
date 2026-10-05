"""API autenticada e interface. Operações ainda sem integração retornam indisponível."""
from contextlib import asynccontextmanager
import logging
import os
from pathlib import Path

import asyncpg
import redis.asyncio as aioredis
from fastapi import FastAPI, HTTPException, Request, Response
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from fastapi.exceptions import RequestValidationError
from pydantic import BaseModel, Field

from .realms import listar_realms
from .consultas import registrar_consultas
from .contas import registrar_contas
from .seguranca import COOKIE, TEMPO_SESSAO, LimitarEntrada, Seguranca

REGISTRO = logging.getLogger("pw_admin")
ESTATICOS = Path(__file__).resolve().parents[1] / "static"


class PedidoLogin(BaseModel):
    usuario: str = Field(min_length=1, max_length=64)
    senha: str = Field(min_length=1, max_length=256)


def criar_aplicacao():
    @asynccontextmanager
    async def ciclo_de_vida(app):
        pool = None
        redis = None
        try:
            pool = await asyncpg.create_pool(os.environ["DATABASE_URL"], min_size=1, max_size=10,
                                            command_timeout=5, timeout=5)
            redis = aioredis.from_url(os.getenv("REDIS_URL", "redis://localhost:6379"),
                                     decode_responses=True, socket_connect_timeout=2, socket_timeout=2)
            await redis.ping()
            app.state.seguranca = Seguranca(pool, redis, os.getenv("ADMIN_REDIS_PREFIX", "pw-admin:"))
            yield
        finally:
            if redis is not None:
                await redis.aclose()
            if pool is not None:
                await pool.close()

    app = FastAPI(title="Administração Perfect World", version="3.0.0", lifespan=ciclo_de_vida,
                  docs_url=None, redoc_url=None, openapi_url=None)

    @app.middleware("http")
    async def proteger(requisicao: Request, proximo):
        caminho = requisicao.url.path
        try:
            if caminho.startswith("/api/"):
                seguranca = getattr(app.state, "seguranca", None)
                if seguranca is None:
                    raise HTTPException(503, "Serviço administrativo indisponível.")
                if not (caminho == "/api/sessao/entrar" and requisicao.method == "POST"):
                    requisicao.state.administrador = await seguranca.autorizar(requisicao)
            resposta = await proximo(requisicao)
        except HTTPException as erro:
            resposta = JSONResponse({"detail": erro.detail}, status_code=erro.status_code)
        except Exception as erro:
            REGISTRO.error("dependencia_admin_indisponivel tipo=%s", type(erro).__name__)
            resposta = JSONResponse({"detail": "Serviço indisponível. Tente novamente."}, status_code=503)
        resposta.headers["X-Content-Type-Options"] = "nosniff"
        resposta.headers["Referrer-Policy"] = "same-origin"
        resposta.headers["Content-Security-Policy"] = (
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; "
            "connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"
        )
        resposta.headers["Cache-Control"] = "no-store"
        return resposta

    @app.exception_handler(RequestValidationError)
    async def dados_invalidos(requisicao, erro):
        # O erro padrão de validação inclui os valores recebidos, inclusive senhas.
        return JSONResponse({"detail": "Confira os campos informados."}, status_code=422)

    @app.post("/api/sessao/entrar")
    async def entrar(pedido: PedidoLogin, requisicao: Request):
        token = await app.state.seguranca.entrar(requisicao, pedido.usuario, pedido.senha)
        resposta = JSONResponse({"estado": "autenticado"})
        resposta.set_cookie(COOKIE, token, max_age=TEMPO_SESSAO, httponly=True, samesite="strict",
                            secure=os.getenv("ADMIN_COOKIE_SECURE", "true").lower() == "true", path="/")
        return resposta

    @app.get("/api/sessao")
    async def sessao(requisicao: Request):
        return requisicao.state.administrador

    @app.post("/api/sessao/sair")
    async def sair(requisicao: Request):
        await app.state.seguranca.sair(requisicao.cookies[COOKIE])
        resposta = Response(status_code=204)
        resposta.delete_cookie(COOKIE, path="/")
        return resposta

    @app.get("/api/realms")
    async def realms():
        return {"realms": await listar_realms(app.state.seguranca.pool)}

    registrar_consultas(app)
    registrar_contas(app)

    @app.api_route("/api/{caminho:path}", methods=["GET", "POST", "PUT", "PATCH", "DELETE"])
    async def operacao_indisponivel(caminho: str):
        raise HTTPException(501, "Operação ainda indisponível na reforma. Nenhuma alteração foi aplicada.")

    @app.get("/", include_in_schema=False)
    async def inicio():
        return FileResponse(ESTATICOS / "index.html")

    app.mount("/static", StaticFiles(directory=ESTATICOS), name="static")
    # Antes do middleware HTTP: ele pode continuar lendo o corpo ao esperar desconexão.
    app.add_middleware(LimitarEntrada)
    return app
