"""
Leitor genérico e reaproveitável de `elements.data`, dirigido pelo catálogo de layouts
desta pasta (`vNNN.json`). É a implementação de referência em Python do algoritmo que
resolveu as 231 tabelas do v156 (ver `specs/elements_155/README.md` para a arqueologia
completa) — o `pw-data-loader` em Rust (`crates/pw-data-loader/src/generic_elements.rs`)
segue exatamente o mesmo algoritmo, que é o do `elementdataman::load_data` do cliente
(`EvolvedPWClient/ElementClient/CCommon/elementdataman.cpp:3879`):

* cada tabela é `count` + `count * record_size`, sem busca nem pontuação;
* depois de `ARMORRUNE_ESSENCE` vem `tag` (0xab7689dd), `len`, `len` bytes e um `time_t`;
* depois de `WAR_TANKCALLIN_ESSENCE` vem `tag` (0xee35679f), `len` e `len` bytes;
* `TALK_PROC` tem laço próprio;
* o arquivo tem de terminar exatamente no último byte.

Até 2026-09-12 os dois blocos de `tag` não eram lidos, e uma busca por "registro
plausível" mais dez overrides por arquivo compensavam — o que só funcionava no arquivo
em que os overrides tinham sido medidos. Os `*_overrides.json` de `specs/elements_155/`
ficaram como registro histórico; este leitor não os usa mais.

Uso típico:

    from pw_elements_reader import load_elements_data

    tables = load_elements_data("data/realm_155/config/elements.data")
    for equip_addon in tables["EQUIPMENT_ADDON"]:
        print(equip_addon["ID"], equip_addon["Name"])

Não sabe nada sobre Docker, caminhos de realm, nem cache — isso é responsabilidade de quem
chama (ver `web-admin/backend/elements_decoder.py` para um exemplo de integração).
"""
from __future__ import annotations

import json
import os
import struct
from typing import Any, Dict, List, Optional, Tuple

LAYOUTS_DIR = os.path.dirname(os.path.abspath(__file__))


class UnsupportedVersionError(Exception):
    """Levantado quando o cabeçalho do arquivo aponta pra uma versão sem layout no catálogo."""


class ElementsFormatError(Exception):
    """Levantado quando o arquivo não bate com nenhuma família de cabeçalho conhecida."""


# =============================================================================
# Detecção de versão pelo cabeçalho -- ver specs/elements_layouts/README.md
# =============================================================================

def detect_header(buf: bytes) -> Dict[str, Any]:
    """Le o cabeçalho do arquivo e devolve `{family, version, header_size, build_timestamp?}`.

    **Achado corrigido nesta sessão**: o campo de versão usa a MESMA codificação
    (`0x3000 << 16 | numero_de_build`, byte baixo = build) em **todas** as versões já
    medidas, incluindo o v7 do 1.2.6 (`data/realm_126/config/elements.data` começa com
    `07 00 00 30` = `0x30000007`) -- não é uma diferença de família só de builds
    recentes como a primeira versão desta função assumia. A diferença real entre
    eras é o **tamanho do cabeçalho**: builds recentes (confirmado no v156) têm mais 4
    bytes de `build_timestamp` (`time_t` da geração do arquivo) depois da versão; builds
    antigas (confirmado no v7) não têm -- os bytes 4-7 já são o `count` da primeira tabela.

    Como não dá pra saber isso só pelo valor da versão, o desempate usa magnitude: um
    `time_t` de verdade (datas de ~2001 em diante) sempre passa de 900 milhões; um `count`
    de tabela, pelo que já vimos em 231 tabelas medidas, nunca chega nem perto disso. Se os
    bytes 4-7 parecerem um timestamp plausível, cabeçalho de 8 bytes; senão, 4.
    """
    if len(buf) < 8:
        raise ElementsFormatError(f"arquivo pequeno demais pra ter cabeçalho ({len(buf)} bytes)")
    version_u32 = struct.unpack_from("<I", buf, 0)[0]
    if (version_u32 >> 16) != 0x3000:
        raise ElementsFormatError(
            f"cabeçalho não reconhecido (esperava 0x3000<<16 | build, achei {version_u32:#x}) -- "
            f"pode ser uma família de formato ainda mais antiga, não implementada"
        )
    next_u32 = struct.unpack_from("<I", buf, 4)[0]
    if next_u32 > 900_000_000:
        return {
            "family": "with_timestamp",
            "version": version_u32 & 0xFFFF,
            "raw_version": version_u32,
            "header_size": 8,
            "build_timestamp": next_u32,
        }
    return {
        "family": "no_timestamp",
        "version": version_u32 & 0xFFFF,
        "raw_version": version_u32,
        "header_size": 4,
    }


# =============================================================================
# Catálogo de layouts
# =============================================================================

def load_layout(version: int, layouts_dir: Optional[str] = None) -> Dict[str, Any]:
    """Carrega `v<version>.json` do catálogo. Levanta `UnsupportedVersionError` se não achar."""
    layouts_dir = layouts_dir or LAYOUTS_DIR
    path = os.path.join(layouts_dir, f"v{version}.json")
    if not os.path.exists(path):
        raise UnsupportedVersionError(
            f"nenhum layout pra versao {version} em {layouts_dir} -- "
            f"gere um v{version}.json (ver README.md desta pasta) antes de carregar este arquivo"
        )
    with open(path, encoding="utf-8") as f:
        return json.load(f)


# =============================================================================
# Decodificação de um registro, dirigida pelos campos do layout
# =============================================================================

def _decode_field(buf: bytes, off: int, field: Dict[str, Any]) -> Tuple[Any, int]:
    ftype = field["type"]
    size = field["size"]
    if ftype == "int32":
        return struct.unpack_from("<i", buf, off)[0], off + 4
    if ftype == "float":
        return struct.unpack_from("<f", buf, off)[0], off + 4
    if ftype == "wstring":
        raw = buf[off:off + size]
        txt = raw.decode("utf-16le", errors="replace")
        nul = txt.find("\x00")
        return (txt[:nul] if nul >= 0 else txt), off + size
    if ftype == "string":
        raw = buf[off:off + size]
        nul = raw.find(b"\x00")
        return (raw[:nul] if nul >= 0 else raw), off + size
    raise ValueError(f"tipo de campo desconhecido no layout: {ftype}")


def decode_record(buf: bytes, off: int, table_def: Dict[str, Any]) -> Dict[str, Any]:
    """Decodifica um registro em um dict `{nome_do_campo: valor}`."""
    record: Dict[str, Any] = {}
    cur = off
    for field in table_def["fields"]:
        val, cur = _decode_field(buf, cur, field)
        record[field["name"]] = val
    return record


# =============================================================================
# TALK_PROC -- a única tabela de tamanho variável nos layouts medidos
# =============================================================================

def _read_wstr(buf: bytes, off: int, nchars: int) -> Tuple[str, int]:
    raw = buf[off:off + nchars * 2]
    txt = raw.decode("utf-16le", errors="replace")
    nul = txt.find("\x00")
    return (txt[:nul] if nul >= 0 else txt), off + nchars * 2


def read_talk_proc_table(buf: bytes, off: int) -> Tuple[List[Dict[str, Any]], int]:
    """Le a árvore de diálogos (`TALK_PROC`). Formato confirmado contra
    `exptypes.h::talk_proc/window/option` do EvolvedPWServer -- ver
    `specs/elements_155/README.md`."""
    count = struct.unpack_from("<I", buf, off)[0]
    cur = off + 4
    talk_procs = []
    for _ in range(count):
        id_talk = struct.unpack_from("<I", buf, cur)[0]
        cur += 4
        text, cur = _read_wstr(buf, cur, 64)
        num_window = struct.unpack_from("<i", buf, cur)[0]
        cur += 4
        windows = []
        for _ in range(num_window):
            wid, id_parent = struct.unpack_from("<Ii", buf, cur)
            cur += 8
            talk_text_len = struct.unpack_from("<i", buf, cur)[0]
            cur += 4
            talk_text, cur = _read_wstr(buf, cur, talk_text_len)
            num_option = struct.unpack_from("<i", buf, cur)[0]
            cur += 4
            options = []
            for _ in range(num_option):
                opt_id = struct.unpack_from("<I", buf, cur)[0]
                cur += 4
                opt_text, cur = _read_wstr(buf, cur, 64)
                opt_param = struct.unpack_from("<I", buf, cur)[0]
                cur += 4
                options.append({"id": opt_id, "text": opt_text, "param": opt_param})
            windows.append({"id": wid, "id_parent": id_parent, "talk_text": talk_text, "options": options})
        talk_procs.append({"id_talk": id_talk, "text": text, "windows": windows})
    return talk_procs, cur


# =============================================================================
# Orquestrador de topo
# =============================================================================

TAG_DO_EXPORTADOR = 0xAB7689DD
TAG_DEPOIS_DOS_TANQUES = 0xEE35679F


def load_elements_data(
    path: str,
    layouts_dir: Optional[str] = None,
    overrides_path: Optional[str] = None,
) -> Dict[str, List[Dict[str, Any]]]:
    """Carrega um `elements.data` inteiro, devolvendo `{nome_da_tabela: [registro, ...]}`.

    `overrides_path` é aceito e ignorado — ver a nota do módulo.
    """
    with open(path, "rb") as f:
        buf = f.read()

    header = detect_header(buf)
    layout = load_layout(header["version"], layouts_dir)

    def u32(o: int) -> int:
        if o + 4 > len(buf):
            raise ElementsFormatError(f"offset {o} passa do fim do arquivo ({len(buf)} bytes)")
        return struct.unpack_from("<I", buf, o)[0]

    result: Dict[str, List[Dict[str, Any]]] = {}
    off = header["header_size"]
    for table_def in layout["tables"]:
        idx = table_def["index"]
        name = table_def["name"]

        if table_def.get("variable_size"):
            if name != "TALK_PROC":
                raise ElementsFormatError(f"tabela '{name}' (índice {idx}) é de tamanho variável mas não tem leitor implementado")
            records, off = read_talk_proc_table(buf, off)
            result[name] = records
            continue

        size = table_def["record_size"]
        count = u32(off)
        inicio = off + 4
        if inicio + count * size > len(buf):
            raise ElementsFormatError(f"tabela '{name}' (índice {idx}): count={count} passa do fim do arquivo")
        result[name] = [decode_record(buf, inicio + i * size, table_def) for i in range(count)]
        off = inicio + count * size

        if header["version"] != 7 and name in ("ARMORRUNE_ESSENCE", "WAR_TANKCALLIN_ESSENCE"):
            esperado = TAG_DO_EXPORTADOR if name == "ARMORRUNE_ESSENCE" else TAG_DEPOIS_DOS_TANQUES
            achado = u32(off)
            if achado != esperado:
                raise ElementsFormatError(f"depois de '{name}' devia vir o tag {esperado:#x} no offset {off}, veio {achado:#x}")
            off += 8 + u32(off + 4)
            if name == "ARMORRUNE_ESSENCE":
                off += 4  # time_t

    if off != len(buf):
        raise ElementsFormatError(
            f"as tabelas terminaram no offset {off}, mas o arquivo tem {len(buf)} bytes -- o layout não é o deste arquivo"
        )
    return result


if __name__ == "__main__":
    import sys

    data_path = sys.argv[1]
    tables = load_elements_data(data_path)
    total_records = sum(len(v) for v in tables.values())
    print(f"{len(tables)} tabelas, {total_records} registros no total")
    for name, records in tables.items():
        if records:
            print(f"  {name}: {len(records)} registros (ex.: {records[0].get('Name', records[0])!r})")
