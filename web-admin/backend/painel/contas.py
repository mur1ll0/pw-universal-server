"""Contas globais e primeira escrita pelo GS com resultado durável, sem SQL de escrita."""
from fastapi import HTTPException, Path, Query, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel, ConfigDict, Field
from .canal import CanalIndisponivel, comandar

PADRAO_ID = r"^[A-Za-z0-9_-]{1,64}$"


class TrocaSenha(BaseModel):
    model_config = ConfigDict(extra="forbid")
    operacao_id: str = Field(pattern=PADRAO_ID)
    senha: str = Field(min_length=1, max_length=64, pattern=r"^[\x20-\x7e]+$")


class CriacaoConta(TrocaSenha):
    usuario: str = Field(min_length=1, max_length=64, pattern=r"^[A-Za-z0-9_]+$")


class GestaoGm(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    operacao_id: str = Field(pattern=PADRAO_ID)
    habilitado: bool


def registrar_contas(app):
    async def validar_realm(realm):
        async with app.state.seguranca.pool.acquire() as conexao:
            if not await conexao.fetchval("SELECT EXISTS(SELECT 1 FROM realms WHERE id=$1)", realm):
                raise HTTPException(404, "Realm não encontrado.")

    async def enviar(realm, administrador, consulta, id_operacao=None):
        await validar_realm(realm)
        id_resultado = id_operacao or consulta["comando_id"]
        try:
            resposta = await comandar(realm, administrador, consulta, id_operacao)
        except CanalIndisponivel:
            return JSONResponse({"operacao_id": id_resultado, "estado": "desconhecido",
                                 "alcance": "global", "codigo": "canal_indisponivel"}, status_code=202)
        dados = resposta["dados"]
        codigo = dados.get("codigo")
        status = {"administrador_recusado": 403, "operacao_em_conflito": 409,
                  "conta_inexistente": 404, "usuario_existente": 409}.get(codigo, 200)
        if resposta["estado"] in ("desconhecido", "pendente"):
            status = 202
        elif resposta["estado"] == "falha" and status == 200:
            status = 400
        return JSONResponse({**dados, "estado": resposta["estado"],
                             "operacao_id": id_resultado, "alcance": "global"}, status_code=status)

    @app.get("/api/contas")
    async def listar(busca: str = Query("", max_length=64)):
        async with app.state.seguranca.pool.acquire() as conexao:
            registros = await conexao.fetch(
                "SELECT id,username,gm_privileges,is_banned,gold_balance FROM accounts "
                "WHERE username ILIKE $1 ORDER BY id LIMIT 50", f"%{busca}%")
        return {"alcance": "global", "contas": [{"id": r["id"], "usuario": r["username"],
                "gm": r["gm_privileges"], "banida": r["is_banned"],
                "gold": str(r["gold_balance"])} for r in registros]}

    @app.post("/api/realms/{realm_id}/contas")
    async def criar(realm_id: str, pedido: CriacaoConta, requisicao: Request):
        return await enviar(realm_id, requisicao.state.administrador["conta_id"],
                            {"tipo": "criar_conta", "usuario": pedido.usuario.lower(),
                             "senha": pedido.senha}, pedido.operacao_id)

    @app.post("/api/realms/{realm_id}/contas/{conta_id}/senha")
    async def trocar(realm_id: str, pedido: TrocaSenha, requisicao: Request,
                     conta_id: int = Path(..., ge=1, le=2_147_483_647)):
        return await enviar(realm_id, requisicao.state.administrador["conta_id"],
                            {"tipo": "trocar_senha", "conta_id": conta_id, "senha": pedido.senha},
                            pedido.operacao_id)

    @app.post("/api/realms/{realm_id}/contas/{conta_id}/gm")
    async def gerir_gm(realm_id: str, pedido: GestaoGm, requisicao: Request,
                      conta_id: int = Path(..., ge=1, le=2_147_483_647)):
        return await enviar(realm_id, requisicao.state.administrador["conta_id"],
                            {"tipo": "definir_gm", "conta_id": conta_id,
                             "habilitado": pedido.habilitado}, pedido.operacao_id)

    @app.get("/api/realms/{realm_id}/operacoes/{operacao_id}")
    async def recuperar(realm_id: str, requisicao: Request,
                        operacao_id: str = Path(..., pattern=PADRAO_ID)):
        return await enviar(realm_id, requisicao.state.administrador["conta_id"],
                            {"tipo": "resultado", "comando_id": operacao_id})
