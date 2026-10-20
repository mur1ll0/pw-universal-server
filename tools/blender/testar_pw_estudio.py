"""Teste da extensão PW Estúdio no Blender, sem janela (B300).

Uso (com a extensão instalada — `blender --command extension install-file -r user_default -e pw_estudio-*.zip`):

    blender --background --python tools/blender/testar_pw_estudio.py -- <pasta>

`<pasta>` tem os GLB baixados do painel ("Baixar GLB", de preferência com uma ação) e um `lista.json`
(`[{"arquivo": "x.glb"}, ...]`). Para cada um: importa a 15 quadros/s pela extensão, limita a 4 ossos, verifica e
exporta `blender_<arquivo>` pela extensão. Depois, `PW_B300=<pasta> cargo test -p pw-data-loader --lib medir_blender
-- --ignored --nocapture` mede o que mudou (esqueleto, ganchos, malhas, ação).
"""
import bpy, os, sys, json

pasta = sys.argv[sys.argv.index("--") + 1]
assert hasattr(bpy.ops.pw_estudio, "importar"), "a extensão PW Estúdio não está instalada/habilitada"
for item in json.load(open(os.path.join(pasta, "lista.json"), encoding="utf-8")):
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.ops.pw_estudio.importar(filepath=os.path.join(pasta, item["arquivo"]), fps='15')
    for o in bpy.context.scene.objects:
        o.select_set(True)
    malhas = [o for o in bpy.context.scene.objects if o.type == 'MESH' and o.vertex_groups]
    if malhas:
        bpy.context.view_layer.objects.active = malhas[0]
        bpy.ops.pw_estudio.limitar()
    bpy.ops.pw_estudio.verificar()
    saida = os.path.join(pasta, "blender_" + item["arquivo"])
    r = bpy.ops.pw_estudio.exportar('EXEC_DEFAULT', filepath=saida)
    print("RES", item["arquivo"], r, bpy.context.scene.pw_estudio_resultado.replace("\n", " | ")[:300])
