"""Liga o rastreador de missões (`bTraceAll`) na configuração gravada de um personagem.

Uso: python scripts/2026_09_17_rastreador_de_missoes_ligado.py <character_id>

A configuração é o bloco de `CECGameRun::SaveConfigsToServer` (`EC_GameRun.cpp:2014-2120`):
versão (DWORD, sem compressão) + zlib(host | layout | opções, cada um com tamanho int).
No layout (`USER_LAYOUT`, `EC_GameUIMan.h:319-388`) o `bTraceAll` do `_USER_LAYOUT_8` fica
no byte 288, logo depois dos `clrGroup` do `_USER_LAYOUT_7`; os bytes 292/296
(`nQuickbarCurPanel1/2`) e 320 (`dwTraceMask`) conferem com o que o cliente grava.

Por que existe: sem resposta à configuração (antes do B52) o `CDlgTask` ficava com
`m_bShowTrace = false` do construtor (`DlgTask.cpp:79`) e o cliente o gravava assim; com ele
desligado `RefreshTaskTrace` sai sem desenhar (`DlgTask.cpp:476`) e "fixar missão" não mostra
nada. O original liga para personagem novo (`EC_GameUIMan.cpp:4738-4743`).
"""
import struct
import subprocess
import sys
import zlib

PSQL = ["docker", "exec", "-i", "pw-postgres", "psql", "-U", "pw_admin", "-d", "pw_database", "-At"]
BYTE_DO_TRACE_ALL = 288


def main(cid: int) -> None:
    hexa = subprocess.run(PSQL + ["-c", f"select encode(ui_config,'hex') from character_client_config where character_id={cid}"],
                          capture_output=True, text=True, check=True).stdout.strip()
    if not hexa:
        print("sem configuração gravada")
        return
    b = bytes.fromhex(hexa)
    d = bytearray(zlib.decompress(b[4:]))
    host = struct.unpack_from("<i", d, 0)[0]
    o = 4 + host
    tam = struct.unpack_from("<i", d, o)[0]
    ui = o + 4
    assert d[ui] >= 10, f"layout versão {d[ui]}"
    print(f"layout v{d[ui]}, {tam} bytes, bTraceAll era {d[ui + BYTE_DO_TRACE_ALL]}")
    d[ui + BYTE_DO_TRACE_ALL] = 1
    novo = b[:4] + zlib.compress(bytes(d))
    subprocess.run(PSQL + ["-c", f"update character_client_config set ui_config = decode('{novo.hex()}','hex') where character_id={cid}"],
                   check=True)
    print("ok")


if __name__ == "__main__":
    main(int(sys.argv[1]))
