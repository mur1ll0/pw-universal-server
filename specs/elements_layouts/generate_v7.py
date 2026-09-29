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
            #
            # B151 — o corte acima tirava o campo errado. O `gs` 1.2.6 (`generate_weapon<NORMAL>`
            # VA 0x81f4eae, `generate_armor<NORMAL>` 0x81f57a4, `generate_decoration<NORMAL>`
            # 0x81f6026) lê:
            # - arma: `require_level` +0x264, o `level` da arma +0x268 (vai à essência),
            #   `fixed_props` +0x26c (`cmp [ess+0x26c], 0` antes de
            #   `generate_equipment_addon_buffer_2`), dano +0x270; `RandSelect` de **4**
            #   `probability_addon_num` em +0x2c4, `probability_unique` +0x2d4, addons +0x2d8,
            #   rands +0x3d8, **16** únicos +0x4d8, `durability_drop` +0x558;
            # - armadura: `require_level` +0x188, `fixed_props` +0x18c, **4** `addon_num` em
            #   +0x21c, addons +0x22c, rands +0x32c, `durability_drop` +0x42c, `pile_num_max`
            #   +0x444, `has_guid` +0x448, `proc_type` +0x44c (sem `id_hair`/`id_hair_texture`
            #   nem `force_all_magic_defences`);
            # - acessório: `require_level` +0x1e4, `fixed_props` +0x1e8, **4** `addon_num` em
            #   +0x250, addons +0x260, rands +0x360, `durability_drop` +0x460, `pile_num_max`
            #   +0x478, `has_guid` +0x47c, `proc_type` +0x480.
            # Ou seja: no v7 não há `require_reputation` (o que se lia ali era o `level`/o
            # `fixed_props`) nem a 5ª/6ª probabilidade de número de addons. Lido do jeito antigo,
            # a foice 15964 saía com `probability_addon_num5` = 6.6e-43 (o id do 1º addon) e a
            # lista de addons deslocada em um par.
            sem = {
                "require_reputation", "probability_hidden",
                "id_drop_after_damaged", "num_drop_after_damaged",
                "probability_addon_num4", "probability_addon_num5",
                "id_hair", "id_hair_texture", "force_all_magic_defences",
            }
            campos_origem = [c for c in campos_origem if c["name"] not in sem
                             and not c["name"].startswith("hiddens_")]
        elif origem["name"] == "TASKDICE_ESSENCE":
            # gs 1.2.6, itemdataman::generate_taskdice (VA 0x81f08a0): task_lists[i].id
            # em +0x144+8*i, e depois `+0x188`, `has_guid == 1` em +0x18c e `+0x190` —
            # pile_num_max, has_guid e proc_type do fonte 1.5.5 (`gs/template/exptypes.h`,
            # TASKDICE_ESSENCE: task_lists[20], use_on_pick, pile_num_max, has_guid,
            # proc_type). No v7 são 8 listas (0x144..0x184) e `use_on_pick` em +0x184:
            # 404 B. O corte do v156 punha 10 listas e perdia os quatro (B122).
            campos_origem = campos_origem[:4] + [
                c for j in range(1, 9) for c in (
                    {"name": f"task_lists_{j}_id", "type": "int32", "size": 4},
                    {"name": f"task_lists_{j}_probability", "type": "float", "size": 4})
            ] + [c for c in campos_origem if c["name"] in {
                "use_on_pick", "pile_num_max", "has_guid", "proc_type"}]
        elif origem["name"] == "PET_ESSENCE":
            # B111: o `gs` 1.2.6 (`pet_dataman::LoadTemplate`, VA 0x8143580) lê
            # `hp_a`…`magic_defence_d` sem o `pet_snd_type` que o corte do v156 punha em
            # 0x154. A lista é a do `v7.json` corrigido no B111 (antes só no JSON; B122
            # a trouxe para cá para que rodar o gerador não a desfaça).
            campos_origem = [
                {"name": "ID", "type": "int32", "size": 4},
                {"name": "id_type", "type": "int32", "size": 4},
                {"name": "Name", "type": "wstring", "size": 64},
                {"name": "file_model", "type": "string", "size": 128},
                {"name": "file_icon", "type": "string", "size": 128},
                {"name": "character_combo_id", "type": "int32", "size": 4},
                {"name": "level_max", "type": "int32", "size": 4},
                {"name": "level_require", "type": "int32", "size": 4},
                {"name": "hp_a", "type": "float", "size": 4},
                {"name": "hp_b", "type": "float", "size": 4},
                {"name": "hp_c", "type": "float", "size": 4},
                {"name": "hp_gen_a", "type": "float", "size": 4},
                {"name": "hp_gen_b", "type": "float", "size": 4},
                {"name": "hp_gen_c", "type": "float", "size": 4},
                {"name": "damage_a", "type": "float", "size": 4},
                {"name": "damage_b", "type": "float", "size": 4},
                {"name": "damage_c", "type": "float", "size": 4},
                {"name": "damage_d", "type": "float", "size": 4},
                {"name": "speed_a", "type": "float", "size": 4},
                {"name": "speed_b", "type": "float", "size": 4},
                {"name": "attack_a", "type": "float", "size": 4},
                {"name": "attack_b", "type": "float", "size": 4},
                {"name": "attack_c", "type": "float", "size": 4},
                {"name": "armor_a", "type": "float", "size": 4},
                {"name": "armor_b", "type": "float", "size": 4},
                {"name": "armor_c", "type": "float", "size": 4},
                {"name": "physic_defence_a", "type": "float", "size": 4},
                {"name": "physic_defence_b", "type": "float", "size": 4},
                {"name": "physic_defence_c", "type": "float", "size": 4},
                {"name": "physic_defence_d", "type": "float", "size": 4},
                {"name": "magic_defence_a", "type": "float", "size": 4},
                {"name": "magic_defence_b", "type": "float", "size": 4},
                {"name": "magic_defence_c", "type": "float", "size": 4},
                {"name": "magic_defence_d", "type": "float", "size": 4},
                {"name": "size", "type": "float", "size": 4},
                {"name": "damage_delay", "type": "float", "size": 4},
                {"name": "attack_range", "type": "float", "size": 4},
                {"name": "attack_speed", "type": "float", "size": 4},
                {"name": "sight_range", "type": "int32", "size": 4},
                {"name": "food_mask", "type": "int32", "size": 4},
                {"name": "inhabit_type", "type": "int32", "size": 4},
                {"name": "unk", "type": "int32", "size": 4},
            ]
        elif origem["name"] == "MONSTER_ESSENCE":
            # No monstro 986, `common_strategy` = 60 em +744 e as chances
            # de drop começam em +1220. O v7 ainda não tem os campos
            # `attack_degree`/`defend_degree` do v156; termina após
            # drop_matters[32]. +736/+740 seguem como velocidades float.
            campos_origem = [c for c in campos_origem if c["name"] not in {
                "attack_degree", "defend_degree",
            }]
        elif origem["name"] == "RECIPE_ESSENCE":
            # B145 — `recipe_manager::LoadTemplate` do `gs` 1.2.6 (VA 0x80f0a12) lê
            # `recipe_level` +0x4c, `id_skill` +0x50, `skill_level` +0x54, os alvos a partir de
            # +0x58, `fail_probability` +0x78, `num_to_make` +0x7c, `price` +0x80, `duration`
            # +0x84 (×20), `exp` +0x88, `skillpoint` +0x8c e os 32 materiais de +0x90 a +0x190:
            # **sem `bind_type`** (o v156 o tem em +0x58) e sem o rabo de melhoria.
            campos_origem = [c for c in campos_origem if c["name"] not in {
                "bind_type", "id_upgrade_equip", "upgrade_rate", "proc_type",
                "character_combo_id", "upgrade_engrave_rate", "upgrade_addon_rate",
            }]
        elif origem["name"] == "NPC_MAKE_SERVICE":
            # B145 — sem `produce_type` no v7 (1224 = 72 + 8 × 144): lido como v156, o
            # `produce_type` saía 0x00730045, o texto UTF-16 do título da primeira página, e
            # cada receita deslocada de uma posição.
            campos_origem = [c for c in campos_origem if c["name"] != "produce_type"]
        elif origem["name"] == "CUSTOMIZEDATA_ESSENCE":
            # B143 — sem `file_icon` no v7: os 8 B finais são `character_combo_id` (1, 2, 8,
            # 64, 128, 192 nos 140 registros do `realm_126`) e `gender_id` (0/1).
            campos_origem = [c for c in campos_origem if c["name"] != "file_icon"]
        elif origem["name"] == "PLAYER_ACTION_INFO_CONFIG":
            # B143 — 11 sufixos de arma (e não 15) e `hide_weapon` nos 4 B finais: 132 + 11×32
            # + 4 = 488; `hide_weapon` 0 em 512 registros e 1 em 61.
            campos_origem = [c for c in campos_origem if not any(
                c["name"] == f"action_weapon_suffix_{k}_suffix" for k in range(12, 16))]
        elif origem["name"] == "FACEPILL_ESSENCE":
            # B143 — 16 arquivos `pllfiles` (e não 24) e o rabo do fonte (`exptypes.h:2151-2159`):
            # `price`, `shop_price`, `pile_num_max`, `has_guid`, `proc_type` — (500, 2000, 10,
            # 0, 0) em 3 dos 5 registros.
            campos_origem = [c for c in campos_origem if not any(
                c["name"] == f"pllfiles_{k}_file" for k in range(17, 25))]
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
            # B122 — os 48 B que eram `_opaco` em +0x194: no fonte 1.5.5
            # (`gs/template/exptypes.h`, MINE_ESSENCE) vêm `npcgen[4]`, `aggros[1]
            # {monster_faction, radius, num}` e `permenent`. No v7 são `npcgen[3]` +
            # `aggros[1]` + `permenent` (48 + 12 + 4): lidos como `npcgen_4`, os 3 registros
            # não nulos davam monstro inexistente e `num` = 0x43480000 (200.0 em float, o
            # `radius` do `aggros`).
            campos_origem += [c for c in origem["fields"] if c["name"] in {
                "num1", "probability1", "num2", "probability2",
                "task_in", "task_out", "uninterruptable",
            } or c["name"].startswith(("npcgen_1_", "npcgen_2_", "npcgen_3_", "aggros_1_"))
                or c["name"] == "permenent"]
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
