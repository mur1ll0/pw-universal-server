"""PW Estúdio — extensão do Blender (4.2+) para o Estúdio do pw-universal-server (B300, fase S6).

Verifica um modelo contra o que o importador do Estúdio (`crates/pw-data-loader/src/importar.rs`) e o formato do jogo
aceitam, corrige o que dá para corrigir sem decidir pelo artista, cria ganchos e exporta o GLB com as opções que o
Estúdio espera. As regras vêm do formato, não de gosto:

- vértices por malha: o índice do `.ski` é `u16` (até 65.536; `A3DSkinMesh`/`A3DRigidMesh`);
- até 4 ossos por vértice (`A3DBVERTEX3`: 3 pesos + 4 índices, `A3DVertex.h:184-191`);
- até 255 ossos de pele por malha (o índice do vértice é um byte);
- nomes (ossos, malhas, materiais, imagens, ações) em GBK — o cliente lê os arquivos em GBK;
- malha sem pele precisa estar presa a um osso (vira malha rígida desse osso — B294);
- textura PNG (o Estúdio converte para DXT); JPEG ainda não é aceito (B292);
- ganchos: vazios `HH_*` (ponto de pendurar) / `CC_*` (centro de encaixe) presos a um osso (`A3DSkeleton.h:62-63`);
- quadros por segundo da cena: 15 ou 30, como as trilhas do jogo (15 em 83% das `.stck` do cliente). O Estúdio grava
  uma chave por quadro no FPS do esqueleto; a cena em 24/s faz o Blender reamostrar fora da grade — medido (B300) num
  modelo real exportado pelo painel e devolvido pelo Blender: rotação com erro até 5e-2 a 24/s, até 1,4e-4 a 15/s.
"""

import json
import struct

import bpy

# B380: o Estúdio exporta cada animação com o nome legível ("Parado · 站立") e o nome do cliente em
# `extras.angelica.nome`; o importador glTF do Blender não leva os `extras` da animação para a ação, então "Importar GLB
# do Estúdio" os grava na ação (`acao["angelica"]["nome"]`), e o exportador glTF devolve as propriedades da ação como
# `extras` (`export_extras`). O Estúdio lê primeiro `extras.angelica.nome`, depois o que vem após o último " · ".
SEPARADOR_DO_NOME = " · "
PREFIXOS_DE_GANCHO = ("HH_", "CC_")
MAX_VERTICES = 65536
MAX_INFLUENCIAS = 4
MAX_OSSOS_DA_PELE = 255
FPS_DO_JOGO = (15, 30)


def cabe_em_gbk(texto):
    try:
        texto.encode("gbk")
        return True
    except UnicodeEncodeError:
        return False


def nome_original(acao):
    """O nome da ação no jogo (o que o cliente procura): o guardado na importação, senão o que vem depois de " · "."""
    a = acao.get("angelica") if hasattr(acao, "get") else None
    if a is not None and hasattr(a, "get") and a.get("nome"):
        return str(a.get("nome"))
    return acao.name.rsplit(SEPARADOR_DO_NOME, 1)[-1].strip()


def nomes_das_animacoes(caminho):
    """(nome glTF, nome original) de cada animação do GLB que tem `extras.angelica.nome`."""
    with open(caminho, "rb") as f:
        dados = f.read()
    if dados[:4] != b"glTF":
        return []
    n = struct.unpack_from("<I", dados, 12)[0]
    doc = json.loads(dados[20:20 + n].decode("utf-8"))
    r = []
    for a in doc.get("animations", []):
        orig = ((a.get("extras") or {}).get("angelica") or {}).get("nome")
        if orig and a.get("name"):
            r.append((a["name"], orig))
    return r


def armadura_de(obj):
    """A armadura que deforma a malha (modificador Armature) ou a mãe dela."""
    for m in obj.modifiers:
        if m.type == 'ARMATURE' and m.object:
            return m.object
    if obj.parent and obj.parent.type == 'ARMATURE':
        return obj.parent
    return None


def verificar(objetos):
    """Lista de (nível, mensagem): 'ERRO' impede a importação no Estúdio; 'AVISO' importa, mas com perda."""
    r = []
    # Formas de desenho de osso (o importador glTF do Blender cria uma "Icosphere" para isso) não são do modelo — e o
    # exportador não as leva como malha.
    formas = {pb.custom_shape for a in bpy.data.objects if a.type == 'ARMATURE' for pb in a.pose.bones if pb.custom_shape}
    malhas = [o for o in objetos if o.type == 'MESH' and o not in formas]
    armaduras = {o for o in objetos if o.type == 'ARMATURE'}
    armaduras.update(a for a in (armadura_de(m) for m in malhas) if a)
    if not malhas:
        r.append(('ERRO', "Nenhuma malha selecionada."))
    for a in armaduras:
        for b in a.data.bones:
            if not cabe_em_gbk(b.name):
                r.append(('ERRO', f"Osso \"{b.name}\": nome fora do GBK."))
    for m in malhas:
        if not cabe_em_gbk(m.name):
            r.append(('ERRO', f"Malha \"{m.name}\": nome fora do GBK."))
        dados = m.data
        # O glTF duplica vértice por quina com normal/UV diferente: o número final é o de quinas distintas, limitado
        # pelo de quinas (loops) — aqui o pior caso.
        if len(dados.loops) > MAX_VERTICES:
            r.append(('AVISO', f"Malha \"{m.name}\": até {len(dados.loops)} vértices no GLB (o .ski aceita {MAX_VERTICES}); divida a malha se a exportação acusar."))
        arm = armadura_de(m)
        if arm is None:
            if not (m.parent and m.parent.type == 'ARMATURE' and m.parent_type == 'BONE' and m.parent_bone):
                r.append(('ERRO', f"Malha \"{m.name}\": sem armadura e sem osso como mãe — o Estúdio não sabe a que osso prendê-la."))
        else:
            nomes_ossos = {b.name for b in arm.data.bones}
            grupos = {g.index: g.name for g in m.vertex_groups if g.name in nomes_ossos}
            mais, sem, usados = 0, 0, set()
            for v in dados.vertices:
                pesos = [g for g in v.groups if g.group in grupos and g.weight > 0.0]
                if len(pesos) > MAX_INFLUENCIAS:
                    mais += 1
                if not pesos:
                    sem += 1
                usados.update(grupos[g.group] for g in pesos)
            if mais:
                r.append(('AVISO', f"Malha \"{m.name}\": {mais} vértice(s) com mais de {MAX_INFLUENCIAS} ossos — use \"Limitar a 4 ossos\" (senão o glTF corta e renormaliza)."))
            if sem and m.parent_type != 'BONE':
                r.append(('ERRO', f"Malha \"{m.name}\": {sem} vértice(s) sem peso em nenhum osso."))
            if len(usados) > MAX_OSSOS_DA_PELE:
                r.append(('ERRO', f"Malha \"{m.name}\": {len(usados)} ossos com peso (o .ski aceita {MAX_OSSOS_DA_PELE} por pele)."))
        for slot in m.material_slots:
            mat = slot.material
            if not mat:
                continue
            if not cabe_em_gbk(mat.name):
                r.append(('ERRO', f"Material \"{mat.name}\": nome fora do GBK."))
            if mat.use_nodes:
                for n in mat.node_tree.nodes:
                    if n.type == 'TEX_IMAGE' and n.image:
                        img = n.image
                        if not cabe_em_gbk(img.name):
                            r.append(('ERRO', f"Imagem \"{img.name}\": nome fora do GBK."))
                        if (img.file_format or "").upper() in ("JPEG", "JPG"):
                            r.append(('AVISO', f"Imagem \"{img.name}\" é JPEG: o Estúdio ainda não converte JPEG — salve como PNG."))
    cena = bpy.context.scene
    fps = cena.render.fps / (cena.render.fps_base or 1.0)
    if bpy.data.actions and round(fps, 3) not in FPS_DO_JOGO:
        r.append(('AVISO', f"A cena está a {fps:g} quadros/s; o jogo usa 15 ou 30 — a exportação reamostra fora da grade e a animação perde precisão (use \"Importar GLB do Estúdio\" ou mude o FPS antes de animar)."))
    for a in bpy.data.actions:
        # B380: o que vai para o jogo é o nome original (o legível "Parado · 站立" é só para ler).
        nome = nome_original(a)
        if not cabe_em_gbk(nome):
            r.append(('ERRO', f"Ação \"{a.name}\": o nome \"{nome}\" fica fora do GBK."))
        if any(c in nome for c in "\\/"):
            r.append(('ERRO', f"Ação \"{a.name}\": o nome vira o arquivo <ação>.stck — sem barras."))
    for o in objetos:
        if o.type == 'EMPTY' and o.name.startswith(PREFIXOS_DE_GANCHO) and not (o.parent and o.parent_type == 'BONE'):
            r.append(('AVISO', f"Gancho \"{o.name}\": não está preso a um osso — vai preso ao esqueleto (osso -1)."))
    return r


class PWE_OT_verificar(bpy.types.Operator):
    bl_idname = "pw_estudio.verificar"
    bl_label = "Verificar para o Perfect World"
    bl_description = "Confere a seleção contra o que o Estúdio e o formato do jogo aceitam"

    def execute(self, context):
        objetos = context.selected_objects or list(context.scene.objects)
        r = verificar(objetos)
        context.scene.pw_estudio_resultado = "\n".join(f"{n}: {m}" for n, m in r) or "OK: nada a corrigir."
        erros = sum(1 for n, _ in r if n == 'ERRO')
        self.report({'ERROR' if erros else ('WARNING' if r else 'INFO')}, f"{erros} erro(s), {len(r) - erros} aviso(s)" if r else "OK")
        return {'FINISHED'}


class PWE_OT_importar(bpy.types.Operator):
    bl_idname = "pw_estudio.importar"
    bl_label = "Importar GLB do Estúdio"
    bl_description = "Acerta a cena para o FPS do jogo e importa o GLB baixado do painel (as chaves caem nos quadros inteiros)"
    bl_options = {'REGISTER', 'UNDO'}

    filepath: bpy.props.StringProperty(subtype='FILE_PATH')
    filter_glob: bpy.props.StringProperty(default="*.glb", options={'HIDDEN'})
    fps: bpy.props.EnumProperty(name="Quadros por segundo", items=[('15', "15 (83% do cliente)", ""), ('30', "30", "")], default='15')

    def execute(self, context):
        context.scene.render.fps = int(self.fps)
        context.scene.render.fps_base = 1.0
        antes = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=self.filepath)
        # B380: o nome do cliente de cada animação vai para a ação (o importador glTF não leva os extras dela).
        novas = [a for a in bpy.data.actions if a not in antes]
        guardados = 0
        for nome, orig in nomes_das_animacoes(self.filepath):
            # O Blender corta nome comprido (63 bytes até o 4.x): casa pelo começo.
            for a in novas:
                if a.name == nome or (len(a.name) >= 20 and nome.startswith(a.name)):
                    a["angelica"] = {"nome": orig}
                    a.use_fake_user = True
                    guardados += 1
                    break
        self.report({'INFO'}, f"Importado a {self.fps} quadros/s ({guardados} ações com o nome do cliente): {self.filepath}")
        return {'FINISHED'}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


class PWE_OT_limitar(bpy.types.Operator):
    bl_idname = "pw_estudio.limitar"
    bl_label = "Limitar a 4 ossos e normalizar"
    bl_description = "Em cada malha selecionada com armadura: no máximo 4 ossos por vértice, pesos somando 1"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, context):
        feitas = 0
        ativo = context.view_layer.objects.active
        for m in [o for o in context.selected_objects if o.type == 'MESH' and armadura_de(o) and o.vertex_groups]:
            context.view_layer.objects.active = m
            bpy.ops.object.vertex_group_limit_total(group_select_mode='BONE_DEFORM', limit=MAX_INFLUENCIAS)
            bpy.ops.object.vertex_group_normalize_all(group_select_mode='BONE_DEFORM', lock_active=False)
            feitas += 1
        context.view_layer.objects.active = ativo
        self.report({'INFO'}, f"{feitas} malha(s) ajustada(s)")
        return {'FINISHED'}


class PWE_OT_gancho(bpy.types.Operator):
    bl_idname = "pw_estudio.gancho"
    bl_label = "Novo gancho no osso ativo"
    bl_description = "Cria um vazio HH_/CC_ preso ao osso ativo (ponto de pendurar ou centro de encaixe)"
    bl_options = {'REGISTER', 'UNDO'}

    nome: bpy.props.StringProperty(name="Nome", default="HH_novo")

    def execute(self, context):
        arm = context.active_object
        if not arm or arm.type != 'ARMATURE' or not arm.data.bones.active:
            self.report({'ERROR'}, "Selecione a armadura com um osso ativo")
            return {'CANCELLED'}
        if not self.nome.startswith(PREFIXOS_DE_GANCHO) or not cabe_em_gbk(self.nome):
            self.report({'ERROR'}, "O nome começa com HH_ ou CC_ e cabe em GBK")
            return {'CANCELLED'}
        osso = arm.data.bones.active
        g = bpy.data.objects.new(self.nome, None)
        g.empty_display_type = 'ARROWS'
        g.empty_display_size = 0.1
        context.collection.objects.link(g)
        g.parent = arm
        g.parent_type = 'BONE'
        g.parent_bone = osso.name
        # Na ponta da cabeça do osso; a matriz relativa ao osso vira a do gancho (`A3DSkeleton.cpp:162-187`).
        g.matrix_world = arm.matrix_world @ osso.matrix_local
        g["angelica"] = {"gancho": {"tipo": 1 if self.nome.startswith("CC_") else 0}}
        self.report({'INFO'}, f"Gancho {self.nome} preso a {osso.name}")
        return {'FINISHED'}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


class PWE_OT_exportar(bpy.types.Operator):
    bl_idname = "pw_estudio.exportar"
    bl_label = "Exportar GLB para o Estúdio"
    bl_description = "Verifica e exporta a seleção como GLB com as opções que o Estúdio espera"

    filepath: bpy.props.StringProperty(subtype='FILE_PATH')
    filter_glob: bpy.props.StringProperty(default="*.glb", options={'HIDDEN'})

    def execute(self, context):
        objetos = context.selected_objects or list(context.scene.objects)
        r = verificar(objetos)
        erros = [m for n, m in r if n == 'ERRO']
        if erros:
            context.scene.pw_estudio_resultado = "\n".join(f"{n}: {m}" for n, m in r)
            self.report({'ERROR'}, f"{len(erros)} erro(s): veja o painel PW Estúdio")
            return {'CANCELLED'}
        caminho = self.filepath if self.filepath.lower().endswith(".glb") else self.filepath + ".glb"
        exportar(caminho, usar_selecao=bool(context.selected_objects))
        context.scene.pw_estudio_resultado = "\n".join(f"{n}: {m}" for n, m in r) or f"Exportado: {caminho}"
        self.report({'INFO'}, f"Exportado: {caminho}")
        return {'FINISHED'}

    def invoke(self, context, event):
        self.filepath = bpy.path.ensure_ext(bpy.data.filepath or "modelo", ".glb")
        context.window_manager.fileselect_add(self)
        return {'RUNNING_MODAL'}


def exportar(caminho, usar_selecao=False):
    """O GLB que o Estúdio importa: binário, Y para cima, com pele (4 ossos), ações separadas e reamostradas, extras (os
    valores do Angelica que o painel grava em `extras.angelica` voltam iguais), sem morph (o `.ski` não tem — B272),
    sem luz nem câmera."""
    bpy.ops.export_scene.gltf(
        filepath=caminho, export_format='GLB', use_selection=usar_selecao, export_extras=True, export_yup=True,
        export_apply=False, export_skins=True, export_influence_nb=MAX_INFLUENCIAS, export_all_influences=False,
        export_def_bones=False, export_animations=True, export_animation_mode='ACTIONS', export_force_sampling=True,
        export_morph=False, export_texcoords=True, export_normals=True, export_tangents=False,
        export_materials='EXPORT', export_image_format='AUTO', export_lights=False, export_cameras=False,
    )


class PWE_PT_painel(bpy.types.Panel):
    bl_label = "PW Estúdio"
    bl_idname = "PWE_PT_painel"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = "PW Estúdio"

    def draw(self, context):
        c = self.layout.column(align=True)
        c.operator(PWE_OT_importar.bl_idname, icon='IMPORT')
        c.operator(PWE_OT_verificar.bl_idname, icon='CHECKMARK')
        c.operator(PWE_OT_limitar.bl_idname, icon='MOD_VERTEX_WEIGHT')
        c.operator(PWE_OT_gancho.bl_idname, icon='EMPTY_ARROWS')
        c.operator(PWE_OT_exportar.bl_idname, icon='EXPORT')
        texto = context.scene.pw_estudio_resultado
        if texto:
            caixa = self.layout.box()
            for linha in texto.split("\n")[:30]:
                caixa.label(text=linha)


CLASSES = (PWE_OT_importar, PWE_OT_verificar, PWE_OT_limitar, PWE_OT_gancho, PWE_OT_exportar, PWE_PT_painel)


def register():
    for c in CLASSES:
        bpy.utils.register_class(c)
    bpy.types.Scene.pw_estudio_resultado = bpy.props.StringProperty(default="")


def unregister():
    del bpy.types.Scene.pw_estudio_resultado
    for c in reversed(CLASSES):
        bpy.utils.unregister_class(c)
