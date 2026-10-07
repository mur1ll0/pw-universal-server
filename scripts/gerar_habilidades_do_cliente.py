"""Gera o mapa de habilidades do painel (B195) a partir dos stubs do cliente 1.5.5.

O cliente acha o ícone de uma habilidade por `GNET::ElementSkill::GetIcon(id)` — o `icon` do stub
`ElementSkill/skillNNN.h` (`SkillNNNStub(): SkillStub(N) { … icon = "<arquivo>.dds"; … }`) —,
tira o título e põe em minúsculas para achar a célula no atlas `IconList_Skill`
(`DlgChariot.cpp:554-559`). Os nomes de arquivo são GBK; o painel os guarda em hexadecimal, como
os ícones de item (B188). O nome em português vem do `skillstr.txt` (chave `id × 10`,
`CECSkill::GetNameDisplay`, `EC_Skill.cpp:213-215`), lido em execução por `painel/textos.py`.

Grava `web-admin/backend/painel/habilidades_do_cliente.json`: `{id: {icone, nivel_maximo, classe}}`
e mede quantos ícones o atlas extraído em `data/icones/` tem.

Uso: python scripts/gerar_habilidades_do_cliente.py
"""
import json
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent
STUBS = pathlib.Path(r"F:\PW\1.5.5\EvolvedPWClient\ElementSkill")
SAIDA = REPO / "web-admin" / "backend" / "painel" / "habilidades_do_cliente.json"
ATLAS = REPO / "data" / "icones" / "iconlist_skill.txt"


def minusculas_ascii(b):
    """`AString::MakeLower`: só A–Z (os bytes GBK de dois bytes não mudam)."""
    return bytes(c + 32 if 65 <= c <= 90 else c for c in b)


def main():
    tabela = {}
    for arquivo in sorted(STUBS.glob("skill*.h")):
        texto = arquivo.read_bytes()
        m = re.search(rb"SkillStub\s*\(\s*(\d+)\s*\)", texto)
        icone = re.search(rb'\bicon\s*=\s*"([^"]*)"', texto)
        if not m or not icone:
            continue
        nivel = re.search(rb"\bmax_level\s*=\s*(\d+)", texto)
        classe = re.search(rb"\bcls\s*=\s*(-?\d+)", texto)
        titulo = minusculas_ascii(icone.group(1).replace(b"\\\\", b"\\").split(b"\\")[-1].split(b"/")[-1])
        tabela[int(m.group(1))] = {"icone": titulo.hex(), "nivel_maximo": int(nivel.group(1)) if nivel else None,
                                    "classe": int(classe.group(1)) if classe else None}
    SAIDA.write_text(json.dumps(dict(sorted(tabela.items())), ensure_ascii=False), encoding="utf-8")
    celulas = {minusculas_ascii(l.strip(b"\r")) for l in ATLAS.read_bytes().split(b"\n")[4:]} if ATLAS.exists() else set()
    com = sum(1 for v in tabela.values() if bytes.fromhex(v["icone"]) in celulas)
    print(f"{len(tabela)} habilidades; {com} com ícone no atlas ({100 * com / max(1, len(tabela)):.1f}%)")


if __name__ == "__main__":
    main()
