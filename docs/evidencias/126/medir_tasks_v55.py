"""Sondas reproduzíveis do tasks.data e do elementclient 1.2.6.

Não interpreta campos. Apenas enumera a tabela do pack e localiza referências x86
às strings do carregador no PE, para que a desassemblagem fique ancorada no binário.
"""
from __future__ import annotations

import argparse
import hashlib
import struct
from pathlib import Path

import capstone
import pefile


def va_para_offset(pe: pefile.PE, va: int) -> int:
    return pe.get_offset_from_rva(va - pe.OPTIONAL_HEADER.ImageBase)


def tasks(path: Path) -> None:
    data = path.read_bytes()
    magic, version, roots = struct.unpack_from("<III", data)
    offsets = struct.unpack_from(f"<{roots}I", data, 12)
    print(f"arquivo={path}")
    print(f"bytes={len(data)} sha256={hashlib.sha256(data).hexdigest()}")
    print(f"magic={magic} version={version} roots={roots}")
    print(f"tabela_inicio=12 tabela_fim={12 + 4 * roots}")
    print(f"primeiros_offsets={list(offsets[:8])}")
    print(f"ultimos_offsets={list(offsets[-8:])}")
    print(f"primeiros_ids={[struct.unpack_from('<I', data, o)[0] for o in offsets[:8]]}")
    print(f"ultimos_ids={[struct.unpack_from('<I', data, o)[0] for o in offsets[-8:]]}")


def xrefs(path: Path, needle: bytes) -> None:
    raw = path.read_bytes()
    pe = pefile.PE(data=raw, fast_load=True)
    image_base = pe.OPTIONAL_HEADER.ImageBase
    locations = []
    start = 0
    while True:
        at = raw.find(needle, start)
        if at < 0:
            break
        va = image_base + pe.get_rva_from_offset(at)
        locations.append((at, va))
        start = at + 1
    print(f"pe={path} sha256={hashlib.sha256(raw).hexdigest()} image_base=0x{image_base:x}")
    print(f"string={needle!r} locations={[f'file=0x{o:x}/va=0x{v:x}' for o, v in locations]}")
    text = next(s for s in pe.sections if s.Name.rstrip(b"\0") == b".text")
    text_off = text.PointerToRawData
    text_end = text_off + text.SizeOfRawData
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    for _, va in locations:
        literal = struct.pack("<I", va)
        refs = []
        pos = text_off
        while True:
            pos = raw.find(literal, pos, text_end)
            if pos < 0:
                break
            refs.append(pos)
            pos += 1
        for ref in refs:
            begin = max(text_off, ref - 32)
            code = raw[begin:min(text_end, ref + 32)]
            rva = pe.get_rva_from_offset(begin)
            print(f"xref file=0x{ref:x} va=0x{image_base + pe.get_rva_from_offset(ref):x}")
            for ins in md.disasm(code, image_base + rva):
                print(f"  0x{ins.address:x}: {ins.mnemonic} {ins.op_str}")


def disassemble(path: Path, va: int, size: int) -> None:
    raw = path.read_bytes()
    pe = pefile.PE(data=raw, fast_load=True)
    offset = va_para_offset(pe, va)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    print(f"disassembly va=0x{va:x} bytes={size}")
    for ins in md.disasm(raw[offset:offset + size], va):
        print(f"0x{ins.address:x}: {ins.mnemonic} {ins.op_str}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("tasks", type=Path)
    parser.add_argument("client", type=Path)
    parser.add_argument("--string", default="LoadPack begin")
    parser.add_argument("--va", type=lambda value: int(value, 0))
    parser.add_argument("--bytes", type=int, default=1024)
    args = parser.parse_args()
    tasks(args.tasks)
    xrefs(args.client, args.string.encode("ascii"))
    if args.va is not None:
        disassemble(args.client, args.va, args.bytes)


if __name__ == "__main__":
    main()
