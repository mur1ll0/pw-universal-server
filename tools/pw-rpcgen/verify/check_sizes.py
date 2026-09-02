#!/usr/bin/env python3
"""Confere o IR do gamedata contra o compilador C++, no alvo original de 32 bits.

Esta é a verificação independente do `gamedata_153.json`: ela não repete a lógica do
parser em Python — gera arquivos de `static_assert` e deixa **o compilador**, lendo os
mesmos cabeçalhos que o `pw-rpcgen` leu, dizer se cada tamanho e cada deslocamento
estão certos.

Dois lados são verificados, cada um com seus próprios cabeçalhos:

* **cliente** — `CElementClient/Network/EC_GPDataType.h` (e `EC_RoleTypes.h`);
* **servidor** — `cgame/common/protocol.h` (opcional, com `--server-src`).

Para cada lado se afirma:

* `sizeof(...)` de toda struct que o IR diz ter tamanho fixo;
* `sizeof` do membro correspondente, para cada struct aninhada;
* `__builtin_offsetof(...)` de todo campo com deslocamento no IR — é aqui que o
  `#pragma pack(1)` é de fato provado, campo a campo.

Nada é linkado: `-fsyntax-only` basta, e assim não é preciso ter libc de 32 bits.

Foi este roteiro que apontou **cinco** erros reais do parser que nenhum teste de unidade
teria pego: código de exemplo dentro de blocos `/* */` lido como campo; `} *data;`
(ponteiro para struct aninhada) tratado como membro embutido; `int64_t`/`uint64_t`
ausentes da tabela de tipos; `struct{` sem espaço antes da chave não reconhecido como
struct aninhada; e campos com nome iniciado em `_` descartados em silêncio.

Uso:

    python3 tools/pw-rpcgen/verify/check_sizes.py \\
        --ir specs/protocol/gamedata_153.json \\
        --client-src <raiz dos fontes do cliente 1.5.3> \\
        [--server-src <raiz dos fontes do servidor 1.5.3>]

Requer `g++` com suporte a `-m32` (não precisa de g++-multilib, só do compilador).
"""

import argparse
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

PREAMBULO_CLIENTE = """\
// GERADO por tools/pw-rpcgen/verify/check_sizes.py — não editar à mão.
//
// Sem cabeçalhos padrão: não há libstdc++ de 32 bits garantida, e nada aqui precisa
// dela. `size_t` vem do próprio compilador, já com a largura do alvo -m32.
typedef __SIZE_TYPE__ size_t;
extern "C" void* memcpy(void*, const void*, size_t);
extern "C" void* memset(void*, int, size_t);
#define assert(x) ((void)0)
#define __int64 long long
#define NULL 0
typedef unsigned long DWORD;
typedef unsigned char BYTE;
typedef unsigned short WORD;
// `player_info_2` e `player_info_3` são usados por `EC_GPDataType.h` mas declarados em
// outro cabeçalho do cliente que não está nos fontes. Os tamanhos aqui são arbitrários:
// as structs que os contêm ficam sem tamanho no IR, então nenhuma asserção depende
// deles. (`ROLEEXTPROP*` já não precisa de substituto: `EC_RoleTypes.h` é lido de
// verdade, tanto pelo pw-rpcgen quanto por esta verificação.)
struct player_info_2 { int _p[16]; };
struct player_info_3 { int _p[16]; };

#include "EC_GPDataType.h"

"""

PREAMBULO_SERVIDOR = """\
// GERADO por tools/pw-rpcgen/verify/check_sizes.py — não editar à mão.
#include "protocol.h"

"""


def nome_cxx(scope: str, prefixo: str) -> str:
    """Converte o escopo do IR no escopo C++ correspondente."""
    if prefixo and scope.startswith(prefixo):
        scope = scope[len(prefixo):]
    return scope


def gerar(ir: dict, prefixo: str, lado: str) -> tuple[str, dict[str, int]]:
    """Monta as asserções para um dos lados.

    `prefixo` é o que separa os escopos daquele lado no IR: vazio para o cliente
    (`S2C`, `C2S`) e `SRV::` para o servidor.
    """
    tamanhos: list[str] = []
    membros: list[str] = []
    deslocamentos: list[str] = []
    structs = ir["structs"]

    for nome, s in structs.items():
        escopo = s["scope"]
        do_lado = escopo.startswith("SRV")
        if do_lado != (lado == "servidor"):
            continue

        cxx_escopo = nome_cxx(escopo, "SRV::")

        # Struct aninhada: quando anônima não tem nome utilizável em C++, então o que
        # se afirma é o tamanho do MEMBRO correspondente na struct externa.
        if "::" in s["name"]:
            externa, membro = s["name"].rsplit("::", 1)
            pai = structs.get(f"{escopo}::{externa}")
            if not pai:
                continue
            campo = next((c for c in pai["fields"] if c["name"] == membro), None)
            if campo and campo["bytes"] is not None:
                alvo = f"{cxx_escopo}::{externa}" if cxx_escopo else externa
                membros.append(
                    f'static_assert(sizeof((({alvo}*)0)->{membro}) == '
                    f'{campo["bytes"]}, "{nome}");'
                )
            continue

        cxx = f"{cxx_escopo}::{s['name']}" if cxx_escopo else s["name"]

        if s["bytes"] is not None:
            tamanhos.append(f'static_assert(sizeof({cxx}) == {s["bytes"]}, "{nome}");')

        for campo in s["fields"]:
            if campo["offset"] is None:
                continue
            deslocamentos.append(
                f'static_assert(__builtin_offsetof({cxx}, {campo["name"]}) == '
                f'{campo["offset"]}, "{nome}.{campo["name"]}");'
            )

    corpo = "\n".join(tamanhos + [""] + membros + [""] + deslocamentos) + "\n"
    return corpo, {
        "tamanhos": len(tamanhos),
        "membros": len(membros),
        "deslocamentos": len(deslocamentos),
    }


def compilar(fonte: pathlib.Path, includes: list[pathlib.Path]) -> subprocess.CompletedProcess:
    cmd = ["g++", "-m32", "-fsyntax-only", "-w"]
    cmd += [f"-I{p}" for p in includes]
    cmd.append(str(fonte))
    return subprocess.run(cmd, capture_output=True, text=True)


def relatar(lado: str, contagem: dict[str, int], r: subprocess.CompletedProcess) -> bool:
    total = sum(contagem.values())
    print(
        f'{lado}: {contagem["tamanhos"]} tamanhos de struct, '
        f'{contagem["membros"]} membros aninhados, '
        f'{contagem["deslocamentos"]} deslocamentos de campo — {total} asserções'
    )
    if r.returncode != 0:
        print(f"\nO compilador discorda do IR ({lado}):\n", file=sys.stderr)
        print(r.stderr, file=sys.stderr)
        return False
    print(f"  todas as {total} batem com o C++ original compilado para i386")
    return True


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--ir", required=True, type=pathlib.Path)
    p.add_argument("--client-src", required=True, type=pathlib.Path)
    p.add_argument("--server-src", type=pathlib.Path)
    args = p.parse_args()

    aqui = pathlib.Path(__file__).resolve().parent
    ir = json.loads(args.ir.read_text(encoding="utf-8"))
    ok = True

    # ---- cliente ----------------------------------------------------------
    rede = args.client_src / "CElementClient" / "Network"
    if not (rede / "EC_GPDataType.h").is_file():
        print(f"não achei EC_GPDataType.h em {rede}", file=sys.stderr)
        return 2

    corpo, contagem = gerar(ir, "", "cliente")
    with tempfile.TemporaryDirectory() as tmp:
        fonte = pathlib.Path(tmp) / "check_cliente.cpp"
        fonte.write_text(PREAMBULO_CLIENTE + corpo, encoding="utf-8")
        # Os cabeçalhos REAIS vêm primeiro; `stubs/` só entra para o que não existe nos
        # fontes (`A3DVector.h`, `vector.h`, `ABaseDef.h`, do motor gráfico). A ordem
        # importa: um stub de `EC_RoleTypes.h` à frente sombrearia o cabeçalho de
        # verdade e a verificação passaria a conferir o layout errado.
        r = compilar(fonte, [rede, args.client_src / "CElementClient", aqui / "stubs"])
    ok &= relatar("cliente", contagem, r)

    # ---- servidor ---------------------------------------------------------
    if args.server_src:
        comum = args.server_src / "cgame" / "common"
        if not (comum / "protocol.h").is_file():
            print(f"não achei protocol.h em {comum}", file=sys.stderr)
            return 2

        corpo, contagem = gerar(ir, "SRV::", "servidor")
        with tempfile.TemporaryDirectory() as tmp:
            tmpdir = pathlib.Path(tmp)
            # O `protocol.h` original inclui `"types.h"`, que puxa a libstdc++. O
            # include entre aspas resolve primeiro na pasta do próprio arquivo, então a
            # única forma de substituí-lo é copiar o cabeçalho para junto do nosso.
            #
            # Os `[]` viram `[1]`: `object_state_notify` tem DOIS membros-array
            # flexíveis, o que o C++ não aceita. Nenhuma asserção depende disso — o IR
            # marca esses campos como irresolúveis e não gera nada para eles.
            texto = (comum / "protocol.h").read_text(encoding="latin-1")
            (tmpdir / "protocol.h").write_text(texto.replace("[]", "[1]"), encoding="utf-8")
            shutil.copy(aqui / "srv_stubs" / "types.h", tmpdir / "types.h")

            fonte = tmpdir / "check_servidor.cpp"
            fonte.write_text(PREAMBULO_SERVIDOR + corpo, encoding="utf-8")
            r = compilar(fonte, [tmpdir])
        ok &= relatar("servidor", contagem, r)

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
