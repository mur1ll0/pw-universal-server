"""Extrai do servidor 1.2.6 os números de cada habilidade, executando os stubs compilados.

Fonte: `files1.2.6/pwserver/gamed/gs` (ELF 32-bit com símbolos) — o `gs` que carrega o
`elements.data` v7 (exige 0x30000007 em `elementdataman::load_data`, VA 0x81b1e3d). Cada
habilidade é uma classe `GNET::SkillNNNStub` (823 no binário, as mesmas 823 do `skillstr.txt`
do cliente 1.2.6), com as mesmas funções virtuais do stub 1.5.5
(`specs/habilidades_155/extrair_habilidades.py`, que lê o fonte C++ gerado).

Método: cada função é **executada** num emulador x86 (unicorn) para os níveis 1..max_level.
As chamadas a `GNET::Skill`/`GNET::PlayerWrapper` são interceptadas:
- `GetLevel` devolve o nível; `GetPlayer` um ponteiro qualquer;
- `GetAttack`/`GetMagicattack` devolvem uma sonda (1000), para achar a base e o fator do dano;
- `PlayerWrapper::GetRange` (float, pilha x87) devolve 0 e 1, para separar `arma × alcance +
  fixo` como o extrator 1.5.5;
- `GetCharging` devolve a carga cheia (o tempo do 1º estado), como no 1.5.5;
- `Set*` guardam o argumento (`SetRatio`, `SetPlus`, `Set<Elemento>damage`...).
Qualquer outro `Get*` (vida, mana, `GetPlus`...) torna o campo `null`: a expressão depende de
mais do que o nível — desconhecido, nunca zero. Função ausente no stub = padrão do
`SkillStub` (`cskill/skill/skill.h:409-436`: `GetMpcost`/`GetRequired*` devolvem 0).

Campos, com o mesmo significado do `habilidades.json` 1.5.5: `estados_ms` (`StateK::GetTime`),
`execucao_ms`, `recarga_ms`, `mp` (`GetMpcost`), `nivel_exigido`/`sp_exigido`/
`dinheiro_exigido`, `alcance` (`GetPraydistance`), `distancia_de_efeito`
(`GetEffectdistance`), `raio` (`GetRadius`), `distancia_de_ataque` (`GetAttackdistance`),
`angulo` (`GetAngle`), `precisao` (`GetHitrate`) e `dano` (o estado cujo `Calculate` chama
`Set<X>damage(k × GetAttack|GetMagicattack)`: base, elemento, fator `k`, `ratio`, `plus`).

Conferido com a captura original do 1.2.6 (B100): 102 (200+700 ms), 250 nível 2 (500+900) e
299 (1.500+1.000) fecham o `HOST_STOP_SKILL` depois de conjuração + execução.

Saída: `habilidades.json` **completo** (mesmo formato do 1.5.5), mesclado com o 1.5.5 no
que não é função de nível. Pode ser copiado para `data/<realm>/catalogo/habilidades.json`.

Uso:
    python specs/habilidades_126/extrair_habilidades_126.py ../files1.2.6/pwserver/gamed/gs
Requer: pyelftools, unicorn.
"""
import json
import math
import re
import struct
import sys
from collections import defaultdict
from pathlib import Path

import unicorn.x86_const as X
from elftools.elf.elffile import ELFFile
from unicorn import UC_ARCH_X86, UC_HOOK_CODE, UC_MODE_32, Uc

AQUI = Path(__file__).resolve().parent
PILHA, FIM = 0x7000_0000, 0x7100_0000
SONDA = 1000
FUNCAO = re.compile(r"_ZNK4GNET\d+Skill(\d+)Stub(?:6State(\d+)(7GetTime|9Calculate)|(\d+)(\w+?))EPNS_5SkillE$")
CHAMADA = re.compile(r"_ZNK?4GNET(?:5Skill|13PlayerWrapper)(\d+)(\w+)$")
ELEMENTOS = {"Damage", "Golddamage", "Wooddamage", "Waterdamage", "Firedamage", "Earthdamage"}


class Desconhecido(Exception):
    """A função consultou algo além do nível: o valor não é tabela."""


class Emulador:
    def __init__(self, caminho_gs: str):
        elf = ELFFile(open(caminho_gs, "rb"))
        self.simbolos = [s for s in elf.get_section_by_name(".symtab").iter_symbols() if s["st_value"]]
        self.chamaveis = {}
        for s in self.simbolos:
            m = CHAMADA.match(s.name)
            if m and ("4GNET5Skill" in s.name or "13PlayerWrapper" in s.name):
                self.chamaveis[s["st_value"]] = (m.group(2)[: int(m.group(1))], s.name.endswith("f"))
        self.mu = Uc(UC_ARCH_X86, UC_MODE_32)
        for seg in elf.iter_segments():
            if seg["p_type"] != "PT_LOAD":
                continue
            ini = seg["p_vaddr"] & ~0xFFF
            fim = (seg["p_vaddr"] + seg["p_memsz"] + 0xFFF) & ~0xFFF
            self.mu.mem_map(ini, fim - ini)
            self.mu.mem_write(seg["p_vaddr"], seg.data())
        self.mu.mem_map(PILHA - 0x10000, 0x20000)
        self.mu.mem_map(FIM, 0x1000)

    # Pilha x87: o `float` volta em ST0; o registrador físico é o do TOP do FPSW.
    def _st0(self) -> float:
        top = (self.mu.reg_read(X.UC_X86_REG_FPSW) >> 11) & 7
        man, exp = self.mu.reg_read(getattr(X, f"UC_X86_REG_FP{top}"))
        if man == 0 and (exp & 0x7FFF) == 0:
            return 0.0
        return (-1 if exp & 0x8000 else 1) * man / (1 << 63) * 2.0 ** ((exp & 0x7FFF) - 16383)

    def _empilhar(self, v: float) -> None:
        sw = self.mu.reg_read(X.UC_X86_REG_FPSW)
        top = (((sw >> 11) & 7) - 1) & 7
        if v == 0:
            man, exp = 0, 0
        else:
            m, e = math.frexp(abs(v))
            man, exp = int(m * (1 << 64)), (e - 1 + 16383) | (0x8000 if v < 0 else 0)
        self.mu.reg_write(getattr(X, f"UC_X86_REG_FP{top}"), (man, exp))
        self.mu.reg_write(X.UC_X86_REG_FPSW, (sw & ~(7 << 11)) | (top << 11))

    def executar(self, endereco, nivel, *, alcance=0.0, ataque=0, magico=0, carga=0, retorno="int"):
        """Executa `f(this, skill)`; devolve (retorno, setters capturados, getters usados)."""
        setters, usados = {}, set()

        def gancho(uc, ad, _t, _d):
            c = self.chamaveis.get(ad)
            if not c:
                return
            nome, arg_float = c
            esp = uc.reg_read(X.UC_X86_REG_ESP)
            volta = struct.unpack("<I", uc.mem_read(esp, 4))[0]
            if nome == "GetLevel":
                uc.reg_write(X.UC_X86_REG_EAX, nivel)
            elif nome == "GetPlayer":
                uc.reg_write(X.UC_X86_REG_EAX, 0x3000)
            elif nome == "GetRange":
                usados.add(nome)
                self._empilhar(alcance)
            elif nome in ("GetAttack", "GetMagicattack", "GetCharging"):
                usados.add(nome)
                uc.reg_write(X.UC_X86_REG_EAX, {"GetAttack": ataque, "GetMagicattack": magico, "GetCharging": carga}[nome])
            elif nome.startswith(("Get", "Is", "Has", "Can")):
                raise Desconhecido(nome)
            else:
                bruto = bytes(uc.mem_read(esp + 8, 4))
                setters[nome] = struct.unpack("<f" if arg_float else "<i", bruto)[0]
            uc.reg_write(X.UC_X86_REG_ESP, esp + 4)
            uc.reg_write(X.UC_X86_REG_EIP, volta)

        h = self.mu.hook_add(UC_HOOK_CODE, gancho)
        esp = PILHA
        for v in (0x1000, 0x2000, FIM):  # Skill*, this, retorno
            esp -= 4
            self.mu.mem_write(esp, struct.pack("<I", v))
        self.mu.reg_write(X.UC_X86_REG_ESP, esp)
        self.mu.reg_write(X.UC_X86_REG_EBP, 0)
        self.mu.reg_write(X.UC_X86_REG_FPSW, 0)
        try:
            self.mu.emu_start(endereco, FIM, count=5000)
        finally:
            self.mu.hook_del(h)
        if retorno == "float":
            valor = self._st0()
        else:
            valor = struct.unpack("<i", struct.pack("<I", self.mu.reg_read(X.UC_X86_REG_EAX)))[0]
        return valor, setters, usados


def arredondar(v: float) -> float:
    return float(f"{v:.6g}")


def main(caminho_gs: str) -> None:
    emu = Emulador(caminho_gs)
    h155 = json.loads((AQUI.parent / "habilidades_155" / "habilidades.json").read_text(encoding="utf-8"))["habilidades"]
    funcoes = defaultdict(dict)
    for s in emu.simbolos:
        m = FUNCAO.match(s.name)
        if not m:
            continue
        sid = int(m.group(1))
        if m.group(2):
            chave = ("GetTime" if m.group(3) == "7GetTime" else "Calculate", int(m.group(2)))
        else:
            chave = m.group(5)[: int(m.group(4))]
        funcoes[sid][chave] = s["st_value"]

    def por_nivel(f, n, **kw):
        try:
            return [emu.executar(f, nv, **kw)[0] for nv in range(1, n + 1)]
        except Desconhecido:
            return None

    def distancia(f, n):
        """`k × GetRange() + fixo` — o formato `{arma, fixo}` do 1.5.5."""
        if f is None:
            return None
        try:
            fixo, usa = [], False
            for nv in range(1, n + 1):
                v0, _, usados = emu.executar(f, nv, alcance=0.0, retorno="float")
                v1, _, _ = emu.executar(f, nv, alcance=1.0, retorno="float")
                usa |= "GetRange" in usados
                if usa and abs(v1 - v0 - 1.0) > 1e-4:
                    return None
                fixo.append(arredondar(v0))
            return {"arma": 1 if usa else 0, "fixo": fixo}
        except Desconhecido:
            return None

    saida = {}
    for sid in sorted(funcoes):
        fs = funcoes[sid]
        n = (h155.get(str(sid)) or {}).get("max_level") or 10
        num_estados = max((k[1] for k in fs if isinstance(k, tuple)), default=0)
        estados = [por_nivel(fs[("GetTime", k)], n) if ("GetTime", k) in fs else None
                   for k in range(1, num_estados + 1)]

        def inteiro(nome):
            return por_nivel(fs[nome], n) if nome in fs else [0] * n

        def decimal(nome):
            if nome not in fs:
                return None
            v = por_nivel(fs[nome], n, retorno="float")
            return None if v is None else [arredondar(x) for x in v]

        mp = decimal("GetMpcost") if "GetMpcost" in fs else [0.0] * n
        dano = None
        for k in range(1, num_estados + 1):
            f = fs.get(("Calculate", k))
            if f is None:
                continue
            cheia = (estados[0] or [0] * n) if estados else [0] * n
            try:
                ratio, plus, achado = [], [], None
                for nv in range(1, n + 1):
                    _, s_fis, u_fis = emu.executar(f, nv, ataque=SONDA, carga=cheia[nv - 1])
                    _, s_mag, _ = emu.executar(f, nv, magico=SONDA, carga=cheia[nv - 1])
                    el = [e for e in s_fis if e.startswith("Set") and e[3:] in ELEMENTOS]
                    if not el:
                        break
                    e = el[0]
                    base = "fisico" if s_fis[e] else "magico"
                    fator = (s_fis[e] or s_mag[e]) / SONDA
                    achado = (e[3:], base, arredondar(fator), "GetCharging" in u_fis)
                    ratio.append(arredondar(s_fis.get("SetRatio", 0.0)))
                    plus.append(arredondar(s_fis.get("SetPlus", 0.0)))
                if achado and len(ratio) == n:
                    dano = {"estado": k - 1, "base": achado[1], "elemento": achado[0], "fator": achado[2],
                            "ratio": ratio, "plus": plus, "carga": achado[3]}
                    break
            except Desconhecido:
                continue
        saida[str(sid)] = {
            "estados_ms": estados,
            "execucao_ms": inteiro("GetExecutetime"),
            "recarga_ms": inteiro("GetCoolingtime"),
            "mp": mp,
            "nivel_exigido": inteiro("GetRequiredLevel"),
            "sp_exigido": inteiro("GetRequiredSp"),
            "dinheiro_exigido": inteiro("GetRequiredMoney"),
            "alcance": distancia(fs.get("GetPraydistance"), n),
            "distancia_de_efeito": distancia(fs.get("GetEffectdistance"), n),
            "raio": decimal("GetRadius"),
            "distancia_de_ataque": decimal("GetAttackdistance"),
            "angulo": decimal("GetAngle"),
            "precisao": decimal("GetHitrate"),
            "dano": dano,
        }
    # A tabela sai **completa**, no formato do `habilidades.json` 1.5.5: o que o stub 1.2.6
    # não tem como função de nível (classe, tipo, pré-requisitos, `time_type`, área, flags,
    # roteiros `no_alvo`/`em_si`) vem do stub 1.5.5 de mesmo id; o que o 1.2.6 deixou `null`
    # (função que lê outra coisa, como `GetHp`) também. Assim o servidor lê esta tabela como
    # lê a do 1.5.5 — e ela pode ir para `data/<realm>/catalogo/habilidades.json`.
    completa = {}
    for sid, h in saida.items():
        base = h155.get(sid)
        if base is None:
            continue
        m = dict(base)
        for campo, valor in h.items():
            if valor is not None or campo in ("estados_ms", "execucao_ms", "recarga_ms"):
                m[campo] = valor
        completa[sid] = m
    doc = {"fonte": "files1.2.6/pwserver/gamed/gs (SkillNNNStub, executado por nível) + "
                    "specs/habilidades_155/habilidades.json (o que não é função de nível)",
           "habilidades": completa}
    (AQUI / "habilidades.json").write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n",
                                           encoding="utf-8")
    nulos = defaultdict(int)
    for h in saida.values():
        for c, v in h.items():
            if v is None:
                nulos[c] += 1
    print(f"{len(saida)} habilidades; campos null: {dict(nulos)}")


if __name__ == "__main__":
    main(sys.argv[1])
