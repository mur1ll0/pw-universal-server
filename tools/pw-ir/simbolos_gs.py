"""Símbolos e desmontagem de um ELF 32 bits com `.symtab` (o `gs` 1.2.6, `files1.2.6/pwserver/gamed/gs`).

Uso: python -I tools/pw-ir/simbolos_gs.py <gs> <regex do símbolo> [x]

Lista as funções cujo nome casa com a regex; com `x`, desmonta (capstone) e nomeia o alvo de cada
`call`. Lê a tabela de símbolos direto (o pyelftools leva minutos no `gs` de 12 MB). B220/B221.
"""
import sys, struct, json, os, re
gs, pat = sys.argv[1], sys.argv[2]
d = open(gs,'rb').read()
shoff, = struct.unpack_from('<I', d, 0x20); shentsize, shnum, shstrndx = struct.unpack_from('<HHH', d, 0x2E)
secs=[struct.unpack_from('<10I', d, shoff+i*shentsize) for i in range(shnum)]
names_off=secs[shstrndx][4]
def nm(o): e=d.index(b'\0',o); return d[o:e].decode('latin-1')
sec={nm(names_off+s[0]):s for s in secs}
st=sec['.symtab']; sstr=sec['.strtab'][4]; tx=sec['.text']
dis = len(sys.argv)>3
if dis:
    from capstone import Cs, CS_ARCH_X86, CS_MODE_32
    md=Cs(CS_ARCH_X86,CS_MODE_32)
SYMS=list(struct.iter_unpack('<IIIBBH', d[st[4]:st[4]+st[5]]))
ADDR={v:nm(sstr+o) for (o,v,s_,i_,_a,_b) in SYMS if s_>0}
for (nmo,val,size,info,oth,shn) in SYMS:
    if size==0 or (info&0xf)!=2: continue
    n=nm(sstr+nmo)
    if re.search(pat,n):
        print('==',n,hex(val),size)
        if dis:
            off=val-tx[3]+tx[4]
            for i in md.disasm(d[off:off+size],val): print(hex(i.address),i.mnemonic,i.op_str, ADDR.get(int(i.op_str,16),'') if i.mnemonic=='call' and i.op_str.startswith('0x') else '')
