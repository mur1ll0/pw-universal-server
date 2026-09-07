import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from disasm import pe, va2off, string_em
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
def dump(exe, alvo, antes=0x80, depois=0x120, marca=None):
    d,base,secs=pe(exe); md=Cs(CS_ARCH_X86,CS_MODE_32)
    start=None
    for s in range(alvo-antes, alvo-2):
        o=va2off(secs,base,s)
        if o is None: continue
        if alvo in {i.address for i in md.disasm(d[o:o+antes+8], s)}:
            start=s; break
    if start is None: start=alvo
    o=va2off(secs,base,start)
    for ins in md.disasm(d[o:o+antes+depois], start):
        com=""
        for tok in ins.op_str.replace("[","").replace("]","").replace("+"," ").replace(",", " ").split():
            if tok.startswith("0x") and len(tok)>=8:
                try: v=int(tok,16)
                except Exception: continue
                s2=string_em(d,secs,base,v)
                if s2: com='   ; "%s"' % s2[:70]
        m=" <<<<" if marca is not None and ins.address==marca else ""
        print("%08X +%-7X %-7s %-40s%s%s" % (ins.address, ins.address-base, ins.mnemonic, ins.op_str, com, m))
if __name__=="__main__":
    dump(sys.argv[1], int(sys.argv[2],0), int(sys.argv[3],0) if len(sys.argv)>3 else 0x80,
         int(sys.argv[4],0) if len(sys.argv)>4 else 0x120, int(sys.argv[2],0))
