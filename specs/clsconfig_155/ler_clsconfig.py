"""Lê os moldes de classe (roles 16..31) do `clsconfig` do gamedbd 1.5.5 original.

O `gamedbd` importa este arquivo na partida (`cnet/gamedbd/clsconfig.h::ImportClsConfig`):
para cada role 16..31, um `GRoleTableClsconfig` = `version` + `GRoleBase` + `GRoleStatus`
+ inventário + equipamento + armazém (`cnet/rpcdata/groletableclsconfig`), gravado sem
compressão (`NullCoder`) no formato de página do banco. O Marshal do GNET é big-endian e
`Octets` é `compact_uint` + bytes.

Em vez de interpretar as páginas do banco, acha cada registro pelo nome do molde
(`cls<N>gender<M>`, UTF-16LE) e lê `GRoleBase` e o começo de `GRoleStatus` a partir dali.

Uso: python ler_clsconfig.py F:/PW/1.5.5/home155/gamedbd/clsconfig
     python ler_clsconfig.py <clsconfig> --sql <realm_id>   (gera o SQL dos moldes)

Com `--sql`, sai um `UPDATE class_templates` por classe com a posição de nascimento
(`GRoleStatus` x/y/z e `worldtag`) e a configuração do cliente (`GRoleBase.config_data`:
barras de atalho, janelas, opções) do molde que o `gamedbd` usa para a classe —
`GetDataRoleId` (`cnet/gamedbd/gamedbmanager.cpp:208`): 0→16, 1→19, 2→20, 3→23, 4→24, 5→27,
6→28, 7→31, 8→18, 9→17, 10→21, 11→22. Só classes cujo molde está no mundo (`worldtag` ≠ 0).
No 1.2.6 o mesmo mapa vale para 0–7 (`GNET::GetDataRoleId`, `gamedbd` 1.2.6 VA 0x810c542;
acima de 7 cai no 16): use `--classes 0,1,3,4,6,7`, as classes que o cliente 1.2.6 cria.
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
fim_do_status = {}
moldes = {}
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
        moldes[rid] = (cls, x, y, z, wt, config)
        fim_do_status[rid] = r.o
        print(f"role {rid} {nome:14} race {race} cls {cls} gender {gender} | lv {level} hp {hp} mp {mp} pp {pp} | pos ({x:.4f}, {y:.4f}, {z:.4f}) mundo {wt} | custom {len(custom)}b config {len(config)}b")
    except Exception as e:
        print("falhou em", ini, e)
print(len(vistos), "moldes")

# `--habilidades`: o `GRoleStatus.skills` de cada molde no mundo, no formato de
# `SkillWrapper::StoreDatabase` (`cskill/skill/skillwrapper.cpp:870-879`): Octets com contador
# e (id, ability, level), little-endian. Os campos entre `worldtag` e `skills` mudam entre
# versões, então o bloco é achado pelo formato: o primeiro Octets de 4 + 12·n bytes cujo
# contador é n e cujas entradas são id < 5000 e nível 0..10 (B108, 1.2.6: +873 bytes).
if "--habilidades" in sys.argv:
    for rid in sorted(fim_do_status):
        cls, _, _, _, wt, _ = moldes[rid]
        if not wt:
            continue
        achou = None
        for o in range(fim_do_status[rid], fim_do_status[rid] + 4000):
            n4 = d[o]
            if n4 < 16 or n4 >= 0x80 or (n4 - 4) % 12:
                continue
            n = (n4 - 4) // 12
            if struct.unpack_from("<I", d, o + 1)[0] != n:
                continue
            sk = [struct.unpack_from("<Iii", d, o + 5 + 12 * i) for i in range(n)]
            if all(0 < s[0] < 5000 and 0 <= s[2] <= 10 for s in sk):
                achou = (o - fim_do_status[rid], [(s[0], s[2]) for s in sk])
                break
        print(f"habilidades role {rid} cls {cls}: " + (f"(+{achou[0]}) " + ", ".join(f"{i} nv {l}" for i, l in achou[1]) if achou else "não achadas"))

MOLDE_DA_CLASSE = {0: 16, 1: 19, 2: 20, 3: 23, 4: 24, 5: 27, 6: 28, 7: 31, 8: 18, 9: 17, 10: 21, 11: 22}
if "--sql" in sys.argv:
    realm = sys.argv[sys.argv.index("--sql") + 1]
    print(f"-- Gerado por: python specs/clsconfig_155/ler_clsconfig.py {sys.argv[1]} --sql {realm}")
    print("BEGIN;")
    so = None
    if "--classes" in sys.argv:
        so = {int(c) for c in sys.argv[sys.argv.index("--classes") + 1].split(",")}
    for cls, rid in sorted(MOLDE_DA_CLASSE.items()):
        if so is not None and cls not in so:
            continue
        m = moldes.get(rid)
        if not m or m[4] == 0:
            continue
        _, x, y, z, wt, config = m
        ui = f"decode('{config.hex()}', 'hex')" if config else "NULL"
        print(f"UPDATE class_templates SET spawn_world_id = {wt}, spawn_x = {x:.4f}, spawn_y = {y:.4f}, "
              f"spawn_z = {z:.4f}, ui_config = {ui}, updated_at = CURRENT_TIMESTAMP "
              f"WHERE realm_id = '{realm}' AND cls = {cls};  -- molde {rid}")
    print("COMMIT;")
