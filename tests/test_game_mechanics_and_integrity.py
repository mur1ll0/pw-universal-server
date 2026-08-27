"""
================================================================================
PW-UNIVERSAL-SERVER: SUÍTE COMPLETA DE TESTES DE MECÂNICAS & INTEGRIDADE DE DADOS
================================================================================
Cobre:
1. Mecânicas de Jogo: Fórmulas de Combate, Redução de Dano, Crítico, Distâncias 3D/2D e Grid Espacial (AOI).
2. Validação Cruzada de Integridade: Referências entre elements.data, npcgen.data, tasks.data, gshop.data e aipolicy.data.
3. Detecção de Falhas e Logs Descritivos de Erro para Fácil Resolução.
================================================================================
"""

import math
import sys
import os
import json
import logging

# Configuração de Logs Descritivos
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)]
)
logger = logging.getLogger("PW_TEST_SUITE")

class TestFailure(Exception):
    pass

# ==============================================================================
# 1. MECÂNICAS DE JOGO: CÁLCULOS 3D E AOI (ÁREA DE INTERESSE)
# ==============================================================================

class Vector3:
    def __init__(self, x: float, y: float, z: float):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)

    def distance_3d(self, other: 'Vector3') -> float:
        return math.sqrt((self.x - other.x)**2 + (self.y - other.y)**2 + (self.z - other.z)**2)

    def distance_2d(self, other: 'Vector3') -> float:
        return math.sqrt((self.x - other.x)**2 + (self.z - other.z)**2)

class SpatialGrid:
    CELL_SIZE = 50.0

    def __init__(self):
        self.cells = {} # (cell_x, cell_z) -> set(entity_id)
        self.positions = {} # entity_id -> (Vector3, is_player)

    def add_entity(self, entity_id: int, pos: Vector3, is_player: bool = False):
        cx = int(math.floor(pos.x / self.CELL_SIZE))
        cz = int(math.floor(pos.z / self.CELL_SIZE))
        self.cells.setdefault((cx, cz), set()).add(entity_id)
        self.positions[entity_id] = (pos, is_player)

    def update_position(self, entity_id: int, new_pos: Vector3):
        if entity_id in self.positions:
            old_pos, is_player = self.positions[entity_id]
            old_cx = int(math.floor(old_pos.x / self.CELL_SIZE))
            old_cz = int(math.floor(old_pos.z / self.CELL_SIZE))
            new_cx = int(math.floor(new_pos.x / self.CELL_SIZE))
            new_cz = int(math.floor(new_pos.z / self.CELL_SIZE))

            self.positions[entity_id] = (new_pos, is_player)
            if (old_cx, old_cz) != (new_cx, new_cz):
                if (old_cx, old_cz) in self.cells:
                    self.cells[(old_cx, old_cz)].discard(entity_id)
                self.cells.setdefault((new_cx, new_cz), set()).add(entity_id)

    def get_entities_in_range(self, center: Vector3, radius: float) -> list:
        found = []
        r_sq = radius * radius
        min_cx = int(math.floor((center.x - radius) / self.CELL_SIZE))
        max_cx = int(math.floor((center.x + radius) / self.CELL_SIZE))
        min_cz = int(math.floor((center.z - radius) / self.CELL_SIZE))
        max_cz = int(math.floor((center.z + radius) / self.CELL_SIZE))

        for cx in range(min_cx, max_cx + 1):
            for cz in range(min_cz, max_cz + 1):
                for eid in self.cells.get((cx, cz), []):
                    pos, _ = self.positions[eid]
                    if (pos.x - center.x)**2 + (pos.y - center.y)**2 + (pos.z - center.z)**2 <= r_sq:
                        found.append(eid)
        return found

def test_spatial_grid_and_distance():
    logger.info(">>> TESTANDO MECÂNICA: CÁLCULO DE DISTÂNCIA 3D E AOI (ÁREA DE VISIBILIDADE)...")
    
    # 1. Teste de Distância 3D Euclidiana
    p1 = Vector3(550.0, 200.0, 650.0) # Centro da Cidade do Dragão
    p2 = Vector3(580.0, 200.0, 690.0)
    dist = p1.distance_3d(p2) # dx=30, dy=0, dz=40 -> dist = 50.0
    
    if abs(dist - 50.0) > 0.001:
        logger.error(f"[MECÂNICA-FALHA] Distância calculada ({dist}) não confere com o esperado (50.0)")
        raise TestFailure("Falha no cálculo de distância euclidiana")
    logger.info(f"  [OK] Cálculo de distância 3D exato: {dist} metros.")

    # 2. Teste de Grid Espacial e Visibilidade
    grid = SpatialGrid()
    grid.add_entity(101, Vector3(550.0, 200.0, 650.0), is_player=True) # Jogador A
    grid.add_entity(102, Vector3(570.0, 200.0, 650.0), is_player=True) # Jogador B (20m de distância)
    grid.add_entity(103, Vector3(850.0, 200.0, 650.0), is_player=True) # Jogador C (300m de distância)
    
    # Jogador A com raio de visão de 100m deve ver apenas Jogador B
    visible = grid.get_entities_in_range(p1, 100.0)
    if 101 not in visible or 102 not in visible:
        logger.error(f"[AOI-FALHA] Entidades próximas não foram detectadas no raio de 100m. Visíveis: {visible}")
        raise TestFailure("Entidades próximas ausentes no grid")
    if 103 in visible:
        logger.error(f"[AOI-FALHA] Entidade distante (300m) foi incluída indevidamente no raio de 100m.")
        raise TestFailure("Entidade distante visível indevidamente")
        
    logger.info("  [OK] Grid Espacial AOI filtra corretamente entidades por raio de visão.")

# ==============================================================================
# 2. MECÂNICAS DE JOGO: FÓRMULAS DE COMBATE E DANO
# ==============================================================================

def calculate_physical_damage(attacker_level: int, attack_val: float, defender_level: int, defense_val: float, is_crit: bool) -> int:
    def_factor = 1.0 / (1.0 + (defense_val / (100.0 * max(1, attacker_level))))
    crit_multiplier = 2.0 if is_crit else 1.0
    return max(1, int(attack_val * def_factor * crit_multiplier))

def test_combat_damage_formulas():
    logger.info(">>> TESTANDO MECÂNICA: FÓRMULAS DE COMBATE E REDUÇÃO POR ARMADURA...")
    
    # Caso 1: Jogador Lv 100 (Ataque 5000) contra Monstro com Defesa 0 (Dano total 5000)
    dmg_zero_def = calculate_physical_damage(100, 5000, 100, 0, False)
    if dmg_zero_def != 5000:
        logger.error(f"[COMBATE-FALHA] Dano com defesa 0 deveria ser 5000, mas deu {dmg_zero_def}")
        raise TestFailure("Falha no cálculo base de dano")

    # Caso 2: Defesa igual a (100 * level) -> Redução de 50%
    # Defesa = 10000, Level = 100 -> fator = 1 / (1 + 10000/10000) = 1/2 = 50% -> 2500 de dano
    dmg_50_pct = calculate_physical_damage(100, 5000, 100, 10000, False)
    if dmg_50_pct != 2500:
        logger.error(f"[COMBATE-FALHA] Dano com 50% de redução deveria ser 2500, mas deu {dmg_50_pct}")
        raise TestFailure("Falha na curva de redução por defesa")

    # Caso 3: Acerto Crítico (Multiplicador de x2.0) -> 2500 * 2 = 5000
    dmg_crit = calculate_physical_damage(100, 5000, 100, 10000, True)
    if dmg_crit != 5000:
        logger.error(f"[COMBATE-FALHA] Dano crítico deveria ser 5000, mas deu {dmg_crit}")
        raise TestFailure("Falha no multiplicador crítico")

    # Caso 4: Defesa extrema -> Dano nunca pode ser zero (mínimo 1)
    dmg_min = calculate_physical_damage(1, 10, 100, 9999999, False)
    if dmg_min < 1:
        logger.error(f"[COMBATE-FALHA] Dano mínimo deve ser pelo menos 1, mas deu {dmg_min}")
        raise TestFailure("Dano zero ou negativo não permitido")

    logger.info("  [OK] Curvas de dano físico, defesa e acerto crítico validadas com sucesso.")

# ==============================================================================
# 3. VALIDAÇÃO DE INTEGRIDADE CRUZADA E FALHAS EM ARQUIVOS .DATA
# ==============================================================================

class DataIntegrityChecker:
    def __init__(self):
        self.elements_items = {}     # item_id -> type
        self.elements_monsters = {}  # monster_id -> {hp, aipolicy_id, drop_table_id}
        self.elements_npcs = {}      # npc_id -> {dialog_id}
        self.aipolicies = set()      # set de policy_ids
        self.tasks = {}              # task_id -> {monster_kills, item_collections}
        self.gshop_items = {}        # shop_id -> item_id
        self.map_spawns = []         # lista de spawns do npcgen

    def run_full_validation(self) -> list:
        errors = []

        # 1. Valida se todo monstro no npcgen existe no elements.data
        for spawn in self.map_spawns:
            map_name = spawn["map"]
            t_id = spawn["template_id"]
            s_type = spawn["spawn_type"]

            if s_type == "MONSTER":
                if t_id not in self.elements_monsters:
                    errors.append({
                        "category": "NPCGEN_ORPHAN_MONSTER",
                        "map": map_name,
                        "location": f"({spawn['x']}, {spawn['y']}, {spawn['z']})",
                        "template_id": t_id,
                        "message": f"O spawn na região '{spawn['area_name']}' no mapa '{map_name}' referencia o Monstro ID {t_id}, mas ele NÃO existe no elements.data!",
                        "remediation": f"Adicione o monstro ID {t_id} na lista de monstros do elements.data ou corrija o ID no arquivo {map_name}/npcgen.data."
                    })
            elif s_type == "NPC":
                if t_id not in self.elements_npcs:
                    errors.append({
                        "category": "NPCGEN_ORPHAN_NPC",
                        "map": map_name,
                        "location": f"({spawn['x']}, {spawn['y']}, {spawn['z']})",
                        "template_id": t_id,
                        "message": f"O NPC de diálogo ID {t_id} no mapa '{map_name}' NÃO existe no elements.data!",
                        "remediation": f"Cadastre o NPC {t_id} no elements.data ou remova a entrada em {map_name}/npcgen.data."
                    })

        # 2. Valida se toda política de IA de monstro existe no aipolicy.data
        for m_id, m_data in self.elements_monsters.items():
            ai_id = m_data.get("aipolicy_id", 0)
            if ai_id > 0 and ai_id not in self.aipolicies:
                errors.append({
                    "category": "ELEMENTS_ORPHAN_AIPOLICY",
                    "monster_id": m_id,
                    "aipolicy_id": ai_id,
                    "message": f"O Monstro ID {m_id} do elements.data referencia a IA Policy ID {ai_id}, que NÃO existe no aipolicy.data!",
                    "remediation": f"Crie a árvore de IA ID {ai_id} no aipolicy.data ou altere o monstro {m_id} para usar policy 0 (IA padrão)."
                })

        # 3. Valida se todo item do GShop existe no elements.data
        for shop_id, item_id in self.gshop_items.items():
            if item_id not in self.elements_items:
                errors.append({
                    "category": "GSHOP_INVALID_ITEM",
                    "shop_id": shop_id,
                    "item_id": item_id,
                    "message": f"A oferta do GShop #{shop_id} vende o item ID {item_id}, mas esse item NÃO existe no elements.data!",
                    "remediation": f"Remova a oferta #{shop_id} do gshop.data ou crie o item {item_id} no elements.data para evitar que o jogador compre um item invisível/nulo."
                })

        # 4. Valida se as missões do tasks.data apontam para monstros e itens válidos
        for task_id, task in self.tasks.items():
            for m_target in task.get("monster_kills", []):
                if m_target not in self.elements_monsters:
                    errors.append({
                        "category": "TASK_INVALID_TARGET_MONSTER",
                        "task_id": task_id,
                        "monster_id": m_target,
                        "message": f"A Missão ID {task_id} exige derrotar o Monstro ID {m_target}, mas ele NÃO existe no elements.data!",
                        "remediation": f"Corrija o objetivo da quest no tasks.data ou cadastre o monstro {m_target}."
                    })

        return errors

def test_data_integrity_and_error_detection():
    logger.info(">>> TESTANDO DETECÇÃO DE FALHAS E INTEGRIDADE DE ARQUIVOS .DATA...")
    
    checker = DataIntegrityChecker()

    # Popula dados válidos
    checker.elements_items = {11208: "MEDICINE", 3000: "WEAPON", 5000: "ARMOR"}
    checker.elements_monsters = {
        1001: {"hp": 500, "aipolicy_id": 1, "drop_table_id": 0},
        1002: {"hp": 12000, "aipolicy_id": 999, "drop_table_id": 0} # ⚠️ AIPolicy 999 não existe!
    }
    checker.elements_npcs = {2001: {"dialog_id": 50}}
    checker.aipolicies = {1, 2, 3}
    checker.gshop_items = {
        1: 11208, # Válido
        2: 99999  # ⚠️ Item 99999 não existe no elements!
    }
    checker.map_spawns = [
        {"map": "world", "area_name": "DragonCity_East", "spawn_type": "MONSTER", "template_id": 1001, "x": 550, "y": 200, "z": 650},
        {"map": "world", "area_name": "BrokenBridge_Boss", "spawn_type": "MONSTER", "template_id": 8888, "x": 120, "y": 200, "z": 300}, # ⚠️ Monstro 8888 não existe!
        {"map": "a01", "area_name": "Dungeon19_Guard", "spawn_type": "NPC", "template_id": 9001, "x": 10, "y": 20, "z": 30} # ⚠️ NPC 9001 não existe!
    ]
    checker.tasks = {
        100: {"monster_kills": [1001]}, # Válido
        101: {"monster_kills": [7777]}  # ⚠️ Monstro 7777 não existe!
    }

    # Executa a varredura
    errors = checker.run_full_validation()

    logger.info(f"Varredura concluída. Foram detectadas propositalmente {len(errors)} falhas de integridade cruzada:")
    for i, err in enumerate(errors, 1):
        print(f"\n  [PROBLEMA #{i}] ----------------------------------------------------")
        print(f"  CATEGORIA:    {err['category']}")
        print(f"  MENSAGEM:     {err['message']}")
        print(f"  COMO RESOLVER:{err['remediation']}")

    if len(errors) != 5:
        logger.error(f"[TESTE-FALHA] O detector deveria ter encontrado 5 falhas, mas encontrou {len(errors)}")
        raise TestFailure("Falha no validador de integridade")

    logger.info("\n  [OK] O sistema de diagnóstico e logs descritivos de integridade funcionou com 100% de precisão!")

# ==============================================================================
# MAIN TEST RUNNER
# ==============================================================================

if __name__ == "__main__":
    print("===============================================================================")
    print("=      SUÍTE DE TESTES: MECÂNICAS, COMBATE & DIAGNÓSTICO DE INTEGRIDADE       =")
    print("===============================================================================\n")

    try:
        test_spatial_grid_and_distance()
        print()
        test_combat_damage_formulas()
        print()
        test_data_integrity_and_error_detection()
        print()
        print("===============================================================================")
        print("=  TODOS OS TESTES DE MECÂNICAS E DIAGNÓSTICO DE FALHAS PASSARAM COM SUCESSO! =")
        print("===============================================================================")
    except Exception as e:
        logger.error(f"FALHA CRÍTICA NOS TESTES: {e}")
        sys.exit(1)
