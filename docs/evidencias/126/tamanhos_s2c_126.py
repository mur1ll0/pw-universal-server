"""Tamanho fixo de cada S2C no validador do elementclient.exe 1.2.6 (VA 0x584610, tabela de
saltos em 0x584e90). Uso: python tamanhos_s2c_126.py > tamanhos_s2c_126.txt.
Cada linha: id, VA do caso, tamanho (None = variável; tupla jmp = caso que segue adiante)."""
import struct, sys
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
exe=open(r'F:\Games\perfectworld_126\element\elementclient.exe','rb').read()
pe=struct.unpack_from('<I',exe,0x3c)[0]; nsec=struct.unpack_from('<H',exe,pe+6)[0]; opt=struct.unpack_from('<H',exe,pe+20)[0]
base=struct.unpack_from('<I',exe,pe+24+28)[0]
secs=[]
o=pe+24+opt
for i in range(nsec):
    va,sz,raw=struct.unpack_from('<III',exe,o+12)[0],struct.unpack_from('<I',exe,o+16)[0],struct.unpack_from('<I',exe,o+20)[0]
    vs=struct.unpack_from('<I',exe,o+8)[0]; secs.append((base+va,max(vs,sz),raw)); o+=40
def off(v):
    for a,s,r in secs:
        if a<=v<a+s: return v-a+r
def code(v,n=0x300): x=off(v); return exe[x:x+n]
md=Cs(CS_ARCH_X86,CS_MODE_32)
import re
tab=0x584e90
def caso(n):
    alvo=struct.unpack_from('<I',exe,off(tab+4*n))[0]
    ult=None
    for i in md.disasm(code(alvo,0x80),alvo):
        s=f"{i.mnemonic} {i.op_str}"
        m=re.match(r'mov dword ptr \[esp(?: \+ 0x[0-9a-f]+)?\], (0x[0-9a-f]+|\d+)$',s)
        if m: ult=int(m.group(1),0)
        if i.mnemonic.startswith('ret'): return alvo, ult
        if i.mnemonic=='jmp': return alvo, ('jmp', i.op_str, ult)
    return alvo, ('?',ult)
for n in range(0,261):
    a,t=caso(n); print(n, hex(a), t)
