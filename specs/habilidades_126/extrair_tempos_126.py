"""Extrai do servidor 1.2.6 os tempos de cada habilidade: conjuração, estados, execução e recarga.

Fonte: `files1.2.6/pwserver/gamed/gs` (ELF 32-bit, não removido os símbolos) — o `gs` que
carrega o `elements.data` v7 (exige 0x30000007 em `elementdataman::load_data`, VA 0x81b1e3d).
Cada habilidade é uma classe `GNET::SkillNNNStub` compilada no binário; o servidor usa
`State1::GetTime` (conjuração enviada no `OBJECT_CAST_SKILL`), `StateK::GetTime` (as fases
seguintes), `GetExecutetime` e `GetCoolingtime` — as mesmas funções do stub 1.5.5
(`specs/habilidades_155/extrair_habilidades.py`).

Método: cada função é **executada** num emulador x86 (unicorn) para os níveis 1..max_level,
com `GNET::Skill::GetLevel` (VA 0x8306954) interceptado para devolver o nível. Das 3.731
funções, 3.718 devolvem constante e 13 dependem do nível; nenhuma lê outra memória (uma
leitura fora do mapeado faria a extração falhar, e a função sairia `null`).

Conferido com a captura original do 1.2.6 (`_sync/capturas/*.pcap`, B100): 102 (200+700 ms),
250 nível 2 (500+900 ms) e 299 (1.500+1.000 ms) terminam no `HOST_STOP_SKILL` depois de
conjuração + execução.

`max_level` vem do `habilidades.json` do 1.5.5 (as 823 habilidades do 1.2.6 existem lá);
sem entrada, 10.

Uso:
    python specs/habilidades_126/extrair_tempos_126.py ../files1.2.6/pwserver/gamed/gs
Requer: pyelftools, unicorn.
"""
import json
import re
import struct
import sys
from collections import defaultdict
from pathlib import Path

from elftools.elf.elffile import ELFFile
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_32, Uc
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_EBP, UC_X86_REG_EIP, UC_X86_REG_ESP

AQUI = Path(__file__).resolve().parent
FUNCAO = re.compile(
    r"_ZNK4GNET\d+Skill(\d+)Stub(?:6State(\d+)7GetTime|14GetCoolingtime|14GetExecutetime)EPNS_5SkillE$")
PILHA, FIM = 0x7000_0000, 0x7100_0000


def main(caminho_gs: str) -> None:
    elf = ELFFile(open(caminho_gs, "rb"))
    simbolos = list(elf.get_section_by_name(".symtab").iter_symbols())
    get_level = {s["st_value"] for s in simbolos if s.name == "_ZNK4GNET5Skill8GetLevelEv"}
    assert get_level, "GNET::Skill::GetLevel não encontrado"

    mu = Uc(UC_ARCH_X86, UC_MODE_32)
    for seg in elf.iter_segments():
        if seg["p_type"] != "PT_LOAD":
            continue
        ini = seg["p_vaddr"] & ~0xFFF
        fim = (seg["p_vaddr"] + seg["p_memsz"] + 0xFFF) & ~0xFFF
        mu.mem_map(ini, fim - ini)
        mu.mem_write(seg["p_vaddr"], seg.data())
    mu.mem_map(PILHA - 0x10000, 0x20000)
    mu.mem_map(FIM, 0x1000)

    def executar(endereco: int, nivel: int) -> int:
        def gancho(uc, ad, _tam, _dados):
            if ad in get_level:
                esp = uc.reg_read(UC_X86_REG_ESP)
                volta = struct.unpack("<I", uc.mem_read(esp, 4))[0]
                uc.reg_write(UC_X86_REG_EAX, nivel)
                uc.reg_write(UC_X86_REG_ESP, esp + 4)
                uc.reg_write(UC_X86_REG_EIP, volta)
        h = mu.hook_add(UC_HOOK_CODE, gancho)
        esp = PILHA
        for v in (0x1000, 0x2000, FIM):  # Skill*, this, retorno
            esp -= 4
            mu.mem_write(esp, struct.pack("<I", v))
        mu.reg_write(UC_X86_REG_ESP, esp)
        mu.reg_write(UC_X86_REG_EBP, 0)
        try:
            mu.emu_start(endereco, FIM, count=2000)
        finally:
            mu.hook_del(h)
        return struct.unpack("<i", struct.pack("<I", mu.reg_read(UC_X86_REG_EAX)))[0]

    h155 = json.loads((AQUI.parent / "habilidades_155" / "habilidades.json").read_text(encoding="utf-8"))
    h155 = h155["habilidades"]
    brutos = defaultdict(dict)
    for s in simbolos:
        m = FUNCAO.match(s.name)
        if not m or not s["st_value"]:
            continue
        sid = int(m.group(1))
        campo = f"estado{m.group(2)}" if m.group(2) else ("recarga" if "Cool" in s.name else "execucao")
        n = (h155.get(str(sid)) or {}).get("max_level") or 10
        try:
            brutos[sid][campo] = [executar(s["st_value"], nv) for nv in range(1, n + 1)]
        except Exception:  # noqa: BLE001 — leitura fora do mapeado: desconhecido, nunca zero
            brutos[sid][campo] = None

    saida = {}
    for sid in sorted(brutos):
        b = brutos[sid]
        k = 1
        estados = []
        while f"estado{k}" in b:
            estados.append(b[f"estado{k}"])
            k += 1
        saida[str(sid)] = {"estados_ms": estados, "execucao_ms": b.get("execucao"),
                           "recarga_ms": b.get("recarga")}
    doc = {"fonte": "files1.2.6/pwserver/gamed/gs (SkillNNNStub, emulado por nível)",
           "habilidades": saida}
    (AQUI / "tempos.json").write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n",
                                      encoding="utf-8")
    print(f"{len(saida)} habilidades")


if __name__ == "__main__":
    main(sys.argv[1])
