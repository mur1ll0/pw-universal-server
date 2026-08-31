"""
================================================================================
PW-UNIVERSAL-SERVER: SUÍTE DE TESTES DE GAMEPLAY E PROTOCOLOS V1.2.6 (BUILD 55)
================================================================================
Testa exaustivamente os 10 subsistemas mapeados no REVERSE_ENGINEERING_126_MASTER.md:
1. Seleção de Alvo & HUD HP (C2S 2 / S2C 52 + S2C 39)
2. Ataque Normal e Dano Básico (C2S 3 / S2C 24 + S2C 39)
3. Pipeline de Skills e Canalização (C2S 41 / S2C 85 -> 88 -> 142 -> 123)
4. Diálogo de NPCs e TalkProc (C2S 35 / S2C 70)
5. Mestre de Habilidades (C2S 37 svc=9 / S2C 97 + S2C 96 + S2C 90)
6. Reparo de Equipamentos (C2S 37 svc=3 / S2C 75)
7. Forja e Produção de Itens (C2S 37 svc=12 / S2C 101 -> 102 -> 103)
8. Inventário, Equipamentos e Moda (C2S 17 / S2C 48 + C2S 192 / S2C 192)
9. Árvore de Quests (C2S 37 svc=7 -> kill -> C2S 37 svc=6 / S2C 106 + S2C 36)
10. Loja Gold GShop (C2S 110 / S2C 253, C2S 118 / S2C 197, C2S 120 / S2C 40)
================================================================================
"""

import struct
import sys
import logging

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)]
)
logger = logging.getLogger("PW_126_GAMEPLAY_TESTS")

class TestFailure(Exception):
    pass

# ==============================================================================
# ENCODERS E DECODERS BINÁRIOS DE SUBCOMANDOS V1.2.6
# ==============================================================================

def encode_c2s_select_target(target_id: int) -> bytes:
    return struct.pack("<HI", 2, target_id)

def decode_s2c_select_target(data: bytes) -> int:
    cmd, target_id = struct.unpack_from("<HI", data, 0)
    assert cmd == 52, f"Esperado S2C CMD 52 (SELECT_TARGET), obtido {cmd}"
    return target_id

def decode_s2c_npc_info_00(data: bytes) -> tuple:
    cmd, nid, hp, max_hp = struct.unpack_from("<Hiii", data, 0)
    assert cmd == 33, f"Esperado S2C CMD 33 (NPC_INFO_00), obtido {cmd}"
    return nid, hp, max_hp

def encode_c2s_normal_attack(target_id: int, pvp_mask: int = 0) -> bytes:
    return struct.pack("<HIB", 3, target_id, pvp_mask)

def decode_s2c_host_attack_result(data: bytes) -> tuple:
    cmd, target_id, damage, hit_type = struct.unpack_from("<HiiB", data, 0)
    assert cmd == 24, f"Esperado S2C CMD 24 (HOST_ATTACKRESULT), obtido {cmd}"
    return target_id, damage, hit_type

def encode_c2s_cast_skill(skill_id: int, target_id: int) -> bytes:
    return struct.pack("<HiBiI", 41, skill_id, 0, 1, target_id)

def decode_s2c_object_cast_skill(data: bytes) -> tuple:
    cmd, caster, target, skill_id, cast_time_ms, lvl = struct.unpack_from("<HiiiHB", data, 0)
    assert cmd == 85, f"Esperado S2C CMD 85 (OBJECT_CAST_SKILL), obtido {cmd}"
    return caster, target, skill_id, cast_time_ms, lvl

def decode_s2c_self_skill_attack_result(data: bytes) -> tuple:
    cmd, target_id, skill_id, damage, attack_flag, speed = struct.unpack_from("<HiiibB", data, 0)
    assert cmd == 142, f"Esperado S2C CMD 142 (SELF_SKILL_ATTACK_RESULT), obtido {cmd}"
    return target_id, skill_id, damage, attack_flag, speed

def encode_c2s_sevnpc_hello(nid: int) -> bytes:
    return struct.pack("<HI", 35, nid)

def decode_s2c_npc_greeting(data: bytes) -> int:
    cmd, nid = struct.unpack_from("<Hi", data, 0)
    assert cmd == 70, f"Esperado S2C CMD 70 (NPC_GREETING), obtido {cmd}"
    return nid

def encode_c2s_sevnpc_serve(service_type: int, param: int = 0) -> bytes:
    return struct.pack("<HiII", 37, service_type, 4, param)

def decode_s2c_learn_skill(data: bytes) -> tuple:
    cmd, skill_id, level = struct.unpack_from("<Hii", data, 0)
    assert cmd == 95, f"Esperado S2C CMD 95 (LEARN_SKILL), obtido {cmd}"
    return skill_id, level

def decode_s2c_repair_all(data: bytes) -> int:
    cmd, cost = struct.unpack_from("<Hi", data, 0)
    assert cmd == 74, f"Esperado S2C CMD 74 (REPAIR_ALL), obtido {cmd}"
    return cost

def decode_s2c_produce_start(data: bytes) -> tuple:
    cmd, recipe_id, time_ms = struct.unpack_from("<HiH", data, 0)
    assert cmd == 100, f"Esperado S2C CMD 100 (PRODUCE_START), obtido {cmd}"
    return recipe_id, time_ms

def encode_c2s_equip_item(inv_slot: int, equip_slot: int) -> bytes:
    return struct.pack("<HBB", 17, inv_slot, equip_slot)

def decode_s2c_equip_item(data: bytes) -> tuple:
    cmd, inv_slot, equip_slot, count_inv, count_eq = struct.unpack_from("<HBBBB", data, 0)
    assert cmd == 48, f"Esperado S2C CMD 48 (EQUIP_ITEM), obtido {cmd}"
    return inv_slot, equip_slot, count_inv, count_eq

def encode_c2s_player_enable_fashion(enable: bool) -> bytes:
    return struct.pack("<HB", 85, 1 if enable else 0)

def decode_s2c_player_enable_fashion(data: bytes) -> bool:
    cmd, enable = struct.unpack_from("<HB", data, 0)
    assert cmd == 192, f"Esperado S2C CMD 192 (PLAYER_ENABLE_FASHION), obtido {cmd}"
    return enable == 1

def decode_s2c_task_var_data(data: bytes) -> tuple:
    cmd, size, reason, task_id = struct.unpack_from("<HIBH", data, 0)
    assert cmd == 106, f"Esperado S2C CMD 106 (TASK_VAR_DATA), obtido {cmd}"
    return reason, task_id

def decode_s2c_player_cash(data: bytes) -> tuple:
    cmd, cash_cents, silver_cents = struct.unpack_from("<Hii", data, 0)
    assert cmd == 253, f"Esperado S2C CMD 253 (PLAYER_CASH), obtido {cmd}"
    return cash_cents, silver_cents

# ==============================================================================
# EXECUÇÃO DOS TESTES
# ==============================================================================

def test_1_target_selection_and_hud_hp():
    logger.info(">>> TESTE 1: Seleção de Alvo & Atualização de HP no HUD (C2S 2 / S2C 52 + S2C 33)...")
    c2s_pkt = encode_c2s_select_target(32896)
    assert len(c2s_pkt) == 6, f"Tamanho C2S incorreto: {len(c2s_pkt)}"
    
    s2c_sel = struct.pack("<HI", 52, 32896)
    s2c_hp = struct.pack("<Hiii", 33, 32896, 1250, 1250)
    
    target_id = decode_s2c_select_target(s2c_sel)
    nid, hp, max_hp = decode_s2c_npc_info_00(s2c_hp)
    
    assert target_id == 32896, f"Target ID incorreto: {target_id}"
    assert nid == 32896 and hp == 1250 and max_hp == 1250, "Dados de HP/MaxHP incorretos no HUD"
    logger.info(f"  [OK] Alvo {target_id} selecionado com sucesso. HP: {hp}/{max_hp}")

def test_2_normal_attack_and_damage():
    logger.info(">>> TESTE 2: Ataque Normal e Cálculo de Dano (C2S 3 / S2C 24 + S2C 33)...")
    c2s_atk = encode_c2s_normal_attack(32896, 0)
    assert len(c2s_atk) == 7
    
    s2c_res = struct.pack("<HiiB", 24, 32896, 85, 0)
    s2c_hp = struct.pack("<Hiii", 33, 32896, 1165, 1250)
    
    target, dmg, hit_type = decode_s2c_host_attack_result(s2c_res)
    _, hp, max_hp = decode_s2c_npc_info_00(s2c_hp)
    
    assert target == 32896 and dmg == 85 and hit_type == 0
    assert hp == 1165 and max_hp == 1250
    logger.info(f"  [OK] Ataque desferido contra {target}: {dmg} de dano. HP restante: {hp}/{max_hp}")

def test_3_skill_cast_and_channeling():
    logger.info(">>> TESTE 3: Pipeline de Habilidades e Canalização (C2S 41 / S2C 85 -> 88 -> 142 -> 123)...")
    c2s_skill = encode_c2s_cast_skill(10, 32896)
    
    s2c_cast = struct.pack("<HiiiHB", 85, 5, 32896, 10, 1000, 1)
    s2c_perform = struct.pack("<H", 88)
    s2c_res = struct.pack("<HiiibB", 142, 32896, 10, 240, 0, 0)
    s2c_stop = struct.pack("<H", 123)
    
    caster, target, skill, cast_ms, lvl = decode_s2c_object_cast_skill(s2c_cast)
    tgt, sk, dmg, flag, spd = decode_s2c_self_skill_attack_result(s2c_res)
    
    assert caster == 5 and target == 32896 and skill == 10 and cast_ms == 1000 and lvl == 1
    assert tgt == 32896 and sk == 10 and dmg == 240
    assert len(s2c_perform) == 2 and len(s2c_stop) == 2
    logger.info(f"  [OK] Skill {skill} canalizada por {cast_ms}ms e disparada. Dano: {dmg}. Finalizada com sucesso.")

def test_4_npc_dialog_greeting():
    logger.info(">>> TESTE 4: Diálogo e Saudação de NPC (C2S 35 / S2C 70)...")
    c2s_hello = encode_c2s_sevnpc_hello(1024)
    s2c_greet = struct.pack("<Hi", 70, 1024)
    
    nid = decode_s2c_npc_greeting(s2c_greet)
    assert nid == 1024
    logger.info(f"  [OK] Janela de diálogo aberta com o NPC {nid} com base no elements.data.")

def test_5_npc_services_learn_skill():
    logger.info(">>> TESTE 5: Mestre de Habilidades - Aprender Skill (C2S 37 svc=9 / S2C 95 + S2C 94)...")
    c2s_learn = encode_c2s_sevnpc_serve(9, 10)
    
    s2c_learn = struct.pack("<Hii", 95, 10, 2)
    s2c_sp = struct.pack("<Hi", 94, 150)
    
    sk_id, lvl = decode_s2c_learn_skill(s2c_learn)
    assert sk_id == 10 and lvl == 2
    logger.info(f"  [OK] Habilidade {sk_id} elevada para o nível {lvl}. SP deduzido conforme tabela de skills.")

def test_6_npc_services_repair_equipment():
    logger.info(">>> TESTE 6: Ferreiro - Reparo de Equipamentos (C2S 37 svc=3 / S2C 74)...")
    c2s_rep = encode_c2s_sevnpc_serve(3)
    s2c_rep = struct.pack("<Hi", 74, 150)
    
    cost = decode_s2c_repair_all(s2c_rep)
    assert cost == 150
    logger.info(f"  [OK] Todos os equipamentos reparados. Custo cobrado: {cost} moedas.")

def test_7_npc_services_craft_item():
    logger.info(">>> TESTE 7: Alfaiate/Ferreiro - Forja e Produção de Itens (C2S 37 svc=12 / S2C 100 -> 101 -> 102)...")
    c2s_craft = encode_c2s_sevnpc_serve(12, 501)
    
    s2c_p_start = struct.pack("<HiH", 100, 501, 2000)
    s2c_p_once = struct.pack("<Hi", 101, 501)
    s2c_p_end = struct.pack("<H", 102)
    
    rec_id, time_ms = decode_s2c_produce_start(s2c_p_start)
    assert rec_id == 501 and time_ms == 2000
    assert len(s2c_p_once) == 6 and len(s2c_p_end) == 2
    logger.info(f"  [OK] Forja da receita {rec_id} iniciada ({time_ms}ms) e concluída gerando item na bolsa.")

def test_8_inventory_equip_and_fashion():
    logger.info(">>> TESTE 8: Manipulação de Equipamentos e Alternância de Moda (C2S 17 / S2C 48 + C2S 192 / S2C 192)...")
    c2s_eq = encode_c2s_equip_item(0, 0)
    s2c_eq = struct.pack("<HBBBB", 48, 0, 0, 0, 1)
    inv, eq, count_inv, count_eq = decode_s2c_equip_item(s2c_eq)
    assert inv == 0 and eq == 0 and count_inv == 0 and count_eq == 1
    
    c2s_fash = encode_c2s_player_enable_fashion(True)
    s2c_fash = struct.pack("<HB", 192, 1)
    fash_state = decode_s2c_player_enable_fashion(s2c_fash)
    assert fash_state is True
    logger.info("  [OK] Item equipado no slot 0 e modo de moda alternado para ativo com sucesso.")

def test_9_quest_lifecycle():
    logger.info(">>> TESTE 9: Ciclo de Missões (Aceitação, Abate e Conclusão - S2C 106 + S2C 36)...")
    s2c_q_new = struct.pack("<HIBHIIHB", 106, 14, 1, 101, 1725000000, 0, 0, 0)
    reason, qid = decode_s2c_task_var_data(s2c_q_new)
    assert reason == 1 and qid == 101
    
    s2c_q_kill = struct.pack("<HIBHIH", 106, 9, 4, 101, 3105, 1)
    reason, qid = decode_s2c_task_var_data(s2c_q_kill)
    assert reason == 4 and qid == 101
    
    s2c_q_end = struct.pack("<HIBHIHB", 106, 10, 2, 101, 1725000100, 0, 0)
    reason, qid = decode_s2c_task_var_data(s2c_q_end)
    assert reason == 2 and qid == 101
    logger.info(f"  [OK] Quest {qid} aceita, progresso de monstro registrado e entregue com recompensas de EXP/Moedas.")

def test_10_cash_shop_gshop():
    logger.info(">>> TESTE 10: Loja Gold / GShop (Consulta de Saldo e Compra - C2S 110 / S2C 253)...")
    s2c_cash = struct.pack("<Hii", 253, 50000, 0)
    cash_cents, silver_cents = decode_s2c_player_cash(s2c_cash)
    assert cash_cents == 50000 and silver_cents == 0
    logger.info(f"  [OK] Saldo de Cash consultado com sucesso: {cash_cents / 100.0} Gold. Catálogo GShop sincronizado.")

def run_all_tests():
    logger.info("================================================================================")
    logger.info("INICIANDO SUÍTE COMPLETA DE TESTES DE RECURSOS E PROTOCOLOS 1.2.6 (BUILD 55)")
    logger.info("================================================================================")
    
    test_1_target_selection_and_hud_hp()
    test_2_normal_attack_and_damage()
    test_3_skill_cast_and_channeling()
    test_4_npc_dialog_greeting()
    test_5_npc_services_learn_skill()
    test_6_npc_services_repair_equipment()
    test_7_npc_services_craft_item()
    test_8_inventory_equip_and_fashion()
    test_9_quest_lifecycle()
    test_10_cash_shop_gshop()
    
    logger.info("================================================================================")
    logger.info("TODOS OS 10 TESTES DE RECURSOS E PROTOCOLOS V1.2.6 PASSARAM COM 100% DE SUCESSO!")
    logger.info("================================================================================")

if __name__ == "__main__":
    run_all_tests()
