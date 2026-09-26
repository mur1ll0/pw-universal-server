"""Uso: python specs/habilidades_126/conferir_roteiros_126.py <gs 1.2.6> [N]

Compara os setters que o StateAttack/BlessMe de cada SkillNNNStub do gs 1.2.6 chama com os
do roteiro herdado do 1.5.5 no specs/habilidades_126/habilidades.json."""
import json, re, sys
from collections import defaultdict
from elftools.elf.elffile import ELFFile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

GS = sys.argv[1]
f = ELFFile(open(GS, 'rb'))
st = f.get_section_by_name('.symtab'); t = f.get_section_by_name('.text'); d = t.data(); base = t['sh_addr']
nomes = {}
funcs = {}
for s in st.iter_symbols():
    if s['st_info']['type'] != 'STT_FUNC' or s['st_size'] == 0:
        continue
    nomes[s['st_value']] = s.name
    m = re.match(r'_ZNK4GNET\d+Skill(\d+)Stub(11StateAttack|7BlessMe)EPNS_5SkillE', s.name)
    if m:
        funcs[(int(m.group(1)), 'no_alvo' if 'StateAttack' in m.group(2) else 'em_si')] = (s['st_value'], s['st_size'])
md = Cs(CS_ARCH_X86, CS_MODE_32)
def setters(a, n):
    r = []
    for x in md.disasm(d[a - base:a - base + n], a):
        if x.mnemonic == 'call':
            try:
                nm = nomes.get(int(x.op_str, 16), '')
            except ValueError:
                continue
            m = re.match(r'_ZN4GNET13PlayerWrapper\d+Set(\w+?)E[fbi]', nm)
            if m:
                r.append(m.group(1))
    return r
cat = json.load(open('specs/habilidades_126/habilidades.json', encoding='utf-8'))['habilidades']
difere = []
for sid, h in cat.items():
    for campo in ('no_alvo', 'em_si'):
        herdado = [p[1] for p in (h.get(campo) or [])]
        f126 = funcs.get((int(sid), campo))
        real = setters(*f126) if f126 else []
        P={'Probability','Time','Ratio','Amount','Value','Showicon'}
        herdado=[x for x in herdado if x not in P]; real=[x for x in real if x not in P]
        if sorted(herdado) != sorted(real):
            difere.append((int(sid), campo, real, herdado))
print(f"{len(cat)} habilidades; {len(difere)} roteiros divergem do gs 1.2.6")
for sid, campo, real, herdado in sorted(difere)[:int(sys.argv[2]) if len(sys.argv) > 2 else 15]:
    print(sid, campo, '1.2.6:', real, '| herdado:', herdado)
