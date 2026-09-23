"""Validação estrutural do ``tasks.data`` v55 (cliente 1.2.6).

O cursor replica somente os ``fread`` do elementclient.exe 1.2.6, sem converter
campos para o modelo do servidor.  A propriedade verificada é forte: cada raiz
da tabela termina precisamente no deslocamento da raiz seguinte (e a última,
no último byte do arquivo).

Prova das contagens e tamanhos: ``tasks-v55-fixed-core.asm.txt`` e
``tasks-v55-loadbinary-full.asm.txt``, gerados do elementclient.exe de SHA256
5fc88d47e01da3caea7d6ce4911b71b0f7085060a2d13889a380fc5ba7f0ed14.
"""
from __future__ import annotations

import argparse
import hashlib
import struct
from dataclasses import dataclass
from pathlib import Path


FIXO = 534
PREM_ITEM = 13
GIVEN_ITEM = 13
TEAM_MEM = 32
MONSTER = 22
ITEM_WANTED = 13
AWARD = 75
TALK_TEXT = 128
TALK_OPTION = 136


class FormatoInvalido(ValueError):
    pass


@dataclass
class Cursor:
    data: bytes
    pos: int
    limite: int
    tarefas: int = 0
    max_profundidade: int = 0

    def exigir(self, n: int) -> None:
        if n < 0 or self.pos + n > self.limite:
            raise FormatoInvalido(f"truncado em {self.pos}: precisa {n}, limite {self.limite}")

    def bytes(self, n: int) -> bytes:
        self.exigir(n)
        inicio = self.pos
        self.pos += n
        return self.data[inicio:self.pos]

    def u32(self) -> int:
        return struct.unpack("<I", self.bytes(4))[0]

    def i32(self) -> int:
        return struct.unpack("<i", self.bytes(4))[0]

    def quantidade(self, descricao: str) -> int:
        n = self.i32()
        if n < 0 or n > 1_000_000:
            raise FormatoInvalido(f"{descricao}: contagem inválida {n} em {self.pos - 4}")
        return n


def premio(c: Cursor) -> None:
    bruto = c.bytes(AWARD)
    candidatos = struct.unpack_from("<I", bruto, 67)[0]
    if candidatos > 1_000_000:
        raise FormatoInvalido(f"prêmio: {candidatos} candidatos em {c.pos - AWARD + 67}")
    # Cada AWARD_ITEMS_CAND é gravado por 0x62d9d0: m_bRandChoose (1 B),
    # m_ulAwardItems (u32) e tantos ITEM_WANTED de 13 B.  Os campos de
    # contagem/pointer que existem no objeto em memória não são serializados.
    for _ in range(candidatos):
        c.bytes(1)
        itens = c.quantidade("itens do candidato de prêmio")
        c.bytes(itens * ITEM_WANTED)


def escala_de_razao(c: Cursor) -> None:
    n = c.quantidade("escala de razão")
    c.bytes(5 * 4)
    for _ in range(n):
        premio(c)


def escala_de_itens(c: Cursor) -> None:
    n = c.quantidade("escala de itens")
    c.bytes(4 + 5 * 4)  # id do item e cinco quantidades-limite
    for _ in range(n):
        premio(c)


def texto_longo(c: Cursor) -> None:
    caracteres = c.quantidade("texto UTF-16")
    c.bytes(caracteres * 2)


def dialogo(c: Cursor) -> None:
    # id_talk (u32), text[64] (128 B) e num_window (i32).
    c.bytes(4 + TALK_TEXT)
    janelas = c.quantidade("janelas de diálogo")
    for _ in range(janelas):
        c.bytes(8)  # id, id_parent
        caracteres = c.quantidade("texto da janela")
        c.bytes(caracteres * 2)
        opcoes = c.quantidade("opções da janela")
        c.bytes(opcoes * TALK_OPTION)


def tarefa(c: Cursor, profundidade: int = 1) -> None:
    c.tarefas += 1
    c.max_profundidade = max(c.max_profundidade, profundidade)
    fixo = c.bytes(FIXO)

    if fixo[0x40]:  # m_bHasSign
        c.bytes(60)  # m_szSignature[30]

    horarios = struct.unpack_from("<I", fixo, 0x4E)[0]
    if horarios > 1_000_000:
        raise FormatoInvalido(f"horários: {horarios} em {c.pos - FIXO + 0x4E}")
    c.bytes(horarios * 2 * 24)  # m_tmStart/m_tmEnd, task_tm = seis i32

    c.bytes(struct.unpack_from("<I", fixo, 0xCA)[0] * PREM_ITEM)
    c.bytes(struct.unpack_from("<I", fixo, 0xD3)[0] * GIVEN_ITEM)
    if fixo[0x176]:
        c.bytes(struct.unpack_from("<I", fixo, 0x191)[0] * TEAM_MEM)
    c.bytes(struct.unpack_from("<I", fixo, 0x1A2)[0] * MONSTER)
    c.bytes(struct.unpack_from("<I", fixo, 0x1AA)[0] * ITEM_WANTED)

    premio(c)
    premio(c)
    escala_de_razao(c)
    escala_de_razao(c)
    escala_de_itens(c)
    escala_de_itens(c)
    # 0x62fc60 lê descrição, texto de sucesso e texto de falha; 0x62fde0,
    # tributo. Todos são ``u32 caracteres`` seguido de UTF-16.
    for _ in range(4):
        texto_longo(c)
    for _ in range(5):
        dialogo(c)

    filhos = c.quantidade("submissões")
    for _ in range(filhos):
        tarefa(c, profundidade + 1)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("arquivo", type=Path)
    args = parser.parse_args()
    data = args.arquivo.read_bytes()
    magic, version, raizes = struct.unpack_from("<III", data)
    if magic != 0x93858361 or version != 55:
        raise FormatoInvalido(f"cabeçalho inesperado: magic=0x{magic:08x}, versão={version}")
    offsets = struct.unpack_from(f"<{raizes}I", data, 12)
    total_tarefas = 0
    max_profundidade = 0
    for indice, inicio in enumerate(offsets):
        fim_esperado = offsets[indice + 1] if indice + 1 < raizes else len(data)
        if not 12 + 4 * raizes <= inicio < fim_esperado <= len(data):
            raise FormatoInvalido(f"raiz {indice}: intervalo inválido {inicio}..{fim_esperado}")
        cursor = Cursor(data, inicio, fim_esperado)
        tarefa(cursor)
        if cursor.pos != fim_esperado:
            raise FormatoInvalido(
                f"raiz {indice}: terminou em {cursor.pos}, esperado {fim_esperado}"
            )
        total_tarefas += cursor.tarefas
        max_profundidade = max(max_profundidade, cursor.max_profundidade)
    print(f"arquivo={args.arquivo}")
    print(f"sha256={hashlib.sha256(data).hexdigest()}")
    print(f"raizes={raizes} tarefas_recursivas={total_tarefas} profundidade_maxima={max_profundidade}")
    print(f"fechamento_exato=sim ultimo_byte={len(data)}")


if __name__ == "__main__":
    main()
