"""
Leitor genérico e reaproveitável de `elements.data`, dirigido pelo catálogo de layouts
desta pasta (`vNNN.json`). É a implementação de referência em Python do algoritmo que
resolveu as 231 tabelas do v156 (ver `specs/elements_155/README.md` para a arqueologia
completa) — qualquer outro consumidor (o `pw-data-loader` em Rust, por exemplo) deve seguir
o mesmo algoritmo: tentar a posição ingênua primeiro (contando `count==0` como resultado
válido, não uma falha), e só cair para correções manuais quando uma tabela específica
precisar (ver `specs/elements_155/realm_155_overrides.json`).

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
import math
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


def load_overrides(path: str) -> Dict[str, Any]:
    """Carrega um `*_overrides.json` (ex. `specs/elements_155/realm_155_overrides.json`).
    Devolve `{}` se o arquivo não existir -- overrides são opcionais, não uma dependência
    obrigatória do formato."""
    if not path or not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    return data.get("overrides", {})


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


def _plausibility_score(record: Dict[str, Any], table_def: Dict[str, Any]) -> float:
    """Reimplementação da pontuação de `decode_record`/`try_table` em
    `specs/elements_155/walk_tables.py` -- ver lá para a explicação de cada regra e para o
    achado de metodologia ("não é confiável sozinha pra tabelas sem campo de texto")."""
    score = 0.0
    for field in table_def["fields"]:
        v = record[field["name"]]
        ftype = field["type"]
        if ftype == "int32":
            if -1_000_000 <= v <= 100_000_000:
                score += 1
            elif v == 0:
                score += 0.5
            else:
                score -= 2
        elif ftype == "float":
            if v == 0.0:
                score += 0.5
            elif math.isfinite(v) and abs(v) < 1e7:
                score += 1
            else:
                score -= 2
        elif ftype == "wstring":
            if len(v) == 0:
                score += 0.2
            elif all(c.isprintable() or ord(c) > 0x3000 for c in v):
                score += 1.5
            else:
                score -= 1.5
        # "string" (bytes, geralmente caminho de arquivo) nao pontua -- conteudo nao
        # confiavelmente legivel sem decodificar GBK/latin1 primeiro.
    return score


def _try_table(buf: bytes, off: int, table_def: Dict[str, Any], window: int = 1024) -> Optional[Tuple[int, int, int]]:
    """Acha `(count_field_offset, count, consumed_bytes)` pra uma tabela de tamanho fixo.
    Tenta a posição ingênua (`off`) primeiro; só faz busca em janela se isso falhar. Ver
    `TABLE_OVERRIDES`/`try_table()` em `specs/elements_155/walk_tables.py` para o histórico
    completo de por que este é o algoritmo que funciona (e o que já tentei que não funcionou)."""
    size = table_def["record_size"]
    filesize = len(buf)

    def eval_at(c_off: int) -> Optional[float]:
        if c_off < 0 or c_off + 4 > filesize:
            return None
        count = struct.unpack_from("<I", buf, c_off)[0]
        if count > 200_000:
            return None
        rec_start = c_off + 4
        if rec_start + size * count > filesize:
            return None
        if count == 0:
            return 0.1
        rec = decode_record(buf, rec_start, table_def)
        return _plausibility_score(rec, table_def)

    good_bar = len(table_def["fields"]) * 0.6
    sc = eval_at(off)
    count_at_off = struct.unpack_from("<I", buf, off)[0] if off + 4 <= filesize else None
    if sc is not None and (count_at_off == 0 or sc > good_bar):
        return off, count_at_off, 4 + count_at_off * size

    best = None
    for cand in range(-64, window):
        c_off = off + cand
        s = eval_at(c_off)
        if s is None:
            continue
        if best is None or s > best[0]:
            best = (s, c_off)
    if best is None:
        return None
    _, c_off = best
    count = struct.unpack_from("<I", buf, c_off)[0]
    return c_off, count, 4 + count * size


# =============================================================================
# TALK_PROC -- a única tabela de tamanho variável em elements.data (v156)
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

def load_elements_data(
    path: str,
    layouts_dir: Optional[str] = None,
    overrides_path: Optional[str] = None,
) -> Dict[str, List[Dict[str, Any]]]:
    """Carrega um `elements.data` inteiro, devolvendo `{nome_da_tabela: [registro, ...]}`.

    Detecta a versão pelo cabeçalho, carrega o layout correspondente do catálogo, e
    percorre as tabelas na ordem em que o `.cfg`/`elementdataman.cpp` original as declara.
    Aplica `overrides_path` (se fornecido) para as tabelas que precisarem.
    """
    with open(path, "rb") as f:
        buf = f.read()

    header = detect_header(buf)
    layout = load_layout(header["version"], layouts_dir)
    overrides = load_overrides(overrides_path) if overrides_path else {}

    result: Dict[str, List[Dict[str, Any]]] = {}
    off = header["header_size"]
    for table_def in layout["tables"]:
        idx = table_def["index"]
        name = table_def["name"]
        override = overrides.get(str(idx))

        if table_def.get("variable_size"):
            if name == "TALK_PROC":
                records, off = read_talk_proc_table(buf, off)
                result[name] = records
                continue
            raise ElementsFormatError(f"tabela '{name}' (índice {idx}) é de tamanho variável mas não tem leitor implementado")

        size = table_def["record_size"]

        if override is not None:
            if "abs_count_off" in override:
                c_off = override["abs_count_off"]
            else:
                c_off = off + override["skip"]
            count = override["count"]
        else:
            found = _try_table(buf, off, table_def)
            if found is None:
                raise ElementsFormatError(
                    f"não achei alinhamento plausível pra tabela '{name}' (índice {idx}) perto de offset {off} -- "
                    f"pode precisar de um override em {overrides_path or '(nenhum overrides_path fornecido)'}"
                )
            c_off, count, _consumed = found

        records = [decode_record(buf, c_off + 4 + i * size, table_def) for i in range(count)]
        result[name] = records
        off = c_off + 4 + count * size

    return result


if __name__ == "__main__":
    import sys

    data_path = sys.argv[1]
    overrides_arg = sys.argv[2] if len(sys.argv) > 2 else None
    tables = load_elements_data(data_path, overrides_path=overrides_arg)
    total_records = sum(len(v) for v in tables.values())
    print(f"{len(tables)} tabelas, {total_records} registros no total")
    for name, records in tables.items():
        if records:
            print(f"  {name}: {len(records)} registros (ex.: {records[0].get('Name', records[0])!r})")
