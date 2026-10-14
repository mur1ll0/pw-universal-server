"""Recursos do cliente (P3, B235): versões publicadas, rascunhos, publicar e reverter.

O painel só repassa: quem guarda os arquivos, assina e publica é o pw-recursos
(`crates/pw-recursos/src/admin.rs`, porta 8101 só na rede interna). Aqui ficam a sessão do
GM (o middleware já autenticou e conferiu o CSRF), o autor de cada pedido e o segredo do
pw-recursos (`RECURSOS_URL`, `RECURSOS_SEGREDO`). Spec: `specs/07_RECURSOS_DO_CLIENTE.md` §6.
"""
import asyncio
import json
import os
import re
import urllib.error
import urllib.parse
import urllib.request

from fastapi import HTTPException, Request
from fastapi.responses import JSONResponse, Response

VERSAO = re.compile(r"^[A-Za-z0-9._-]{1,16}$")
TEMPO = 600  # publicar e enviar arquivo grande podem demorar


def _base():
    url = os.getenv("RECURSOS_URL", "").rstrip("/")
    segredo = os.getenv("RECURSOS_SEGREDO", "")
    if not url or len(segredo) < 16:
        raise HTTPException(503, "Recursos do cliente indisponíveis: RECURSOS_URL/RECURSOS_SEGREDO não configurados.")
    return url, segredo


def _versao(versao):
    if not VERSAO.match(versao):
        raise HTTPException(404, "Versão de cliente inválida.")
    return versao


def _pedir(metodo, caminho, consulta=None, corpo=None, tipo=None):
    url, segredo = _base()
    alvo = f"{url}{caminho}"
    if consulta:
        alvo += "?" + urllib.parse.urlencode({k: v for k, v in consulta.items() if v is not None})
    cabecalhos = {"x-recursos-segredo": segredo}
    if tipo:
        cabecalhos["content-type"] = tipo
    pedido = urllib.request.Request(alvo, data=corpo, method=metodo, headers=cabecalhos)
    try:
        with urllib.request.urlopen(pedido, timeout=TEMPO) as r:
            return r.status, r.headers, r.read()
    except urllib.error.HTTPError as erro:
        return erro.code, erro.headers, erro.read()
    except (urllib.error.URLError, OSError):
        raise HTTPException(503, "O serviço de recursos do cliente não respondeu.")


async def _repassar(metodo, caminho, consulta=None, dados=None, corpo=None, tipo=None):
    if dados is not None:
        corpo, tipo = json.dumps(dados).encode(), "application/json"
    status, cabecalhos, conteudo = await asyncio.to_thread(_pedir, metodo, caminho, consulta, corpo, tipo)
    if cabecalhos.get("content-type", "").startswith("application/json"):
        return JSONResponse(json.loads(conteudo or b"null"), status_code=status)
    if status >= 400:
        return JSONResponse({"detail": "Falha no serviço de recursos do cliente."}, status_code=502)
    saida = Response(conteudo, status_code=status, media_type="application/octet-stream")
    if cabecalhos.get("content-disposition"):
        saida.headers["Content-Disposition"] = cabecalhos["content-disposition"]
    return saida


def _autor(requisicao):
    return requisicao.state.administrador["conta_id"]


def _operacao(dados):
    operacao = str((dados or {}).get("operacao_id", ""))
    if not 8 <= len(operacao) <= 64:
        raise HTTPException(422, "operacao_id inválido.")
    return operacao


def registrar_recursos(app):
    @app.get("/api/recursos/versoes")
    async def versoes():
        return await _repassar("GET", "/admin/versoes")

    @app.get("/api/recursos/{versao}/alteracoes")
    async def alteracoes(versao: str, limite: int = 50):
        return await _repassar("GET", f"/admin/{_versao(versao)}/alteracoes", {"limite": max(1, min(limite, 500))})

    @app.get("/api/recursos/{versao}/arquivos")
    async def arquivos(versao: str, busca: str = "", limite: int = 50):
        return await _repassar("GET", f"/admin/{_versao(versao)}/arquivos",
                               {"busca": busca[:200], "limite": max(1, min(limite, 500))})

    @app.get("/api/recursos/{versao}/arquivo")
    async def baixar(versao: str, caminho: str):
        return await _repassar("GET", f"/admin/{_versao(versao)}/arquivo", {"caminho": caminho[:400]})

    @app.post("/api/recursos/{versao}/rascunhos")
    async def criar(versao: str, requisicao: Request):
        dados = await requisicao.json()
        descricao = str(dados.get("descricao", "")).strip()[:500]
        if not descricao:
            raise HTTPException(422, "Descreva a alteração.")
        return await _repassar("POST", f"/admin/{_versao(versao)}/rascunhos", dados={
            "autor": _autor(requisicao), "descricao": descricao, "operacao_id": _operacao(dados)})

    @app.put("/api/recursos/{versao}/rascunhos/{rascunho}/arquivo")
    async def enviar(versao: str, rascunho: int, caminho: str, requisicao: Request, destino: str = "cliente"):
        if destino not in ("cliente", "servidor", "ambos"):
            raise HTTPException(422, "Destino inválido.")
        corpo = await requisicao.body()
        if not corpo:
            raise HTTPException(422, "Arquivo vazio.")
        return await _repassar("PUT", f"/admin/{_versao(versao)}/rascunhos/{rascunho}/arquivo",
                               {"caminho": caminho[:400], "destino": destino}, corpo=corpo,
                               tipo="application/octet-stream")

    @app.delete("/api/recursos/{versao}/rascunhos/{rascunho}/arquivo")
    async def tirar(versao: str, rascunho: int, caminho: str):
        return await _repassar("DELETE", f"/admin/{_versao(versao)}/rascunhos/{rascunho}/arquivo",
                               {"caminho": caminho[:400]})

    @app.post("/api/recursos/{versao}/rascunhos/{rascunho}/remover")
    async def remover(versao: str, rascunho: int, caminho: str):
        return await _repassar("POST", f"/admin/{_versao(versao)}/rascunhos/{rascunho}/remover",
                               {"caminho": caminho[:400]})

    @app.post("/api/recursos/{versao}/rascunhos/{rascunho}/descartar")
    async def descartar(versao: str, rascunho: int):
        return await _repassar("POST", f"/admin/{_versao(versao)}/rascunhos/{rascunho}/descartar")

    @app.post("/api/recursos/{versao}/publicar")
    async def publicar(versao: str, requisicao: Request):
        dados = await requisicao.json()
        rascunhos = dados.get("rascunhos")
        if not isinstance(rascunhos, list) or not rascunhos or not all(isinstance(x, int) for x in rascunhos):
            raise HTTPException(422, "Escolha os rascunhos a publicar.")
        minima = dados.get("minima")
        if minima is not None and not isinstance(minima, int):
            raise HTTPException(422, "Versão mínima inválida.")
        return await _repassar("POST", f"/admin/{_versao(versao)}/publicar", dados={
            "rascunhos": rascunhos[:100], "autor": _autor(requisicao), "minima": minima})

    @app.post("/api/recursos/{versao}/alteracoes/{alteracao}/reverter")
    async def reverter(versao: str, alteracao: int, requisicao: Request):
        dados = await requisicao.json()
        return await _repassar("POST", f"/admin/{_versao(versao)}/alteracoes/{alteracao}/reverter", dados={
            "autor": _autor(requisicao), "operacao_id": _operacao(dados)})


# ------------------------------------------------------------------ Estúdio (S1, B237)
# O Estúdio é do realm (`docs/admin/ESTUDIO_DO_PAINEL.md` §0): a versão do realm decide o
# arquivo e o layout. As edições ficam nos rascunhos da versão de cliente do realm.

TABELA = re.compile(r"^[A-Z0-9_]{1,64}$")
REALM = re.compile(r"^[A-Za-z0-9_]{1,32}$")


async def _versao_do_realm(requisicao, realm):
    if not REALM.match(realm):
        raise HTTPException(404, "Realm inválido.")
    async with requisicao.app.state.seguranca.pool.acquire() as conexao:
        versao = await conexao.fetchval("SELECT version FROM realms WHERE id=$1", realm)
    if not versao:
        raise HTTPException(404, "Realm inexistente.")
    return _versao(versao)


def _tabela(tabela):
    if not TABELA.match(tabela):
        raise HTTPException(404, "Tabela inválida.")
    return tabela


def registrar_estudio(app):
    @app.get("/api/realms/{realm}/estudio")
    async def estudio_versao(realm: str, requisicao: Request):
        return {"versao": await _versao_do_realm(requisicao, realm)}

    @app.get("/api/realms/{realm}/estudio/tabelas")
    async def estudio_tabelas(realm: str, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/tabelas", {"rascunho": rascunho})

    @app.get("/api/realms/{realm}/estudio/tabelas/{tabela}")
    async def estudio_listar(realm: str, tabela: str, requisicao: Request, rascunho: int | None = None,
                             busca: str = "", pagina: int = 0, por_pagina: int = 50):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/tabelas/{_tabela(tabela)}", {
            "rascunho": rascunho, "busca": busca[:100], "pagina": max(0, pagina),
            "por_pagina": max(1, min(por_pagina, 200))})

    @app.get("/api/realms/{realm}/estudio/tabelas/{tabela}/{indice}")
    async def estudio_ficha(realm: str, tabela: str, indice: int, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/tabelas/{_tabela(tabela)}/{max(0, indice)}",
                               {"rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/tabelas/{tabela}/{indice}")
    async def estudio_editar(realm: str, tabela: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if not isinstance(dados.get("campos"), dict) or not dados["campos"]:
            raise HTTPException(422, "Nenhum campo alterado.")
        return await _repassar("PUT", f"/admin/{v}/estudio/tabelas/{_tabela(tabela)}/{max(0, indice)}",
                               {"rascunho": rascunho}, dados={"campos": dados["campos"]})

    @app.post("/api/realms/{realm}/estudio/tabelas/{tabela}/{indice}/clonar")
    async def estudio_clonar(realm: str, tabela: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("POST", f"/admin/{v}/estudio/tabelas/{_tabela(tabela)}/{max(0, indice)}/clonar",
                               {"rascunho": rascunho})

    @app.delete("/api/realms/{realm}/estudio/tabelas/{tabela}/{indice}")
    async def estudio_remover(realm: str, tabela: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("DELETE", f"/admin/{v}/estudio/tabelas/{_tabela(tabela)}/{max(0, indice)}",
                               {"rascunho": rascunho})
