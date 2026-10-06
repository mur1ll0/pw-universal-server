"""Consulta de configuração e conectividade; não presume presença ou mapas vivos."""
import asyncio
import json
import os

from .canal import CanalIndisponivel, configuracao


async def consultar_gateway(host, porta):
    escritor = None
    try:
        _, escritor = await asyncio.wait_for(asyncio.open_connection(host, porta), timeout=0.5)
        return "acessivel"
    except (OSError, asyncio.TimeoutError):
        return "inacessivel"
    finally:
        if escritor is not None:
            escritor.close()
            try:
                await asyncio.wait_for(escritor.wait_closed(), timeout=0.5)
            except (OSError, asyncio.TimeoutError):
                pass


def capacidades(versao):
    alvo = versao in ("1.2.6", "1.5.5")
    return [
        {"id": "configuracao", "nome": "Configuração do realm", "estado": "disponivel",
         "detalhe": "Identificação e contagem de personagens persistidos no banco."},
        {"id": "protocolo", "nome": "Protocolo do cliente", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "WorldProtocol presente no servidor." if alvo else "Versão fora dos alvos validados desta reforma."},
        {"id": "consulta_viva", "nome": "Personagens e mapas ao vivo", "estado": "indisponivel",
         "detalhe": "Aguarda canal administrativo autenticado com o servidor de mundo."},
        {"id": "edicao", "nome": "Edição de personagens", "estado": "indisponivel",
         "detalhe": "Aguarda coordenação com entrada, autosave e saída do jogo."},
        {"id": "rates", "nome": "Rates de EXP, SP, drop e moedas", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "Lidas pelo servidor de mundo na partida e trocadas na hora pelo painel."},
        {"id": "mapas", "nome": "Ligar e desligar mapas", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "Desligar salva e desconecta quem está no mapa e recusa novas entradas."},
        {"id": "dados", "nome": "Catálogo e ícones do realm", "estado": "indisponivel",
         "detalhe": "O leitor existente ainda precisa de validação de origem antes de ser exposto."},
    ]


def tem_canal(realm_id):
    try:
        configuracao(realm_id)
        return True
    except CanalIndisponivel:
        return False


async def listar_realms(pool):
    alvos = json.loads(os.getenv("ADMIN_GATEWAYS", "{}"))
    async with pool.acquire() as conexao:
        registros = await conexao.fetch(
            "SELECT r.id, r.name, r.version, r.host, r.port, r.double_exp_multiplier, "
            "r.double_sp_multiplier, r.double_drop_multiplier, r.double_gold_multiplier, "
            "(SELECT COUNT(*) FROM characters c WHERE c.realm_id=r.id AND NOT c.is_deleted) AS personagens "
            "FROM realms r WHERE r.id NOT LIKE 't\\_%' ESCAPE '\\' ORDER BY r.id"
        )

    async def consultar(registro):
        host = alvos.get(registro["id"], registro["host"])
        estado = await consultar_gateway(host, registro["port"])
        return {
            "id": registro["id"], "nome": registro["name"], "versao": registro["version"],
            "porta": registro["port"], "gateway": estado, "mundo": "desconhecido",
            "jogadores_online": None, "personagens_persistidos": registro["personagens"],
            "origem": "banco e teste TCP do gateway", "capacidades": capacidades(registro["version"]),
            "canal_administrativo": tem_canal(registro["id"]),
            # Colunas realms.double_*_multiplier (gravadas). O que vale no mundo vem da
            # consulta viva (`/estado` → `taxas`); a UI compara as duas.
            "rates": {"exp": float(registro["double_exp_multiplier"]),
                      "sp": float(registro["double_sp_multiplier"]),
                      "drop": float(registro["double_drop_multiplier"]),
                      "moedas": float(registro["double_gold_multiplier"]),
                      },
        }

    limite = asyncio.Semaphore(8)

    async def consultar_limitado(registro):
        async with limite:
            return await consultar(registro)

    return await asyncio.gather(*(consultar_limitado(registro) for registro in registros))
