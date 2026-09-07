#!/usr/bin/env python3
"""Extrator de pacotes `.pck` (+`.pkx`) do Angelica Engine (client Perfect World).

# Por que este script existe

Achado em 2026-09-03, investigando um crash do client 1.5.5 na primeira renderização
completa (logo depois de "Entrando em Perfect World" sumir): o client falhava con-
sistentemente em abrir `Models\\Weapons\\...\\木剑\\木剑.ecm` ("Wooden Sword" — usado
como referência base de qualquer personagem, não só quem tem essa arma equipada), e
o servidor estava mandando dados 100% corretos (`item_id` batendo com o registro
certo em `elements.data`, model path idêntico ao que falhava no log).

A causa real está no PRÓPRIO CLIENT, em `AngelicaFile/Source/AFilePackGame.cpp`,
função `AFilePackGame::InnerOpen` (por volta da linha 155-161):

    int nOffset;                                   // <- int de 32 bits, COM SINAL
    ...
    if( m_bHasSafeHeader )
        nOffset = (int)m_safeHeader.offset;         // <- cast de DWORD pra int assinado

`m_safeHeader.offset` guarda o tamanho total do pacote lógico (`.pck` + `.pkx`
somados, quando o pacote passa do limite de ~2GB por arquivo — `MAX_FILE_PACKAGE =
0x7fffff00`, ver `AFilePackBase.h`). Pra qualquer pacote cujo tamanho total passe de
`INT32_MAX` (2.147.483.647 — e é exatamente esse o caso de `models.pck`+`models.pkx`
e `litmodels.pck`+`litmodels.pkx` do client 1.5.5, ~3,2GB e ~3,0GB respectivamente),
esse cast estoura pra um número NEGATIVO. Os `seek()` que vêm depois
(`nOffset - sizeof(DWORD)`, etc.) recebem uma posição negativa, o
`CPackageFile::seek()` clampa pra 0, o client lê lixo tentando achar a versão do
pacote (`0x00020003`) onde ela não está, e `InnerOpen` devolve `false` — **o pacote
inteiro nunca abre**. Isso explica por que arquivos "básicos" como a espada de
madeira falhavam: não é o arquivo que está faltando, é o pacote inteiro que nunca
foi aberto com sucesso.

Esse bug está no binário do client (fechado, não temos como recompilar igual ao que
foi distribuído) — não dá pra corrigir mandando dados diferentes do servidor. A
saída viável é ler os pacotes por fora (este script, sem o bug de 32 bits, já que
Python não tem overflow de inteiro) e colocar os arquivos que faltam como arquivos
soltos no diretório do client — o `AFile` do client sempre procura em disco antes
do pacote (confirmado nesta mesma sessão, com `Configs\\element_client.cfg`).

# Formato (Angelica File Package, versão 0x00020003, "safe header")

Cada `.pck` pode ter um `.pkx` companheiro (mesmo nome, extensão trocada) quando o
pacote lógico passa de `MAX_FILE_PACKAGE = 0x7fffff00` (2.147.483.392) bytes. Nesse
caso o client trata os dois arquivos como um único stream lógico: os primeiros
`MAX_FILE_PACKAGE` bytes vêm do `.pck`, o resto vem do `.pkx` — é exatamente isso
que a classe `LogicalPackageFile` abaixo replica.

Layout (todos os inteiros little-endian):

  offset 0, 12 bytes  — SAFEFILEHEADER: tag1(u32)=0x4DCA23EF, offset(u32), tag2(u32)=0x56A089B7
                        `offset` = tamanho total do pacote lógico (pck+pkx).
  em `offset`-4        — DWORD dwVersion "externo": 0x00020002 ou 0x00020003
                        (medido nos dois; o `AFilePackGame::InnerOpen` do source
                        só aceita 0x00020003 — mais um caso de build drift)
  em `offset`-8        — int32 iNumFile
  em `offset`-280      — FILEHEADER (272 bytes, achado por inspeção binária — o
                        source tem 280 ou 276 dependendo de qual dos dois structs
                        `FILEHEADER` do engine, nenhum bate exatamente): guardByte0(u32),
                        dwVersion(u32), dwEntryOffset(u32, XOR com AFPCK_MASKDWORD),
                        dwFlags(u32), szDescription(252 bytes), guardByte1(u32).
                        guardByte0 tem que ser 0xFDFDFEEE, guardByte1 0xF00DBEEF
                        (depois do XOR do dwEntryOffset)
  em dwEntryOffset     — `iNumFile` entradas, cada uma:
                          int32 nCompressedSize (XOR AFPCK_MASKDWORD)
                          int32 nCheckSize (XOR AFPCK_CHECKMASK XOR AFPCK_MASKDWORD,
                                            tem que bater com nCompressedSize)
                          nCompressedSize bytes — um FILEENTRY_INFILE (276 bytes) cru
                          se nCompressedSize == 276, senão comprimido em zlib.

  FILEENTRY_INFILE (276 bytes, também recalibrado por inspeção — o source declara
  288 com um `dwOffset` de 64 bits e dois campos "unknown" que a build real não
  tem): szFileName[260] (C string, GBK/CP936), dwOffset(u32 — posição do conteúdo
  do arquivo no pacote lógico), dwLength(u32, tamanho descomprimido),
  dwCompressedLength(u32), iAccessCnt(u32).

  Conteúdo de um arquivo: em dwOffset, dwCompressedLength bytes (ou dwLength se
  dwLength <= dwCompressedLength, ou seja sem compressão); zlib se comprimido;
  "Decrypt" (rotação de 16 bits + XOR) só roda se PACKFLAG_ENCRYPT (bit 0x80000000
  de dwFlags) estiver ligado — não é o caso normal de `models.pck`/`litmodels.pck`.

As constantes de máscara (`AFPCK_MASKDWORD` etc.) vêm de
`AngelicaFile/Source/AFilePackMan.cpp` no source do client 1.5.5 e bateram
direto, sem precisar recalibrar. O que precisou de inspeção binária pra
recalibrar foi só o layout exato dos dois structs (`FILEHEADER` e
`FILEENTRY_INFILE`) — ver a seção "Detalhe de build drift" no `README.md` deste
diretório.

# Uso

    python pw_pck_extract.py list   <arquivo.pck>
    python pw_pck_extract.py find   <arquivo.pck> <pedaço-do-nome>
    python pw_pck_extract.py get    <arquivo.pck> <caminho-exato-dentro-do-pacote> <arquivo-de-saida>
    python pw_pck_extract.py get-all <arquivo.pck> <pasta-de-saida>   # cuidado: pode ser muita coisa

`<arquivo.pck>` pode ser passado com ou sem o `.pkx` ao lado — o script procura
sozinho (mesmo nome, extensão `.pkx`) e trata os dois como um pacote só quando o
`.pkx` existir. Funciona igual se o pacote for menor que 2GB e não tiver `.pkx`.
"""

from __future__ import annotations

import argparse
import os
import struct
import sys
import zlib
from dataclasses import dataclass
from typing import BinaryIO, Optional

MAX_FILE_PACKAGE = 0x7FFFFF00  # AFilePackBase.h: limite de bytes por arquivo físico

SAFE_TAG1 = 0x4DCA23EF
SAFE_TAG2 = 0x56A089B7
# Achado por inspeção binária (2026-09-03, `configs.pck` do client 1.5.5 real): o
# `dwVersion` de 4 bytes logo antes do offset do safe header é 0x00020002, não o
# 0x00020003 que o `AFilePackGame::InnerOpen` do source (mais antigo que o binário
# que temos — o mesmo "desvio de build" já visto várias vezes nesta sessão) espera.
# Aceita os dois: o formato do header é idêntico nos dois, só o número mudou.
PACK_VERSIONS_OK = {0x00020002, 0x00020003}

AFPCK_GUARDBYTE0 = 0xFDFDFEEE
AFPCK_GUARDBYTE1 = 0xF00DBEEF
AFPCK_MASKDWORD = 0xA8937462
AFPCK_CHECKMASK = 0x59374231

PACKFLAG_ENCRYPT = 0x80000000

# FILEHEADER real (achado por inspeção binária, não bate exatamente com nenhum dos
# dois structs `FILEHEADER` do source — build drift, de novo): 272 bytes —
# guardByte0(4) + dwVersion(4) + dwEntryOffset(4, mascarado) + dwFlags(4) +
# szDescription(252) + guardByte1(4). Sem o campo "disabled offset" que o source
# tem; a build real deste client não usa ele.
FILEHEADER_SIZE = 272
# FILEENTRY_INFILE real: 276 bytes — szFileName[260] + dwOffset(u32) + dwLength(u32)
# + dwCompressedLength(u32) + iAccessCnt(u32). Sem os campos "unknown"/"unknown2" e
# sem o dwOffset de 64 bits que o source de AFilePackGame.cpp tem — o mesmo desvio
# de build do FILEHEADER acima. Por ser u32, um pacote lógico (pck+pkx) não pode
# passar de ~4,29GB (limite do offset por arquivo, não só do pck físico de ~2GB).
FILEENTRY_SIZE = 276


def _u32(v: int) -> int:
    return v & 0xFFFFFFFF


class LogicalPackageFile:
    """Une `<nome>.pck` e `<nome>.pkx` (se existir) num único stream lógico,
    igual ao `AFilePackBase::CPackageFile` do client — mas com posições em Python
    (sem limite de 32 bits, ao contrário do client)."""

    def __init__(self, pck_path: str):
        base, ext = os.path.splitext(pck_path)
        self.pck_path = pck_path
        self.pkx_path = base + ".pkx"
        self._f1: BinaryIO = open(pck_path, "rb")
        self._size1 = os.fstat(self._f1.fileno()).st_size
        self._f2: Optional[BinaryIO] = None
        self._size2 = 0
        if os.path.exists(self.pkx_path):
            self._f2 = open(self.pkx_path, "rb")
            self._size2 = os.fstat(self._f2.fileno()).st_size

    @property
    def total_size(self) -> int:
        return self._size1 + self._size2

    def close(self) -> None:
        self._f1.close()
        if self._f2:
            self._f2.close()

    def __enter__(self) -> "LogicalPackageFile":
        return self

    def __exit__(self, *exc) -> None:
        self.close()

    def read_at(self, pos: int, length: int) -> bytes:
        """Lê `length` bytes a partir da posição lógica `pos`, atravessando o
        limite pck/pkx se precisar — igual ao `CPackageFile::read` do client."""
        if pos < 0:
            raise ValueError(f"posição negativa ({pos}) — isso é o próprio bug que este script existe pra contornar")
        end = pos + length
        if end <= MAX_FILE_PACKAGE:
            self._f1.seek(pos)
            data = self._f1.read(length)
        elif pos < MAX_FILE_PACKAGE:
            self._f1.seek(pos)
            part1 = self._f1.read(MAX_FILE_PACKAGE - pos)
            if not self._f2:
                raise IOError(f"pacote passa de {MAX_FILE_PACKAGE} bytes mas não achei {self.pkx_path}")
            self._f2.seek(0)
            part2 = self._f2.read(length - len(part1))
            data = part1 + part2
        else:
            if not self._f2:
                raise IOError(f"pacote passa de {MAX_FILE_PACKAGE} bytes mas não achei {self.pkx_path}")
            self._f2.seek(pos - MAX_FILE_PACKAGE)
            data = self._f2.read(length)
        if len(data) != length:
            raise IOError(f"esperava {length} bytes em {pos}, li {len(data)} (arquivo truncado?)")
        return data


@dataclass
class FileEntry:
    name: str
    offset: int
    length: int
    compressed_length: int


@dataclass
class PackageIndex:
    entries: list[FileEntry]
    encrypted: bool
    description: str


def _decrypt(buf: bytearray, encrypted: bool) -> None:
    """Espelha `AFilePackGame::Decrypt` — só faz algo se `encrypted` (bit
    PACKFLAG_ENCRYPT do header). `models.pck`/`litmodels.pck` normais não usam isso."""
    if not encrypted:
        return
    n = len(buf)
    mask = _u32(n + 0x739802AB)
    i = 0
    while i + 3 < n:
        data = (buf[i] << 24) | (buf[i + 1] << 16) | (buf[i + 2] << 8) | buf[i + 3]
        data = _u32((data << 16) | (data >> 16))
        data ^= mask
        buf[i] = (data >> 24) & 0xFF
        buf[i + 1] = (data >> 16) & 0xFF
        buf[i + 2] = (data >> 8) & 0xFF
        buf[i + 3] = data & 0xFF
        i += 4


def read_index(pkg: LogicalPackageFile) -> PackageIndex:
    safe_header = pkg.read_at(0, 12)
    tag1, offset, tag2 = struct.unpack("<III", safe_header)
    if tag1 != SAFE_TAG1 or tag2 != SAFE_TAG2:
        raise ValueError(
            f"{pkg.pck_path}: não achei o SAFEFILEHEADER esperado "
            f"(tag1=0x{tag1:08X}, tag2=0x{tag2:08X}) — não é um .pck com 'safe header', "
            "ou o formato é de outra versão do engine"
        )
    n_offset = offset  # o valor que o client corrompe fazendo `(int)` disto — aqui fica inteiro

    (version,) = struct.unpack("<I", pkg.read_at(n_offset - 4, 4))
    if version not in PACK_VERSIONS_OK:
        raise ValueError(f"{pkg.pck_path}: versão do pacote 0x{version:08X}, esperava uma de {[hex(v) for v in PACK_VERSIONS_OK]}")

    (num_files,) = struct.unpack("<i", pkg.read_at(n_offset - 8, 4))

    header_raw = pkg.read_at(n_offset - (FILEHEADER_SIZE + 8), FILEHEADER_SIZE)
    guard0, hdr_version, entry_offset, flags = struct.unpack_from("<IIII", header_raw, 0)
    description = header_raw[16 : 16 + 252].split(b"\x00", 1)[0].decode("latin1", errors="replace")
    (guard1,) = struct.unpack_from("<I", header_raw, 16 + 252)

    entry_offset ^= AFPCK_MASKDWORD
    entry_offset = _u32(entry_offset)

    if guard0 != AFPCK_GUARDBYTE0 or guard1 != AFPCK_GUARDBYTE1:
        raise ValueError(
            f"{pkg.pck_path}: guard bytes não batem (0x{guard0:08X}/0x{guard1:08X}) — "
            "índice corrompido ou constantes de mascaramento mudaram nesta build"
        )

    encrypted = bool(flags & PACKFLAG_ENCRYPT)

    entries: list[FileEntry] = []
    pos = entry_offset
    for _ in range(num_files):
        n_compressed, n_check = struct.unpack("<ii", pkg.read_at(pos, 8))
        pos += 8
        n_compressed = _u32(n_compressed ^ AFPCK_MASKDWORD)
        n_check = _u32(n_check ^ AFPCK_CHECKMASK ^ AFPCK_MASKDWORD)
        if n_compressed != n_check:
            raise ValueError(f"{pkg.pck_path}: check byte não bate lendo a entrada em {pos} — índice corrompido")

        raw = bytearray(pkg.read_at(pos, n_compressed))
        pos += n_compressed

        if n_compressed == FILEENTRY_SIZE:
            entry_bytes = bytes(raw)
        else:
            entry_bytes = zlib.decompress(bytes(raw))
            if len(entry_bytes) != FILEENTRY_SIZE:
                raise ValueError(f"entrada descomprimida tem {len(entry_bytes)} bytes, esperava {FILEENTRY_SIZE}")

        name_raw, off64, length, clength, _acc = struct.unpack(
            "<260sIIII", entry_bytes
        )
        name = name_raw.split(b"\x00", 1)[0].decode("cp936", errors="replace").replace("/", "\\")
        entries.append(FileEntry(name=name, offset=off64, length=length, compressed_length=clength))

    return PackageIndex(entries=entries, encrypted=encrypted, description=description)


def read_file_data(pkg: LogicalPackageFile, entry: FileEntry, encrypted: bool) -> bytes:
    if entry.length > entry.compressed_length:
        raw = bytearray(pkg.read_at(entry.offset, entry.compressed_length))
        _decrypt(raw, encrypted)
        data = zlib.decompress(bytes(raw))
        if len(data) != entry.length:
            raise ValueError(f"{entry.name}: descomprimiu {len(data)} bytes, esperava {entry.length}")
        return data
    else:
        raw = bytearray(pkg.read_at(entry.offset, entry.length))
        _decrypt(raw, encrypted)
        return bytes(raw)


def cmd_list(pck_path: str) -> None:
    with LogicalPackageFile(pck_path) as pkg:
        idx = read_index(pkg)
        print(f"{pkg.pck_path} (+{pkg.pkx_path if pkg._f2 else '(sem .pkx)'}) — {pkg.total_size} bytes lógicos")
        print(f"descrição: {idx.description!r}  criptografado: {idx.encrypted}  {len(idx.entries)} arquivos")
        for e in idx.entries:
            print(f"  {e.length:>12}  {e.name}")


def cmd_find(pck_path: str, needle: str) -> None:
    with LogicalPackageFile(pck_path) as pkg:
        idx = read_index(pkg)
        needle_low = needle.lower()
        found = [e for e in idx.entries if needle_low in e.name.lower()]
        if not found:
            print(f"nada batendo com {needle!r} em {len(idx.entries)} entradas")
            return
        for e in found:
            print(f"  {e.length:>12}  offset={e.offset:<14}  {e.name}")


def cmd_get(pck_path: str, exact_name: str, out_path: str) -> None:
    with LogicalPackageFile(pck_path) as pkg:
        idx = read_index(pkg)
        target = exact_name.lower().replace("/", "\\")
        matches = [e for e in idx.entries if e.name.lower() == target]
        if not matches:
            close = [e for e in idx.entries if target in e.name.lower()]
            hint = f" (mas achei {len(close)} parecidos — tente `find` primeiro)" if close else ""
            raise SystemExit(f"não achei {exact_name!r} no pacote{hint}")
        entry = matches[0]
        data = read_file_data(pkg, entry, idx.encrypted)
        os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)
        with open(out_path, "wb") as f:
            f.write(data)
        print(f"gravei {len(data)} bytes em {out_path}")


def cmd_get_matching(pck_path: str, needle: str, out_dir: str) -> None:
    """Extrai todas as entradas cujo nome contém `needle`, preservando a estrutura
    de pastas do pacote dentro de `out_dir` — útil pra pegar um modelo inteiro (ecm
    + bon + ski + smd + texturas) de uma vez, não só um arquivo."""
    with LogicalPackageFile(pck_path) as pkg:
        idx = read_index(pkg)
        needle_low = needle.lower()
        matches = [e for e in idx.entries if needle_low in e.name.lower()]
        if not matches:
            print(f"nada batendo com {needle!r} em {len(idx.entries)} entradas")
            return
        failed: list[tuple[FileEntry, str]] = []
        for e in matches:
            try:
                data = read_file_data(pkg, e, idx.encrypted)
            except Exception as ex:
                failed.append((e, str(ex)))
                print(f"  FALHOU  {e.name}: {ex}", file=sys.stderr)
                continue
            dest = os.path.join(out_dir, e.name.replace("\\", os.sep))
            os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
            with open(dest, "wb") as f:
                f.write(data)
            print(f"  {len(data):>10}  {e.name}  ->  {dest}")
        print(f"extraí {len(matches) - len(failed)}/{len(matches)} arquivo(s) batendo com {needle!r} em {out_dir}")
        if failed:
            print(f"  {len(failed)} falharam (entrada corrompida no pacote de origem, não no extrator — ver mensagens acima)")


def cmd_get_all(pck_path: str, out_dir: str) -> None:
    """Extrai tudo. Uma entrada individual corrompida no pacote de origem (raro,
    mas acontece — zlib com checksum errado) não aborta a extração inteira: fica
    registrada e pulada, o resto continua."""
    with LogicalPackageFile(pck_path) as pkg:
        idx = read_index(pkg)
        total = len(idx.entries)
        failed: list[tuple[FileEntry, str]] = []
        for i, e in enumerate(idx.entries, 1):
            try:
                data = read_file_data(pkg, e, idx.encrypted)
            except Exception as ex:
                failed.append((e, str(ex)))
                print(f"FALHOU ({i}/{total}) {e.name}: {ex}", file=sys.stderr)
                continue
            dest = os.path.join(out_dir, e.name.replace("\\", os.sep))
            os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
            with open(dest, "wb") as f:
                f.write(data)
            if i % 200 == 0 or i == total:
                print(f"  {i}/{total}", file=sys.stderr)
    print(f"extraí {total - len(failed)}/{total} arquivos em {out_dir}")
    if failed:
        print(f"{len(failed)} entrada(s) falharam (corrompidas no pacote de origem):")
        for e, err in failed:
            print(f"  {e.name}: {err}")


def main() -> None:
    # Muitos nomes dentro dos pacotes são em chinês (GBK/CP936); o console do
    # Windows normalmente não tem essa code page ativa. `errors="replace"` evita
    # crash ao imprimir — o pior caso é um nome ilegível na tela, nunca um erro.
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_list = sub.add_parser("list", help="lista tudo que tem no pacote")
    p_list.add_argument("pck")

    p_find = sub.add_parser("find", help="procura entradas cujo nome contém um pedaço de texto")
    p_find.add_argument("pck")
    p_find.add_argument("needle")

    p_get = sub.add_parser("get", help="extrai um arquivo específico (caminho exato dentro do pacote)")
    p_get.add_argument("pck")
    p_get.add_argument("name")
    p_get.add_argument("out")

    p_match = sub.add_parser("get-matching", help="extrai todas as entradas cujo nome contém um pedaço de texto, preservando a estrutura de pastas")
    p_match.add_argument("pck")
    p_match.add_argument("needle")
    p_match.add_argument("out_dir")

    p_all = sub.add_parser("get-all", help="extrai tudo (cuidado: pode ser muito grande)")
    p_all.add_argument("pck")
    p_all.add_argument("out_dir")

    args = parser.parse_args()
    if args.cmd == "list":
        cmd_list(args.pck)
    elif args.cmd == "find":
        cmd_find(args.pck, args.needle)
    elif args.cmd == "get":
        cmd_get(args.pck, args.name, args.out)
    elif args.cmd == "get-matching":
        cmd_get_matching(args.pck, args.needle, args.out_dir)
    elif args.cmd == "get-all":
        cmd_get_all(args.pck, args.out_dir)


if __name__ == "__main__":
    main()
