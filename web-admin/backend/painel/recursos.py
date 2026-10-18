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

from .canal import CanalIndisponivel, aplicar_no_realm, consultar_daemons
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
    tipo_original = cabecalhos.get("content-type", "")
    saida = Response(conteudo, status_code=status,
                     media_type="image/png" if tipo_original == "image/png" else "application/octet-stream")
    if tipo_original == "image/png":
        saida.headers["Cache-Control"] = "private, max-age=86400"
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

    @app.get("/api/realms/{realm}/estudio/espacos/{espaco}")
    async def estudio_buscar(realm: str, espaco: str, requisicao: Request, rascunho: int | None = None,
                             busca: str = "", limite: int = 30):
        v = await _versao_do_realm(requisicao, realm)
        if espaco not in ("ESSENCE", "CONFIG", "FACE", "RECIPE", "ADDON", "TALK"):
            raise HTTPException(404, "Espaço inválido.")
        return await _repassar("GET", f"/admin/{v}/estudio/espacos/{espaco}", {
            "rascunho": rascunho, "busca": busca[:100], "limite": max(1, min(limite, 200))})

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


def registrar_conversas(app):
    """Conversas (`TALK_PROC`) do Estúdio, fase S3 (B240)."""
    @app.get("/api/realms/{realm}/estudio/funcoes")
    async def estudio_funcoes(realm: str, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/funcoes")

    @app.get("/api/realms/{realm}/estudio/conversas")
    async def estudio_conversas(realm: str, requisicao: Request, rascunho: int | None = None, busca: str = "",
                                pagina: int = 0, por_pagina: int = 50):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/conversas", {
            "rascunho": rascunho, "busca": busca[:100], "pagina": max(0, pagina), "por_pagina": max(1, min(por_pagina, 200))})

    @app.post("/api/realms/{realm}/estudio/conversas")
    async def estudio_nova_conversa(realm: str, rascunho: int, requisicao: Request, base: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("POST", f"/admin/{v}/estudio/conversas", {"rascunho": rascunho, "base": base})

    @app.get("/api/realms/{realm}/estudio/conversas/{indice}")
    async def estudio_conversa(realm: str, indice: int, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/conversas/{max(0, indice)}", {"rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/conversas/{indice}")
    async def estudio_salvar_conversa(realm: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if not isinstance(dados.get("conversa"), dict):
            raise HTTPException(422, "Conversa inválida.")
        return await _repassar("PUT", f"/admin/{v}/estudio/conversas/{max(0, indice)}", {"rascunho": rascunho},
                               dados={"conversa": dados["conversa"]})

    @app.delete("/api/realms/{realm}/estudio/conversas/{indice}")
    async def estudio_remover_conversa(realm: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("DELETE", f"/admin/{v}/estudio/conversas/{max(0, indice)}", {"rascunho": rascunho})


MAPA = re.compile(r"^[A-Za-z0-9_-]{1,32}$")


def registrar_mapas(app):
    """Mapas do Estúdio, fase S4 (B242): lista, peças do minimapa (PNG) e criaturas do npcgen."""
    @app.get("/api/realms/{realm}/estudio/mapas")
    async def estudio_mapas(realm: str, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/mapas")

    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/pecas/{peca}")
    async def estudio_peca(realm: str, caminho: str, peca: str, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho) or not re.match(r"^\d{4}\.png$", peca):
            raise HTTPException(404, "Peça inválida.")
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/pecas/{peca}")

    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/criaturas")
    async def estudio_criaturas(realm: str, caminho: str, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho):
            raise HTTPException(404, "Mapa inválido.")
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/criaturas")

    # Edição do npcgen (B243): áreas de criatura ("areas") e de recurso ("recursos") num rascunho.
    def _mapa_e_tipo(caminho, tipo):
        if not MAPA.match(caminho) or tipo not in ("areas", "recursos", "objetos", "controladores"):
            raise HTTPException(404, "Área inválida.")

    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/npcgen")
    async def estudio_npcgen(realm: str, caminho: str, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho):
            raise HTTPException(404, "Mapa inválido.")
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/npcgen", {"rascunho": rascunho})

    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}")
    async def estudio_area(realm: str, caminho: str, tipo: str, indice: int, requisicao: Request,
                           rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        _mapa_e_tipo(caminho, tipo)
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{max(0, indice)}",
                               {"rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}")
    async def estudio_editar_area(realm: str, caminho: str, tipo: str, indice: int, rascunho: int,
                                  requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        _mapa_e_tipo(caminho, tipo)
        dados = await requisicao.json()
        campos, filhos = dados.get("campos", {}), dados.get("filhos")
        if not isinstance(campos, dict) or not (filhos is None or (isinstance(filhos, list)
                                                                  and all(isinstance(f, dict) for f in filhos))):
            raise HTTPException(422, "Área inválida.")
        if not campos and filhos is None:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{max(0, indice)}",
                               {"rascunho": rascunho}, dados={"campos": campos, "filhos": filhos})

    @app.post("/api/realms/{realm}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}/clonar")
    async def estudio_clonar_area(realm: str, caminho: str, tipo: str, indice: int, rascunho: int,
                                  requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        _mapa_e_tipo(caminho, tipo)
        return await _repassar("POST", f"/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{max(0, indice)}/clonar",
                               {"rascunho": rascunho})

    @app.delete("/api/realms/{realm}/estudio/mapas/{caminho}/npcgen/{tipo}/{indice}")
    async def estudio_remover_area(realm: str, caminho: str, tipo: str, indice: int, rascunho: int,
                                   requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        _mapa_e_tipo(caminho, tipo)
        return await _repassar("DELETE", f"/admin/{v}/estudio/mapas/{caminho}/npcgen/{tipo}/{max(0, indice)}",
                               {"rascunho": rascunho})

    # Altura do chão (B248): os .hmap do mapa no realm, para "Posicionar pelo clique". Declarada antes da rota
    # genérica /mapas/{caminho}/{arquivo}, que a capturaria (o FastAPI usa a primeira que casa).
    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/altura")
    async def estudio_altura(realm: str, caminho: str, x: float, z: float, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho) or not (abs(x) < 1e6 and abs(z) < 1e6):
            raise HTTPException(404, "Mapa inválido.")
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/altura", {"x": x, "z": z})

    # Distritos (precinct.sev) e portais (region.sev) do mapa (B244): ponto de renascer e ponto de chegada.
    @app.get("/api/realms/{realm}/estudio/mapas/{caminho}/{arquivo}")
    async def estudio_sev(realm: str, caminho: str, arquivo: str, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho) or arquivo not in ("distritos", "portais", "teleportes", "rastreador"):
            raise HTTPException(404, "Mapa inválido.")
        return await _repassar("GET", f"/admin/{v}/estudio/mapas/{caminho}/{arquivo}", {"rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/mapas/{caminho}/{arquivo}/{indice}")
    async def estudio_editar_sev(realm: str, caminho: str, arquivo: str, indice: int, rascunho: int,
                                 requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho) or arquivo not in ("distritos", "portais"):
            raise HTTPException(404, "Mapa inválido.")
        campos = (await requisicao.json()).get("campos")
        if not isinstance(campos, dict) or not campos:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/mapas/{caminho}/{arquivo}/{max(0, indice)}",
                               {"rascunho": rascunho}, dados={"campos": campos})

    # Teleportes (B245): o world_targets.sev é um só para o realm; edita-se pelo id do destino.
    @app.put("/api/realms/{realm}/estudio/teleportes/{id_destino}")
    async def estudio_editar_teleporte(realm: str, id_destino: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        campos = (await requisicao.json()).get("campos")
        if not isinstance(campos, dict) or not campos:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/teleportes/{id_destino}", {"rascunho": rascunho},
                               dados={"campos": campos})

    # Rastreador de NPCs (B246): data\task_npc.data do cliente, pelo id do NPC.
    @app.put("/api/realms/{realm}/estudio/rastreador/{id_npc}")
    async def estudio_editar_rastreador(realm: str, id_npc: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        campos = (await requisicao.json()).get("campos")
        if not isinstance(campos, dict) or not campos or id_npc < 0:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/rastreador/{id_npc}", {"rascunho": rascunho},
                               dados={"campos": campos})


def registrar_reinicio(app):
    """B249: "aguarda reinício" e reiniciar o servidor de mundo pelo painel. O GS lê os arquivos do `config` uma vez,
    na partida; um item de servidor publicado só vale depois de reiniciar."""
    async def situacao(realm, requisicao):
        v = await _versao_do_realm(requisicao, realm)
        status, _, corpo = await asyncio.to_thread(_pedir, "GET", f"/admin/{v}/servidor/ultima")
        if status != 200:
            raise HTTPException(503, "O serviço de recursos do cliente não respondeu.")
        ultima = json.loads(corpo)
        try:
            respostas = await consultar_daemons(realm, requisicao.state.administrador["conta_id"], {"tipo": "mundos"})
        except CanalIndisponivel as erro:
            return {"versao": v, "ultima": ultima, "iniciado_em": None, "aguarda_reinicio": None, "aviso": str(erro)}
        inicios = [r.get("iniciado_em") for r in respostas if isinstance(r.get("iniciado_em"), int)]
        iniciado = min(inicios) if len(inicios) == len(respostas) and inicios else None
        publicada = ultima.get("publicada_em")
        aguarda = None if iniciado is None else bool(publicada and publicada > iniciado)
        online = sum(m.get("jogadores_online", 0) for r in respostas for m in r.get("mapas", []))
        return {"versao": v, "ultima": ultima, "iniciado_em": iniciado, "aguarda_reinicio": aguarda, "jogadores_online": online}

    @app.get("/api/realms/{realm}/servidor")
    async def servidor_do_realm(realm: str, requisicao: Request):
        return await situacao(realm, requisicao)

    @app.post("/api/realms/{realm}/reiniciar")
    async def reiniciar_realm(realm: str, requisicao: Request):
        """Desconecta todos (salvando) e encerra cada GS do realm; o Docker os sobe de novo."""
        await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if dados.get("confirmar") != realm:
            raise HTTPException(422, "Confirme digitando o id do realm.")
        resultado = await aplicar_no_realm(realm, requisicao.state.administrador["conta_id"], {"tipo": "reiniciar"})
        if isinstance(resultado, str):
            return JSONResponse({"estado": "falha", "mensagem": resultado}, status_code=503)
        return {"estado": "aplicado", "realm": realm, "online": sum(d.get("online", 0) for d in resultado),
                "processos": len(resultado)}

    # Distritos e portais novos/removidos (B251): gravam o .sev do realm e o .clt do cliente juntos.
    @app.post("/api/realms/{realm}/estudio/mapas/{caminho}/{arquivo}")
    async def estudio_novo_sev(realm: str, caminho: str, arquivo: str, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        # B252: destino de teleporte novo pelo mesmo caminho (base = id do destino a copiar).
        # B253: "alinhar" refaz o .sev do realm a partir do .clt do cliente (sem corpo).
        if not MAPA.match(caminho) or arquivo not in ("distritos", "portais", "teleportes", "alinhar"):
            raise HTTPException(404, "Mapa inválido.")
        if arquivo == "alinhar":
            return await _repassar("POST", f"/admin/{v}/estudio/mapas/{caminho}/alinhar", {"rascunho": rascunho})
        dados = await requisicao.json()
        base, campos = dados.get("base"), dados.get("campos", {})
        if not isinstance(base, int) or (base < 0 and arquivo != "teleportes") or not isinstance(campos, dict):
            raise HTTPException(422, "Pedido inválido.")
        return await _repassar("POST", f"/admin/{v}/estudio/mapas/{caminho}/{arquivo}", {"rascunho": rascunho},
                               dados={"base": base, "campos": campos})

    @app.delete("/api/realms/{realm}/estudio/mapas/{caminho}/{arquivo}/{indice}")
    async def estudio_remover_sev(realm: str, caminho: str, arquivo: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if not MAPA.match(caminho) or arquivo not in ("distritos", "portais"):
            raise HTTPException(404, "Mapa inválido.")
        return await _repassar("DELETE", f"/admin/{v}/estudio/mapas/{caminho}/{arquivo}/{max(0, indice)}", {"rascunho": rascunho})

    # Destinos de teleporte novos/removidos (B252): world_targets.sev do realm e world_targets.txt do cliente juntos.
    @app.delete("/api/realms/{realm}/estudio/teleportes/{id_destino}")
    async def estudio_remover_teleporte(realm: str, id_destino: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("DELETE", f"/admin/{v}/estudio/teleportes/{id_destino}", {"rascunho": rascunho})

    # Alinhamento cliente × servidor de todos os mapas (B253).
    @app.get("/api/realms/{realm}/estudio/alinhamento")
    async def estudio_alinhamento(realm: str, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/alinhamento", {"rascunho": rascunho})

    # Loja Gold (B254): data\gshop*.data, item "ambos" (o realm usa o mesmo arquivo do cliente).
    LOJA = ("gshop.data", "gshop1.data", "gshop2.data")

    @app.get("/api/realms/{realm}/estudio/loja/{arquivo}")
    async def estudio_loja(realm: str, arquivo: str, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if arquivo not in LOJA:
            raise HTTPException(404, "Loja inválida.")
        return await _repassar("GET", f"/admin/{v}/estudio/loja/{arquivo}", {"rascunho": rascunho})

    @app.post("/api/realms/{realm}/estudio/loja/{arquivo}")
    async def estudio_clonar_oferta(realm: str, arquivo: str, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        base = (await requisicao.json()).get("base")
        if arquivo not in LOJA or not isinstance(base, int) or base < 0:
            raise HTTPException(422, "Pedido inválido.")
        return await _repassar("POST", f"/admin/{v}/estudio/loja/{arquivo}", {"rascunho": rascunho}, dados={"base": base})

    @app.post("/api/realms/{realm}/estudio/loja/{arquivo}/categorias")
    async def estudio_categoria_da_loja(realm: str, arquivo: str, rascunho: int, requisicao: Request):
        """B257: renomear categoria/subcategoria, acrescentar ou tirar a última subcategoria."""
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if arquivo not in LOJA or dados.get("acao") not in ("renomear", "nova", "remover") or not isinstance(dados.get("categoria"), int):
            raise HTTPException(422, "Pedido inválido.")
        corpo = {k: dados[k] for k in ("acao", "categoria", "sub", "nome") if k in dados}
        return await _repassar("POST", f"/admin/{v}/estudio/loja/{arquivo}/categorias", {"rascunho": rascunho}, dados=corpo)

    @app.get("/api/realms/{realm}/estudio/loja/{arquivo}/{indice}")
    async def estudio_oferta(realm: str, arquivo: str, indice: int, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if arquivo not in LOJA:
            raise HTTPException(404, "Loja inválida.")
        return await _repassar("GET", f"/admin/{v}/estudio/loja/{arquivo}/{max(0, indice)}", {"rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/loja/{arquivo}/{indice}")
    async def estudio_editar_oferta(realm: str, arquivo: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        campos = (await requisicao.json()).get("campos")
        if arquivo not in LOJA or not isinstance(campos, dict) or not campos:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/loja/{arquivo}/{max(0, indice)}", {"rascunho": rascunho},
                               dados={"campos": campos})

    @app.delete("/api/realms/{realm}/estudio/loja/{arquivo}/{indice}")
    async def estudio_remover_oferta(realm: str, arquivo: str, indice: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        if arquivo not in LOJA:
            raise HTTPException(404, "Loja inválida.")
        return await _repassar("DELETE", f"/admin/{v}/estudio/loja/{arquivo}/{max(0, indice)}", {"rascunho": rascunho})

    # Missões (B255, só leitura): o tasks.data do realm.
    @app.get("/api/realms/{realm}/estudio/missoes")
    async def estudio_missoes(realm: str, requisicao: Request, busca: str = "", topo: bool = True, pagina: int = 0,
                              por_pagina: int = 100, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/missoes", {
            "rascunho": rascunho, "busca": busca[:100], "topo": "true" if topo else "false", "pagina": max(0, pagina),
            "por_pagina": max(1, min(por_pagina, 500))})

    @app.get("/api/realms/{realm}/estudio/missoes/{id_missao}")
    async def estudio_missao(realm: str, id_missao: int, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if id_missao <= 0:
            raise HTTPException(404, "Missão inválida.")
        return await _repassar("GET", f"/admin/{v}/estudio/missoes/{id_missao}", {"rascunho": rascunho})

    # Editar missão no lugar (B256): campos de tamanho fixo; item "ambos" (data\tasks.data).
    @app.put("/api/realms/{realm}/estudio/missoes/{id_missao}")
    async def estudio_editar_missao(realm: str, id_missao: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        campos = (await requisicao.json()).get("campos")
        if id_missao <= 0 or not isinstance(campos, dict) or not campos:
            raise HTTPException(422, "Nada a mudar.")
        return await _repassar("PUT", f"/admin/{v}/estudio/missoes/{id_missao}", {"rascunho": rascunho}, dados={"campos": campos})

    # Textos do cliente (B260): fixed_msg, item_desc, item_ext_desc (item "cliente").
    TEXTOS = ("fixed_msg.txt", "item_desc.txt", "item_ext_desc.txt")

    def _tabela_de_texto(arquivo: str) -> str:
        if arquivo not in TEXTOS:
            raise HTTPException(404, "Tabela de texto inválida.")
        return arquivo

    @app.get("/api/realms/{realm}/estudio/textos/{arquivo}")
    async def estudio_textos(realm: str, arquivo: str, requisicao: Request, busca: str = "", pagina: int = 0,
                             rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/textos/{_tabela_de_texto(arquivo)}",
                               {"busca": busca[:200], "pagina": max(0, pagina), "rascunho": rascunho})

    @app.put("/api/realms/{realm}/estudio/textos/{arquivo}/{chave}")
    async def estudio_editar_texto(realm: str, arquivo: str, chave: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if not isinstance(dados.get("texto"), str):
            raise HTTPException(422, "Texto inválido.")
        return await _repassar("PUT", f"/admin/{v}/estudio/textos/{_tabela_de_texto(arquivo)}/{chave}", {"rascunho": rascunho},
                               dados={"texto": dados["texto"]})

    @app.post("/api/realms/{realm}/estudio/textos/{arquivo}")
    async def estudio_novo_texto(realm: str, arquivo: str, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if not isinstance(dados.get("id"), int) or not isinstance(dados.get("texto"), str):
            raise HTTPException(422, "Pedido inválido.")
        return await _repassar("POST", f"/admin/{v}/estudio/textos/{_tabela_de_texto(arquivo)}", {"rascunho": rascunho},
                               dados={"id": dados["id"], "texto": dados["texto"]})

    # Comportamento dos monstros (B258, só leitura): o aipolicy.data do realm.
    @app.get("/api/realms/{realm}/estudio/comportamentos")
    async def estudio_comportamentos(realm: str, requisicao: Request, busca: str = ""):
        v = await _versao_do_realm(requisicao, realm)
        return await _repassar("GET", f"/admin/{v}/estudio/comportamentos", {"busca": busca[:100]})

    @app.get("/api/realms/{realm}/estudio/comportamentos/{id_politica}")
    async def estudio_comportamento(realm: str, id_politica: int, requisicao: Request, rascunho: int | None = None):
        v = await _versao_do_realm(requisicao, realm)
        if id_politica <= 0:
            raise HTTPException(404, "Política inválida.")
        return await _repassar("GET", f"/admin/{v}/estudio/comportamentos/{id_politica}", {"rascunho": rascunho})

    # Editar política no lugar (B259): flags do gatilho e parâmetros de tamanho fixo; item "servidor".
    @app.put("/api/realms/{realm}/estudio/comportamentos/{id_politica}")
    async def estudio_editar_comportamento(realm: str, id_politica: int, rascunho: int, requisicao: Request):
        v = await _versao_do_realm(requisicao, realm)
        dados = await requisicao.json()
        if id_politica <= 0 or not isinstance(dados.get("gatilho"), int):
            raise HTTPException(422, "Pedido inválido.")
        corpo = {k: dados[k] for k in ("gatilho", "flags", "operacao", "condicao") if k in dados}
        return await _repassar("PUT", f"/admin/{v}/estudio/comportamentos/{id_politica}", {"rascunho": rascunho}, dados=corpo)
