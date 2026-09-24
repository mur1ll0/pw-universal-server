"""Gera o catálogo v7 a partir da ordem do carregador e dos tamanhos medidos.

Ordem: source_client_153/CCommon/elementdataman.cpp:3611-3800 e
source_client_153/CCommon/ExpTypes.h:4930-5000. O carregador binário do
cliente 1.2.6 (`F:/Games/perfectworld_126/element/elementclient.exe`,
VA 0x60ff5d-0x60ffb0) confirma versão 0x30000007, contagem u32 imediatamente
após a versão e registro de 0x54 bytes da primeira tabela. As leituras
seguintes usam 68 B (VA 0x610012), 356 B (0x61007b) e 1404 B (0x6100e4).
Tamanhos e nomes: `ORDEM`, tirada do gs 1.2.6 (ver abaixo); o arquivo
data/realm_126/config/elements.data fecha no último byte com eles.
Campos partem do catálogo v156 apenas onde cabem no registro; qualquer trecho
cuja semântica não foi conferida deve ser marcado opaco no JSON.
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
# Ordem e `sizeof(T)` de cada tabela de registro fixo, extraídos do servidor
# 1.2.6 (files1.2.6/pwserver/gamed/gs, ELF 32-bit com símbolos): a sequência de
# chamadas `elementdataman::array<T>::load` em `elementdataman::load_data`
# (VA 0x81b1df4, que exige a versão 0x30000007 em 0x81b1e3d) e o imediato de
# tamanho em cada `array<T>::load`. TALK_PROC fica entre NPC_ESSENCE e
# FACE_TEXTURE_ESSENCE. Até o B94 as tabelas 98-112 usavam tamanhos "medidos"
# que também fechavam o arquivo no último byte, mas com contagens erradas.
ORDEM = [
    ("EQUIPMENT_ADDON", 84), ("WEAPON_MAJOR_TYPE", 68), ("WEAPON_SUB_TYPE", 356),
    ("WEAPON_ESSENCE", 1404), ("ARMOR_MAJOR_TYPE", 68), ("ARMOR_SUB_TYPE", 72),
    ("ARMOR_ESSENCE", 1104), ("DECORATION_MAJOR_TYPE", 68), ("DECORATION_SUB_TYPE", 72),
    ("DECORATION_ESSENCE", 1156), ("MEDICINE_MAJOR_TYPE", 68), ("MEDICINE_SUB_TYPE", 68),
    ("MEDICINE_ESSENCE", 376), ("MATERIAL_MAJOR_TYPE", 68), ("MATERIAL_SUB_TYPE", 68),
    ("MATERIAL_ESSENCE", 368), ("DAMAGERUNE_SUB_TYPE", 68), ("DAMAGERUNE_ESSENCE", 364),
    ("ARMORRUNE_SUB_TYPE", 68), ("ARMORRUNE_ESSENCE", 624), ("SKILLTOME_SUB_TYPE", 68),
    ("SKILLTOME_ESSENCE", 348), ("FLYSWORD_ESSENCE", 516), ("WINGMANWING_ESSENCE", 488),
    ("TOWNSCROLL_ESSENCE", 348), ("UNIONSCROLL_ESSENCE", 348), ("REVIVESCROLL_ESSENCE", 352),
    ("ELEMENT_ESSENCE", 348), ("TASKMATTER_ESSENCE", 208), ("TOSSMATTER_ESSENCE", 888),
    ("PROJECTILE_TYPE", 68), ("PROJECTILE_ESSENCE", 892), ("QUIVER_SUB_TYPE", 68),
    ("QUIVER_ESSENCE", 340), ("STONE_SUB_TYPE", 68), ("STONE_ESSENCE", 436),
    ("MONSTER_ADDON", 84), ("MONSTER_TYPE", 196), ("MONSTER_ESSENCE", 1500),
    ("NPC_TALK_SERVICE", 72), ("NPC_SELL_SERVICE", 1224), ("NPC_BUY_SERVICE", 72),
    ("NPC_REPAIR_SERVICE", 72), ("NPC_INSTALL_SERVICE", 200), ("NPC_UNINSTALL_SERVICE", 200),
    ("NPC_TASK_IN_SERVICE", 196), ("NPC_TASK_OUT_SERVICE", 196), ("NPC_TASK_MATTER_SERVICE", 644),
    ("NPC_SKILL_SERVICE", 584), ("NPC_HEAL_SERVICE", 72), ("NPC_TRANSMIT_SERVICE", 460),
    ("NPC_TRANSPORT_SERVICE", 328), ("NPC_PROXY_SERVICE", 72), ("NPC_STORAGE_SERVICE", 68),
    ("NPC_MAKE_SERVICE", 1224), ("NPC_DECOMPOSE_SERVICE", 72), ("NPC_TYPE", 68),
    ("NPC_ESSENCE", 848), ("FACE_TEXTURE_ESSENCE", 476), ("FACE_SHAPE_ESSENCE", 348),
    ("FACE_EMOTION_TYPE", 196), ("FACE_EXPRESSION_ESSENCE", 336), ("FACE_HAIR_ESSENCE", 468),
    ("FACE_MOUSTACHE_ESSENCE", 340), ("COLORPICKER_ESSENCE", 208), ("CUSTOMIZEDATA_ESSENCE", 204),
    ("RECIPE_MAJOR_TYPE", 68), ("RECIPE_SUB_TYPE", 68), ("RECIPE_ESSENCE", 400),
    ("ENEMY_FACTION_CONFIG", 196), ("CHARRACTER_CLASS_CONFIG", 160), ("PARAM_ADJUST_CONFIG", 612),
    ("PLAYER_ACTION_INFO_CONFIG", 488), ("TASKDICE_ESSENCE", 404), ("TASKNORMALMATTER_ESSENCE", 344),
    ("FACE_FALING_ESSENCE", 340), ("PLAYER_LEVELEXP_CONFIG", 668), ("MINE_TYPE", 68),
    ("MINE_ESSENCE", 452), ("NPC_IDENTIFY_SERVICE", 72), ("FASHION_MAJOR_TYPE", 68),
    ("FASHION_SUB_TYPE", 72), ("FASHION_ESSENCE", 404), ("FACETICKET_MAJOR_TYPE", 68),
    ("FACETICKET_SUB_TYPE", 68), ("FACETICKET_ESSENCE", 488), ("FACEPILL_MAJOR_TYPE", 68),
    ("FACEPILL_SUB_TYPE", 68), ("FACEPILL_ESSENCE", 2412), ("SUITE_ESSENCE", 292),
    ("GM_GENERATOR_TYPE", 68), ("GM_GENERATOR_ESSENCE", 344), ("PET_TYPE", 68),
    ("PET_ESSENCE", 476), ("PET_EGG_ESSENCE", 628), ("PET_FOOD_ESSENCE", 360),
    ("PET_FACETICKET_ESSENCE", 344), ("FIREWORKS_ESSENCE", 480), ("WAR_TANKCALLIN_ESSENCE", 344),
    ("NPC_WAR_TOWERBUILD_SERVICE", 148), ("PLAYER_SECONDLEVEL_CONFIG", 1092), ("NPC_RESETPROP_SERVICE", 368),
    ("NPC_PETNAME_SERVICE", 76), ("NPC_PETLEARNSKILL_SERVICE", 584), ("NPC_PETFORGETSKILL_SERVICE", 76),
    ("SKILLMATTER_ESSENCE", 356), ("REFINE_TICKET_ESSENCE", 436), ("DESTROYING_ESSENCE", 344),
    ("NPC_EQUIPBIND_SERVICE", 76), ("NPC_EQUIPDESTROY_SERVICE", 76), ("NPC_EQUIPUNDESTROY_SERVICE", 76),
    ("BIBLE_ESSENCE", 384), ("SPEAKER_ESSENCE", 348), ("AUTOHP_ESSENCE", 356),
    ("AUTOMP_ESSENCE", 356), ("DOUBLE_EXP_ESSENCE", 348), ("TRANSMITSCROLL_ESSENCE", 344),
    ("DYE_TICKET_ESSENCE", 368),
]


def main() -> None:
    novo = json.loads((ROOT / "v156.json").read_text(encoding="utf-8"))
    por_nome = {t["name"]: t for t in novo["tables"]}
    tabelas = []
    for i, (nome, tamanho) in enumerate(ORDEM):
        origem = por_nome[nome]
        campos_origem = origem["fields"]
        if origem["name"] == "FLYSWORD_ESSENCE":
            # No v7 há três caminhos de modelo: o `file_model2` de 128 B ainda
            # não existe. 452..516 contém preço, nível, velocidades, tempos e
            # máscara de classes; ver bytes do item 2092 do realm.
            campos_origem = [c for c in campos_origem if c["name"] != "file_model2"]
        elif origem["name"] == "NPC_TASK_OUT_SERVICE":
            # O gs 1.2.6 (files1.2.6/pwserver/gamed/gs, ELF com símbolos,
            # npc_stubs_manager::LoadTemplate VA 0x80ef014-0x80ef055) varre
            # id_tasks[i] em +0x44+4*i para i < 32, logo após ID+Name: no v7
            # ainda não existem os 7 campos `storage_*` do v156 (com eles, o
            # serviço 3531 do NPC 3518 saía com storage_id=1177 e
            # storage_open_item=1178, que são as missões 1177/1178).
            campos_origem = campos_origem[:2] + [
                {"name": f"id_tasks_{j}", "type": "int32", "size": 4} for j in range(1, 33)]
        elif origem["name"] == "NPC_SKILL_SERVICE":
            # gs 1.2.6, LoadTemplate VA 0x80ef202-0x80ef22b: id_skills[i] em
            # +0x44+4*i para i < 128 (DT 0x31). Os 4 B finais (584 = 68 +
            # 512 + 4) são o `id_dialog` que fecha a struct no fonte 1.5.3
            # (source_client_153/CCommon/ExpTypes.h, NPC_SKILL_SERVICE).
            campos_origem = campos_origem[:2] + [
                {"name": f"id_skills_{j}", "type": "int32", "size": 4} for j in range(1, 129)
            ] + [c for c in campos_origem if c["name"] == "id_dialog"]
        elif origem["name"] in {"WEAPON_ESSENCE", "ARMOR_ESSENCE", "DECORATION_ESSENCE"}:
            # No item 6, dano 10/18/18 está em +624/+628/+632,
            # attack_range=3.0 em +648 e price/shop_price=120/240 em
            # +672/+676. `fixed_props` e `probability_hidden` do v156
            # deslocariam tudo isso em 8 B. Armadura e ornamento têm o mesmo
            # `fixed_props` inexistente no v7: com ele, a armadura 139 saía com
            # fixed_props=defence_low=552 e `repairfee` como float, e só 18
            # de 1.036 armaduras tinham shop_price coerente com price.
            campos_origem = [c for c in campos_origem if c["name"] not in {
                "fixed_props", "probability_hidden",
                "uniques_16_id_unique", "uniques_16_probability_unique",
                "id_drop_after_damaged", "num_drop_after_damaged",
            } and not c["name"].startswith("hiddens_")]
        elif origem["name"] == "PET_ESSENCE":
            # O Cavalo 8784 grava 8.0/0.1 em +380/+384: `damage_d` do
            # v156 ainda não existe no v7, deslocando speed_a/b em 4 B.
            campos_origem = [c for c in campos_origem if c["name"] != "damage_d"]
        elif origem["name"] == "MONSTER_ESSENCE":
            # No monstro 986, `common_strategy` = 60 em +744 e as chances
            # de drop começam em +1220. O v7 ainda não tem os campos
            # `attack_degree`/`defend_degree` do v156; termina após
            # drop_matters[32]. +736/+740 seguem como velocidades float.
            campos_origem = [c for c in campos_origem if c["name"] not in {
                "attack_degree", "defend_degree",
            }]
        elif origem["name"] == "MINE_ESSENCE":
            # O v7 tem 16 materiais de 8 B (id, probabilidade), sem `life`.
            # No item 6849, o monstro 3360/quantidade 1/raio 1.0 está em
            # +388/+392/+396. Os 48 B finais ainda não foram nomeados.
            campos_origem = [c for c in campos_origem if c["name"] in {
                "ID", "id_type", "Name", "level", "level_required",
                "id_equipment_required", "eliminate_tool", "time_min",
                "time_max", "exp", "skillpoint", "file_model",
            }]
            for j in range(1, 17):
                campos_origem += [
                    {"name": f"materials_{j}_id", "type": "int32", "size": 4},
                    {"name": f"materials_{j}_probability", "type": "float", "size": 4},
                ]
            campos_origem += [c for c in origem["fields"] if c["name"] in {
                "num1", "probability1", "num2", "probability2",
                "task_in", "task_out", "uninterruptable",
                "npcgen_1_id_monster", "npcgen_1_num",
                "npcgen_1_radius", "npcgen_1_life_time",
            }]
        campos = []
        pos = 0
        for campo in campos_origem:
            if pos + campo["size"] > tamanho:
                break
            campos.append(campo.copy())
            pos += campo["size"]
        if pos < tamanho:
            campos.append({"name": "_opaco", "type": "string", "size": tamanho - pos})
        tabelas.append({"index": i if i < 58 else i + 1,
                        "name": origem["name"], "variable_size": False,
                        "record_size": tamanho, "fields": campos})
    tabelas.insert(58, {"index": 58, "name": "TALK_PROC",
                        "variable_size": True, "record_size": None, "fields": []})
    saida = {"format": "pw_elements_data", "version": 7,
             "header": {"size": 4, "family": "hex_build",
                        "fields": [{"name": "version", "type": "uint32"}]},
             "source_order": "files1.2.6/pwserver/gamed/gs: elementdataman::load_data (VA 0x81b1df4)",
             "source_data": "data/realm_126/config/elements.data",
             "table_count": len(tabelas), "tables": tabelas}
    (ROOT / "v7.json").write_text(json.dumps(saida, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
