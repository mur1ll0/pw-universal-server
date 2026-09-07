"""Reconstroi a cadeia de chamadas de um minidump do elementclient SEM PDB.

Metodo (sem chute): le a pilha capturada no minidump, pega todo DWORD que caia dentro da
secao de codigo do .exe e so aceita como endereco de retorno se os bytes IMEDIATAMENTE
ANTES desse endereco, no proprio .exe em disco, forem uma instrucao CALL (E8 rel32,
FF 15 abs, FF D0-D7 reg, FF 50/51/52... [reg+disp]). Isso e verificacao, nao heuristica:
um DWORD qualquer que so "parece" endereco quase nunca tem um CALL colado antes dele.
"""
import struct, sys, os

def le_dump(p):
    f=open(p,"rb"); d=f.read(32)
    n,rva=struct.unpack_from("<II",d,8)
    f.seek(rva); dirs=f.read(n*12); st={}
    for i in range(n):
        t,sz,r=struct.unpack_from("<III",dirs,i*12); st[t]=(sz,r)
    sz,r=st[6]; f.seek(r); e=f.read(sz)
    code,flags,rec,addr=struct.unpack_from("<IIQQ",e,8)
    csz,crva=struct.unpack_from("<II",e,160); f.seek(crva); c=f.read(csz)
    regs=dict(zip(["Edi","Esi","Ebx","Edx","Ecx","Eax","Ebp","Eip"],struct.unpack_from("<8I",c,156)))
    regs["Esp"]=struct.unpack_from("<I",c,196)[0]
    sz,r=st[5]; f.seek(r); nr=struct.unpack_from("<I",f.read(4),0)[0]
    f.seek(r+4); raw=f.read(nr*16); ranges=[]
    for i in range(nr):
        s,dsz,drva=struct.unpack_from("<QII",raw,i*16); ranges.append((s,dsz,drva))
    sz,r=st[4]; f.seek(r); m=f.read(sz); nm=struct.unpack_from("<I",m,0)[0]
    mods=[]
    for i in range(nm):
        off=4+i*108; b,s,_,_,nrva=struct.unpack_from("<QIIII",m,off)
        f.seek(nrva); ln=struct.unpack_from("<I",f.read(4),0)[0]
        mods.append((b,s,f.read(ln).decode("utf-16-le","replace")))
    return f,regs,ranges,mods,addr

def secoes(exe):
    d=open(exe,"rb").read()
    e=struct.unpack_from("<I",d,0x3c)[0]
    nsec=struct.unpack_from("<H",d,e+6)[0]
    optsz=struct.unpack_from("<H",d,e+20)[0]
    base=struct.unpack_from("<I",d,e+24+28)[0]
    secs=[]
    off=e+24+optsz
    for i in range(nsec):
        nome=d[off:off+8].rstrip(b"\0").decode("latin-1")
        vsz,va,rsz,rof=struct.unpack_from("<IIII",d,off+8)
        carac=struct.unpack_from("<I",d,off+36)[0]
        secs.append((nome,va,vsz,rof,rsz,carac)); off+=40
    return d,base,secs

def va2off(secs,base,va):
    r=va-base
    for nome,sva,vsz,rof,rsz,c in secs:
        if sva<=r<sva+max(vsz,rsz): return rof+(r-sva)
    return None

def eh_retorno_de_call(d,secs,base,va):
    """Devolve a descricao da instrucao CALL que termina exatamente em `va`, ou None."""
    o=va2off(secs,base,va)
    if o is None or o<8 or o+1>len(d): return None
    # E8 rel32 (5 bytes)
    if d[o-5]==0xE8:
        rel=struct.unpack_from("<i",d,o-4)[0]
        return ("call 0x%08X" % (va+rel), va+rel)
    # FF /2 formas comuns
    if d[o-6]==0xFF and d[o-5]==0x15: return ("call dword [0x%08X]" % struct.unpack_from("<I",d,o-4)[0], None)
    if d[o-2]==0xFF and 0xD0<=d[o-1]<=0xD7: return ("call reg", None)
    if d[o-3]==0xFF and 0x50<=d[o-2]<=0x57: return ("call [reg+0x%02X]" % d[o-1], None)
    if d[o-6]==0xFF and 0x90<=d[o-5]<=0x97: return ("call [reg+0x%08X]" % struct.unpack_from("<I",d,o-4)[0], None)
    if d[o-3]==0xFF and 0x10<=d[o-2]<=0x17: return ("call [reg]", None)
    return None

if __name__=="__main__":
    dmp,exe=sys.argv[1],sys.argv[2]
    f,regs,ranges,mods,addr=le_dump(dmp)
    d,base,secs=secoes(exe)
    modexe=[m for m in mods if m[2].lower().endswith(("elementclient.exe",))][0]
    mbase,msize=modexe[0],modexe[1]
    print("modulo: base=0x%X size=0x%X | EIP=0x%X (+0x%X)" % (mbase,msize,regs["Eip"],regs["Eip"]-mbase))
    print("regs:", {k:hex(v) for k,v in regs.items()})
    esp=regs["Esp"]
    pilha=None
    for s,dsz,drva in ranges:
        if s<=esp<s+dsz:
            f.seek(drva+(esp-s)); pilha=f.read(dsz-(esp-s)); pini=esp
    if pilha is None: print("pilha nao capturada"); sys.exit(1)
    print("pilha capturada: 0x%X..0x%X (%d bytes)\n" % (pini,pini+len(pilha),len(pilha)))
    print("%-10s %-12s %s" % ("[esp+]","valor","instrucao CALL que retorna pra la"))
    achados=0
    for i in range(0,len(pilha)-4,4):
        v=struct.unpack_from("<I",pilha,i)[0]
        if not (mbase<v<mbase+msize): continue
        r=eh_retorno_de_call(d,secs,base,v)
        if r:
            achados+=1
            alvo="" if r[1] is None else "  -> alvo +0x%X" % (r[1]-base)
            print("+0x%-8X 0x%08X (+0x%X)  %s%s" % (i,v,v-mbase,r[0],alvo))
            if achados>=40: break
    print("\ntotal de retornos confirmados:", achados)
