"""Consultas somente de leitura com origem explícita e alvo validado no banco."""
from fastapi import HTTPException, Path, Query, Request, Response
from .icones import png_do_icone
from .canal import CanalIndisponivel, consultar_daemons, resposta_do_primeiro
import json
import pathlib

# Gerado por scripts/gerar_catalogo_de_mapas.py (B183); lido uma vez.
CATALOGO_DE_MAPAS = json.loads((pathlib.Path(__file__).parent / "catalogo_mapas.json").read_text(encoding="utf-8"))


def registrar_consultas(app):
    async def validar_realm(realm_id):
        async with app.state.seguranca.pool.acquire() as conexao:
            existe = await conexao.fetchval("SELECT 1 FROM realms WHERE id=$1", realm_id)
        if not existe:
            raise HTTPException(404, "Realm não encontrado.")

    @app.get("/api/realms/{realm_id}/estado")
    async def consultar_estado(realm_id: str, requisicao: Request):
        await validar_realm(realm_id)
        try:
            respostas = await consultar_daemons(realm_id, requisicao.state.administrador["conta_id"], {"tipo": "mundos"})
            if any(resposta.get("presenca") != "observada" for resposta in respostas):
                return {"realm_id": realm_id, "estado": "em_transicao", "jogadores_online": None, "mapas": []}
            mapas = [mapa for resposta in respostas for mapa in resposta["mapas"]]
            if len({mapa["mapa"] for mapa in mapas}) != len(mapas):
                raise CanalIndisponivel("Mais de um daemon declarou o mesmo mapa; roteamento ambíguo.")
            # Rates em memória (E7): só afirma se todos os daemons do realm concordam.
            taxas = [resposta.get("taxas") for resposta in respostas]
            # Mapas que algum processo sabe montar (B183): o painel só oferece ligar esses.
            carregaveis = sorted({m for resposta in respostas for m in resposta.get("carregaveis", [])})
            return {"realm_id": realm_id, "estado": "consultado", "origem": "viva", "mapas": mapas,
                    "carregaveis": carregaveis,
                    "jogadores_online": sum(mapa["jogadores_online"] for mapa in mapas),
                    "taxas": taxas[0] if taxas and all(t == taxas[0] for t in taxas) else None}
        except CanalIndisponivel as erro:
            return {"realm_id": realm_id, "estado": "desconhecido", "jogadores_online": None,
                    "mapas": [], "aviso": str(erro)}

    @app.get("/api/icones/{sexo}/{titulo}.png")
    async def icone(sexo: str = Path(..., pattern="^[mf]$"), titulo: str = Path(..., pattern="^[0-9a-f]{2,256}$")):
        """B188: o ícone do item (arquivo em bytes GBK, em hexadecimal, como o GS manda),
        recortado do atlas do cliente; `m`/`f` = atlas masculino/feminino."""
        try:
            png = png_do_icone(sexo, titulo)
        except (OSError, ValueError):
            raise HTTPException(503, "Atlas de ícones indisponível.")
        if png is None:
            raise HTTPException(404, "Ícone não está no atlas.")
        return Response(content=png, media_type="image/png", headers={"Cache-Control": "private, max-age=86400"})

    @app.get("/api/realms/{realm_id}/catalogo-mapas")
    async def catalogo_de_mapas(realm_id: str):
        """Todos os mapas da versão do realm (B183), do `gs.conf` original com os nomes do
        pwadmin — `scripts/gerar_catalogo_de_mapas.py`. Versão sem catálogo: lista vazia."""
        async with app.state.seguranca.pool.acquire() as conexao:
            versao = await conexao.fetchval("SELECT version FROM realms WHERE id=$1", realm_id)
        if versao is None:
            raise HTTPException(404, "Realm não encontrado.")
        return {"realm_id": realm_id, "versao": versao, "mapas": CATALOGO_DE_MAPAS.get(versao, [])}

    async def consultar_primeiro(realm_id, requisicao, consulta):
        """Leitura que qualquer GS do realm responde igual (banco + elements.data): o primeiro."""
        await validar_realm(realm_id)
        try:
            estado, dados = await resposta_do_primeiro(realm_id, requisicao.state.administrador["conta_id"], consulta)
        except CanalIndisponivel as erro:
            raise HTTPException(503, str(erro))
        if estado != "consultado":
            raise HTTPException(503, str(dados.get("codigo") or "Consulta recusada pelo servidor de mundo."))
        return dados

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}/inventario")
    async def inventario(realm_id: str, requisicao: Request, personagem_id: int = Path(..., ge=1, le=2_147_483_647)):
        """E6 (B186): bolsa, equipamento, armazém e bolsa de missão, com nomes do elements.data."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "inventario", "personagem_id": personagem_id})
        # Atlas de ícones por sexo (B188): `IconList_IvtrM`/`F` pelo `gender` do personagem
        # (`EC_GameUIMan.cpp:591-594`; 0 = masculino).
        async with app.state.seguranca.pool.acquire() as conexao:
            genero = await conexao.fetchval("SELECT gender FROM characters WHERE realm_id=$1 AND id=$2", realm_id, personagem_id)
        return {"realm_id": realm_id, "personagem_id": personagem_id, "sexo": "f" if genero else "m",
                "recipientes": dados.get("recipientes", {})}

    @app.get("/api/realms/{realm_id}/itens")
    async def buscar_itens(realm_id: str, requisicao: Request, busca: str = Query(..., min_length=1, max_length=64)):
        """E6 (B186): até 30 itens do elements.data do realm por nome ou id."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "buscar_itens", "texto": busca})
        return {"realm_id": realm_id, "itens": dados.get("itens", [])}

    @app.get("/api/realms/{realm_id}/personagens")
    async def buscar_personagens(realm_id: str, busca: str = Query("", max_length=64)):
        await validar_realm(realm_id)
        async with app.state.seguranca.pool.acquire() as conexao:
            registros = await conexao.fetch(
                "SELECT id,name,cls,level FROM characters WHERE realm_id=$1 AND NOT is_deleted "
                "AND name ILIKE $2 ORDER BY id LIMIT 50", realm_id, f"%{busca}%",
            )
        return {"realm_id": realm_id, "origem": "persistida",
                "personagens": [{"id": r["id"], "nome": r["name"], "classe": r["cls"], "nivel": r["level"]} for r in registros]}

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}")
    async def consultar_personagem(realm_id: str, requisicao: Request,
                                  personagem_id: int = Path(..., ge=1, le=2_147_483_647)):
        await validar_realm(realm_id)
        async with app.state.seguranca.pool.acquire() as conexao:
            registro = await conexao.fetchrow(
                "SELECT id,name,cls,level,cultivation,exp,sp,money,hp,mp,strength,agility,vitality,energy,"
                "potential_points,world_id,pos_x,pos_y,pos_z,updated_at FROM characters "
                "WHERE realm_id=$1 AND id=$2 AND NOT is_deleted", realm_id, personagem_id,
            )
        if registro is None:
            raise HTTPException(404, "Personagem não encontrado neste realm.")
        ficha = {
            "id": registro["id"], "nome": registro["name"], "classe": registro["cls"], "nivel": registro["level"],
            "cultivo": registro["cultivation"], "exp": str(registro["exp"]), "alma": str(registro["sp"]),
            "dinheiro": str(registro["money"]), "vida": registro["hp"], "mana": registro["mp"],
            "forca": registro["strength"], "agilidade": registro["agility"], "vitalidade": registro["vitality"],
            "energia": registro["energy"], "pontos": registro["potential_points"], "mapa": registro["world_id"],
            "posicao": {"x": registro["pos_x"], "y": registro["pos_y"], "z": registro["pos_z"]},
            "gravado_em": registro["updated_at"].isoformat(),
        }
        resultado = {"realm_id": realm_id, "personagem_id": personagem_id, "origem": "persistida",
                     "presenca": "desconhecida", "ficha": ficha, "edicao_disponivel": False}
        try:
            respostas = await consultar_daemons(realm_id, requisicao.state.administrador["conta_id"],
                                               {"tipo": "personagem", "personagem_id": personagem_id})
            online = [resposta for resposta in respostas if resposta.get("presenca") == "online"]
            if len(online) > 1:
                raise CanalIndisponivel("Personagem declarado online por mais de um daemon.")
            if any(resposta.get("presenca") == "inconsistente" for resposta in respostas):
                resultado["aviso"] = "O GS declarou estado de presença inconsistente. Dados persistidos; edição indisponível."
            elif any(resposta.get("presenca") == "em_transicao" for resposta in respostas):
                resultado["presenca"] = "em_transicao"
                resultado["aviso"] = "Entrada, saída ou troca de mapa em andamento. Atualize a consulta."
            elif online:
                if online[0]["ficha"]["id"] != personagem_id:
                    raise CanalIndisponivel("O daemon respondeu com outro personagem.")
                resultado.update(origem="viva", presenca="online", ficha=online[0]["ficha"])
            elif all(resposta.get("presenca") == "ausente" for resposta in respostas):
                resultado["presenca"] = "ausente_nos_daemons"
                resultado["aviso"] = "Dados persistidos; ausência observada não autoriza edição offline."
            else:
                raise CanalIndisponivel("Presença não reconhecida na resposta do daemon.")
        except CanalIndisponivel as erro:
            resultado["aviso"] = str(erro)
        return resultado
