"""Le um npcgen.data inteiro (areas de IA, recursos, objetos dinamicos, controladores)
espelhando crates/pw-data-loader/src/npcgen.rs, e COBRA que o offset final bata exatamente
com o tamanho do arquivo -- e a prova de que a leitura esta certa, nao so plausivel."""
import struct, sys, os

def ler(path):
    b=open(path,"rb").read()
    if len(b)<12: return None
    o=0
    def u(fmt):
        nonlocal o
        v=struct.unpack_from(fmt,b,o); o+=struct.calcsize(fmt); return v
    (ver,)=u("<I"); num_ai,num_res=u("<ii")
    num_dyn=u("<i")[0] if ver>=6 else 0
    num_ctrl=u("<i")[0] if ver>=7 else 0
    mob=set(); res=set(); dyn=set()
    resto = 44 if ver>=11 else 40
    area_hdr = 71 if ver>=7 else 59
    for _ in range(num_ai):
        base=o
        _t,num_gen=struct.unpack_from("<ii",b,o)
        o=base+area_hdr
        for _ in range(num_gen):
            tid=struct.unpack_from("<I",b,o)[0]; o+=20+resto
            if tid: mob.add(tid)
    for _ in range(num_res):
        base=o
        num_r=struct.unpack_from("<i",b,o+20)[0]
        o=base+42
        for _ in range(num_r):
            tid=struct.unpack_from("<I",b,o+4)[0]; o+=20
            if tid: res.add(tid)
    for _ in range(num_dyn):
        did=struct.unpack_from("<I",b,o)[0]; o+=24
        if did: dyn.add(did)
    o += num_ctrl*199
    return dict(ver=ver, mob=mob, res=res, dyn=dyn, fim=o, tam=len(b), ok=(o==len(b)))

if __name__=="__main__":
    raiz=sys.argv[1]
    M=set(); R=set(); D=set(); vers={}; ruins=[]; n=0
    for d in sorted(os.listdir(raiz)):
        p=os.path.join(raiz,d,"npcgen.data")
        if not os.path.exists(p): continue
        n+=1
        try: r=ler(p)
        except Exception as e: ruins.append((d,"EXC "+str(e)[:40])); continue
        if r is None: continue
        vers[r["ver"]]=vers.get(r["ver"],0)+1
        if not r["ok"]: ruins.append((d,"sobra %d de %d (v%d)"%(r["tam"]-r["fim"],r["tam"],r["ver"])))
        M|=r["mob"]; R|=r["res"]; D|=r["dyn"]
    print("mapas: %d   versoes: %s" % (n, dict(sorted(vers.items()))))
    print("arquivos que NAO fecharam byte a byte: %d %s" % (len(ruins), ruins[:6]))
    print("ids IA=%d  recurso=%d  dynobj=%d" % (len(M),len(R),len(D)))
    if len(sys.argv)>2:
        import json; json.dump({"ia":sorted(M),"res":sorted(R),"dyn":sorted(D)}, open(sys.argv[2],"w"))
        print("gravado:", sys.argv[2])
