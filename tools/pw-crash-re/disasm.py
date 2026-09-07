"""Desmontador do elementclient.exe (x86 32 bits, capstone) com resolucao de strings.

Uso:
  python disasm.py <exe> fn <VA>            desmonta a funcao que contem esse VA
  python disasm.py <exe> range <VA> <n>     desmonta n instrucoes a partir do VA
"""
import struct, sys
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

def pe(exe):
    d=open(exe,"rb").read()
    e=struct.unpack_from("<I",d,0x3c)[0]
    nsec=struct.unpack_from("<H",d,e+6)[0]; optsz=struct.unpack_from("<H",d,e+20)[0]
    base=struct.unpack_from("<I",d,e+24+28)[0]
    secs=[]; off=e+24+optsz
    for i in range(nsec):
        nome=d[off:off+8].rstrip(b"\0").decode("latin-1")
        vsz,va,rsz,rof=struct.unpack_from("<IIII",d,off+8)
        secs.append((nome,va,vsz,rof,rsz)); off+=40
    return d,base,secs

def va2off(secs,base,va):
    r=va-base
    for _,sva,vsz,rof,rsz in secs:
        if sva<=r<sva+max(vsz,rsz):
            o=rof+(r-sva)
            return o if o<10**9 else None
    return None

def string_em(d,secs,base,va,maxlen=90):
    o=va2off(secs,base,va)
    if o is None or o>=len(d): return None
    b=d[o:o+maxlen]
    fim=b.find(b"\0")
    if fim<4: 
        # tenta wide
        if len(b)>8 and b[1]==0 and b[3]==0 and 32<=b[0]<127:
            w=b[:80].decode("utf-16-le","ignore").split("\0")[0]
            return ("W:"+w) if len(w)>=4 and all(31<ord(c)<0x3000 for c in w) else None
        return None
    s=b[:fim]
    try: t=s.decode("latin-1")
    except Exception: return None
    if all(32<=c<127 or c in (9,10,13) for c in s): return t
    return None

def inicio_da_funcao(d,secs,base,va,limite=0x4000):
    o=va2off(secs,base,va)
    for back in range(0,limite):
        p=o-back
        if p<1: break
        # padding int3 antes do prologo
        if d[p-1]==0xCC and d[p]==0x55 and d[p+1]==0x8B and d[p+2]==0xEC: return va-back
        if d[p-1]==0xCC and d[p]==0x53: return va-back
        if d[p-1]==0xCC and d[p]==0x56: return va-back
        if d[p-1]==0xCC and d[p]==0x83 and d[p+1]==0xEC: return va-back
        if d[p-1]==0xCC and d[p]==0x81 and d[p+1]==0xEC: return va-back
        if d[p-1]==0xCC and d[p]==0x8B and d[p+1]==0xFF: return va-back
    return None

def desmonta(exe, va_ini, nbytes, marca=None):
    d,base,secs=pe(exe)
    o=va2off(secs,base,va_ini)
    md=Cs(CS_ARCH_X86,CS_MODE_32); md.detail=False
    code=d[o:o+nbytes]
    for ins in md.disasm(code, va_ini):
        com=""
        for tok in ins.op_str.replace("[","").replace("]","").replace("+"," ").split():
            if tok.startswith("0x") and len(tok)>=8:
                try: v=int(tok,16)
                except Exception: continue
                s=string_em(d,secs,base,v)
                if s: com='   ; "%s"' % s[:70]
        marcador=" <<<< CRASH" if marca is not None and ins.address==marca else ""
        print("%08X  +%-8X %-8s %-42s%s%s" % (ins.address, ins.address-base, ins.mnemonic, ins.op_str, com, marcador))

if __name__=="__main__":
    exe=sys.argv[1]; modo=sys.argv[2]
    if modo=="fn":
        va=int(sys.argv[3],0)
        d,base,secs=pe(exe)
        ini=inicio_da_funcao(d,secs,base,va)
        print("### funcao que contem 0x%X: inicio provavel 0x%X (+0x%X)" % (va,ini or 0,(ini or base)-base))
        desmonta(exe, ini or va, (va-(ini or va))+96, marca=va)
    else:
        desmonta(exe, int(sys.argv[3],0), int(sys.argv[4],0))
