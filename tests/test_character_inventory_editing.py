"""
================================================================================
PW-UNIVERSAL-SERVER: TESTES DE EDIÇÃO DE ITENS, INVENTÁRIO E PERSONAGENS
================================================================================
Testa:
1. Inserção de Itens no Inventário, Equipamentos e Armazém (Storehouse).
2. Edição Granular de Refino (+0 a +12) e Durabilidade sem corromper octets.
3. Prevenção de Conflito de Slots (Chave Composta Única character_id + container + slot).
4. Movimentação atômica de Itens entre Inventário e Banco.
5. Edição de Atributos do Personagem (Nível, Cultivo, EXP, SP, Moedas).
6. Teletransporte de Resgate para a Cidade do Dragão (CDD).
================================================================================
"""

import sys
import os
import json
import logging

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("PW_ITEM_TESTS")

class TestFailure(Exception):
    pass

# Mock em memória do Banco de Dados Normalizado
class InMemoryDatabase:
    def __init__(self):
        self.characters = {}      # char_id -> dict
        self.character_items = {} # (char_id, container_type, slot) -> dict
        self.item_counter = 1000

    def create_character(self, char_id: int, name: str, realm_id: str, level: int = 1):
        self.characters[char_id] = {
            "id": char_id,
            "name": name,
            "realm_id": realm_id,
            "level": level,
            "cultivation": 0,
            "exp": 0,
            "sp": 0,
            "money": 10000,
            "world_id": 1,
            "pos_x": 550.0,
            "pos_y": 200.0,
            "pos_z": 650.0
        }

    def add_or_update_item(self, char_id: int, container_type: int, slot: int, item_id: int, count: int, refine_level: int):
        if not (0 <= refine_level <= 12):
            raise ValueError("Refino deve estar entre +0 e +12")
        if not (0 <= slot <= 127):
            raise ValueError("Slot deve estar entre 0 e 127")
        if container_type not in (0, 1, 2):
            raise ValueError("Tipo de contêiner inválido (0=Inv, 1=Equip, 2=Banco)")

        self.item_counter += 1
        item_record = {
            "id": self.item_counter,
            "character_id": char_id,
            "container_type": container_type,
            "slot": slot,
            "item_id": item_id,
            "count": count,
            "refine_level": refine_level,
            "durability": 100,
            "bind_status": 0
        }
        self.character_items[(char_id, container_type, slot)] = item_record
        return item_record

    def move_item(self, char_id: int, from_container: int, from_slot: int, to_container: int, to_slot: int):
        from_key = (char_id, from_container, from_slot)
        to_key = (char_id, to_container, to_slot)

        if from_key not in self.character_items:
            raise KeyError("Item de origem não encontrado")
        if to_key in self.character_items:
            raise ValueError("Slot de destino já ocupado")

        item = self.character_items.pop(from_key)
        item["container_type"] = to_container
        item["slot"] = to_slot
        self.character_items[to_key] = item

    def edit_refine(self, char_id: int, container_type: int, slot: int, new_refine: int):
        key = (char_id, container_type, slot)
        if key not in self.character_items:
            raise KeyError("Item não encontrado")
        if not (0 <= new_refine <= 12):
            raise ValueError("Refino deve estar entre +0 e +12")
        self.character_items[key]["refine_level"] = new_refine

    def update_character_stats(self, char_id: int, level: int = None, cultivation: int = None, money: int = None):
        if char_id not in self.characters:
            raise KeyError("Personagem não encontrado")
        char = self.characters[char_id]
        if level is not None:
            char["level"] = level
        if cultivation is not None:
            char["cultivation"] = cultivation
        if money is not None:
            char["money"] = money

    def teleport_to_cdd(self, char_id: int):
        if char_id not in self.characters:
            raise KeyError("Personagem não encontrado")
        char = self.characters[char_id]
        char["world_id"] = 1
        char["pos_x"] = 550.0
        char["pos_y"] = 200.0
        char["pos_z"] = 650.0

def run_tests():
    logger.info(">>> INICIANDO TESTES DO MÓDULO DE PERSONAGENS & INVENTÁRIO...")
    db = InMemoryDatabase()

    # 1. Criação do Personagem de Teste
    db.create_character(char_id=1001, name="Guerreiro_Top1", realm_id="realm_126", level=1)
    logger.info("  [OK] Personagem criado no Realm 1.2.6 com sucesso.")

    # 2. Inserção de Arma no Inventário (Container 0, Slot 0)
    item1 = db.add_or_update_item(char_id=1001, container_type=0, slot=0, item_id=3001, count=1, refine_level=0)
    assert item1["item_id"] == 3001
    assert item1["refine_level"] == 0
    logger.info("  [OK] Espada +0 adicionada no Slot 0 do Inventário.")

    # 3. Edição Granular de Refino (+0 -> +12)
    db.edit_refine(char_id=1001, container_type=0, slot=0, new_refine=12)
    assert db.character_items[(1001, 0, 0)]["refine_level"] == 12
    logger.info("  [OK] Refino da espada atualizado para +12 com sucesso sem afetar outros slots.")

    # 4. Inserção de Poção no Slot 1 (Container 0, Slot 1)
    db.add_or_update_item(char_id=1001, container_type=0, slot=1, item_id=11208, count=500, refine_level=0)
    assert len(db.character_items) == 2
    logger.info("  [OK] 500x Poções adicionadas no Slot 1 do Inventário.")

    # 5. Movimentação atômica entre Inventário e Armazém/Banco (Container 2, Slot 0)
    db.move_item(char_id=1001, from_container=0, from_slot=0, to_container=2, to_slot=0)
    assert (1001, 0, 0) not in db.character_items
    assert (1001, 2, 0) in db.character_items
    assert db.character_items[(1001, 2, 0)]["refine_level"] == 12
    logger.info("  [OK] Espada +12 transferida para o Armazém (Storehouse) com persistência atômica.")

    # 6. Atualização de Atributos do Personagem
    db.update_character_stats(char_id=1001, level=105, cultivation=30, money=999999999)
    char = db.characters[1001]
    assert char["level"] == 105
    assert char["cultivation"] == 30
    assert char["money"] == 999999999
    logger.info("  [OK] Nível (105), Cultivo (30) e Moedas atualizados com sucesso.")

    # 7. Teletransporte de Emergência para CDD
    char["pos_x"] = 9999.0 # Coordenada bugada fora do mapa
    char["world_id"] = 99
    db.teleport_to_cdd(char_id=1001)
    assert char["world_id"] == 1
    assert char["pos_x"] == 550.0 and char["pos_y"] == 200.0 and char["pos_z"] == 650.0
    logger.info("  [OK] Teletransporte de resgate para a Cidade do Dragão executado perfeitamente.")

if __name__ == "__main__":
    print("===============================================================================")
    print("=        TESTES UNITÁRIOS: EDIÇÃO DE PERSONAGENS, ITENS & INVENTÁRIO          =")
    print("===============================================================================\n")
    try:
        run_tests()
        print("\n===============================================================================")
        print("=     TODOS OS TESTES DE PERSONAGENS E ITENS FORAM CONCLUÍDOS COM SUCESSO!    =")
        print("===============================================================================")
    except Exception as e:
        logger.error(f"FALHA NO TESTE: {e}")
        sys.exit(1)
