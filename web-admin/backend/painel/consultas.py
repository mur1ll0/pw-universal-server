"""Consultas somente de leitura com origem explícita e alvo validado no banco."""
from typing import Optional
from fastapi import HTTPException, Path, Query, Request, Response
from .icones import png_do_icone
from .dica import catalogo_de_efeitos, linhas_da_dica, texto_do_efeito
from .textos import habilidades_do_cliente, nome_da_classe, nomes_das_habilidades
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
    async def icone(sexo: str = Path(..., pattern="^(m|f|habilidade|mascote)$"),
                    titulo: str = Path(..., pattern="^[0-9a-f]{2,256}$")):
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

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}/itens/{recipiente}/{slot}/dica")
    async def dica_do_item(realm_id: str, requisicao: Request,
                           personagem_id: int = Path(..., ge=1, le=2_147_483_647),
                           recipiente: str = Path(..., pattern="^(bolsa|missao|equipamento|armazem)$"),
                           slot: int = Path(..., ge=0, le=255)):
        """E6 (B189): a dica do item do slot, montada como o cliente (`painel/dica.py`)."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "detalhe_item", "personagem_id": personagem_id,
                                                                "recipiente": recipiente, "slot": slot})
        # Versão (quantas classes há) e classe do personagem (a cor da linha de classe), B192.
        async with app.state.seguranca.pool.acquire() as conexao:
            dono = await conexao.fetchrow("SELECT r.version, c.cls FROM characters c JOIN realms r ON r.id=c.realm_id "
                                          "WHERE c.realm_id=$1 AND c.id=$2", realm_id, personagem_id)
        try:
            linhas = linhas_da_dica(dados, versao=dono["version"] if dono else None, classe=dono["cls"] if dono else None)
            # B206: a descrição de cada efeito, para a edição do item mostrar o mesmo que a dica.
            for efeito in ((dados.get("equipamento") or {}).get("efeitos") or []):
                efeito["texto"] = " ".join(texto_do_efeito(efeito) or [])
        except OSError:
            raise HTTPException(503, "Textos do cliente indisponíveis (data/textos).")
        # Nomes das classes da versão, para a edição do item (B194): 8 no 1.2.x, 12 no 1.5.5.
        total = 8 if dono and str(dono["version"]).startswith("1.2") else 12
        return {"realm_id": realm_id, "item": dados, "linhas": linhas,
                "classes": [nome_da_classe(i) for i in range(total)]}

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}/habilidades")
    async def habilidades_do_personagem(realm_id: str, personagem_id: int = Path(..., ge=1, le=2_147_483_647)):
        """E6 (B195): as habilidades aprendidas. O banco é a fonte que o GS usa — aprender grava
        nele e a entrada no mundo lê dele (`SKILL_DATA`) —, online ou offline. Nome do
        `skillstr.txt` e ícone do stub do cliente."""
        await validar_realm(realm_id)
        async with app.state.seguranca.pool.acquire() as conexao:
            existe = await conexao.fetchval("SELECT 1 FROM characters WHERE realm_id=$1 AND id=$2 AND NOT is_deleted",
                                            realm_id, personagem_id)
            if not existe:
                raise HTTPException(404, "Personagem não encontrado neste realm.")
            registros = await conexao.fetch("SELECT skill_id, level, ability FROM character_skills "
                                            "WHERE character_id=$1 ORDER BY skill_id", personagem_id)
        try:
            nomes, cliente = nomes_das_habilidades(), habilidades_do_cliente()
        except OSError:
            nomes, cliente = {}, {}
        return {"realm_id": realm_id, "origem": "persistida", "habilidades": [
            {"id": r["skill_id"], "nivel": r["level"], "proficiencia": r["ability"],
             "nome": nomes.get(r["skill_id"], f"Habilidade {r['skill_id']}"),
             "icone": (cliente.get(r["skill_id"]) or {}).get("icone", ""),
             "nivel_maximo": (cliente.get(r["skill_id"]) or {}).get("nivel_maximo")} for r in registros]}

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}/mascotes")
    async def mascotes_do_personagem(realm_id: str, requisicao: Request,
                                     personagem_id: int = Path(..., ge=1, le=2_147_483_647)):
        """E6 (B197): a jaula (registro de cada mascote, modelo, ícone; qual está invocado)."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "mascotes", "personagem_id": personagem_id})
        try:
            nomes = nomes_das_habilidades()
        except OSError:
            nomes = {}
        for m in dados.get("mascotes", []):
            m["habilidades"] = [{"id": i, "nivel": n, "nome": nomes.get(i, f"Habilidade {i}")} for i, n in m.get("habilidades", [])]
        return {"realm_id": realm_id, "mascotes": dados.get("mascotes", [])}

    @app.get("/api/habilidades")
    async def buscar_habilidades(busca: str = Query(..., min_length=1, max_length=64)):
        """E6 (B196): até 30 habilidades do cliente por nome (`skillstr.txt`) ou id, para ensinar."""
        try:
            nomes, cliente = nomes_das_habilidades(), habilidades_do_cliente()
        except OSError:
            raise HTTPException(503, "Textos do cliente indisponíveis (data/textos).")
        termo = busca.strip().lower()
        achadas = [i for i in sorted(cliente) if termo == str(i) or termo in nomes.get(i, "").lower()][:30]
        return {"habilidades": [{"id": i, "nome": nomes.get(i, f"Habilidade {i}"), "icone": cliente[i]["icone"],
                                 "nivel_maximo": cliente[i]["nivel_maximo"], "classe": cliente[i]["classe"]} for i in achadas]}

    @app.get("/api/realms/{realm_id}/personagens/{personagem_id}/missoes")
    async def missoes_do_personagem(realm_id: str, requisicao: Request,
                                    personagem_id: int = Path(..., ge=1, le=2_147_483_647),
                                    pagina: int = Query(0, ge=0, le=99),
                                    busca: Optional[str] = Query(None, max_length=64)):
        """E6 (B198): missões ativas (com o pai e o estado) e uma página de 200 concluídas, com o
        nome do tasks.data. Em jogo o GS lê a memória; fora, o banco."""
        await validar_realm(realm_id)
        async with app.state.seguranca.pool.acquire() as conexao:
            existe = await conexao.fetchval("SELECT 1 FROM characters WHERE realm_id=$1 AND id=$2 AND NOT is_deleted",
                                            realm_id, personagem_id)
        if not existe:
            raise HTTPException(404, "Personagem não encontrado neste realm.")
        consulta = {"tipo": "missoes", "personagem_id": personagem_id, "pagina": pagina}
        if busca and busca.strip():
            consulta["busca"] = busca.strip()
        dados = await consultar_primeiro(realm_id, requisicao, consulta)
        return {"realm_id": realm_id, **{k: dados.get(k) for k in
                ("ativas", "concluidas", "total_concluidas", "pagina", "por_pagina", "origem")}}

    @app.get("/api/realms/{realm_id}/missoes")
    async def buscar_missoes(realm_id: str, requisicao: Request, busca: str = Query(..., min_length=1, max_length=64)):
        """E6 (B198): até 30 missões de topo do tasks.data do realm por nome ou id."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "buscar_missoes", "texto": busca})
        return {"realm_id": realm_id, "missoes": dados.get("missoes", [])}

    @app.get("/api/efeitos")
    async def buscar_efeitos(busca: str = Query(..., min_length=1, max_length=64)):
        """B206: até 30 efeitos de item pelo texto que o cliente mostra ou pelo id."""
        try:
            catalogo = catalogo_de_efeitos()
        except OSError:
            raise HTTPException(503, "Textos do cliente indisponíveis (data/textos).")
        alvo = busca.strip().lower()
        achados = [(i, t) for i, t in catalogo if str(i) == alvo or alvo in t.lower()]
        achados.sort(key=lambda x: (str(x[0]) != alvo, x[0]))
        return {"efeitos": [{"id": i, "texto": t} for i, t in achados[:30]]}

    @app.get("/api/efeitos/{idprop}")
    async def descrever_efeito(idprop: int = Path(..., ge=1, le=8191),
                               args: str = Query("", max_length=64, pattern=r"^[-0-9, ]*$")):
        """B206: a descrição de um efeito com os parâmetros dados (`1, 2`), como a dica."""
        valores = [int(v) for v in args.replace(" ", "").split(",") if v not in ("", "-")][:3]
        try:
            linhas = texto_do_efeito({"id": idprop, "args": valores})
        except OSError:
            raise HTTPException(503, "Textos do cliente indisponíveis (data/textos).")
        return {"id": idprop, "linhas": linhas or []}

    @app.get("/api/realms/{realm_id}/efeitos")
    async def efeitos_para_item(realm_id: str, requisicao: Request, item: int = Query(..., ge=1, le=2_147_483_647),
                                busca: Optional[str] = Query(None, min_length=1, max_length=64),
                                ids: Optional[str] = Query(None, max_length=1200, pattern=r"^[0-9,]*$")):
        """B209: efeitos para a peça `item` do realm, com o valor do id e a edição que o GS dá.

        `busca`: até 30 efeitos que o cliente descreve com o texto (ou o id), filtrados pelo GS —
        só os que existem no realm, servem na família da peça e não são de refino/pedra/temporário.
        `ids`: a informação de cada id pedido (as linhas que o item já tem), sem filtro."""
        if busca:
            try:
                catalogo = catalogo_de_efeitos()
            except OSError:
                raise HTTPException(503, "Textos do cliente indisponíveis (data/textos).")
            alvo = busca.strip().lower()
            candidatos = [i for i, t in catalogo if str(i) == alvo or alvo in t.lower()]
            if alvo.isdigit() and int(alvo) not in candidatos and 1 <= int(alvo) <= 8191:
                candidatos.insert(0, int(alvo))
            candidatos.sort(key=lambda i: (str(i) != alvo, i))
        else:
            candidatos = [int(v) for v in (ids or "").split(",") if v and 1 <= int(v) <= 8191]
        candidatos = candidatos[:200]
        if not candidatos:
            return {"realm_id": realm_id, "efeitos": []}
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "efeitos_para_item", "item_id": item, "ids": candidatos})
        lista = dados.get("efeitos", [])
        if busca:
            lista = [e for e in lista if not e.get("recusa") and e.get("busca")][:30]
        for e in lista:
            if e.get("args") is not None:
                try:
                    e["texto"] = " ".join(texto_do_efeito({"id": e["id"], "args": e["args"]}) or [])
                except OSError:
                    e["texto"] = ""
        return {"realm_id": realm_id, "efeitos": lista}

    @app.get("/api/realms/{realm_id}/pedras")
    async def buscar_pedras(realm_id: str, requisicao: Request, busca: str = Query(..., min_length=1, max_length=64)):
        """B206: até 30 pedras (`STONE_ESSENCE`) do realm por nome ou id, com o grau."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "buscar_itens", "texto": busca, "categoria": "pedra"})
        return {"realm_id": realm_id, "pedras": dados.get("itens", [])}

    @app.get("/api/realms/{realm_id}/itens")
    async def buscar_itens(realm_id: str, requisicao: Request, busca: str = Query(..., min_length=1, max_length=64)):
        """E6 (B186): até 30 itens do elements.data do realm por nome ou id."""
        dados = await consultar_primeiro(realm_id, requisicao, {"tipo": "buscar_itens", "texto": busca})
        return {"realm_id": realm_id, "itens": dados.get("itens", [])}

    @app.get("/api/realms/{realm_id}/personagens")
    async def buscar_personagens(realm_id: str, busca: str = Query("", max_length=64),
                                 pagina: int = Query(1, ge=1, le=100_000),
                                 por_pagina: int = Query(12, ge=1, le=48)):
        """B190: personagens do realm de todas as contas, por nome do personagem ou da conta."""
        await validar_realm(realm_id)
        # Curingas do ILIKE escapados, como na busca de contas.
        padrao = "%" + busca.replace("!", "!!").replace("%", "!%").replace("_", "!_") + "%"
        filtro = ("FROM characters c JOIN accounts a ON a.id=c.account_id WHERE c.realm_id=$1 "
                  "AND NOT c.is_deleted AND (c.name ILIKE $2 ESCAPE '!' OR a.username ILIKE $2 ESCAPE '!')")
        async with app.state.seguranca.pool.acquire() as conexao:
            total = await conexao.fetchval(f"SELECT count(*) {filtro}", realm_id, padrao)
            registros = await conexao.fetch(
                f"SELECT c.id,c.name,c.cls,c.level,c.gender,a.id AS conta_id,a.username {filtro} "
                "ORDER BY c.id LIMIT $3 OFFSET $4", realm_id, padrao, por_pagina, (pagina - 1) * por_pagina,
            )
        return {"realm_id": realm_id, "origem": "persistida", "total": total, "pagina": pagina,
                "por_pagina": por_pagina,
                "personagens": [{"id": r["id"], "nome": r["name"], "classe": r["cls"],
                                 "classe_nome": nome_da_classe(r["cls"]), "nivel": r["level"],
                                 "sexo": "f" if r["gender"] == 1 else "m",
                                 "conta_id": r["conta_id"], "usuario": r["username"]} for r in registros]}

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
        if isinstance(resultado["ficha"].get("classe"), int):
            resultado["ficha"]["classe_nome"] = nome_da_classe(resultado["ficha"]["classe"])
        return resultado
