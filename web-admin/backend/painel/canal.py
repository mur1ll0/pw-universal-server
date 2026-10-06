"""Cliente do canal de consulta: framing limitado e HMAC com desafio por conexão."""
import asyncio
import hmac
import json
import os
import struct
import uuid


class CanalIndisponivel(Exception):
    pass


class CanalNaoEnviado(CanalIndisponivel):
    """O pedido não saiu do painel (sem chave, daemon recusou a conexão ou não
    cumprimentou): nada foi aplicado e repetir é seguro. Antes do B174 isso era
    mostrado como "resultado desconhecido"."""


def configuracao(realm_id):
    try:
        alvos = json.loads(os.getenv("ADMIN_DAEMONS", "{}")).get(realm_id, [])
        chaves = json.loads(os.getenv("ADMIN_CHANNEL_SECRETS", "{}"))
        chave = bytes.fromhex(chaves.get(realm_id, ""))
        if not alvos or len(chave) != 32:
            raise ValueError()
        if any(not isinstance(alvo["host"], str) or not 1 <= int(alvo["porta"]) <= 65535 for alvo in alvos):
            raise ValueError()
        if len({(alvo["host"], int(alvo["porta"])) for alvo in alvos}) != len(alvos):
            raise ValueError()
        return alvos, chave
    except (KeyError, TypeError, ValueError):
        raise CanalNaoEnviado("Canal administrativo não configurado para este realm.")


async def ler_quadro(leitor):
    tamanho = struct.unpack("!I", await leitor.readexactly(4))[0]
    if not 0 < tamanho <= 8192:
        raise CanalIndisponivel("Resposta administrativa fora do limite.")
    return await leitor.readexactly(tamanho)


async def enviar_alvo(alvo, chave, realm_id, administrador_id, consulta, operacao_id=None):
    escritor = None
    enviado = False
    try:
        leitor, escritor = await asyncio.open_connection(alvo["host"], int(alvo["porta"]))
        saudacao = json.loads(await ler_quadro(leitor))
        if saudacao["protocolo"] != 1:
            raise CanalIndisponivel("Protocolo administrativo não suportado.")
        desafio = bytes.fromhex(saudacao["desafio"])
        if len(desafio) != 32:
            raise CanalIndisponivel("Desafio administrativo inválido.")
        operacao_id = operacao_id or uuid.uuid4().hex
        pedido = json.dumps({"operacao_id": operacao_id, "realm_id": realm_id,
                             "administrador_id": administrador_id, "consulta": consulta},
                            separators=(",", ":"), ensure_ascii=False).encode()
        assinatura = hmac.digest(chave, desafio + b"pedido" + pedido, "sha256")
        if len(pedido) > 8192:
            raise CanalIndisponivel("Pedido administrativo fora do limite.")
        enviado = True
        escritor.write(struct.pack("!I", len(pedido)) + pedido + assinatura)
        await escritor.drain()
        corpo = await ler_quadro(leitor)
        recebida = await leitor.readexactly(32)
        esperada = hmac.digest(chave, desafio + b"resposta" + corpo, "sha256")
        if not hmac.compare_digest(recebida, esperada):
            raise CanalIndisponivel("Assinatura do daemon recusada.")
        resposta = json.loads(corpo)
        if resposta["operacao_id"] != operacao_id or resposta["realm_id"] != realm_id:
            raise CanalIndisponivel("Resposta administrativa de outro alvo ou operação.")
        return resposta
    except (OSError, asyncio.IncompleteReadError, KeyError, TypeError, ValueError):
        if not enviado:
            raise CanalNaoEnviado("Daemon administrativo indisponível; nada foi enviado.")
        raise CanalIndisponivel("Daemon administrativo indisponível ou resposta inválida.")
    finally:
        if escritor is not None:
            escritor.close()
            try:
                await asyncio.wait_for(escritor.wait_closed(), timeout=0.5)
            except (OSError, asyncio.TimeoutError):
                pass


async def consultar_alvo(alvo, chave, realm_id, administrador_id, consulta):
    resposta = await enviar_alvo(alvo, chave, realm_id, administrador_id, consulta)
    if resposta["estado"] != "consultado":
        raise CanalIndisponivel("Consulta recusada pelo daemon.")
    return resposta["dados"]


async def comandar(realm_id, administrador_id, consulta, operacao_id=None):
    alvos, chave = configuracao(realm_id)
    # Contas/registro são globais no PostgreSQL. Um GS autenticado é suficiente;
    # nova tentativa usa o mesmo ID, inclusive se for encaminhada por outro realm.
    try:
        resposta = await asyncio.wait_for(enviar_alvo(
            alvos[0], chave, realm_id, administrador_id, consulta, operacao_id), timeout=3)
        if resposta["estado"] not in ("salvo", "falha", "desconhecido", "pendente", "aplicado", "substituido"):
            raise CanalIndisponivel("Estado administrativo inválido.")
        dados = resposta.get("dados")
        if not isinstance(dados, dict):
            raise CanalIndisponivel("Resultado administrativo inválido.")
        # Edição de personagem (E5): escopo do realm, sem conta; confere o alvo.
        if consulta["tipo"] == "editar_personagem" or dados.get("tipo") == "editar_personagem":
            if (consulta["tipo"] == "editar_personagem" and "personagem_id" in dados
                    and dados["personagem_id"] != consulta["personagem_id"]):
                raise CanalIndisponivel("Resultado administrativo de outro personagem.")
            return resposta
        if resposta["estado"] in ("salvo", "pendente", "aplicado", "substituido"):
            tipo = dados.get("tipo")
            conta = dados.get("conta_id")
            if (tipo not in ("trocar_senha", "criar_conta", "definir_gm", "ajustar_gold", "definir_ban")
                    or dados.get("alcance") != "global"
                    or type(conta) is not int or not 1 <= conta <= 2_147_483_647
                    or (consulta["tipo"] != "resultado" and tipo != consulta["tipo"])
                    or (consulta["tipo"] in ("trocar_senha", "definir_gm", "ajustar_gold", "definir_ban")
                        and conta != consulta["conta_id"])
                    or (tipo == "criar_conta" and (not isinstance(dados.get("usuario"), str)
                        or (consulta["tipo"] == "criar_conta"
                            and dados["usuario"] != consulta["usuario"].lower())))):
                raise CanalIndisponivel("Resultado administrativo de outro alvo.")
            if resposta["estado"] in ("pendente", "aplicado", "substituido") and (
                    tipo != "definir_gm" or type(dados.get("habilitado")) is not bool
                    or type(dados.get("revisao")) is not int or dados["revisao"] <= 0
                    or dados.get("persistencia") != "salva"
                    or (consulta["tipo"] == "definir_gm" and dados["habilitado"] != consulta["habilitado"])):
                raise CanalIndisponivel("Resultado da coordenação GM inválido.")
        return resposta
    except asyncio.TimeoutError:
        raise CanalIndisponivel("Resultado desconhecido; consulte o mesmo identificador.")


async def transmitir(administrador_id, consulta):
    """Mesma ação a todos os daemons de todos os realms com canal (desconectar a conta,
    reenviar o saldo). Ações idempotentes por conta, sem registro durável: repetir é seguro.
    Devolve {realm: [dados por daemon] | "mensagem de erro"}."""
    try:
        realms = list(json.loads(os.getenv("ADMIN_DAEMONS", "{}")))
    except ValueError:
        realms = []

    async def um_realm(realm_id):
        return realm_id, await aplicar_no_realm(realm_id, administrador_id, consulta)

    return dict(await asyncio.gather(*(um_realm(r) for r in realms)))


async def resposta_do_primeiro(realm_id, administrador_id, consulta):
    """A consulta só ao primeiro daemon do realm (B183: carregar um mapa que nenhum serve —
    com vários processos, um só monta). Devolve (estado, dados)."""
    alvos, chave = configuracao(realm_id)
    try:
        r = await asyncio.wait_for(enviar_alvo(alvos[0], chave, realm_id, administrador_id, consulta), timeout=8)
    except asyncio.TimeoutError:
        raise CanalIndisponivel("Tempo esgotado esperando o servidor de mundo.")
    return r["estado"], r["dados"]


async def respostas_do_realm(realm_id, administrador_id, consulta):
    """A mesma consulta a cada daemon do realm; devolve as respostas (estado, dados) cruas."""
    alvos, chave = configuracao(realm_id)
    try:
        respostas = await asyncio.wait_for(asyncio.gather(*(
            enviar_alvo(alvo, chave, realm_id, administrador_id, consulta) for alvo in alvos
        )), timeout=8)
    except asyncio.TimeoutError:
        raise CanalIndisponivel("Tempo esgotado esperando o servidor de mundo.")
    return [(r["estado"], r["dados"]) for r in respostas]


async def aplicar_no_realm(realm_id, administrador_id, consulta):
    """Ação a todos os daemons de UM realm. Devolve [dados por daemon] ou a mensagem de erro."""
    try:
        alvos, chave = configuracao(realm_id)
        respostas = await asyncio.wait_for(asyncio.gather(*(
            enviar_alvo(alvo, chave, realm_id, administrador_id, consulta) for alvo in alvos
        )), timeout=5)
        if any(r["estado"] != "aplicado" for r in respostas):
            codigos = {r["dados"].get("codigo") for r in respostas if r["estado"] != "aplicado"}
            return "recusado pelo daemon: " + ", ".join(sorted(str(c) for c in codigos))
        return [r["dados"] for r in respostas]
    except (CanalIndisponivel, asyncio.TimeoutError) as erro:
        return str(erro) or "tempo esgotado"


async def consultar_daemons(realm_id, administrador_id, consulta):
    alvos, chave = configuracao(realm_id)
    try:
        # Se qualquer processo falta, não afirmar ausência no realm inteiro.
        return await asyncio.wait_for(asyncio.gather(*(
            consultar_alvo(alvo, chave, realm_id, administrador_id, consulta) for alvo in alvos
        )), timeout=3)
    except asyncio.TimeoutError:
        raise CanalIndisponivel("Tempo de consulta ao daemon esgotado; presença desconhecida.")
