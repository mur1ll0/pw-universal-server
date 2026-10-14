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


def capacidades(versao, canal=False):
    """O que o painel faz neste realm hoje (B218: textos do que existe, não de etapas antigas).

    `canal`: há canal administrativo configurado para o realm (consulta viva e edição passam por ele).
    """
    alvo = versao in ("1.2.6", "1.5.5")
    vivo = alvo and canal
    pelo_gs = "implementado" if vivo else ("indisponivel" if alvo else "nao_validado")
    sem_canal = "Sem canal administrativo configurado para este realm." if alvo else "Versão fora dos alvos validados desta reforma."
    return [
        {"id": "configuracao", "nome": "Configuração do realm", "estado": "disponivel",
         "detalhe": "Identificação e contagem de personagens persistidos no banco."},
        {"id": "protocolo", "nome": "Protocolo do cliente", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "WorldProtocol presente no servidor." if alvo else "Versão fora dos alvos validados desta reforma."},
        {"id": "consulta_viva", "nome": "Personagens e mapas ao vivo", "estado": pelo_gs,
         "detalhe": "Ficha, inventário, habilidades, mascotes, missões e mapas lidos do GS (em jogo: a memória; fora: o banco)." if vivo else sem_canal},
        {"id": "edicao", "nome": "Edição de personagens", "estado": pelo_gs,
         "detalhe": "Pelo GS: vale na hora com o personagem em jogo; fora do jogo é gravada sob trava que recusa o login." if vivo else sem_canal},
        {"id": "rates", "nome": "Rates de EXP, SP, drop e moedas", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "Lidas pelo servidor de mundo na partida e trocadas na hora pelo painel."},
        {"id": "mapas", "nome": "Ligar e desligar mapas", "estado": "implementado" if alvo else "nao_validado",
         "detalhe": "Desligar salva e desconecta quem está no mapa e recusa novas entradas."},
        {"id": "templates", "nome": "Templates de criação", "estado": pelo_gs,
         "detalhe": "Com o que cada classe nasce; vale para os personagens criados depois." if vivo else sem_canal},
        {"id": "dados", "nome": "Catálogo, textos e ícones",
         "estado": "implementado" if versao == "1.5.5" else ("parcial" if alvo else "nao_validado"),
         "detalhe": ("Nomes e valores dos .data do realm pelo GS; textos e ícones do cliente 1.5.5 BR."
                     if versao == "1.5.5" else
                     "Nomes e valores dos .data do realm pelo GS; textos e ícones do cliente 1.5.5 BR, não conferidos no cliente desta versão.")},
        {"id": "editores_de_dados", "nome": "Editores de elements, tasks e npcgen", "estado": "indisponivel",
         "detalhe": "Fora desta reforma: só leitura. Contrato em docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md."},
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
            "origem": "banco e teste TCP do gateway",
            "capacidades": capacidades(registro["version"], tem_canal(registro["id"])),
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
