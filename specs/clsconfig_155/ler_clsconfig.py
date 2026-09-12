"""Lê os moldes de classe (roles 16..31) do `clsconfig` do gamedbd 1.5.5 original.

O `gamedbd` importa este arquivo na partida (`cnet/gamedbd/clsconfig.h::ImportClsConfig`):
para cada role 16..31, um `GRoleTableClsconfig` = `version` + `GRoleBase` + `GRoleStatus`
+ inventário + equipamento + armazém (`cnet/rpcdata/groletableclsconfig`), gravado sem
compressão (`NullCoder`) no formato de página do banco. O Marshal do GNET é big-endian e
`Octets` é `compact_uint` + bytes.

Em vez de interpretar as páginas do banco, acha cada registro pelo nome do molde
(`cls<N>gender<M>`, UTF-16LE) e lê `GRoleBase` e o começo de `GRoleStatus` a partir dali.

Uso: python ler_clsconfig.py F:/PW/1.5.5/home155/gamedbd/clsconfig
Ver o item 47 do docs/ESTADO_E_RETOMADA.md.
"""
import re, struct, sys

d = open(sys.argv[1], "rb").read()

class R:
    def __init__(s, o): s.o = o
    def u8(s): v = d[s.o]; s.o += 1; return v
    def i32(s): v = struct.unpack_from(">i", d, s.o)[0]; s.o += 4; return v
    def u32(s): v = struct.unpack_from(">I", d, s.o)[0]; s.o += 4; return v
    def u16(s): v = struct.unpack_from(">H", d, s.o)[0]; s.o += 2; return v
    def f(s): v = struct.unpack_from(">f", d, s.o)[0]; s.o += 4; return v
    def cu(s):
        b = d[s.o]
        if b < 0x80: s.o += 1; return b
        if b < 0xC0: v = struct.unpack_from(">H", d, s.o)[0] & 0x3FFF; s.o += 2; return v
        if b < 0xE0: v = struct.unpack_from(">I", d, s.o)[0] & 0x1FFFFFFF; s.o += 4; return v
        s.o += 1; return s.u32()
    def oct(s):
        n = s.cu(); v = d[s.o:s.o + n]; s.o += n; return v

vistos = set()
for m in re.finditer(rb"c\x00l\x00s\x00(\d)\x00(?:(\d)\x00)?g\x00e\x00n\x00d\x00e\x00r\x00(\d)\x00", d):
    ini = m.start()
    nome_len = (m.end() - ini)
    # recua: name Octets (cu 1 byte) <- id (4) <- version (1) <- version da tabela (1)
    o = ini - 1 - 4 - 1
    r = R(o)
    try:
        ver = r.u8(); rid = r.u32(); nome = r.oct().decode("utf-16le")
        if not (16 <= rid < 32) or rid in vistos:
            continue
        race = r.i32(); cls = r.i32(); gender = r.u8()
        custom = r.oct(); config = r.oct()
        stamp = r.u32(); status = r.u8(); dt = r.i32(); ct = r.i32(); ll = r.i32()
        nforbid = r.cu()
        for _ in range(nforbid):
            r.u8(); r.i32(); r.i32(); r.oct()
        help_states = r.oct(); spouse = r.u32(); userid = r.u32(); cross = r.oct(); r.u8(); r.u8(); r.u8()
        # GRoleStatus
        sver = r.u8(); level = r.i32(); level2 = r.i32(); exp = r.i32(); sp = r.i32(); pp = r.i32(); hp = r.i32(); mp = r.i32()
        x, y, z = r.f(), r.f(), r.f(); wt = r.i32()
        vistos.add(rid)
        print(f"role {rid} {nome:14} race {race} cls {cls} gender {gender} | lv {level} hp {hp} mp {mp} pp {pp} | pos ({x:.1f}, {y:.1f}, {z:.1f}) mundo {wt} | custom {len(custom)}b config {len(config)}b")
    except Exception as e:
        print("falhou em", ini, e)
print(len(vistos), "moldes")
