"""Contas globais e primeira escrita pelo GS com resultado durável, sem SQL de escrita."""
from typing import Annotated, Literal, Optional
from fastapi import HTTPException, Path, Query, Request
from fastapi.responses import JSONResponse
from .textos import habilidades_do_cliente
from pydantic import BaseModel, ConfigDict, Field
from .canal import (CanalIndisponivel, CanalNaoEnviado, aplicar_no_realm, comandar, resposta_do_primeiro,
                    respostas_do_realm, transmitir)

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


class AjusteGold(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    operacao_id: str = Field(pattern=PADRAO_ID)
    # Unidades do cash (100 = 1 gold); teto = PLAYER_CASH i32 (administracao.rs).
    # Só dar (B180): tirar arrisca saldo negativo.
    delta: int = Field(ge=1, le=2_147_483_647)


class Banimento(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    operacao_id: str = Field(pattern=PADRAO_ID)
    banida: bool
    motivo: Optional[str] = Field(default=None, max_length=120, pattern=r"^[^\x00-\x1f\x7f]*$")


class Rates(BaseModel):
    """Rates do realm (E7). Colunas NUMERIC(3,1): 0,1 a 99,9, uma casa (o GS arredonda)."""
    model_config = ConfigDict(extra="forbid")
    exp: float = Field(ge=0.1, le=99.9)
    sp: float = Field(ge=0.1, le=99.9)
    drop: float = Field(ge=0.1, le=99.9)
    moedas: float = Field(ge=0.1, le=99.9)


class EstadoDoMapa(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    ligado: bool


class PosicaoPedida(BaseModel):
    """E6 (B185): mapa e coordenadas; `y` ausente = o chão (o GS confere mapa e terreno).
    ±100 000 é política do painel (os mapas cabem em ±50 000)."""
    model_config = ConfigDict(extra="forbid", strict=True)
    mapa: int = Field(ge=1, le=2_147_483_647)
    x: float = Field(ge=-100_000, le=100_000, allow_inf_nan=False)
    y: Optional[float] = Field(default=None, ge=-100_000, le=100_000, allow_inf_nan=False)
    z: float = Field(ge=-100_000, le=100_000, allow_inf_nan=False)


class ItemPedido(BaseModel):
    """E6 (B186): item a dar; 1–100 000 por operação é política do painel."""
    model_config = ConfigDict(extra="forbid", strict=True)
    id: int = Field(ge=1, le=2_147_483_647)
    quantidade: int = Field(ge=1, le=100_000)


class RemocaoPedida(BaseModel):
    """E6 (B187): tirar do slot; `id` confere o item; sem quantidade = a pilha inteira."""
    model_config = ConfigDict(extra="forbid", strict=True)
    recipiente: Literal["bolsa", "missao", "equipamento", "armazem"]
    slot: int = Field(ge=0, le=255)
    id: int = Field(ge=1, le=2_147_483_647)
    quantidade: Optional[int] = Field(default=None, ge=1, le=100_000)


class MovimentoPedido(BaseModel):
    """E6 (B191): arrastar o item `id` do `slot_de` de `de` para o `slot_para` de `para`,
    trocando com o que estiver lá. Pares, limites e posição no corpo: quem confere é o GS."""
    model_config = ConfigDict(extra="forbid", strict=True)
    de: Literal["bolsa", "missao", "equipamento", "armazem"]
    slot_de: int = Field(ge=0, le=255)
    id: int = Field(ge=1, le=2_147_483_647)
    para: Literal["bolsa", "missao", "equipamento", "armazem"]
    slot_para: int = Field(ge=0, le=255)


class RequisitosItem(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    nivel: int = Field(ge=0, le=32_767)
    classes: int = Field(ge=0, le=0xFFFF)
    forca: int = Field(ge=0, le=32_767)
    agilidade: int = Field(ge=0, le=32_767)
    vitalidade: int = Field(ge=0, le=32_767)
    energia: int = Field(ge=0, le=32_767)


class EfeitoItem(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    id: int = Field(ge=1, le=0x1FFF)
    args: list[int] = Field(default_factory=list, max_length=3)


class EdicaoItem(BaseModel):
    """E6 (B194): edição livre nos valores, não no formato — limites do bloco do item (i16 nos
    requisitos, 5 furos, 32 efeitos, nome de 20 caracteres, refino 0–12). Quem confere o
    resto (item é equipamento, pedra existe, bloco fecha) é o GS."""
    model_config = ConfigDict(extra="forbid", strict=True)
    quantidade: Optional[int] = Field(default=None, ge=1, le=2_147_483_647)
    durabilidade: Optional[int] = Field(default=None, ge=0, le=2_147_483_647)
    durabilidade_maxima: Optional[int] = Field(default=None, ge=0, le=2_147_483_647)
    requisitos: Optional[RequisitosItem] = None
    fabricante: Optional[str] = Field(default=None, max_length=20)
    efeitos: Optional[list[EfeitoItem]] = Field(default=None, max_length=32)
    refino: Optional[int] = Field(default=None, ge=0, le=12)
    pedras: Optional[list[Annotated[int, Field(ge=0, le=2_147_483_647)]]] = Field(default=None, max_length=5)


class ItemEditadoPedido(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    recipiente: Literal["bolsa", "missao", "equipamento", "armazem"]
    slot: int = Field(ge=0, le=255)
    id: int = Field(ge=1, le=2_147_483_647)
    edicao: EdicaoItem


class HabilidadePedida(BaseModel):
    """E6 (B196): `nivel` 0 remove; senão define o nível. O teto é o `max_level` do stub do cliente
    (`painel/habilidades_do_cliente.json`), conferido na rota."""
    model_config = ConfigDict(extra="forbid", strict=True)
    id: int = Field(ge=1, le=65_535)
    nivel: int = Field(ge=0, le=255)


class EdicaoPersonagem(BaseModel):
    """E5: um só de dinheiro (dar), exp/sp (somar), pontos (dar pontos livres), nivel (alvo,
    só sobe) ou cultivo (B182). Tetos: pacotes de recompensa de missão (u32/i32), teto de
    dinheiro do mundo (2 000 000 000) e 10 000 pontos por operação (política do painel).
    Teto de nível e cultivos válidos dependem do realm: quem confere é o GS. B184: `atributos`
    [força, agilidade, vitalidade, energia] (0–100 000, política do painel; total e piso
    conferidos pelo GS) ou `redistribuir: true`."""
    model_config = ConfigDict(extra="forbid", strict=True)
    operacao_id: str = Field(pattern=PADRAO_ID)
    dinheiro: Optional[int] = Field(default=None, ge=1, le=2_000_000_000)  # só dar (B180)
    exp: Optional[int] = Field(default=None, ge=0, le=2_000_000_000)
    sp: Optional[int] = Field(default=None, ge=0, le=2_000_000_000)
    pontos: Optional[int] = Field(default=None, ge=1, le=10_000)
    nivel: Optional[int] = Field(default=None, ge=2, le=2_147_483_647)
    cultivo: Optional[int] = Field(default=None, ge=0, le=255)
    atributos: Optional[list[Annotated[int, Field(ge=0, le=100_000)]]] = Field(default=None, min_length=4, max_length=4)
    redistribuir: Optional[Literal[True]] = None
    posicao: Optional[PosicaoPedida] = None
    item: Optional[ItemPedido] = None
    remover_item: Optional[RemocaoPedida] = None
    mover_item: Optional[MovimentoPedido] = None
    editar_item: Optional[ItemEditadoPedido] = None
    habilidade: Optional[HabilidadePedida] = None


def somar_desconexoes(resultado):
    total, falhas = 0, {}
    for realm, dados in resultado.items():
        if isinstance(dados, list):
            total += sum(int(d.get("desconectados", 0)) for d in dados)
        else:
            falhas[realm] = dados
    return total, falhas


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
        except CanalNaoEnviado as erro:
            # Nada saiu do painel: falha definitiva desta tentativa, repetir é seguro.
            return JSONResponse({"operacao_id": id_resultado, "estado": "falha",
                                 "alcance": "global", "codigo": "canal_nao_enviado",
                                 "mensagem": str(erro)}, status_code=503)
        except CanalIndisponivel:
            return JSONResponse({"operacao_id": id_resultado, "estado": "desconhecido",
                                 "alcance": "global", "codigo": "canal_indisponivel"}, status_code=202)
        dados = resposta["dados"]
        codigo = dados.get("codigo")
        status = {"administrador_recusado": 403, "operacao_em_conflito": 409,
                  "conta_inexistente": 404, "usuario_existente": 409,
                  "precisa_estar_online": 409, "em_transicao": 409}.get(codigo, 200)
        if resposta["estado"] in ("desconhecido", "pendente"):
            status = 202
        elif resposta["estado"] == "falha" and status == 200:
            status = 400
        return JSONResponse({**dados, "estado": resposta["estado"],
                             "operacao_id": id_resultado, "alcance": "global"}, status_code=status)

    @app.get("/api/contas")
    async def listar(busca: str = Query("", max_length=64), pagina: int = Query(1, ge=1, le=100_000),
                     por_pagina: int = Query(12, ge=1, le=48)):
        # Curingas do ILIKE escapados: "_" é comum em nomes de conta.
        padrao = "%" + busca.replace("!", "!!").replace("%", "!%").replace("_", "!_") + "%"
        async with app.state.seguranca.pool.acquire() as conexao:
            total = await conexao.fetchval(
                "SELECT count(*) FROM accounts WHERE username ILIKE $1 ESCAPE '!'", padrao)
            registros = await conexao.fetch(
                "SELECT a.id,a.username,a.gm_privileges,a.is_banned,a.gold_balance,a.created_at,"
                "a.last_login_at,(SELECT count(*) FROM characters c WHERE c.account_id=a.id "
                "AND NOT c.is_deleted) AS personagens FROM accounts a "
                "WHERE a.username ILIKE $1 ESCAPE '!' ORDER BY a.id LIMIT $2 OFFSET $3",
                padrao, por_pagina, (pagina - 1) * por_pagina)
        return {"alcance": "global", "total": total, "pagina": pagina, "por_pagina": por_pagina,
                "contas": [{"id": r["id"], "usuario": r["username"],
                "gm": r["gm_privileges"], "banida": r["is_banned"],
                "gold": str(r["gold_balance"]), "personagens": r["personagens"],
                "criada_em": r["created_at"].isoformat() if r["created_at"] else None,
                "ultimo_login": r["last_login_at"].isoformat() if r["last_login_at"] else None}
                for r in registros]}

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

    @app.post("/api/realms/{realm_id}/contas/{conta_id}/gold")
    async def ajustar_gold(realm_id: str, pedido: AjusteGold, requisicao: Request,
                           conta_id: int = Path(..., ge=1, le=2_147_483_647)):
        administrador = requisicao.state.administrador["conta_id"]
        resposta = await enviar(realm_id, administrador,
                                {"tipo": "ajustar_gold", "conta_id": conta_id, "delta": pedido.delta},
                                pedido.operacao_id)
        if resposta.status_code == 200:
            # Saldo já salvo; reenviar a quem está online é melhor esforço (sem isso o
            # cliente vê o valor novo ao abrir a Loja Gold, QUERY_CASH_INFO).
            await transmitir(administrador, {"tipo": "atualizar_cash", "conta_id": conta_id})
        return resposta

    @app.post("/api/realms/{realm_id}/contas/{conta_id}/ban")
    async def banir(realm_id: str, pedido: Banimento, requisicao: Request,
                    conta_id: int = Path(..., ge=1, le=2_147_483_647)):
        administrador = requisicao.state.administrador["conta_id"]
        motivo = (pedido.motivo or "").strip() or None
        resposta = await enviar(realm_id, administrador,
                                {"tipo": "definir_ban", "conta_id": conta_id, "banida": pedido.banida,
                                 "motivo": motivo if pedido.banida else None}, pedido.operacao_id)
        if resposta.status_code == 200 and pedido.banida:
            # Ban salvo: o login já recusa; quem está em jogo sai agora.
            await transmitir(administrador, {"tipo": "desconectar", "conta_id": conta_id})
        return resposta

    @app.post("/api/realms/{realm_id}/rates")
    async def definir_rates(realm_id: str, pedido: Rates, requisicao: Request):
        await validar_realm(realm_id)
        resultado = await aplicar_no_realm(realm_id, requisicao.state.administrador["conta_id"],
                                           {"tipo": "definir_taxas", **pedido.model_dump()})
        if isinstance(resultado, list):
            return {"estado": "aplicado", "realm_id": realm_id, "taxas": resultado[0].get("taxas")}
        return JSONResponse({"estado": "falha", "realm_id": realm_id, "mensagem": resultado},
                            status_code=400 if resultado.startswith("recusado") else 503)

    @app.post("/api/realms/{realm_id}/mapas/{mapa}")
    async def definir_mapa(realm_id: str, pedido: EstadoDoMapa, requisicao: Request,
                           mapa: int = Path(..., ge=1, le=2_147_483_647)):
        """E7 (B177, B183): o processo que serve (ou está carregando) o mapa aplica. Se nenhum
        serve, o primeiro do realm grava o estado e, ao ligar, carrega o mapa na hora."""
        await validar_realm(realm_id)
        administrador = requisicao.state.administrador["conta_id"]
        consulta = {"tipo": "definir_mapa", "mapa": mapa, "ligado": pedido.ligado}
        try:
            respostas = await respostas_do_realm(realm_id, administrador, consulta)
            if all(dados.get("codigo") == "mapa_nao_servido" for _, dados in respostas):
                respostas = [await resposta_do_primeiro(realm_id, administrador, {**consulta, "carregar": True})]
        except CanalIndisponivel as erro:
            return JSONResponse({"estado": "falha", "mensagem": str(erro)}, status_code=503)
        aplicados = [dados for estado, dados in respostas if estado == "aplicado"]
        if aplicados:
            return aplicados[0]
        codigos = sorted({str(dados.get("codigo")) for _, dados in respostas})
        status = 404 if codigos in (["mapa_nao_servido"], ["mapa_sem_dados"]) else 503
        return JSONResponse({"estado": "falha", "codigo": codigos[0] if len(codigos) == 1 else "falha",
                             "mensagem": ", ".join(codigos)}, status_code=status)

    @app.post("/api/realms/{realm_id}/personagens/{personagem_id}/editar")
    async def editar_personagem(realm_id: str, pedido: EdicaoPersonagem, requisicao: Request,
                                personagem_id: int = Path(..., ge=1, le=2_147_483_647)):
        """E5 (B179, B182): online aplica no jogo; offline grava no banco o que não exige a
        entidade (tudo menos EXP/SP). Quem decide online/offline é o GS."""
        experiencia = bool(pedido.exp or pedido.sp)
        simples = [k for k in ("dinheiro", "pontos", "nivel", "cultivo", "atributos", "redistribuir", "posicao", "item",
                               "remover_item", "mover_item", "editar_item", "habilidade") if getattr(pedido, k) is not None]
        if len(simples) + experiencia != 1:
            raise HTTPException(422, "Informe uma só edição: dinheiro, EXP/SP, pontos, nível, cultivo, atributos ou posição.")
        consulta = {"tipo": "editar_personagem", "personagem_id": personagem_id}
        if simples == ["editar_item"] and not pedido.editar_item.edicao.model_dump(exclude_none=True):
            raise HTTPException(422, "Informe ao menos um campo do item.")
        if simples == ["habilidade"]:
            try:
                conhecida = habilidades_do_cliente().get(pedido.habilidade.id)
            except OSError:
                raise HTTPException(503, "Dados de habilidade do cliente indisponíveis.")
            if conhecida is None:
                raise HTTPException(422, "Habilidade que o cliente não conhece.")
            if conhecida.get("nivel_maximo") and pedido.habilidade.nivel > conhecida["nivel_maximo"]:
                raise HTTPException(422, f"Nível acima do máximo da habilidade ({conhecida['nivel_maximo']}).")
        if simples in (["posicao"], ["item"], ["remover_item"], ["mover_item"], ["editar_item"], ["habilidade"]):
            consulta[simples[0]] = getattr(pedido, simples[0]).model_dump(exclude_none=True)
        elif simples:
            consulta[simples[0]] = getattr(pedido, simples[0])
        else:
            consulta.update({k: v for k, v in (("exp", pedido.exp), ("sp", pedido.sp)) if v})
        resposta = await enviar(realm_id, requisicao.state.administrador["conta_id"], consulta,
                                pedido.operacao_id)
        return resposta

    @app.post("/api/contas/{conta_id}/desconectar")
    async def desconectar(requisicao: Request, conta_id: int = Path(..., ge=1, le=2_147_483_647)):
        resultado = await transmitir(requisicao.state.administrador["conta_id"],
                                     {"tipo": "desconectar", "conta_id": conta_id})
        if not resultado:
            raise HTTPException(503, "Nenhum realm com canal administrativo.")
        total, falhas = somar_desconexoes(resultado)
        estado = "aplicado" if not falhas else ("parcial" if len(falhas) < len(resultado) else "falha")
        return JSONResponse({"estado": estado, "conta_id": conta_id, "desconectados": total,
                             "realms_sem_resposta": falhas},
                            status_code=200 if estado != "falha" else 503)

    @app.get("/api/realms/{realm_id}/operacoes/{operacao_id}")
    async def recuperar(realm_id: str, requisicao: Request,
                        operacao_id: str = Path(..., pattern=PADRAO_ID)):
        return await enviar(realm_id, requisicao.state.administrador["conta_id"],
                            {"tipo": "resultado", "comando_id": operacao_id})
