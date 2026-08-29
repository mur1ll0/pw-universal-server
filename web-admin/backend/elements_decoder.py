"""
PW-ADMIN: Decoder de elements.data e Gerenciador de Ícones do Surfaces/Iconset
Permite decodificação binária de arquivos elements.data de múltiplos realms do servidor,
extraindo nomes, descrições, categorias, atributos de forja e ícones fiéis ao jogo
a partir dos arquivos surfaces.pck / iconset (iconlist_ivtrm.dds e iconlist_skill.dds).
"""

import os
import io
import struct
import unicodedata
from typing import Dict, List, Any, Optional, Tuple

def normalize_search_string(text: str) -> str:
    """Normaliza texto removendo acentos e convertendo para minúsculas"""
    if not text:
        return ""
    nfkd = unicodedata.normalize('NFKD', str(text))
    return "".join(c for c in nfkd if not unicodedata.combining(c)).lower().strip()

try:
    from PIL import Image
    HAS_PIL = True
except Exception:
    Image = None
    HAS_PIL = False

# Tabela de tamanhos de registros para elements.data v7 (PW 1.2.6)
TABLE_SIZES_V7 = [
    84, 68, 356, 1404, 68, 72, 1104, 68, 72, 1156,
    68, 68, 376, 68, 68, 368, 68, 364, 68, 624,
    68, 348, 516, 488, 348, 348, 352, 348, 208, 888,
    68, 892, 68, 340, 68, 436, 84, 196, 1500, 72,
    1224, 72, 72, 200, 200, 196, 196, 644, 584, 72,
    460, 328, 72, 68, 1224, 72, 68, 848, 476, 348,
    196, 336, 468, 340, 208, 204, 68, 68, 400, 196,
    160, 612, 488, 404, 344, 340, 668, 68, 452, 72,
    68, 72, 404, 68, 68, 488, 68, 68, 2412, 292,
    68, 344, 68, 476, 628, 360, 344, 480, 344, 148,
    1092, 368, 76, 584, 76, 356, 436, 344, 76, 76,
    76, 384, 348, 356, 356, 348, 344, 368,
]

TABLE_CATEGORIES = {
    3: "Arma",
    6: "Armadura",
    9: "Ornamento / Jóia",
    12: "Poção / Medicamento",
    15: "Material de Forja",
    17: "Item de Missão",
    22: "Pergaminho / Retorno",
    23: "Voo / Montaria Alada",
    24: "Moda / Roupas",
    27: "Projétil / Flecha",
    31: "Pedra da Alma / Gema",
    34: "Hierograma / Amuleto",
    35: "Livro Sagrado / Tomo",
}

# ==============================================================================
# CODEC DE OCTETS BINÁRIOS DE ITENS (PERFECT WORLD ENGINE)
# ==============================================================================

class ItemOctetCodec:
    """
    Codificador e Decodificador de Octets Binários de Itens do Perfect World.
    Implementação fiel ao GameServer (gs) e ao protocolo do cliente (Little-Endian).
    """

    @staticmethod
    def hex_to_bytes(hex_str: Any) -> bytes:
        if not hex_str:
            return b""
        if isinstance(hex_str, (bytes, bytearray)):
            return bytes(hex_str)
        clean = str(hex_str).strip().replace(" ", "").replace("0x", "").replace("\n", "").replace("\r", "")
        if len(clean) % 2 != 0:
            clean = "0" + clean
        try:
            return bytes.fromhex(clean)
        except Exception:
            return b""

    @staticmethod
    def bytes_to_hex(data: Optional[bytes]) -> str:
        if not data:
            return ""
        if isinstance(data, str):
            return data
        return data.hex()

    @classmethod
    def build_item_octets(
        cls,
        category: str = "Arma",
        level: int = 1,
        race_mask: int = 255,
        str_req: int = 0,
        vit_req: int = 0,
        agi_req: int = 0,
        eng_req: int = 0,
        durability: int = 2800,
        max_durability: int = 2800,
        creator_name: str = "",
        refine_level: int = 0,
        sockets_count: int = 0,
        socket_stones: Optional[List[int]] = None,
        dmg_low: int = 10,
        dmg_high: int = 20,
        def_phys: int = 10,
        def_magic: int = 5,
        color: int = 0x00FFFFFF,
        weapon_type: int = 0,
        weapon_class: int = 1,
    ) -> bytes:
        cat = (category or "").lower()
        if not any(k in cat for k in [
            "arma", "armadura", "ornamento", "jóia", "joia", "voo", "montaria", "moda", "livro", "tomo", "projétil", "projetil"
        ]):
            return b""

        socket_stones = socket_stones or []
        buf = bytearray()

        dura = int(durability) if durability is not None and durability > 0 else 2800
        max_dura = int(max_durability) if max_durability is not None and max_durability > 0 else dura
        buf.extend(struct.pack(
            "<hhhhhhII",
            int(level),
            int(race_mask),
            int(str_req),
            int(vit_req),
            int(agi_req),
            int(eng_req),
            dura,
            max_dura
        ))

        c_bytes = (creator_name or "").encode("gbk", errors="ignore")[:31]
        tag_type = 1 if len(c_bytes) > 0 else 0
        tag_size = len(c_bytes)

        if "arma" in cat:
            ess_size = 44
            ess_bytes = struct.pack(
                "<hhiiiiiiiiff",
                int(weapon_type),
                0,
                int(weapon_class),
                int(level),
                0,
                int(dmg_low),
                int(dmg_high),
                0,
                0,
                20,
                3.5,
                0.0
            )
        elif "armadura" in cat:
            ess_size = 36
            ess_bytes = struct.pack(
                "<iiiiiiiii",
                int(def_phys),
                0,
                0,
                0,
                int(def_magic),
                int(def_magic),
                int(def_magic),
                int(def_magic),
                int(def_magic)
            )
        elif any(k in cat for k in ["ornamento", "jóia", "joia"]):
            ess_size = 36
            ess_bytes = struct.pack(
                "<iiiiiiiii",
                0,
                0,
                int(def_phys),
                0,
                int(def_magic),
                int(def_magic),
                int(def_magic),
                int(def_magic),
                int(def_magic)
            )
        elif "moda" in cat:
            ess_size = 8
            ess_bytes = struct.pack("<ii", int(color), int(level))
        elif any(k in cat for k in ["voo", "montaria"]):
            ess_size = 24
            ess_bytes = struct.pack("<iiiiii", int(level), 200, 400, 0, 0, 1)
        elif any(k in cat for k in ["livro", "tomo"]):
            ess_size = 4
            ess_bytes = struct.pack("<i", int(level))
        elif any(k in cat for k in ["projétil", "projetil"]):
            ess_size = 20
            ess_bytes = struct.pack("<iiiii", 0, int(dmg_high), 0, 1, 10)
        else:
            ess_size = 8
            ess_bytes = struct.pack("<ii", int(level), 0)

        buf.extend(struct.pack("<HBB", ess_size, tag_type, tag_size))
        if tag_size > 0:
            buf.extend(c_bytes)
        buf.extend(ess_bytes)

        sock_cnt = min(max(0, int(sockets_count)), 4)
        buf.extend(struct.pack("<HH", sock_cnt, 0))
        for i in range(sock_cnt):
            st_id = int(socket_stones[i]) if i < len(socket_stones) else 0
            buf.extend(struct.pack("<I", st_id))

        addons = []
        if refine_level > 0:
            ref_val = int(refine_level) * (15 if "arma" in cat else 10)
            ref_addon_id = 0x0001 | (1 << 13)
            addons.append((ref_addon_id, [ref_val]))

        buf.extend(struct.pack("<I", len(addons)))
        for a_id, args in addons:
            buf.extend(struct.pack("<I", a_id))
            for arg in args:
                buf.extend(struct.pack("<I", int(arg)))

        return bytes(buf)

    @classmethod
    def parse_item_octets(cls, raw_data: Any) -> Dict[str, Any]:
        raw_bytes = cls.hex_to_bytes(raw_data) if isinstance(raw_data, str) else (raw_data or b"")
        if not raw_bytes or len(raw_bytes) < 24:
            return {
                "has_octets": False,
                "raw_hex": raw_bytes.hex() if raw_bytes else "",
                "level": 1,
                "durability": 1000,
                "max_durability": 1000,
                "refine_level": 0,
                "sockets_count": 0,
                "socket_stones": [],
                "creator_name": "",
                "addons": []
            }

        try:
            lvl, race, st, vit, agi, eng, dura, max_dura = struct.unpack("<hhhhhhII", raw_bytes[:20])
            offset = 20
            ess_size, tag_type, tag_size = struct.unpack("<HBB", raw_bytes[offset:offset+4])
            offset += 4

            creator = ""
            if tag_size > 0 and offset + tag_size <= len(raw_bytes):
                creator = raw_bytes[offset:offset+tag_size].decode("gbk", errors="ignore").strip()
                offset += tag_size

            essence_raw = raw_bytes[offset:offset+ess_size] if offset + ess_size <= len(raw_bytes) else b""
            offset += ess_size

            sockets = []
            sock_count = 0
            mod_mask = 0
            if offset + 4 <= len(raw_bytes):
                sock_count, mod_mask = struct.unpack("<HH", raw_bytes[offset:offset+4])
                offset += 4
                for _ in range(sock_count):
                    if offset + 4 <= len(raw_bytes):
                        st_id = struct.unpack("<I", raw_bytes[offset:offset+4])[0]
                        sockets.append(st_id)
                        offset += 4

            addons = []
            refine_level = 0
            if offset + 4 <= len(raw_bytes):
                addon_cnt = struct.unpack("<I", raw_bytes[offset:offset+4])[0]
                offset += 4
                for _ in range(addon_cnt):
                    if offset + 4 <= len(raw_bytes):
                        a_id = struct.unpack("<I", raw_bytes[offset:offset+4])[0]
                        offset += 4
                        param_num = (a_id >> 13) & 0x03
                        pure_id = a_id & (~(0x03 << 13))
                        args = []
                        for _ in range(param_num):
                            if offset + 4 <= len(raw_bytes):
                                arg_val = struct.unpack("<I", raw_bytes[offset:offset+4])[0]
                                args.append(arg_val)
                                offset += 4
                        addons.append({"id": a_id, "pure_id": pure_id, "args": args})
                        if pure_id in [1, 2] and args:
                            val = args[0]
                            refine_level = max(1, min(12, int(val / 15 if val % 15 == 0 else val / 10)))

            return {
                "has_octets": True,
                "level": lvl,
                "race_mask": race,
                "strength": st,
                "vitality": vit,
                "agility": agi,
                "energy": eng,
                "durability": dura,
                "max_durability": max_dura,
                "creator_name": creator,
                "tag_type": tag_type,
                "essence_size": ess_size,
                "sockets_count": sock_count,
                "socket_stones": sockets,
                "refine_level": refine_level,
                "addons": addons,
                "raw_hex": raw_bytes.hex()
            }
        except Exception as e:
            return {
                "has_octets": False,
                "error": str(e),
                "raw_hex": raw_bytes.hex()
            }


class SkillOctetCodec:
    """
    Codificador e Decodificador de Octets Binários de Habilidades (Skills) do Perfect World.
    Implementa o protocolo binário oficial do GameServer C++ e o formato clássico Skill HexGen (Little-Endian).
    
    Estrutura Binária em Memória:
      [4 bytes uint32: count]
      Para cada habilidade (12 bytes):
        - [4 bytes uint32: skill_id]
        - [4 bytes uint32: progress / force (0)]
        - [4 bytes uint32: level (1..10, 11=God, 12=Evil)]
    """

    @staticmethod
    def build_skills_octets(skills_list: List[Dict[str, Any]]) -> bytes:
        """Gera o payload binário Little-Endian para a lista de habilidades informada"""
        valid_skills = [
            s for s in skills_list 
            if s and int(s.get("skill_id", s.get("id", 0))) > 0 and int(s.get("level", 1)) > 0
        ]
        count = len(valid_skills)
        buf = bytearray()
        buf.extend(struct.pack("<I", count))
        for sk in valid_skills:
            sid = int(sk.get("skill_id", sk.get("id", 0)))
            progress = int(sk.get("progress", 0))
            lvl = int(sk.get("level", 1))
            buf.extend(struct.pack("<III", sid, progress, lvl))
        return bytes(buf)

    @staticmethod
    def parse_skills_octets(octets_data: Any) -> List[Dict[str, int]]:
        """Decodifica um payload binário ou string hex de habilidades em lista de dicionários {skill_id, level, progress}"""
        if not octets_data:
            return []
        raw = ItemOctetCodec.hex_to_bytes(octets_data)
        if len(raw) < 4:
            return []
        
        count = struct.unpack("<I", raw[0:4])[0]
        results = []
        offset = 4
        for _ in range(count):
            if offset + 12 > len(raw):
                break
            sid, progress, lvl = struct.unpack("<III", raw[offset:offset+12])
            if sid > 0:
                results.append({
                    "skill_id": sid,
                    "level": lvl,
                    "progress": progress
                })
            offset += 12
        return results

    @staticmethod
    def bytes_to_hex(raw_bytes: bytes) -> str:
        return raw_bytes.hex() if raw_bytes else ""

    @staticmethod
    def hex_to_bytes(hex_str: str) -> bytes:
        return ItemOctetCodec.hex_to_bytes(hex_str)


# Base de Habilidades do Jogo por Classe com mapeamento preciso de IDs, traduções e ícones do atlas DDS
PW_SKILLS_DATABASE = [
    # =========================================================================
    # GUERREIRO (Classe 0 - Blademaster)
    # =========================================================================
    {"id": 1, "name": "Golpe do Tigre", "name_en": "Tiger Maw", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-hand-back-fist", "icon_file": "虎击.dds", "max_lv": 10, "desc": "Golpe inicial frontal causando dano físico adicional."},
    {"id": 2, "name": "Corte Sangrento", "name_en": "Draw Blood", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-droplet", "icon_file": "断岳.dds", "max_lv": 10, "desc": "Corta pontos vitais causando sangramento contínuo no alvo."},
    {"id": 3, "name": "Lâmina Eólica", "name_en": "Aeolian Blade", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-wind", "icon_file": "霸王断岳.dds", "max_lv": 10, "desc": "Corta o ar lançando uma lâmina cortante que pode atordoar o alvo."},
    {"id": 4, "name": "Sino Dourado", "name_en": "Aura of the Golden Bell", "class_id": 0, "class_name": "Guerreiro", "type": "Buff em Grupo", "icon": "fa-solid fa-shield-halved", "icon_file": "金钟罩.dds", "max_lv": 10, "desc": "Aumenta a defesa física de todos os membros do grupo."},
    {"id": 5, "name": "Raio do Dragão", "name_en": "Drake's Ray", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque à Distância", "icon": "fa-solid fa-bolt", "icon_file": "龙击.dds", "max_lv": 10, "desc": "Dispara um raio de energia marcial à distância."},
    {"id": 6, "name": "Rugido do Leão", "name_en": "Roar of the Pride", "class_id": 0, "class_name": "Guerreiro", "type": "Controle em Área", "icon": "fa-solid fa-bullhorn", "icon_file": "狮子吼.dds", "max_lv": 10, "desc": "Rugido ensurdecedor que atordoa todos os inimigos ao redor."},
    {"id": 54, "name": "Golpe Fluente", "name_en": "Stream Strike", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-water", "icon_file": "流水.dds", "max_lv": 10, "desc": "Golpe contínuo que reduz a velocidade de ataque do adversário."},
    {"id": 55, "name": "Leque de Chamas", "name_en": "Fan of Flames", "class_id": 0, "class_name": "Guerreiro", "type": "Dano de Fogo", "icon": "fa-solid fa-fire", "icon_file": "火焰扇.dds", "max_lv": 10, "desc": "Ataque cônico de fogo causando dano em área."},
    {"id": 56, "name": "Varredura do Dragão", "name_en": "Drake Sweep", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque em Área", "icon": "fa-solid fa-dragon", "icon_file": "横扫千军.dds", "max_lv": 10, "desc": "Gira a arma em 360 graus atingindo todos os alvos próximos."},
    {"id": 57, "name": "Mar Adentro", "name_en": "Ocean's Edge", "class_id": 0, "class_name": "Guerreiro", "type": "Dano de Água", "icon": "fa-solid fa-water", "icon_file": "沧海.dds", "max_lv": 10, "desc": "Imbui a arma com a força das marés causando dano físico e mágico."},
    {"id": 58, "name": "Salto para Trás", "name_en": "Leap Back", "class_id": 0, "class_name": "Guerreiro", "type": "Mobilidade", "icon": "fa-solid fa-person-walking-arrow-right", "icon_file": "后跳.dds", "max_lv": 1, "desc": "Recua rapidamente para esquivar de golpes inimigos."},
    {"id": 59, "name": "Salto do Tigre", "name_en": "Tiger Leap", "class_id": 0, "class_name": "Guerreiro", "type": "Mobilidade", "icon": "fa-solid fa-person-running", "icon_file": "虎跃.dds", "max_lv": 1, "desc": "Avança instantaneamente em direção ao inimigo."},
    {"id": 60, "name": "Palma do Vácuo", "name_en": "Vacuous Palm", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Punho", "icon": "fa-solid fa-hand", "icon_file": "空手夺白刃.dds", "max_lv": 10, "desc": "Golpe rápido com as mãos desarmadas que reduz a velocidade do alvo."},
    {"id": 61, "name": "Chute sem Sombra", "name_en": "Shadowless Kick", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Punho", "icon": "fa-solid fa-shoe-prints", "icon_file": "无影脚.dds", "max_lv": 10, "desc": "Sequência veloz de chutes que interrompe a conjuração do alvo."},
    {"id": 62, "name": "Calcanhar Furacão", "name_en": "Cyclone Heel", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque em Área", "icon": "fa-solid fa-tornado", "icon_file": "旋风腿.dds", "max_lv": 10, "desc": "Chute giratório causando dano físico a todos ao redor."},
    {"id": 63, "name": "Impacto do Dragão", "name_en": "Drake's Breath Bash", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Punho", "icon": "fa-solid fa-hand-fist", "icon_file": "龙息击.dds", "max_lv": 10, "desc": "Golpe marcial concentrado causando alto dano crítico."},
    {"id": 64, "name": "Ventos Cortantes", "name_en": "Piercing Winds", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Lança", "icon": "fa-solid fa-wind", "icon_file": "穿风刺.dds", "max_lv": 10, "desc": "Estocada penetrante de lança com dano contínuo."},
    {"id": 65, "name": "Golpe Distante", "name_en": "Farstrike", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Lança", "icon": "fa-solid fa-location-arrow", "icon_file": "远击.dds", "max_lv": 10, "desc": "Ataque de lança de longo alcance com perfuração de armadura."},
    {"id": 66, "name": "Meteoro", "name_en": "Meteor Rush", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Lança", "icon": "fa-solid fa-meteor", "icon_file": "流星赶月.dds", "max_lv": 10, "desc": "Estocada rápida que empurra e repele o oponente."},
    {"id": 67, "name": "Lança Glacial", "name_en": "Glacial Spike", "class_id": 0, "class_name": "Guerreiro", "type": "Dano em Linha", "icon": "fa-solid fa-icicles", "icon_file": "冰刺.dds", "max_lv": 10, "desc": "Lança uma onda de choque frontal em linha reta."},
    {"id": 68, "name": "Golpe Desarmante", "name_en": "Drake Bash", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Machado", "icon": "fa-solid fa-gavel", "icon_file": "破甲击.dds", "max_lv": 10, "desc": "Pancada brutal que atordoa o oponente."},
    {"id": 69, "name": "Dragão Voador (Heaven's Flame)", "name_en": "Heaven's Flame", "class_id": 0, "class_name": "Guerreiro", "type": "Ultimate em Área", "icon": "fa-solid fa-dragon", "icon_file": "天火狂龙.dds", "max_lv": 10, "desc": "Invoca o Dragão Celestial causando dano massivo e dobrando o dano recebido pelos alvos."},
    {"id": 70, "name": "Corte das Terras Altas", "name_en": "Highland Cleave", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Machado", "icon": "fa-solid fa-mountain", "icon_file": "开山斧.dds", "max_lv": 10, "desc": "Golpe pesado de machado causando dano frontal."},
    {"id": 71, "name": "Fissura", "name_en": "Fissure", "class_id": 0, "class_name": "Guerreiro", "type": "Dano de Fogo em Área", "icon": "fa-solid fa-fire-burner", "icon_file": "裂地击.dds", "max_lv": 10, "desc": "Esmaga o solo liberando magma e reduzindo a defesa de fogo dos inimigos."},
    {"id": 72, "name": "Corte Fantasma", "name_en": "Mage Bane", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Espada", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "断魂剑.dds", "max_lv": 10, "desc": "Ataque de espada que queima a mana do adversário."},
    {"id": 73, "name": "Caçador de Espíritos", "name_en": "Spirit Chaser", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Espada", "icon": "fa-solid fa-ghost", "icon_file": "追魂剑.dds", "max_lv": 10, "desc": "Dispara ondas cortantes de espada com velocidade ampliada."},
    {"id": 74, "name": "Golpe Atmosférico", "name_en": "Atmos Strike", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque de Espada", "icon": "fa-solid fa-burst", "icon_file": "风云破.dds", "max_lv": 10, "desc": "Corta o espaço aéreo causando dano físico explosivo."},
    {"id": 75, "name": "10 Mil Lâminas (Myriad Swords)", "name_en": "Myriad Sword Stance", "class_id": 0, "class_name": "Guerreiro", "type": "Ultimate em Área", "icon": "fa-solid fa-khanda", "icon_file": "万剑决.dds", "max_lv": 10, "desc": "Faz chover milhares de espadas astrais reduzindo o ataque físico e mágico dos inimigos."},
    {"id": 147, "name": "Mestria em Punhos", "name_en": "Fist Mastery", "class_id": 0, "class_name": "Guerreiro", "type": "Passiva / Mestria", "icon": "fa-solid fa-hand-fist", "icon_file": "拳套专精.dds", "max_lv": 10, "desc": "Aumenta o dano de armas de punho e garras."},
    {"id": 148, "name": "Mestria em Lanças", "name_en": "Spear Mastery", "class_id": 0, "class_name": "Guerreiro", "type": "Passiva / Mestria", "icon": "fa-solid fa-pen-nib", "icon_file": "长枪专精.dds", "max_lv": 10, "desc": "Aumenta o dano de armas de haste e lanças."},
    {"id": 157, "name": "Mestria em Espadas", "name_en": "Blade and Sword Mastery", "class_id": 0, "class_name": "Guerreiro", "type": "Passiva / Mestria", "icon": "fa-solid fa-khanda", "icon_file": "剑术专精.dds", "max_lv": 10, "desc": "Aumenta o dano de espadas e lâminas."},
    {"id": 162, "name": "Mestria em Machados", "name_en": "Axe and Hammer Mastery", "class_id": 0, "class_name": "Guerreiro", "type": "Passiva / Mestria", "icon": "fa-solid fa-gavel", "icon_file": "斧锤专精.dds", "max_lv": 10, "desc": "Aumenta o dano de machados, martelos e clavas."},
    {"id": 350, "name": "Sutra Interior (Magia)", "name_en": "Diamond Sutra", "class_id": 0, "class_name": "Guerreiro", "type": "Buff / Postura", "icon": "fa-solid fa-yin-yang", "icon_file": "易筋经.dds", "max_lv": 10, "desc": "Reduz a defesa física para aumentar enormemente a defesa mágica."},
    {"id": 351, "name": "Sutra Exterior (Física)", "name_en": "Great Diamond Sutra", "class_id": 0, "class_name": "Guerreiro", "type": "Buff / Postura", "icon": "fa-solid fa-shield-heart", "icon_file": "易骨经.dds", "max_lv": 10, "desc": "Reduz a defesa mágica para aumentar enormemente a defesa física."},

    # =========================================================================
    # MAGO (Classe 1 - Wizard)
    # =========================================================================
    {"id": 7, "name": "Coroa de Chamas", "name_en": "Crown of Flame", "class_id": 1, "class_name": "Mago", "type": "Magia de Fogo", "icon": "fa-solid fa-crown", "icon_file": "烈焰冠.dds", "max_lv": 10, "desc": "Cobre o alvo com uma coroa de fogo que queima continuamente ao longo do tempo."},
    {"id": 8, "name": "Tempestade de Chamas", "name_en": "Emberstorm", "class_id": 1, "class_name": "Mago", "type": "Magia de Fogo em Área", "icon": "fa-solid fa-volcano", "icon_file": "火雨.dds", "max_lv": 10, "desc": "Chuva de brasas em área atingindo múltiplos oponentes."},
    {"id": 9, "name": "Mestria em Água", "name_en": "Aqua Spirit", "class_id": 1, "class_name": "Mago", "type": "Passiva / Mestria", "icon": "fa-solid fa-droplet", "icon_file": "水系专精.dds", "max_lv": 10, "desc": "Aumenta permanentemente todo o dano causado por magias de água."},
    {"id": 10, "name": "Armadilha de Terra", "name_en": "Pitfall", "class_id": 1, "class_name": "Mago", "type": "Magia de Terra", "icon": "fa-solid fa-mountain", "icon_file": "陷地术.dds", "max_lv": 10, "desc": "Faz o solo afundar sob os pés do alvo causando dano contínuo de terra."},
    {"id": 53, "name": "Mestria em Fogo", "name_en": "Fire Mastery", "class_id": 1, "class_name": "Mago", "type": "Passiva / Mestria", "icon": "fa-solid fa-fire", "icon_file": "火系专精.dds", "max_lv": 10, "desc": "Aumenta permanentemente todo o dano causado por magias de fogo."},
    {"id": 81, "name": "Flecha de Chamas", "name_en": "Pyrogram", "class_id": 1, "class_name": "Mago", "type": "Magia de Fogo", "icon": "fa-solid fa-fire-flame-curved", "icon_file": "烈火符.dds", "max_lv": 10, "desc": "Dispara uma esfera flamejante veloz contra o alvo."},
    {"id": 84, "name": "Vontade da Fênix", "name_en": "Will of the Phoenix", "class_id": 1, "class_name": "Mago", "type": "Magia de Fogo", "icon": "fa-solid fa-dove", "icon_file": "凤翼天翔.dds", "max_lv": 10, "desc": "Invoca a fênix ardente que repele e incinera inimigos frontais."},
    {"id": 85, "name": "Flecha Divina de Chamas", "name_en": "Divine Pyrogram", "class_id": 1, "class_name": "Mago", "type": "Magia de Fogo", "icon": "fa-solid fa-fire-glow", "icon_file": "神火符.dds", "max_lv": 10, "desc": "Poderoso projétil de fogo concentrado de alto impacto."},
    {"id": 86, "name": "Sopro do Dragão", "name_en": "The Dragon's Breath", "class_id": 1, "class_name": "Mago", "type": "Magia Contínua em Área", "icon": "fa-solid fa-dragon", "icon_file": "龙息.dds", "max_lv": 10, "desc": "Cria um círculo de labaredas constantes ao redor do mago."},
    {"id": 87, "name": "Tempestade de Lâminas (Ultimate)", "name_en": "Blade Tempest", "class_id": 1, "class_name": "Mago", "type": "Ultimate de Fogo e Metal", "icon": "fa-solid fa-burst", "icon_file": "火刃风暴.dds", "max_lv": 10, "desc": "Conjura uma tempestade cataclísmica de fogo e espadas em área."},
    {"id": 88, "name": "Gotejamento d'Água", "name_en": "Gush", "class_id": 1, "class_name": "Mago", "type": "Magia de Água", "icon": "fa-solid fa-water", "icon_file": "涌泉.dds", "max_lv": 10, "desc": "Dispara um jato de água gelada que reduz a velocidade de movimento do alvo."},
    {"id": 89, "name": "Manancial", "name_en": "Wellspring Quaff", "class_id": 1, "class_name": "Mago", "type": "Buff de Mana", "icon": "fa-solid fa-bottle-droplet", "icon_file": "甘露.dds", "max_lv": 10, "desc": "Aumenta a velocidade de conjuração e fluxo de mana."},
    {"id": 90, "name": "Orvalho da Manhã", "name_en": "Morning Dew", "class_id": 1, "class_name": "Mago", "type": "Cura", "icon": "fa-solid fa-heart-pulse", "icon_file": "晨露.dds", "max_lv": 10, "desc": "Magia de cura aquática que recupera o HP do mago ou aliado."},
    {"id": 91, "name": "Lâmina de Gelo", "name_en": "Frostblade", "class_id": 1, "class_name": "Mago", "type": "Buff de Ataque", "icon": "fa-solid fa-icicles", "icon_file": "寒冰刃.dds", "max_lv": 10, "desc": "Encanta a arma do aliado adicionando dano de água a cada ataque físico."},
    {"id": 92, "name": "Prisão Glacial", "name_en": "Glacial Snare", "class_id": 1, "class_name": "Mago", "type": "Magia de Gelo", "icon": "fa-solid fa-snowflake", "icon_file": "冰封.dds", "max_lv": 10, "desc": "Congela as pernas do oponente impedindo seu movimento."},
    {"id": 93, "name": "Dragão de Gelo (Black Ice)", "name_en": "Black Ice Dragon Strike", "class_id": 1, "class_name": "Mago", "type": "Ultimate de Gelo", "icon": "fa-solid fa-dragon", "icon_file": "玄冰龙.dds", "max_lv": 10, "desc": "Invoca o dragão de gelo negro causando dano massivo em área."},
    {"id": 96, "name": "Armadura de Fogo", "name_en": "Pyroshell", "class_id": 1, "class_name": "Mago", "type": "Buff Mágico", "icon": "fa-solid fa-shield", "icon_file": "烈火甲.dds", "max_lv": 10, "desc": "Aumenta a defesa física e regeneração de HP."},
    {"id": 97, "name": "Chuva de Pedras", "name_en": "Stone Rain", "class_id": 1, "class_name": "Mago", "type": "Magia de Terra", "icon": "fa-solid fa-gem", "icon_file": "落石术.dds", "max_lv": 10, "desc": "Faz chover pedras pesadas esmagando o oponente."},
    {"id": 98, "name": "Tempestade de Areia", "name_en": "Sandstorm", "class_id": 1, "class_name": "Mago", "type": "Magia de Terra", "icon": "fa-solid fa-wind", "icon_file": "沙尘暴.dds", "max_lv": 10, "desc": "Furação de poeira e rochas que reduz a precisão do alvo."},
    {"id": 99, "name": "Ira da Montanha (Mountain's Seize)", "name_en": "Mountain's Seize", "class_id": 1, "class_name": "Mago", "type": "Ultimate de Terra", "icon": "fa-solid fa-mountain-sun", "icon_file": "泰山压顶.dds", "max_lv": 10, "desc": "Esmaga os inimigos com o peso de uma montanha atordoando todos em área."},
    {"id": 100, "name": "Passo no Ar / Teleporte", "name_en": "Distance Shrink", "class_id": 1, "class_name": "Mago", "type": "Teleporte", "icon": "fa-solid fa-person-falling-burst", "icon_file": "缩地术.dds", "max_lv": 1, "desc": "Teleporta instantaneamente o mago para frente."},
    {"id": 101, "name": "Mestria em Terra", "name_en": "Earthen Spirit", "class_id": 1, "class_name": "Mago", "type": "Passiva / Mestria", "icon": "fa-solid fa-mountain", "icon_file": "土系专精.dds", "max_lv": 10, "desc": "Aumenta permanentemente todo o dano causado por magias de terra."},
    {"id": 180, "name": "Armadura de Gelo", "name_en": "Glacial Embrace", "class_id": 1, "class_name": "Mago", "type": "Buff Mágico", "icon": "fa-solid fa-snowflake", "icon_file": "寒冰甲.dds", "max_lv": 10, "desc": "Aumenta a defesa de água e a velocidade de regeneração de mana."},
    {"id": 181, "name": "Barreira de Pedra", "name_en": "Stone Barrier", "class_id": 1, "class_name": "Mago", "type": "Buff Mágico", "icon": "fa-solid fa-cubes", "icon_file": "磐石甲.dds", "max_lv": 10, "desc": "Aumenta expressivamente a defesa física e defesa de terra."},
    {"id": 182, "name": "Granizo", "name_en": "Hailstorm", "class_id": 1, "class_name": "Mago", "type": "Magia de Água em Área", "icon": "fa-solid fa-cloud-showers-water", "icon_file": "冰雹.dds", "max_lv": 10, "desc": "Tempestade de granizo que congela e lentifica inimigos em área."},
    {"id": 183, "name": "Sutra de Magia", "name_en": "Essential Sutra", "class_id": 1, "class_name": "Mago", "type": "Buff de Conjuração", "icon": "fa-solid fa-bolt-lightning", "icon_file": "静心经.dds", "max_lv": 1, "desc": "Reduz o tempo de conjuração de todas as magias a zero durante alguns segundos."},
    {"id": 184, "name": "Força da Vontade (Silêncio)", "name_en": "Force of Will", "class_id": 1, "class_name": "Mago", "type": "Controle", "icon": "fa-solid fa-volume-xmark", "icon_file": "封印术.dds", "max_lv": 10, "desc": "Silencia e impede o inimigo de conjurar habilidades."},

    # =========================================================================
    # ESPIRITUALISTA / PSÍQUICO (Classe 2 - Psychic)
    # =========================================================================
    {"id": 1450, "name": "Força da Alma", "name_en": "Soulforce", "class_id": 2, "class_name": "Espiritualista", "type": "Passiva / Mestria", "icon": "fa-solid fa-ghost", "icon_file": "soulforce.dds", "max_lv": 10, "desc": "Amplifica o dano com base na alma do personagem."},
    {"id": 1451, "name": "Canhão Aquático", "name_en": "Aqua Cannon", "class_id": 2, "class_name": "Espiritualista", "type": "Magia de Água", "icon": "fa-solid fa-water", "icon_file": "aqua_cannon.dds", "max_lv": 10, "desc": "Dispara uma esfera aquática de alto impacto."},
    {"id": 1452, "name": "Vetor de Terra", "name_en": "Earth Vector", "class_id": 2, "class_name": "Espiritualista", "type": "Magia de Terra", "icon": "fa-solid fa-mountain", "icon_file": "earth_vector.dds", "max_lv": 10, "desc": "Lança estacas de pedra do solo atordoando o alvo."},
    {"id": 1453, "name": "Vontade Psíquica", "name_en": "Psychic Will", "class_id": 2, "class_name": "Espiritualista", "type": "Buff Defensivo", "icon": "fa-solid fa-shield", "icon_file": "psychic_will.dds", "max_lv": 10, "desc": "Concede imunidade a debuffs e dano físico temporário."},
    {"id": 1454, "name": "Maldição da Alma", "name_en": "Soul Curse", "class_id": 2, "class_name": "Espiritualista", "type": "Debuff", "icon": "fa-solid fa-skull", "icon_file": "soul_curse.dds", "max_lv": 10, "desc": "Amaldiçoa a alma do oponente causando dano reflexivo."},

    # =========================================================================
    # BÁRBARO (Classe 3 - Barbarian)
    # =========================================================================
    {"id": 12, "name": "Inchaço", "name_en": "Swell", "class_id": 3, "class_name": "Bárbaro", "type": "Buff de HP", "icon": "fa-solid fa-heart", "icon_file": "充血.dds", "max_lv": 10, "desc": "Aumenta a reserva de vitalidade do guerreiro selvagem."},
    {"id": 13, "name": "Armagedom (Ultimate)", "name_en": "Armageddon", "class_id": 3, "class_name": "Bárbaro", "type": "Ultimate de Sacrifício", "icon": "fa-solid fa-volcano", "icon_file": "毁天灭地.dds", "max_lv": 10, "desc": "Sacrifica metade do HP e MP para causar dano cataclísmico em área."},
    {"id": 82, "name": "Sangue Feroz / Inspiração", "name_en": "Beast King's Inspiration", "class_id": 3, "class_name": "Bárbaro", "type": "Buff em Grupo", "icon": "fa-solid fa-heart-pulse", "icon_file": "兽王鼓舞.dds", "max_lv": 10, "desc": "Aumenta consideravelmente o HP máximo de todo o grupo."},
    {"id": 83, "name": "Força dos Titãs", "name_en": "Strength of the Titans", "class_id": 3, "class_name": "Bárbaro", "type": "Buff em Grupo", "icon": "fa-solid fa-hand-fist", "icon_file": "泰坦之力.dds", "max_lv": 10, "desc": "Aumenta o ataque físico de todos os membros do grupo."},
    {"id": 102, "name": "Golpe do Rei das Feras", "name_en": "Stomp of the Beast King", "class_id": 3, "class_name": "Bárbaro", "type": "Ataque Físico", "icon": "fa-solid fa-paw", "icon_file": "虎击.dds", "max_lv": 10, "desc": "Pancada com a pata do tigre gerando ameaça."},
    {"id": 103, "name": "Mestria em Natação", "name_en": "Swimming Mastery", "class_id": 3, "class_name": "Bárbaro", "type": "Passiva", "icon": "fa-solid fa-person-swimming", "icon_file": "游泳专精.dds", "max_lv": 1, "desc": "Aumenta a velocidade de nado do tigre."},
    {"id": 104, "name": "Balanço Poderoso", "name_en": "Mighty Swing", "class_id": 3, "class_name": "Bárbaro", "type": "Ataque Físico", "icon": "fa-solid fa-gavel", "icon_file": "重锤.dds", "max_lv": 10, "desc": "Golpe com força bruta que pode atordoar o inimigo."},
    {"id": 105, "name": "Tempestade de Fogo", "name_en": "Firestorm", "class_id": 3, "class_name": "Bárbaro", "type": "Dano de Fogo em Área", "icon": "fa-solid fa-fire", "icon_file": "烈火暴.dds", "max_lv": 10, "desc": "Gera uma explosão flamejante ao redor do bárbaro."},
    {"id": 106, "name": "Desarmar / Penetrar", "name_en": "Penetrate Armor", "class_id": 3, "class_name": "Bárbaro", "type": "Debuff Físico", "icon": "fa-solid fa-shield-virus", "icon_file": "破甲.dds", "max_lv": 10, "desc": "Reduz a armadura física do oponente."},
    {"id": 107, "name": "Pancada do Urso", "name_en": "Slam", "class_id": 3, "class_name": "Bárbaro", "type": "Ataque Físico", "icon": "fa-solid fa-hammer", "icon_file": "熊击.dds", "max_lv": 10, "desc": "Pancada esmagadora que interrompe conjurações."},
    {"id": 108, "name": "Investida Bestial", "name_en": "Beastial Onslaught", "class_id": 3, "class_name": "Bárbaro", "type": "Ataque em Área", "icon": "fa-solid fa-bullseye", "icon_file": "兽冲.dds", "max_lv": 10, "desc": "Ataque giratório que atinge múltiplos inimigos e reduz a esquiva deles."},
    {"id": 109, "name": "Regeneração Feral", "name_en": "Feral Regeneration", "class_id": 3, "class_name": "Bárbaro", "type": "Passiva / Regeneração", "icon": "fa-solid fa-heart-circle-bolt", "icon_file": "野性回复.dds", "max_lv": 10, "desc": "Aumenta a regeneração passiva contínua de HP."},
    {"id": 111, "name": "Banho de Sangue", "name_en": "Blood Bath", "class_id": 3, "class_name": "Bárbaro", "type": "Buff de Precisão", "icon": "fa-solid fa-droplet", "icon_file": "浴血.dds", "max_lv": 10, "desc": "Sacrifica um pouco de HP para aumentar consideravelmente a precisão."},
    {"id": 112, "name": "Transformação em Tigre Branco", "name_en": "Change to White Tiger", "class_id": 3, "class_name": "Bárbaro", "type": "Transformação", "icon": "fa-solid fa-cat", "icon_file": "白虎变.dds", "max_lv": 10, "desc": "Transforma o personagem em um Tigre Branco aumentando HP, defesas e velocidade."},
    {"id": 149, "name": "Corrida da Fera", "name_en": "Alacrity of the Beast", "class_id": 3, "class_name": "Bárbaro", "type": "Mobilidade", "icon": "fa-solid fa-person-running", "icon_file": "兽影奔袭.dds", "max_lv": 10, "desc": "Aumenta a velocidade de movimento na forma de tigre."},
    {"id": 150, "name": "Mordida Selvagem (Aggro)", "name_en": "Flesh Ream", "class_id": 3, "class_name": "Bárbaro", "type": "Aggro / Sangramento", "icon": "fa-solid fa-paw", "icon_file": "撕咬.dds", "max_lv": 10, "desc": "Morde o alvo gerando enorme ameaça para manter a atenção dos monstros."},
    {"id": 151, "name": "Devorar", "name_en": "Devour", "class_id": 3, "class_name": "Bárbaro", "type": "Debuff de Armadura", "icon": "fa-solid fa-teeth-open", "icon_file": "吞噬.dds", "max_lv": 10, "desc": "Devora a carne do alvo reduzindo drasticamente a defesa física dele."},
    {"id": 152, "name": "Impacto da Onda", "name_en": "Surf Impact", "class_id": 3, "class_name": "Bárbaro", "type": "Dano em Área", "icon": "fa-solid fa-water", "icon_file": "浪击.dds", "max_lv": 10, "desc": "Dispara ondas que desaceleram todos os inimigos ao redor."},
    {"id": 153, "name": "Rachar (Sunder)", "name_en": "Sunder", "class_id": 3, "class_name": "Bárbaro", "type": "Dano em Área", "icon": "fa-solid fa-burst", "icon_file": "地裂.dds", "max_lv": 10, "desc": "Ataque em área que causa sangramento prolongado e regenera HP."},
    {"id": 154, "name": "Forma Bestial", "name_en": "Shapeshifting Intensity", "class_id": 3, "class_name": "Bárbaro", "type": "Passiva", "icon": "fa-solid fa-shield", "icon_file": "变身强化.dds", "max_lv": 10, "desc": "Aumenta o bônus de defesa da forma de tigre."},
    {"id": 155, "name": "Presas Venenosas", "name_en": "Poison Fang", "class_id": 3, "class_name": "Bárbaro", "type": "Buff de Madeira", "icon": "fa-solid fa-skull-crossbones", "icon_file": "毒牙.dds", "max_lv": 10, "desc": "Adiciona dano contínuo de madeira aos ataques físicos na forma animal."},
    {"id": 156, "name": "Aterrorizar", "name_en": "Frighten", "class_id": 3, "class_name": "Bárbaro", "type": "Debuff em Área", "icon": "fa-solid fa-ghost", "icon_file": "恐吓.dds", "max_lv": 10, "desc": "Assusta os inimigos ao redor reduzindo o poder de ataque deles."},
    {"id": 185, "name": "Rugido de Desafio", "name_en": "Roar", "class_id": 3, "class_name": "Bárbaro", "type": "Aggro em Área", "icon": "fa-solid fa-bullhorn", "icon_file": "咆哮.dds", "max_lv": 1, "desc": "Força todos os inimigos em área a atacarem o bárbaro instantaneamente."},
    {"id": 186, "name": "Casca de Tartaruga (Invoke)", "name_en": "Invoke the Spirit", "class_id": 3, "class_name": "Bárbaro", "type": "Buff Defensivo Extremo", "icon": "fa-solid fa-shield-cat", "icon_file": "玄武附体.dds", "max_lv": 1, "desc": "Reduz em 90% todo o dano recebido durante 20 segundos."},
    {"id": 188, "name": "Fúria Bestial", "name_en": "Beastial Rage", "class_id": 3, "class_name": "Bárbaro", "type": "Geração de Chi", "icon": "fa-solid fa-fire", "icon_file": "兽王怒火.dds", "max_lv": 1, "desc": "Gera Chi continuamente a cada golpe sofrido."},
    {"id": 195, "name": "Garrote / Asfixia", "name_en": "Garrotte", "class_id": 3, "class_name": "Bárbaro", "type": "Ataque Físico", "icon": "fa-solid fa-handcuffs", "icon_file": "绞杀.dds", "max_lv": 10, "desc": "Prende o pescoço do oponente causando dano contínuo de sangramento."},

    # =========================================================================
    # FEITICEIRA (Classe 4 - Venomancer)
    # =========================================================================
    {"id": 299, "name": "Escaravelho Venenoso (Ferrão)", "name_en": "Venomous Scarab", "class_id": 4, "class_name": "Feiticeira", "type": "Magia de Madeira", "icon": "fa-solid fa-bug", "icon_file": "剧毒蛊.dds", "max_lv": 10, "desc": "Dispara insetos venenosos causando dano contínuo de madeira."},
    {"id": 300, "name": "Escaravelho de Ferro", "name_en": "Ironwood Scarab", "class_id": 4, "class_name": "Feiticeira", "type": "Debuff Físico", "icon": "fa-solid fa-cubes", "icon_file": "铁木蛊.dds", "max_lv": 10, "desc": "Destrói a armadura física do oponente reduzindo sua defesa."},
    {"id": 301, "name": "Escaravelho Flamejante", "name_en": "Blazing Scarab", "class_id": 4, "class_name": "Feiticeira", "type": "Magia de Fogo", "icon": "fa-solid fa-fire", "icon_file": "烈炎蛊.dds", "max_lv": 10, "desc": "Insetos de fogo que causam dano flamejante ao longo do tempo."},
    {"id": 302, "name": "Escaravelho Glacial", "name_en": "Frost Scarab", "class_id": 4, "class_name": "Feiticeira", "type": "Magia de Água", "icon": "fa-solid fa-snowflake", "icon_file": "寒冰蛊.dds", "max_lv": 10, "desc": "Reduz a velocidade de movimento do alvo com frio congelante."},
    {"id": 303, "name": "Gás Nocivo", "name_en": "Noxious Gas", "class_id": 4, "class_name": "Feiticeira", "type": "Dano de Madeira em Área", "icon": "fa-solid fa-smog", "icon_file": "毒雾.dds", "max_lv": 10, "desc": "Nuvem de esporos venenosos atingindo múltiplos inimigos."},
    {"id": 304, "name": "Escaravelho da Sorte", "name_en": "Lucky Scarab", "class_id": 4, "class_name": "Feiticeira", "type": "Controle", "icon": "fa-solid fa-clover", "icon_file": "吉星蛊.dds", "max_lv": 10, "desc": "Atordoa o oponente com alta chance de crítico."},
    {"id": 305, "name": "Praga dos Gafanhotos (Ultimate)", "name_en": "Parasitic Nova", "class_id": 4, "class_name": "Feiticeira", "type": "Ultimate em Área", "icon": "fa-solid fa-locust", "icon_file": "万蛊食天.dds", "max_lv": 10, "desc": "Enxame devastador que sela e paralisa todos os inimigos na área."},
    {"id": 306, "name": "Armadura de Espinhos", "name_en": "Bramble Guard", "class_id": 4, "class_name": "Feiticeira", "type": "Buff Refletor", "icon": "fa-solid fa-shield-virus", "icon_file": "荆棘术.dds", "max_lv": 10, "desc": "Reflete parte do dano físico corpo-a-corpo de volta ao atacante."},
    {"id": 307, "name": "Transferir Vida", "name_en": "Metabolic Boost", "class_id": 4, "class_name": "Feiticeira", "type": "Suporte / Cura", "icon": "fa-solid fa-heart-pulse", "icon_file": "生命转换.dds", "max_lv": 10, "desc": "Converte mana em pontos de vida instantaneamente."},
    {"id": 308, "name": "Graça da Natureza", "name_en": "Nature's Grace", "class_id": 4, "class_name": "Feiticeira", "type": "Suporte de Mana", "icon": "fa-solid fa-leaf", "icon_file": "自然之恩.dds", "max_lv": 10, "desc": "Converte pontos de vida em mana."},
    {"id": 309, "name": "Transferir Chi (Lending Hand)", "name_en": "Lending Hand", "class_id": 4, "class_name": "Feiticeira", "type": "Suporte", "icon": "fa-solid fa-hand-holding-hand", "icon_file": "元气传递.dds", "max_lv": 1, "desc": "Transfere faíscas de Chi para um aliado da equipe."},
    {"id": 310, "name": "Manto de Espinhos", "name_en": "Bramble Hood", "class_id": 4, "class_name": "Feiticeira", "type": "Buff Defensivo", "icon": "fa-solid fa-shield", "icon_file": "荆棘阵.dds", "max_lv": 1, "desc": "Reduz o dano recebido em 75% e reflete dano físico ampliado."},
    {"id": 311, "name": "Transfusão de Alma", "name_en": "Soul Transfusion", "class_id": 4, "class_name": "Feiticeira", "type": "Equilíbrio", "icon": "fa-solid fa-scale-balanced", "icon_file": "灵魂转换.dds", "max_lv": 1, "desc": "Iguala as porcentagens de HP e MP da feiticeira."},
    {"id": 312, "name": "Forma da Raposa", "name_en": "Fox Form", "class_id": 4, "class_name": "Feiticeira", "type": "Transformação", "icon": "fa-solid fa-dog", "icon_file": "灵狐变.dds", "max_lv": 10, "desc": "Assume a forma astral da raposa ganhando velocidade e evasão."},
    {"id": 313, "name": "Golpe da Raposa", "name_en": "Fox Wallop", "class_id": 4, "class_name": "Feiticeira", "type": "Ataque Físico da Raposa", "icon": "fa-solid fa-paw", "icon_file": "狐击.dds", "max_lv": 10, "desc": "Ataque frontal rápido na forma de raposa."},
    {"id": 314, "name": "Névoa Desorientadora", "name_en": "Befuddling Mist", "class_id": 4, "class_name": "Feiticeira", "type": "Debuff de Precisão", "icon": "fa-solid fa-smog", "icon_file": "迷雾.dds", "max_lv": 10, "desc": "Reduz a precisão do oponente."},
    {"id": 315, "name": "Golpe Atordoante", "name_en": "Stunning Blow", "class_id": 4, "class_name": "Feiticeira", "type": "Controle da Raposa", "icon": "fa-solid fa-hand-fist", "icon_file": "击晕.dds", "max_lv": 10, "desc": "Atordoa o alvo com a pata da raposa."},
    {"id": 316, "name": "Suga-Alma / Dreno", "name_en": "Leech", "class_id": 4, "class_name": "Feiticeira", "type": "Dreno de Vida", "icon": "fa-solid fa-droplet", "icon_file": "吸血.dds", "max_lv": 10, "desc": "Drena a vida do oponente recuperando o HP da feiticeira."},
    {"id": 317, "name": "Consumir Espírito", "name_en": "Consume Spirit", "class_id": 4, "class_name": "Feiticeira", "type": "Dreno de Mana", "icon": "fa-solid fa-ghost", "icon_file": "吸魔.dds", "max_lv": 10, "desc": "Drena a mana do oponente."},
    {"id": 318, "name": "Esmagamento Maléfico", "name_en": "Malefic Crush", "class_id": 4, "class_name": "Feiticeira", "type": "Ultimate da Raposa", "icon": "fa-solid fa-volcano", "icon_file": "天狐怒火.dds", "max_lv": 10, "desc": "Ataque devastador em área na forma de raposa queimando a mana dos inimigos."},
    {"id": 319, "name": "Exílio (Purge)", "name_en": "Purge", "class_id": 4, "class_name": "Feiticeira", "type": "Remoção de Buffs", "icon": "fa-solid fa-eraser", "icon_file": "驱逐.dds", "max_lv": 1, "desc": "Remove instantaneamente todos os buffs e efeitos positivos do alvo."},
    {"id": 320, "name": "Ferida Cortante (Amplify)", "name_en": "Amplify Damage", "class_id": 4, "class_name": "Feiticeira", "type": "Amplificação de Dano", "icon": "fa-solid fa-heart-crack", "icon_file": "破甲蛊.dds", "max_lv": 10, "desc": "Faz o alvo receber dano aumentado de todas as fontes."},
    {"id": 321, "name": "Degeneração da Alma", "name_en": "Soul Degeneration", "class_id": 4, "class_name": "Feiticeira", "type": "Debuff de Regeneração", "icon": "fa-solid fa-skull", "icon_file": "阻滞.dds", "max_lv": 10, "desc": "Impede o alvo de regenerar HP naturalmente ou por poções."},
    {"id": 322, "name": "Esmagar Vigor", "name_en": "Crush Vigor", "class_id": 4, "class_name": "Feiticeira", "type": "Debuff de Chi", "icon": "fa-solid fa-battery-empty", "icon_file": "散元.dds", "max_lv": 10, "desc": "Reduz o Chi do oponente."},
    {"id": 323, "name": "Mestria em Natação", "name_en": "Swimming Mastery", "class_id": 4, "class_name": "Feiticeira", "type": "Passiva", "icon": "fa-solid fa-person-swimming", "icon_file": "游泳专精.dds", "max_lv": 1, "desc": "Aumenta a velocidade de nado da raposa."},
    {"id": 324, "name": "Mestria em Luta", "name_en": "Melee Mastery", "class_id": 4, "class_name": "Feiticeira", "type": "Passiva", "icon": "fa-solid fa-hand-back-fist", "icon_file": "近战专精.dds", "max_lv": 10, "desc": "Aumenta o ataque físico corpo-a-corpo na forma de raposa."},
    {"id": 325, "name": "Mestria em Madeira", "name_en": "Wood Mastery", "class_id": 4, "class_name": "Feiticeira", "type": "Passiva / Mestria", "icon": "fa-solid fa-tree", "icon_file": "木系专精.dds", "max_lv": 10, "desc": "Aumenta todo o dano mágico de madeira."},
    {"id": 328, "name": "Domesticar Mascote", "name_en": "Tame Beast", "class_id": 4, "class_name": "Feiticeira", "type": "Mascote", "icon": "fa-solid fa-heart", "icon_file": "驯服宠物.dds", "max_lv": 1, "desc": "Permite capturar monstros do mundo para servirem como mascotes de combate."},
    {"id": 329, "name": "Reviver Mascote", "name_en": "Revive Pet", "class_id": 4, "class_name": "Feiticeira", "type": "Mascote", "icon": "fa-solid fa-cross", "icon_file": "复活宠物.dds", "max_lv": 1, "desc": "Ressuscita uma mascote caída em combate."},
    {"id": 330, "name": "Curar Mascote", "name_en": "Heal Pet", "class_id": 4, "class_name": "Feiticeira", "type": "Mascote / Cura", "icon": "fa-solid fa-hand-holding-medical", "icon_file": "治疗宠物.dds", "max_lv": 10, "desc": "Recupera o HP da mascote da feiticeira."},
    {"id": 762, "name": "Passo no Vento (Summer Sprint)", "name_en": "Summer Sprint", "class_id": 4, "class_name": "Feiticeira", "type": "Mobilidade", "icon": "fa-solid fa-person-running", "icon_file": "神行百变.dds", "max_lv": 10, "desc": "Aumenta drasticamente a velocidade de corrida da feiticeira."},

    # =========================================================================
    # MERCENÁRIO (Classe 5 - Assassin)
    # =========================================================================
    {"id": 1400, "name": "Ataque Duplo", "name_en": "Twin Strike", "class_id": 5, "class_name": "Mercenário", "type": "Ataque Físico", "icon": "fa-solid fa-khanda", "icon_file": "twin_strike.dds", "max_lv": 10, "desc": "Golpe rápido duplo com as adagas."},
    {"id": 1401, "name": "Corte Sangrento", "name_en": "Bloodcut", "class_id": 5, "class_name": "Mercenário", "type": "Ataque Físico", "icon": "fa-solid fa-droplet", "icon_file": "bloodcut.dds", "max_lv": 10, "desc": "Corta pontos vitais gerando sangramento severo."},
    {"id": 1402, "name": "Picada Profunda", "name_en": "Deep Sting", "class_id": 5, "class_name": "Mercenário", "type": "Controle / Sono", "icon": "fa-solid fa-bed", "icon_file": "deep_sting.dds", "max_lv": 10, "desc": "Aplica sonífero no oponente deixando-o adormecido."},
    {"id": 1403, "name": "Andar nas Sombras (Furtividade)", "name_en": "Shadow Walk", "class_id": 5, "class_name": "Mercenário", "type": "Furtividade", "icon": "fa-solid fa-ghost", "icon_file": "shadow_walk.dds", "max_lv": 10, "desc": "Torna-se invisível para monstros e jogadores."},
    {"id": 1404, "name": "Fuga das Sombras", "name_en": "Shadow Escape", "class_id": 5, "class_name": "Mercenário", "type": "Furtividade em Combate", "icon": "fa-solid fa-person-walking-dashed-line-arrow-right", "icon_file": "shadow_escape.dds", "max_lv": 1, "desc": "Entra instantaneamente em furtividade mesmo estando em combate ativo."},
    {"id": 1405, "name": "Dança das Adagas", "name_en": "Tackling Slash", "class_id": 5, "class_name": "Mercenário", "type": "Imobilização", "icon": "fa-solid fa-shoe-prints", "icon_file": "tackling_slash.dds", "max_lv": 10, "desc": "Avança e imobiliza o oponente no chão."},
    {"id": 1406, "name": "Dragão Ascendente", "name_en": "Rising Dragon", "class_id": 5, "class_name": "Mercenário", "type": "Geração de Chi", "icon": "fa-solid fa-dragon", "icon_file": "rising_dragon.dds", "max_lv": 10, "desc": "Ataque especial que concede grande quantidade de Chi."},
    {"id": 1407, "name": "Mestria em Adagas", "name_en": "Dagger Mastery", "class_id": 5, "class_name": "Mercenário", "type": "Passiva / Mestria", "icon": "fa-solid fa-khanda", "icon_file": "dagger_mastery.dds", "max_lv": 10, "desc": "Aumenta o poder de ataque com adagas."},
    {"id": 1408, "name": "Ataque Submarino", "name_en": "Subsea Strike", "class_id": 5, "class_name": "Mercenário", "type": "Ultimate em Área", "icon": "fa-solid fa-water", "icon_file": "subsea_strike.dds", "max_lv": 10, "desc": "Ataque em área que amplifica o dano recebido por todos os inimigos."},

    # =========================================================================
    # ARQUEIRO (Classe 6 - Archer)
    # =========================================================================
    {"id": 234, "name": "Mira Certeira", "name_en": "Take Aim", "class_id": 6, "class_name": "Arqueiro", "type": "Ataque Carregado", "icon": "fa-solid fa-crosshairs", "icon_file": "百步穿杨.dds", "max_lv": 10, "desc": "Carrega o arco para disparar uma flecha de alta precisão e poder de ataque."},
    {"id": 235, "name": "Disparo Rápido", "name_en": "Quickshot", "class_id": 6, "class_name": "Arqueiro", "type": "Ataque Físico Veloz", "icon": "fa-solid fa-bolt", "icon_file": "连射.dds", "max_lv": 10, "desc": "Dispara flechas com alta velocidade de repetição."},
    {"id": 236, "name": "Flecha Repulsora", "name_en": "Knockback Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Controle / Repulsão", "icon": "fa-solid fa-arrow-right-from-bracket", "icon_file": "击退矢.dds", "max_lv": 10, "desc": "Empurra o alvo para longe mantendo a distância do arqueiro."},
    {"id": 237, "name": "Mirar Baixo", "name_en": "Aim Low", "class_id": 6, "class_name": "Arqueiro", "type": "Controle / Lenta", "icon": "fa-solid fa-arrow-down", "icon_file": "定身矢.dds", "max_lv": 10, "desc": "Prende as pernas do inimigo imobilizando-o temporariamente."},
    {"id": 238, "name": "Flecha Atordoante", "name_en": "Stunning Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Controle / Stun", "icon": "fa-solid fa-star", "icon_file": "击晕矢.dds", "max_lv": 10, "desc": "Atordoa o alvo ao acertar um ponto sensível."},
    {"id": 239, "name": "Disparo Mortal", "name_en": "Deadly Shot", "class_id": 6, "class_name": "Arqueiro", "type": "Ataque Poderoso", "icon": "fa-solid fa-skull", "icon_file": "致命矢.dds", "max_lv": 10, "desc": "Disparo perfurante que ignora parte da defesa física do alvo."},
    {"id": 240, "name": "Chuva de Flechas (Ultimate)", "name_en": "Barrage of Arrows", "class_id": 6, "class_name": "Arqueiro", "type": "Ultimate Contínua em Área", "icon": "fa-solid fa-cloud-showers-heavy", "icon_file": "箭阵.dds", "max_lv": 10, "desc": "Cria uma tempestade mortal e contínua de flechas chovendo sobre a área."},
    {"id": 241, "name": "Golpe do Relâmpago", "name_en": "Lightning Strike", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Metal", "icon": "fa-solid fa-bolt-lightning", "icon_file": "雷击矢.dds", "max_lv": 10, "desc": "Imbui a flecha com energia elétrica causando dano de metal à distância."},
    {"id": 242, "name": "Choque de Trovão", "name_en": "Thunder Shock", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Metal", "icon": "fa-solid fa-cloud-bolt", "icon_file": "惊雷矢.dds", "max_lv": 10, "desc": "Flecha metálica que reduz a defesa mágica de metal do oponente."},
    {"id": 243, "name": "Explosão Trovejante", "name_en": "Thunderous Blast", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Metal em Área", "icon": "fa-solid fa-burst", "icon_file": "雷光矢.dds", "max_lv": 10, "desc": "Explosão elétrica de metal atingindo múltiplos oponentes."},
    {"id": 244, "name": "Águia da Tempestade", "name_en": "Stormrage Eagleon", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Metal Contínuo", "icon": "fa-solid fa-feather", "icon_file": "风暴之鹰.dds", "max_lv": 10, "desc": "Invoca a águia celestial causando dano de metal contínuo no solo."},
    {"id": 245, "name": "Flecha Flamejante", "name_en": "Blazing Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Buff de Fogo", "icon": "fa-solid fa-fire", "icon_file": "烈火矢.dds", "max_lv": 10, "desc": "Adiciona dano de fogo constante a todos os disparos com arco."},
    {"id": 246, "name": "Flecha Congelante", "name_en": "Frost Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Água / Lenta", "icon": "fa-solid fa-snowflake", "icon_file": "寒冰矢.dds", "max_lv": 10, "desc": "Flecha embebida em gelo que reduz a velocidade do alvo."},
    {"id": 247, "name": "Flecha Venenosa", "name_en": "Vicious Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Dano de Madeira", "icon": "fa-solid fa-skull-crossbones", "icon_file": "毒矢.dds", "max_lv": 10, "desc": "Dispara flecha envenenada causando dano contínuo de madeira."},
    {"id": 248, "name": "Flecha Serrilhada", "name_en": "Serrated Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Sangramento", "icon": "fa-solid fa-droplet", "icon_file": "锯齿矢.dds", "max_lv": 10, "desc": "Causa ferimentos profundos de sangramento no alvo."},
    {"id": 249, "name": "Escudo Alado / Casulo", "name_en": "Winged Shell", "class_id": 6, "class_name": "Arqueiro", "type": "Escudo Absorvedor", "icon": "fa-solid fa-shield", "icon_file": "羽盾.dds", "max_lv": 10, "desc": "Cria um casulo de penas sagradas que absorve danos e regenera mana."},
    {"id": 250, "name": "Promessa Alada", "name_en": "Winged Pledge", "class_id": 6, "class_name": "Arqueiro", "type": "Ataque Corpo-a-Corpo", "icon": "fa-solid fa-hand-back-fist", "icon_file": "近身矢.dds", "max_lv": 10, "desc": "Golpe físico corpo-a-corpo para afastar inimigos colados."},
    {"id": 251, "name": "Envergadura de Asas", "name_en": "Wingspan", "class_id": 6, "class_name": "Arqueiro", "type": "Dano em Área", "icon": "fa-solid fa-feather-pointed", "icon_file": "羽翼伸展.dds", "max_lv": 10, "desc": "Abre as asas aladas atingindo e repelindo todos ao redor."},
    {"id": 252, "name": "Asas da Graça", "name_en": "Wings of Grace", "class_id": 6, "class_name": "Arqueiro", "type": "Buff de Velocidade", "icon": "fa-solid fa-wind", "icon_file": "神鹰羽翼.dds", "max_lv": 1, "desc": "Concede imunidade a efeitos de lentidão e aumenta a velocidade de corrida."},
    {"id": 253, "name": "Flecha Dente Afiado", "name_en": "Sharpened Tooth Arrow", "class_id": 6, "class_name": "Arqueiro", "type": "Debuff de HP Máximo", "icon": "fa-solid fa-teeth", "icon_file": "利齿矢.dds", "max_lv": 10, "desc": "Reduz temporariamente o HP máximo do alvo."},
    {"id": 254, "name": "Asas de Proteção", "name_en": "Wings of Protection", "class_id": 6, "class_name": "Arqueiro", "type": "Buff de Evasão", "icon": "fa-solid fa-shield-halved", "icon_file": "守护之翼.dds", "max_lv": 10, "desc": "Aumenta a esquiva e defesas contra golpes físicos."},
    {"id": 255, "name": "Benção das Asas", "name_en": "Winged Blessing", "class_id": 6, "class_name": "Arqueiro", "type": "Buff de Alcance", "icon": "fa-solid fa-arrows-to-eye", "icon_file": "神翼祝福.dds", "max_lv": 10, "desc": "Aumenta a distância de tiro e precisão do arqueiro."},
    {"id": 256, "name": "Mestria em Arcos", "name_en": "Bow Mastery", "class_id": 6, "class_name": "Arqueiro", "type": "Passiva / Mestria", "icon": "fa-solid fa-bow-arrow", "icon_file": "弓弩专精.dds", "max_lv": 10, "desc": "Aumenta todo o dano físico causado por arcos, balestras e fundas."},
    {"id": 274, "name": "Mestria em Voo Alado", "name_en": "Flight Mastery", "class_id": 6, "class_name": "Arqueiro", "type": "Passiva de Voo", "icon": "fa-solid fa-plane-departure", "icon_file": "飞行专精.dds", "max_lv": 10, "desc": "Aumenta a velocidade de voo das asas naturais dos alados."},

    # =========================================================================
    # SACERDOTE (Classe 7 - Cleric)
    # =========================================================================
    {"id": 11, "name": "Mestria em Metal", "name_en": "Metal Mastery", "class_id": 7, "class_name": "Sacerdote", "type": "Passiva / Mestria", "icon": "fa-solid fa-bolt", "icon_file": "金系专精.dds", "max_lv": 10, "desc": "Aumenta todo o dano mágico de metal do sacerdote."},
    {"id": 15, "name": "Feixe de Cura Cromático", "name_en": "Chromatic Healing Beam", "class_id": 7, "class_name": "Sacerdote", "type": "Cura em Grupo", "icon": "fa-solid fa-sun", "icon_file": "极光咒.dds", "max_lv": 10, "desc": "Feixe de luz que cura o sacerdote e os aliados em linha reta."},
    {"id": 16, "name": "Aura de Regeneração (Bolha Azul)", "name_en": "Regeneration Aura", "class_id": 7, "class_name": "Sacerdote", "type": "Ultimate de Cura Contínua", "icon": "fa-solid fa-circle-dot", "icon_file": "神光护体.dds", "max_lv": 10, "desc": "Cria um campo sagrado sustentado que cura e reduz todo o dano recebido pela equipe em 50%."},
    {"id": 17, "name": "Selo Elemental", "name_en": "Elemental Seal", "class_id": 7, "class_name": "Sacerdote", "type": "Debuff Mágico", "icon": "fa-solid fa-atom", "icon_file": "五行符.dds", "max_lv": 10, "desc": "Reduz as defesas mágicas de todos os cinco elementos no alvo."},
    {"id": 18, "name": "Ressurreição (Revive)", "name_en": "Revive", "class_id": 7, "class_name": "Sacerdote", "type": "Reviver", "icon": "fa-solid fa-cross", "icon_file": "还魂咒.dds", "max_lv": 10, "desc": "Traz um aliado derrotado de volta à vida restaurando EXP perdida."},
    {"id": 19, "name": "Escudo de Penas (Plume Shell)", "name_en": "Plume Shell", "class_id": 7, "class_name": "Sacerdote", "type": "Buff Defensivo", "icon": "fa-solid fa-shield-virus", "icon_file": "羽盾.dds", "max_lv": 10, "desc": "Cria barreira sagrada que absorve dano físico convertendo o dano em mana consumida."},
    {"id": 113, "name": "Benção do Coração Puro", "name_en": "Blessing of the Purehearted", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Rápida", "icon": "fa-solid fa-hand-holding-medical", "icon_file": "清心咒.dds", "max_lv": 10, "desc": "Cura rápida de conjuração veloz para emergências."},
    {"id": 114, "name": "Prece da Calmaria (Ironheart)", "name_en": "Ironheart Blessing", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Contínua / HoT", "icon": "fa-solid fa-hands-holding", "icon_file": "静心符.dds", "max_lv": 10, "desc": "Aplica bênção regenerativa que cura o aliado continuamente ao longo de 15 segundos (acumulável)."},
    {"id": 115, "name": "Onda de Vitalidade", "name_en": "Wellspring Surge", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Maior", "icon": "fa-solid fa-heart-circle-plus", "icon_file": "醍醐灌顶.dds", "max_lv": 10, "desc": "Cura instantânea de alto valor de pontos de vida."},
    {"id": 116, "name": "Corrente de Rejuvenescimento", "name_en": "Stream of Rejuvenation", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Poderosa", "icon": "fa-solid fa-droplet", "icon_file": "醍醐.dds", "max_lv": 10, "desc": "Restaura uma enorme quantidade de vida ao alvo."},
    {"id": 117, "name": "Selo Dimensional", "name_en": "Dimensional Seal", "class_id": 7, "class_name": "Sacerdote", "type": "Debuff Físico", "icon": "fa-solid fa-shield-halved", "icon_file": "定身符.dds", "max_lv": 10, "desc": "Reduz a defesa física do oponente facilitando o dano de guerreiros e arqueiros."},
    {"id": 118, "name": "Selo Silencioso", "name_en": "Silent Seal", "class_id": 7, "class_name": "Sacerdote", "type": "Silêncio / Mute", "icon": "fa-solid fa-volume-xmark", "icon_file": "封印符.dds", "max_lv": 10, "desc": "Impede o adversário de usar habilidades ou magias."},
    {"id": 119, "name": "Selo Cromático (Sono)", "name_en": "Chromatic Seal", "class_id": 7, "class_name": "Sacerdote", "type": "Controle / Sono", "icon": "fa-solid fa-bed", "icon_file": "睡眠符.dds", "max_lv": 10, "desc": "Adormece o alvo impedindo qualquer ação até sofrer dano."},
    {"id": 120, "name": "Espírito de Vanguarda / Ferro", "name_en": "Vanguard Spirit", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-shield-heart", "icon_file": "坚甲符.dds", "max_lv": 10, "desc": "Aumenta a defesa física de todos os membros do grupo."},
    {"id": 121, "name": "Espírito Égide", "name_en": "Aegis Spirit", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-shield", "icon_file": "聚神符.dds", "max_lv": 10, "desc": "Aumenta a defesa mágica de todos os membros do grupo."},
    {"id": 122, "name": "Renovação Exaltada", "name_en": "Exalted Renewal", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-heart-pulse", "icon_file": "神灵符.dds", "max_lv": 10, "desc": "Aumenta a regeneração de HP e MP de todo o grupo."},
    {"id": 123, "name": "Ira Celestial (Bolha Vermelha)", "name_en": "Heaven's Wrath", "class_id": 7, "class_name": "Sacerdote", "type": "Ultimate de Ataque em Área", "icon": "fa-solid fa-circle-radiation", "icon_file": "狂雷天劫.dds", "max_lv": 10, "desc": "Campo sagrado ofensivo que amplia a velocidade de ataque e conjuração de todos os aliados em 20%."},
    {"id": 124, "name": "Empoderamento Arcano", "name_en": "Arcane Empowerment", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "灵助符.dds", "max_lv": 10, "desc": "Aumenta o ataque mágico de todos os companheiros do grupo."},
    {"id": 125, "name": "Flecha de Pluma", "name_en": "Plume Shot", "class_id": 7, "class_name": "Sacerdote", "type": "Dano Mágico / Físico", "icon": "fa-solid fa-feather-pointed", "icon_file": "羽箭.dds", "max_lv": 10, "desc": "Dispara plumas sagradas cortantes causando dano físico à distância."},
    {"id": 126, "name": "Plumas Cortantes", "name_en": "Razor Feathers", "class_id": 7, "class_name": "Sacerdote", "type": "Dano Físico em Área", "icon": "fa-solid fa-feather", "icon_file": "羽刃.dds", "max_lv": 10, "desc": "Lança lâminas de penas cortando todos os inimigos próximos."},
    {"id": 127, "name": "Redemoinho", "name_en": "Whirlwind", "class_id": 7, "class_name": "Sacerdote", "type": "Dano de Metal / Lenta", "icon": "fa-solid fa-tornado", "icon_file": "龙卷风.dds", "max_lv": 10, "desc": "Cria um redemoinho elétrico causando dano de metal e desacelerando o alvo."},
    {"id": 128, "name": "Esfera de Trovão", "name_en": "Thunderball", "class_id": 7, "class_name": "Sacerdote", "type": "Dano de Metal Contínuo", "icon": "fa-solid fa-bolt", "icon_file": "雷球.dds", "max_lv": 10, "desc": "Esfera de eletricidade que eletrocuta continuamente o inimigo."},
    {"id": 129, "name": "Beijo da Sereia", "name_en": "Siren's Kiss", "class_id": 7, "class_name": "Sacerdote", "type": "Dano de Metal em Área", "icon": "fa-solid fa-kiss-wink-heart", "icon_file": "神雷.dds", "max_lv": 10, "desc": "Explosão de eletricidade sagrada atingindo múltiplos alvos."},
    {"id": 130, "name": "Tempestade de Raios (Tempest)", "name_en": "Tempest", "class_id": 7, "class_name": "Sacerdote", "type": "Ultimate de Metal em Área", "icon": "fa-solid fa-cloud-bolt", "icon_file": "雷霆万钧.dds", "max_lv": 10, "desc": "Faz cair múltiplos raios do céu desintegrando os inimigos com chance de lentidão."},
    {"id": 163, "name": "Raio Sagrado (Wield Thunder)", "name_en": "Wield Thunder", "class_id": 7, "class_name": "Sacerdote", "type": "Dano de Metal Pesado", "icon": "fa-solid fa-bolt-lightning", "icon_file": "掌心雷.dds", "max_lv": 10, "desc": "Descarrega um raio de alto impacto causando enorme dano de metal."},
    {"id": 189, "name": "Purificação (Purify)", "name_en": "Purify", "class_id": 7, "class_name": "Sacerdote", "type": "Remoção de Debuffs", "icon": "fa-solid fa-sparkles", "icon_file": "玄净咒.dds", "max_lv": 10, "desc": "Remove todos os debuffs, maldições, venenos e efeitos negativos do aliado."},
    {"id": 190, "name": "Mestria em Voo Alado", "name_en": "Flight Mastery", "class_id": 7, "class_name": "Sacerdote", "type": "Passiva de Voo", "icon": "fa-solid fa-plane-departure", "icon_file": "飞行专精.dds", "max_lv": 10, "desc": "Aumenta a velocidade de voo das asas naturais."},
    {"id": 191, "name": "Concha Mágica", "name_en": "Magic Shell", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-shield", "icon_file": "魔甲符.dds", "max_lv": 10, "desc": "Aumenta a defesa mágica de todos os membros do grupo."},
    {"id": 192, "name": "Selo do Guardião Celestial", "name_en": "Celestial Guardian's Seal", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-shield-heart", "icon_file": "无极聚神符.dds", "max_lv": 10, "desc": "Aumenta as defesas física e mágica de toda a equipe durante 1 hora."},
    {"id": 193, "name": "Dádiva Espiritual", "name_en": "Spirit's Gift", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-wand-magic", "icon_file": "无极灵助符.dds", "max_lv": 10, "desc": "Aumenta o ataque mágico de toda a equipe durante 1 hora."},
    {"id": 194, "name": "Grande Aura Protetora", "name_en": "Greater Protective Aura", "class_id": 7, "class_name": "Sacerdote", "type": "Buff em Grupo", "icon": "fa-solid fa-certificate", "icon_file": "无极神灵符.dds", "max_lv": 10, "desc": "Aumenta a regeneração de HP e MP de toda a equipe durante 1 hora."},

    # =========================================================================
    # ARCANO (Classe 8 - Seeker)
    # =========================================================================
    {"id": 1500, "name": "Busca Corações", "name_en": "Heartseeker", "class_id": 8, "class_name": "Arcano", "type": "Ataque Físico / Metal", "icon": "fa-solid fa-khanda", "icon_file": "heartseeker.dds", "max_lv": 10, "desc": "Lança uma onda de choque de metal com a espada."},
    {"id": 1501, "name": "Afinidade de Lâmina", "name_en": "Blade Affinity", "class_id": 8, "class_name": "Arcano", "type": "Buff de Conjuração", "icon": "fa-solid fa-bolt", "icon_file": "blade_affinity.dds", "max_lv": 10, "desc": "Aumenta a velocidade de conjuração das técnicas de espada."},
    {"id": 1502, "name": "Vórtice de Yataghan", "name_en": "Yataghan Vortex", "class_id": 8, "class_name": "Arcano", "type": "Ultimate Contínua em Área", "icon": "fa-solid fa-tornado", "icon_file": "yataghan_vortex.dds", "max_lv": 10, "desc": "Cria um turbilhão constante de lâminas giratórias ao redor do arcano."},
    {"id": 1503, "name": "Mestria em Sabres", "name_en": "Saber Mastery", "class_id": 8, "class_name": "Arcano", "type": "Passiva / Mestria", "icon": "fa-solid fa-khanda", "icon_file": "saber_mastery.dds", "max_lv": 10, "desc": "Aumenta todo o dano físico com sabres e espadas."},

    # =========================================================================
    # MÍSTICO (Classe 9 - Mystic)
    # =========================================================================
    {"id": 1600, "name": "Vingança da Natureza", "name_en": "Nature's Vengeance", "class_id": 9, "class_name": "Místico", "type": "Magia de Madeira", "icon": "fa-solid fa-seedling", "icon_file": "natures_vengeance.dds", "max_lv": 10, "desc": "Dispara projéteis de energia floral causando dano de madeira."},
    {"id": 1601, "name": "Névoa Giratória", "name_en": "Swirling Mist", "class_id": 9, "class_name": "Místico", "type": "Dano de Madeira / Lenta", "icon": "fa-solid fa-smog", "icon_file": "swirling_mist.dds", "max_lv": 10, "desc": "Névoa de esporos que desacelera o oponente."},
    {"id": 1602, "name": "Mestria em Madeira", "name_en": "Wood Mastery", "class_id": 9, "class_name": "Místico", "type": "Passiva / Mestria", "icon": "fa-solid fa-tree", "icon_file": "wood_mastery.dds", "max_lv": 10, "desc": "Aumenta permanentemente o dano de magias de madeira do místico."},
    {"id": 1603, "name": "Invocar Chihyu", "name_en": "Summon Devil Chihyu", "class_id": 9, "class_name": "Místico", "type": "Invocação de Pet", "icon": "fa-solid fa-dragon", "icon_file": "summon_chihyu.dds", "max_lv": 10, "desc": "Invoca o guerreiro guardião elemental Chihyu para lutar ao seu lado."},
    {"id": 1604, "name": "Invocar Senhora da Tempestade", "name_en": "Summon Storm Mistress", "class_id": 9, "class_name": "Místico", "type": "Invocação Mágica", "icon": "fa-solid fa-cloud-bolt", "icon_file": "summon_storm.dds", "max_lv": 10, "desc": "Invoca a senhora das tempestades com magias elétricas de metal."},
    {"id": 1605, "name": "Invocar Fada Curativa", "name_en": "Summon Healing Sprite", "class_id": 9, "class_name": "Místico", "type": "Invocação de Suporte", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "summon_sprite.dds", "max_lv": 10, "desc": "Invoca uma fada floral que cura continuamente o grupo."},

    # =========================================================================
    # RETALHADOR (Classe 10 - Duskblade)
    # =========================================================================
    {"id": 1700, "name": "Lâmina Oculta", "name_en": "Hidden Blade", "class_id": 10, "class_name": "Retalhador", "type": "Ataque Físico", "icon": "fa-solid fa-moon", "icon_file": "hidden_blade.dds", "max_lv": 10, "desc": "Golpe rápido lunar com a foice."},
    {"id": 1701, "name": "Dança da Foice", "name_en": "Scythe Dance", "class_id": 10, "class_name": "Retalhador", "type": "Ataque em Área", "icon": "fa-solid fa-burst", "icon_file": "scythe_dance.dds", "max_lv": 10, "desc": "Gira a foice atingindo todos os alvos ao redor."},

    # =========================================================================
    # TORMENTADOR (Classe 11 - Stormbringer)
    # =========================================================================
    {"id": 1750, "name": "Trovão Trovejante", "name_en": "Thundering Roar", "class_id": 11, "class_name": "Tormentador", "type": "Magia de Metal / Água", "icon": "fa-solid fa-cloud-bolt", "icon_file": "thundering_roar.dds", "max_lv": 10, "desc": "Dispara orbes elementais de tempestade e gelo."},

    # =========================================================================
    # HABILIDADES COMUNS / PRODUÇÃO / CHI (-1 - Common / Global)
    # =========================================================================
    {"id": 158, "name": "Forjador de Armas (Blacksmith)", "name_en": "Blacksmith", "class_id": -1, "class_name": "Comum", "type": "Produção / Forja", "icon": "fa-solid fa-hammer", "icon_file": "铁匠.dds", "max_lv": 10, "desc": "Habilidade de forjar armas de combate."},
    {"id": 159, "name": "Alfaiate de Armaduras (Tailor)", "name_en": "Tailor", "class_id": -1, "class_name": "Comum", "type": "Produção / Forja", "icon": "fa-solid fa-shirt", "icon_file": "裁缝.dds", "max_lv": 10, "desc": "Habilidade de costurar armaduras e roupas."},
    {"id": 160, "name": "Artesão de Acessórios (Craftsman)", "name_en": "Craftsman", "class_id": -1, "class_name": "Comum", "type": "Produção / Forja", "icon": "fa-solid fa-gem", "icon_file": "巧匠.dds", "max_lv": 10, "desc": "Habilidade de confeccionar anéis, colares e ornamentos."},
    {"id": 161, "name": "Boticário de Poções (Apothecary)", "name_en": "Apothecary", "class_id": -1, "class_name": "Comum", "type": "Produção / Alquimia", "icon": "fa-solid fa-flask", "icon_file": "药师.dds", "max_lv": 10, "desc": "Habilidade de criar poções e elixires medicinais."},
    {"id": 167, "name": "Portal da Cidade (Town Portal)", "name_en": "Town Portal", "class_id": -1, "class_name": "Comum", "type": "Teleporte de Retorno", "icon": "fa-solid fa-archway", "icon_file": "回城术.dds", "max_lv": 1, "desc": "Teleporta o personagem de volta para a cidade mais próxima."},
    {"id": 232, "name": "Explosão de Chi 1", "name_en": "Spark Eruption 1", "class_id": -1, "class_name": "Comum", "type": "Chi / Cultivo", "icon": "fa-solid fa-sun", "icon_file": "爆气1.dds", "max_lv": 1, "desc": "Consome 1 faísca de Chi para amplificar temporariamente o ataque e conceder invulnerabilidade momentânea."},
    {"id": 233, "name": "Explosão de Chi 2", "name_en": "Spark Eruption 2", "class_id": -1, "class_name": "Comum", "type": "Chi / Cultivo", "icon": "fa-solid fa-sun", "icon_file": "爆气2.dds", "max_lv": 1, "desc": "Consome 2 faíscas de Chi para amplificar enormemente os atributos marciais."},
    {"id": 372, "name": "Explosão de Chi Imortal (God)", "name_en": "Celestial Spark Eruption", "class_id": -1, "class_name": "Comum", "type": "Chi / Cultivo God", "icon": "fa-solid fa-certificate", "icon_file": "爆气1.dds", "max_lv": 1, "desc": "Consome 3 faíscas de Chi. Concede imensa amplificação de ataque mágico/físico e regeneração de vida."},
    {"id": 373, "name": "Explosão de Chi Demoníaco (Evil)", "name_en": "Demonic Spark Eruption", "class_id": -1, "class_name": "Comum", "type": "Chi / Cultivo Evil", "icon": "fa-solid fa-fire", "icon_file": "爆气2.dds", "max_lv": 1, "desc": "Consome 3 faíscas de Chi. Concede extrema velocidade de ataque e poder destrutivo."}
]


class SurfacesIconManager:
    """Gerencia atlas DDS de iconset, mapeamento de coordenadas e extração em tempo real de PNGs 64x64"""
    def __init__(self, workspace_root: str = "f:/Python_C_Projects/PWSource1.5.3"):
        self.workspace_root = workspace_root
        self.iconset_cache: Dict[str, Dict[str, Any]] = {}
        self.png_crop_cache: Dict[Tuple[str, str], bytes] = {}
        self.atlas_images: Dict[str, Any] = {}

    def _get_iconset_dir_for_realm(self, realm_id: str) -> Optional[str]:
        data_env = os.getenv("PW_DATA_DIR", "")
        possible_dirs = [
            f"/app/data/{realm_id}/surfaces/iconset",
            f"/data/{realm_id}/surfaces/iconset",
            os.path.join(data_env, realm_id, "surfaces", "iconset") if data_env else "",
            os.path.join(os.path.dirname(__file__), "..", "..", "data", realm_id, "surfaces", "iconset"),
            os.path.join(os.path.dirname(__file__), "data", realm_id, "surfaces", "iconset"),
            os.path.join(self.workspace_root, "pw-universal-server", "data", realm_id, "surfaces", "iconset"),
            os.path.join(self.workspace_root, "data", realm_id, "surfaces", "iconset"),
            os.path.join(self.workspace_root, "pw-universal-server", "data", "realm_126", "surfaces", "iconset"),
            os.path.join("F:/Games/perfectworld_126/element/surfaces.pck.files/surfaces/iconset"),
        ]
        for d in possible_dirs:
            if d and os.path.exists(d) and os.path.isdir(d):
                return d
        return None

    def load_realm_iconset(self, realm_id: str) -> Optional[Dict[str, Any]]:
        """Carrega e indexa os atlas de ícones do iconset em memória"""
        if realm_id in self.iconset_cache:
            return self.iconset_cache[realm_id]

        iconset_dir = self._get_iconset_dir_for_realm(realm_id)
        if not iconset_dir:
            return None

        result = {
            "items_map": {},
            "skills_map": {},
            "iconset_dir": iconset_dir
        }

        # 1. Carrega Atlas de Itens Masculino / Geral (iconlist_ivtrm.dds + txt)
        # No formato do PW: Linha 1=largura(32), Linha 2=altura(32), Linha 3=linhas(53), Linha 4=colunas por linha(64)
        ivtr_dds = os.path.join(iconset_dir, "iconlist_ivtrm.dds")
        ivtr_txt = os.path.join(iconset_dir, "iconlist_ivtrm.txt")
        if os.path.exists(ivtr_dds) and os.path.exists(ivtr_txt):
            try:
                with open(ivtr_txt, "r", encoding="gbk", errors="ignore") as f:
                    lines = [l.strip() for l in f.readlines()]
                if len(lines) >= 4:
                    icon_w = int(lines[0])
                    icon_h = int(lines[1])
                    row_count = int(lines[2])
                    col_count = int(lines[3])
                    icon_names = lines[4:]

                    items_map = {}
                    for idx, name in enumerate(icon_names):
                        if name:
                            col = idx % col_count
                            row = idx // col_count
                            items_map[name.lower()] = (col, row, icon_w, icon_h, ivtr_dds)
                            items_map[name] = (col, row, icon_w, icon_h, ivtr_dds)

                    # Registra default unknown.dds (idx 0)
                    items_map["__default__"] = (0, 0, icon_w, icon_h, ivtr_dds)
                    result["items_map"] = items_map
            except Exception as e:
                print(f"[SurfacesIconManager] Aviso ao ler iconlist_ivtrm: {e}")

        # 1.1 Carrega Atlas Feminino (iconlist_ivtrf.dds + txt) como extensão se disponível
        ivtrf_dds = os.path.join(iconset_dir, "iconlist_ivtrf.dds")
        ivtrf_txt = os.path.join(iconset_dir, "iconlist_ivtrf.txt")
        if os.path.exists(ivtrf_dds) and os.path.exists(ivtrf_txt):
            try:
                with open(ivtrf_txt, "r", encoding="gbk", errors="ignore") as f:
                    lines_f = [l.strip() for l in f.readlines()]
                if len(lines_f) >= 4:
                    icon_w = int(lines_f[0])
                    icon_h = int(lines_f[1])
                    col_count = int(lines_f[3])
                    for idx, name in enumerate(lines_f[4:]):
                        if name and name not in result["items_map"]:
                            col = idx % col_count
                            row = idx // col_count
                            result["items_map"][name.lower()] = (col, row, icon_w, icon_h, ivtrf_dds)
                            result["items_map"][name] = (col, row, icon_w, icon_h, ivtrf_dds)
            except Exception as e:
                print(f"[SurfacesIconManager] Aviso ao ler iconlist_ivtrf: {e}")

        # 2. Carrega Atlas de Skills (iconlist_skill.dds + txt)
        # Linha 1=32, Linha 2=32, Linha 3=19 linhas, Linha 4=32 colunas por linha
        skill_dds = os.path.join(iconset_dir, "iconlist_skill.dds")
        skill_txt = os.path.join(iconset_dir, "iconlist_skill.txt")
        if os.path.exists(skill_dds) and os.path.exists(skill_txt):
            try:
                with open(skill_txt, "r", encoding="gbk", errors="ignore") as f:
                    lines = [l.strip() for l in f.readlines()]
                if len(lines) >= 4:
                    icon_w = int(lines[0])
                    icon_h = int(lines[1])
                    row_count = int(lines[2])
                    col_count = int(lines[3])
                    icon_names = lines[4:]

                    skills_map = {}
                    for idx, name in enumerate(icon_names):
                        if name:
                            col = idx % col_count
                            row = idx // col_count
                            skills_map[name.lower()] = (col, row, icon_w, icon_h, skill_dds)
                            skills_map[name] = (col, row, icon_w, icon_h, skill_dds)

                    # Registra default unknown.dds (idx 0)
                    skills_map["__default__"] = (0, 0, icon_w, icon_h, skill_dds)
                    result["skills_map"] = skills_map
            except Exception as e:
                print(f"[SurfacesIconManager] Aviso ao ler iconlist_skill: {e}")

        self.iconset_cache[realm_id] = result
        return result

    def _get_atlas_image(self, atlas_path: str):
        if not HAS_PIL or not Image:
            return None
        if atlas_path not in self.atlas_images:
            if os.path.exists(atlas_path):
                self.atlas_images[atlas_path] = Image.open(atlas_path)
            else:
                return None
        return self.atlas_images[atlas_path]

    def get_item_icon_png(self, realm_id: str, icon_file: str) -> Optional[bytes]:
        """Recorta e retorna os bytes em PNG 64x64 de um ícone de item a partir do atlas"""
        if not HAS_PIL:
            return None

        clean_name = (icon_file or "").replace("/", "\\").split("\\")[-1].strip()
        cache_key = (realm_id, clean_name.lower())
        if cache_key in self.png_crop_cache:
            return self.png_crop_cache[cache_key]

        iconset = self.load_realm_iconset(realm_id)
        if not iconset:
            return None

        items_map = iconset.get("items_map", {})
        coords = items_map.get(clean_name) or items_map.get(clean_name.lower()) or items_map.get("__default__")
        if not coords:
            return None

        col, row, w, h, atlas_path = coords
        im = self._get_atlas_image(atlas_path)
        if not im:
            return None

        try:
            box = (col * w, row * h, (col + 1) * w, (row + 1) * h)
            crop_img = im.crop(box)
            # Redimensiona para 64x64 mantendo pixel art nítido
            crop_64 = crop_img.resize((64, 64), Image.NEAREST)

            buf = io.BytesIO()
            crop_64.save(buf, format="PNG")
            png_bytes = buf.getvalue()

            self.png_crop_cache[cache_key] = png_bytes
            return png_bytes
        except Exception as e:
            print(f"[SurfacesIconManager] Erro ao recortar ícone {clean_name}: {e}")
            return None

    def get_skill_icon_png(self, realm_id: str, icon_file: str) -> Optional[bytes]:
        """Recorta e retorna os bytes em PNG 64x64 de uma habilidade a partir do atlas de skills"""
        if not HAS_PIL:
            return None

        clean_name = (icon_file or "").replace("/", "\\").split("\\")[-1].strip()
        cache_key = (realm_id, f"skill_{clean_name.lower()}")
        if cache_key in self.png_crop_cache:
            return self.png_crop_cache[cache_key]

        iconset = self.load_realm_iconset(realm_id)
        if not iconset:
            return None

        skills_map = iconset.get("skills_map", {})
        coords = skills_map.get(clean_name) or skills_map.get(clean_name.lower()) or skills_map.get("__default__")
        if not coords:
            return None

        col, row, w, h, atlas_path = coords
        im = self._get_atlas_image(atlas_path)
        if not im:
            return None

        try:
            box = (col * w, row * h, (col + 1) * w, (row + 1) * h)
            crop_img = im.crop(box)
            # Redimensiona para 64x64 mantendo pixel art nítido
            crop_64 = crop_img.resize((64, 64), Image.NEAREST)

            buf = io.BytesIO()
            crop_64.save(buf, format="PNG")
            png_bytes = buf.getvalue()

            self.png_crop_cache[cache_key] = png_bytes
            return png_bytes
        except Exception as e:
            print(f"[SurfacesIconManager] Erro ao recortar skill {clean_name}: {e}")
            return None


class ElementsDecoder:
    def __init__(self, workspace_root: str = "f:/Python_C_Projects/PWSource1.5.3"):
        self.workspace_root = workspace_root
        self.realms_cache: Dict[str, Dict[int, Dict[str, Any]]] = {}
        self.realms_skills_cache: Dict[str, Dict[int, Dict[str, Any]]] = {}
        self.popular_items_cache: Dict[int, Dict[str, Any]] = {}
        self.icon_manager = SurfacesIconManager(workspace_root=workspace_root)
        self._init_popular_items()

    def _init_popular_items(self):
        """Catálogo de itens populares com mapeamento de arquivos DDS do surfaces"""
        defaults = [
            # Armas Iniciais e Notáveis
            {"id": 2097, "name": "Espada de Madeira", "type": "Arma (Espada)", "category": "Arma", "level": 1, "quality": "normal", "icon": "fa-solid fa-khanda", "icon_file": "木剑.dds", "atk_phys": "8-12", "desc": "Espada simples de madeira utilizada no treinamento de novos guerreiros."},
            {"id": 6, "name": "☆Lâmina com Ponta de Aço", "type": "Arma (Espada)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-khanda", "icon_file": "钢刀.dds", "atk_phys": "15-28", "desc": "Espada forjada em aço leve com ponta afiada."},
            {"id": 8, "name": "☆Arco de Madeira Longo", "type": "Arma (Arco)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-bullseye", "icon_file": "硬木短弓.dds", "atk_phys": "18-35", "desc": "Arco de madeira envergada para tiros precisos."},
            {"id": 10, "name": "☆Machado de Batalha", "type": "Arma (Machado)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-axe", "icon_file": "生铁双斧.dds", "atk_phys": "22-45", "desc": "Machado pesado de duas mãos ideal para bárbaros ferozes."},
            {"id": 2867, "name": "☆Cajado dos Espíritos", "type": "Arma (Cajado)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "桃木杖.dds", "atk_magic": "30-50", "desc": "Cajado encantado com ressonância mágica natural."},
            {"id": 2258, "name": "☆Martelo da Besta", "type": "Arma (Martelo)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-hammer", "icon_file": "生铁双锤.dds", "atk_phys": "20-40", "desc": "Martelo pesado construído com ossos e metal."},
            {"id": 2250, "name": "☆Besta Leve de Caça", "type": "Arma (Besta)", "category": "Arma", "level": 1, "quality": "magic", "icon": "fa-solid fa-crosshairs", "icon_file": "硬木弩.dds", "atk_phys": "25-38", "desc": "Besta veloz com mira calibrada para caçadores alados."},
            {"id": 14945, "name": "☆☆☆Espada do Imperador Dragão", "type": "Arma (Warsoul)", "category": "Arma", "level": 100, "quality": "warsoul", "icon": "fa-solid fa-dragon", "icon_file": "龙泉宝剑.dds", "atk_phys": "1250-1980", "desc": "Arma divina lendária forjada nas entranhas da Cidade das Feras."},

            # Armaduras e Equipamentos
            {"id": 128, "name": "☆Armadura de Batalha Protetora", "type": "Armadura (Peitoral)", "category": "Armadura", "level": 1, "quality": "magic", "icon": "fa-solid fa-shirt", "icon_file": "守护战甲.dds", "def_phys": 25, "def_magic": 15, "desc": "Armadura resistente forjada para proteção de guerreiros."},
            {"id": 133, "name": "Calça de Couro Cru", "type": "Armadura (Calça)", "category": "Armadura", "level": 1, "quality": "normal", "icon": "fa-solid fa-shield", "icon_file": "皮护腿.dds", "def_phys": 20, "def_magic": 10, "desc": "Calça resistente adequada para viagens longas."},
            {"id": 140, "name": "Botas de Caçador", "type": "Armadura (Botas)", "category": "Armadura", "level": 1, "quality": "normal", "icon": "fa-solid fa-shoe-prints", "icon_file": "皮护靴.dds", "def_phys": 12, "def_magic": 8, "desc": "Botas confortáveis que facilitam a movimentação."},
            {"id": 150, "name": "Braçadeiras de Couro", "type": "Armadura (Braçadeiras)", "category": "Armadura", "level": 1, "quality": "normal", "icon": "fa-solid fa-hand", "icon_file": "皮护手.dds", "def_phys": 10, "def_magic": 5, "desc": "Proteção para pulsos e antebraços."},
            {"id": 174, "name": "☆Colar da Chuva Branca", "type": "Ornamento (Colar)", "category": "Ornamento / Jóia", "level": 1, "quality": "magic", "icon": "fa-solid fa-gem", "icon_file": "白霖项链.dds", "def_phys": 15, "desc": "Colar brilhante que concede resistência ao portador."},
            {"id": 251, "name": "☆Pingente do Céu", "type": "Ornamento (Cinto)", "category": "Ornamento / Jóia", "level": 1, "quality": "magic", "icon": "fa-solid fa-ring", "icon_file": "通天腰佩.dds", "def_magic": 20, "desc": "Pingente talhado em jade com inscrições arcanas."},
            {"id": 180, "name": "☆Anel de Bronze Antigo", "type": "Ornamento (Anel)", "category": "Ornamento / Jóia", "level": 1, "quality": "magic", "icon": "fa-solid fa-ring", "icon_file": "古铜戒.dds", "atk_phys": "10-15", "desc": "Anel forjado em bronze com gravuras ancestrais."},
            {"id": 181, "name": "☆Anel Mágico de Prata", "type": "Ornamento (Anel)", "category": "Ornamento / Jóia", "level": 1, "quality": "magic", "icon": "fa-solid fa-ring", "icon_file": "秘银戒.dds", "atk_magic": "15-20", "desc": "Anel de prata reluzente que intensifica magias arcanas."},
            {"id": 2271, "name": "Flecha de Ferro", "type": "Projétil", "category": "Projétil / Flecha", "level": 1, "quality": "normal", "icon": "fa-solid fa-location-arrow", "icon_file": "狼牙箭.dds", "desc": "Munição padrão para arcos e bestas."},

            # Consumíveis, Poções e Utilidades
            {"id": 2100, "name": "Pergaminho de Retorno", "type": "Consumível", "category": "Pergaminho / Retorno", "level": 1, "quality": "normal", "icon": "fa-solid fa-scroll", "icon_file": "回城香.dds", "desc": "Teletransporta o jogador instantaneamente para a cidade mais próxima."},
            {"id": 1796, "name": "Poção Pequena de Cura (HP)", "type": "Poção", "category": "Poção / Medicamento", "level": 1, "quality": "normal", "icon": "fa-solid fa-flask text-rose-400", "icon_file": "小金创药.dds", "desc": "Restaura 300 pontos de Vida gradualmente."},
            {"id": 1801, "name": "Poção Pequena de Mana (MP)", "type": "Poção", "category": "Poção / Medicamento", "level": 1, "quality": "normal", "icon": "fa-solid fa-flask text-sky-400", "icon_file": "小行气散.dds", "desc": "Restaura 300 pontos de Mana gradualmente."},
            {"id": 1797, "name": "Poção Média de Cura (HP)", "type": "Poção", "category": "Poção / Medicamento", "level": 20, "quality": "normal", "icon": "fa-solid fa-flask text-rose-400", "icon_file": "中金创药.dds", "desc": "Restaura 750 pontos de Vida."},
            {"id": 1802, "name": "Poção Média de Mana (MP)", "type": "Poção", "category": "Poção / Medicamento", "level": 20, "quality": "normal", "icon": "fa-solid fa-flask text-sky-400", "icon_file": "中行气散.dds", "desc": "Restaura 750 pontos de Mana."},
            {"id": 1798, "name": "Poção Grande de Cura (HP)", "type": "Poção", "category": "Poção / Medicamento", "level": 40, "quality": "normal", "icon": "fa-solid fa-flask text-rose-400", "icon_file": "大金创药.dds", "desc": "Restaura 1500 pontos de Vida."},
            {"id": 1803, "name": "Poção Grande de Mana (MP)", "type": "Poção", "category": "Poção / Medicamento", "level": 40, "quality": "normal", "icon": "fa-solid fa-flask text-sky-400", "icon_file": "大行气散.dds", "desc": "Restaura 1500 pontos de Mana."},

            # Pedras da Alma / Gemas
            {"id": 11208, "name": "Pedra de Nível 7 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 7, "quality": "rare", "icon": "fa-solid fa-gem text-amber-400", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +50 HP | Armas: +15 Ataque Físico."},
            {"id": 11209, "name": "Pedra de Nível 8 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 8, "quality": "rare", "icon": "fa-solid fa-gem text-amber-400", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +75 HP | Armas: +25 Ataque Físico."},
            {"id": 11210, "name": "Pedra de Nível 9 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 9, "quality": "rare", "icon": "fa-solid fa-gem text-amber-400", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +100 HP | Armas: +40 Ataque Físico."},
            {"id": 11211, "name": "Pedra de Nível 10 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 10, "quality": "legendary", "icon": "fa-solid fa-gem text-amber-300", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +130 HP | Armas: +60 Ataque Físico."},
            {"id": 11212, "name": "Pedra de Nível 11 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 11, "quality": "legendary", "icon": "fa-solid fa-gem text-amber-300", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +165 HP | Armas: +85 Ataque Físico."},
            {"id": 11213, "name": "Pedra de Nível 12 (HP)", "type": "Gema", "category": "Pedra da Alma / Gema", "level": 12, "quality": "warsoul", "icon": "fa-solid fa-gem text-rose-500", "icon_file": "火红之石.dds", "desc": "Incrustação em armaduras: +200 HP | Armas: +120 Ataque Físico."},

            # Voos e Modas
            {"id": 12979, "name": "Asas Celestiais (Alados)", "type": "Voo", "category": "Voo / Montaria Alada", "level": 30, "quality": "rare", "icon": "fa-solid fa-feather", "icon_file": "洁白之翼.dds", "desc": "Velocidade de voo +2.0 m/s. Exclusivo para a raça Alada."},
            {"id": 12980, "name": "Espada Voadora da Fênix", "type": "Voo", "category": "Voo / Montaria Alada", "level": 30, "quality": "rare", "icon": "fa-solid fa-plane-up", "icon_file": "青灵剑.dds", "desc": "Velocidade de voo +2.0 m/s. Exclusivo para a raça Humana."},
            {"id": 12981, "name": "Manta Voadora da Arraia", "type": "Voo", "category": "Voo / Montaria Alada", "level": 30, "quality": "rare", "icon": "fa-solid fa-dove", "icon_file": "紫金葫芦.dds", "desc": "Velocidade de voo +2.0 m/s. Exclusivo para a raça Selvagem."},
            {"id": 8500, "name": "Penteado Aristocrata", "type": "Moda (Cabeça)", "category": "Moda / Roupas", "level": 1, "quality": "magic", "icon": "fa-solid fa-hat-wizard", "icon_file": "贵族男发.dds", "desc": "Penteado elegante com acessórios dourados."},
            {"id": 8501, "name": "Terno de Gala Superior", "type": "Moda (Peitoral)", "category": "Moda / Roupas", "level": 1, "quality": "magic", "icon": "fa-solid fa-vest", "icon_file": "晚礼服男上衣.dds", "desc": "Elegante traje social masculino para eventos nobres."},
            {"id": 8502, "name": "Calça de Gala Social", "type": "Moda (Calça)", "category": "Moda / Roupas", "level": 1, "quality": "magic", "icon": "fa-solid fa-user-tie", "icon_file": "晚礼服男裤.dds", "desc": "Calça social combinando com o terno de gala."},
            {"id": 8503, "name": "Sapatos de Gala de Couro", "type": "Moda (Sapatos)", "category": "Moda / Roupas", "level": 1, "quality": "magic", "icon": "fa-solid fa-shoe-prints", "icon_file": "晚礼服男鞋.dds", "desc": "Sapatos de verniz engraxados."},
            {"id": 8504, "name": "Luvas de Seda Fina", "type": "Moda (Braçadeiras)", "category": "Moda / Roupas", "level": 1, "quality": "magic", "icon": "fa-solid fa-hand", "icon_file": "晚礼服男护手.dds", "desc": "Luvas macias tecidas em pura seda."},

            # Hierogramas / Amuletos
            {"id": 8412, "name": "Hierograma de Platina (HP)", "type": "Amuleto", "category": "Hierograma / Amuleto", "level": 1, "quality": "legendary", "icon": "fa-solid fa-heart-pulse text-rose-500", "icon_file": "白金护身符.dds", "desc": "Recupera automaticamente o HP ao atingir menos de 50%. Capacidade: 1.800.000 HP."},
            {"id": 8413, "name": "Hierograma de Platina (MP)", "type": "Amuleto", "category": "Hierograma / Amuleto", "level": 1, "quality": "legendary", "icon": "fa-solid fa-bolt text-sky-400", "icon_file": "白金守神符.dds", "desc": "Recupera automaticamente o MP ao atingir menos de 75%. Capacidade: 2.700.000 MP."},
            {"id": 13396, "name": "Livro Divino: Dominador", "type": "Livro", "category": "Livro Sagrado / Tomo", "level": 1, "quality": "legendary", "icon": "fa-solid fa-book", "icon_file": "天书.dds", "desc": "Livro sagrado de nível 5: +45 Força, +45 Agilidade, +1.0% Crítico."},
        ]
        for itm in defaults:
            itm["icon_img"] = f"/api/elements/icon/realm_126/{itm['id']}.png" if itm.get("icon_file") else None
            self.popular_items_cache[itm["id"]] = itm

    def _get_elements_path_for_realm(self, realm_id: str) -> Optional[str]:
        data_env = os.getenv("PW_DATA_DIR", "")
        possible_paths = [
            f"/app/data/{realm_id}/config/elements.data",
            f"/data/{realm_id}/config/elements.data",
            os.path.join(data_env, realm_id, "config", "elements.data") if data_env else "",
            os.path.join(os.path.dirname(__file__), "..", "..", "data", realm_id, "config", "elements.data"),
            os.path.join(os.path.dirname(__file__), "data", realm_id, "config", "elements.data"),
            os.path.join(self.workspace_root, "pw-universal-server", "data", realm_id, "config", "elements.data"),
            os.path.join(self.workspace_root, "data", realm_id, "config", "elements.data"),
            os.path.join(self.workspace_root, "pw-universal-server", "data", "realm_126", "config", "elements.data"),
            os.path.join(self.workspace_root, "pwclient_153v145", "element", "data", "elements.data"),
            os.path.join(self.workspace_root, "files1.2.6", "pwserver", "gamed", "config", "elements.data"),
        ]
        for p in possible_paths:
            if p and os.path.exists(p):
                return p
        return None

    def load_realm_elements(self, realm_id: str) -> Dict[int, Dict[str, Any]]:
        """Carrega e decodifica todos os itens de elements.data de um Realm com cache em memória"""
        if realm_id in self.realms_cache:
            return self.realms_cache[realm_id]

        items_db = dict(self.popular_items_cache)
        elements_file = self._get_elements_path_for_realm(realm_id)

        # Pré-carrega iconset do realm se disponível
        iconset_data = self.icon_manager.load_realm_iconset(realm_id)

        if elements_file and os.path.exists(elements_file):
            try:
                with open(elements_file, "rb") as f:
                    header = f.read(4)
                    if len(header) == 4:
                        version, signature = struct.unpack("<hh", header)
                        
                        # Parser para v7 (1.2.6)
                        if version == 7:
                            for t_idx in range(min(len(TABLE_SIZES_V7), 58)):
                                count_bytes = f.read(4)
                                if len(count_bytes) < 4:
                                    break
                                count = struct.unpack("<I", count_bytes)[0]
                                sz = TABLE_SIZES_V7[t_idx]
                                
                                for _ in range(count):
                                    raw = f.read(sz)
                                    if len(raw) < 76:
                                        continue
                                    item_id = struct.unpack("<I", raw[:4])[0]
                                    if item_id == 0 or item_id in items_db:
                                        continue
                                    
                                    # Extrai nome UTF-16LE
                                    name_raw = raw[12:76].decode("utf-16le", errors="ignore").split("\x00")[0].strip()
                                    if not name_raw:
                                        continue
                                    
                                    # Extrai arquivo de textura .dds
                                    icon_file = ""
                                    idx_dds = raw.find(b".dds")
                                    if idx_dds != -1:
                                        start = raw.rfind(b"\x00", 0, idx_dds)
                                        start = 0 if start == -1 else start + 1
                                        icon_file_raw = raw[start:idx_dds+4].decode("gbk", errors="ignore").strip()
                                        icon_file = icon_file_raw.replace("/", "\\").split("\\")[-1]

                                    cat_name = TABLE_CATEGORIES.get(t_idx, "Geral")
                                    quality = "normal"
                                    if "☆☆☆" in name_raw:
                                        quality = "legendary"
                                    elif "☆☆" in name_raw:
                                        quality = "rare"
                                    elif "☆" in name_raw:
                                        quality = "magic"

                                    # Ícone representativo por categoria (vetorial / fallback)
                                    icon = "fa-solid fa-box"
                                    if t_idx == 3: icon = "fa-solid fa-khanda"
                                    elif t_idx == 6: icon = "fa-solid fa-shield-halved"
                                    elif t_idx == 9: icon = "fa-solid fa-gem"
                                    elif t_idx == 12: icon = "fa-solid fa-flask"
                                    elif t_idx == 23: icon = "fa-solid fa-feather-pointed"
                                    elif t_idx == 24: icon = "fa-solid fa-shirt"
                                    elif t_idx == 27: icon = "fa-solid fa-location-arrow"
                                    elif t_idx == 31: icon = "fa-solid fa-gem"
                                    elif t_idx == 34: icon = "fa-solid fa-heart-pulse"
                                    elif t_idx == 35: icon = "fa-solid fa-book"

                                    # Verifica se o ícone existe no iconset
                                    icon_img = None
                                    if icon_file and iconset_data and icon_file in iconset_data.get("items_map", {}):
                                        icon_img = f"/api/elements/icon/{realm_id}/{item_id}.png"

                                    items_db[item_id] = {
                                        "id": item_id,
                                        "name": name_raw,
                                        "type": cat_name,
                                        "category": cat_name,
                                        "level": 1,
                                        "quality": quality,
                                        "icon": icon,
                                        "icon_file": icon_file,
                                        "icon_img": icon_img,
                                        "desc": f"Item {name_raw} registrado no elements.data do servidor (ID {item_id})."
                                    }
            except Exception as e:
                print(f"[ElementsDecoder] Aviso ao ler elements.data para {realm_id}: {e}")

        self.realms_cache[realm_id] = items_db
        return items_db

    def get_item_info(self, realm_id: str, item_id: int) -> Dict[str, Any]:
        """Obtém os dados completos decodificados de um item"""
        db = self.load_realm_elements(realm_id)
        if item_id in db:
            return db[item_id]
        
        return {
            "id": item_id,
            "name": f"Item #{item_id}",
            "type": "Item",
            "category": "Geral",
            "level": 1,
            "quality": "normal",
            "icon": "fa-solid fa-box",
            "icon_file": "",
            "icon_img": None,
            "desc": f"Item ID {item_id} customizado ou registrado no servidor."
        }

    def generate_octets_for_item(
        self,
        realm_id: str,
        item_id: int,
        refine_level: int = 0,
        sockets_count: int = 0,
        socket_stones: Optional[List[int]] = None,
        durability: Optional[int] = None,
        max_durability: Optional[int] = None,
        creator_name: str = ""
    ) -> bytes:
        """Gera automaticamente o payload binário de octets correspondente ao item"""
        item_info = self.get_item_info(realm_id, item_id)
        cat = item_info.get("category", "Geral")
        level = item_info.get("level", 1)

        dura = durability if durability is not None and durability > 0 else 2800
        max_dura = max_durability if max_durability is not None and max_durability > 0 else dura

        return ItemOctetCodec.build_item_octets(
            category=cat,
            level=level,
            race_mask=255,
            str_req=0,
            vit_req=0,
            agi_req=0,
            eng_req=0,
            durability=dura,
            max_durability=max_dura,
            creator_name=creator_name,
            refine_level=refine_level,
            sockets_count=sockets_count,
            socket_stones=socket_stones or [],
            dmg_low=10,
            dmg_high=20,
            def_phys=10,
            def_magic=5
        )


    def _is_item_compatible_with_slot(
        self,
        item: Dict[str, Any],
        container_type: Optional[int],
        slot: Optional[int],
        class_id: Optional[int]
    ) -> bool:
        """Verifica se o item é compatível com o slot e a classe especificados"""
        if container_type is None and slot is None and class_id is None:
            return True

        name = item.get("name", "").lower()
        cat = item.get("category", "").lower()
        typ = item.get("type", "").lower()

        # Container 1: Equipamento de Combate no Corpo
        if container_type == 1:
            if slot == 0:  # Arma na Mão
                if "arma" not in cat and "arma" not in typ:
                    return False
                
                # Validação de classe para armas
                if class_id is not None:
                    if class_id in [1, 4, 7, 9]:
                        return any(w in name for w in ["cajado", "varinha", "espada mágica", "patena", "espírito", "roda", "orbe", "flauta"]) or "mágica" in typ
                    elif class_id == 5:
                        return any(w in name for w in ["adaga", "punhal", "lâmina curta", "garra"])
                    elif class_id == 6:
                        return any(w in name for w in ["arco", "besta", "funda", "tiro", "pluma"])
                    elif class_id == 3:
                        return any(w in name for w in ["machado", "martelo", "maça", "porrete", "machadinha"])
                    elif class_id == 8:
                        return any(w in name for w in ["espada", "lâmina", "espada dupla", "sabre"])
                    elif class_id == 0:
                        return any(w in name for w in ["espada", "machado", "lança", "punho", "martelo", "lâmina", "alabarda", "arco"])
                return True

            elif slot == 1:  # Elmo
                return "armadura" in cat and any(w in name for w in ["elmo", "capacete", "touca", "coroa", "chapéu", "máscara", "tiara"])
            elif slot == 2:  # Colar
                return "ornamento" in cat and "colar" in name
            elif slot == 3:  # Capa
                return "armadura" in cat and any(w in name for w in ["capa", "manto"])
            elif slot == 4:  # Peitoral
                return "armadura" in cat and any(w in name for w in ["peitoral", "armadura", "túnica", "manto", "robe", "traje", "camisa", "gibão", "cota"])
            elif slot == 5:  # Cinto
                return "ornamento" in cat and any(w in name for w in ["cinto", "pingente", "ornamento", "faixa", "cordão"])
            elif slot == 6:  # Calça
                return "armadura" in cat and any(w in name for w in ["calça", "perneira", "saia"])
            elif slot == 7:  # Botas
                return "armadura" in cat and any(w in name for w in ["bota", "calçado", "sapato", "sandália", "greva"])
            elif slot == 8:  # Braçadeiras
                return "armadura" in cat and any(w in name for w in ["braçadeira", "luva", "bracelete", "munhequeira"])
            elif slot in [9, 10]:  # Anéis
                return "ornamento" in cat and any(w in name for w in ["anel", "aliança"])
            elif slot == 11:  # Flechas / Projéteis
                return "projétil" in cat or any(w in name for w in ["flecha", "virote", "munição", "projetil"])
            elif slot == 12:  # Voo
                return "voo" in cat or any(w in name for w in ["asa", "voo", "espada voadora", "manta", "arraia", "águia", "pássaro"])
            elif slot == 17:  # Livro Divino
                return "livro" in cat or any(w in name for w in ["livro", "tomo", "sagrado", "escritura"])
            elif slot == 19:  # Hierograma HP
                return "hierograma" in cat or any(w in name for w in ["hp", "vida", "amuleto"])
            elif slot == 20:  # Hierograma MP
                return "hierograma" in cat or any(w in name for w in ["mp", "mana", "espírito"])

        # Container 3: Roupas e Cosméticos (Moda / Fashion)
        elif container_type == 3:
            if slot == 0:  # Cabeça Moda
                return "moda" in cat and any(w in name for w in ["cabelo", "penteado", "cabeça", "chapéu", "touca", "coroa", "máscara", "tiara"])
            elif slot == 1:  # Camisa / Top Moda
                return "moda" in cat and any(w in name for w in ["blusa", "camisa", "top", "vestido", "paletó", "jaqueta", "terno", "manto", "traje"])
            elif slot == 2:  # Calça Moda
                return "moda" in cat and any(w in name for w in ["calça", "saia", "short", "bermuda"])
            elif slot == 3:  # Sapatos Moda
                return "moda" in cat and any(w in name for w in ["sapato", "bota", "salto", "sandália", "chinelo", "tênis"])
            elif slot == 4:  # Luvas Moda
                return "moda" in cat and any(w in name for w in ["luva", "braçadeira", "munhequeira", "bracelete"])
            elif slot == 5:  # Arma Visual Fashion
                return ("moda" in cat or "arma" in cat) and any(w in name for w in ["arma", "espada", "adaga", "arco", "cajado", "machado", "visual"])

        return True

    def search_items(
        self,
        realm_id: str,
        query: str = "",
        category: str = "",
        container_type: Optional[int] = None,
        slot_filter: Optional[int] = None,
        class_id: Optional[int] = None,
        limit: int = 50
    ) -> List[Dict[str, Any]]:
        """Pesquisa itens decodificados com suporte a filtros de texto, categoria, slot e classe (insensível a acentos)"""
        db = self.load_realm_elements(realm_id)
        query_norm = normalize_search_string(query)
        cat_norm = normalize_search_string(category)

        results = []
        for item in db.values():
            if not self._is_item_compatible_with_slot(item, container_type, slot_filter, class_id):
                continue

            if cat_norm and cat_norm != "todos" and cat_norm not in normalize_search_string(item.get("category", "")):
                continue
            
            if not query_norm:
                results.append(item)
            else:
                name_norm = normalize_search_string(item.get("name", ""))
                type_norm = normalize_search_string(item.get("type", ""))
                sid_str = str(item["id"])

                if query_norm in name_norm or query_norm in type_norm or query_norm == sid_str:
                    results.append(item)
            
            if len(results) >= limit:
                break

        if query_norm.isdigit():
            id_num = int(query_norm)
            if not any(r["id"] == id_num for r in results):
                exact = self.get_item_info(realm_id, id_num)
                results.insert(0, exact)

        return results[:limit]

    def get_realm_skills(self, realm_id: str = "realm_126") -> Dict[int, Dict[str, Any]]:
        """Retorna o catálogo completo e canônico de habilidades filtrado pelas classes suportadas no Realm"""
        if realm_id in self.realms_skills_cache:
            return self.realms_skills_cache[realm_id]

        iconset_data = self.icon_manager.load_realm_iconset(realm_id)
        skills_db: Dict[int, Dict[str, Any]] = {}

        # Mapeamento rigoroso de classes existentes por versão do servidor
        REALM_ALLOWED_CLASSES = {
            "realm_126": {0, 1, 3, 4, 6, 7, -1}, # PW 1.2.6: WR, MG, WB, WF, EA, EP e Comuns
            "realm_148": {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1}, # PW 1.4.8: + ESP, MER, ARC, MIS
            "realm_153": {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, -1}, # PW 1.5.3: + RET, TOR
        }
        allowed_classes = REALM_ALLOWED_CLASSES.get(realm_id, {0, 1, 3, 4, 6, 7, -1})

        for sk in PW_SKILLS_DATABASE:
            if sk.get("class_id") not in allowed_classes:
                continue

            sk_copy = dict(sk)
            icon_file = sk_copy.get("icon_file", "")
            icon_img = None
            if iconset_data and icon_file and icon_file in iconset_data.get("skills_map", {}):
                icon_img = f"/api/elements/skill-icon/{realm_id}/{sk['id']}.png"
            elif iconset_data and "skills_map" in iconset_data:
                icon_img = f"/api/elements/skill-icon/{realm_id}/{sk['id']}.png"
            sk_copy["icon_img"] = icon_img
            skills_db[sk["id"]] = sk_copy

        self.realms_skills_cache[realm_id] = skills_db
        return skills_db

    def get_skill_info(self, skill_id: int, realm_id: str = "realm_126") -> Dict[str, Any]:
        """Obtém detalhes de uma habilidade cadastrada ou fallback de não encontrada"""
        skills_db = self.get_realm_skills(realm_id)
        if skill_id in skills_db:
            return skills_db[skill_id]

        return {
            "id": skill_id,
            "name": f"Habilidade #{skill_id}",
            "name_en": f"Skill #{skill_id}",
            "class_id": -1,
            "class_name": "Geral / Desconhecida",
            "type": "Habilidade",
            "icon": "fa-solid fa-wand-magic-sparkles text-indigo-400",
            "icon_file": "unknown.dds",
            "icon_img": f"/api/elements/skill-icon/{realm_id}/{skill_id}.png",
            "max_lv": 10,
            "desc": f"Habilidade ID {skill_id} registrada no servidor."
        }

    def search_skills(
        self,
        query: str = "",
        class_id: Optional[int] = None,
        realm_id: str = "realm_126",
        limit: int = 60
    ) -> List[Dict[str, Any]]:
        """Pesquisa habilidades no catálogo do jogo por ID, nome (PT/EN), tipo ou classe com normalização de acentos"""
        skills_db = self.get_realm_skills(realm_id)
        query_norm = normalize_search_string(query)
        results = []

        for sk in skills_db.values():
            # Filtro por classe
            if class_id is not None and str(class_id).strip() != "" and str(class_id) != "todos":
                try:
                    c_id = int(class_id)
                    # Se c_id == -1 (Comum), traz apenas comuns
                    if c_id == -1:
                        if sk["class_id"] != -1:
                            continue
                    elif c_id >= 0:
                        # Se filtrou classe específica, aceita as da classe ou comuns se não houver busca
                        if sk["class_id"] != c_id and sk["class_id"] != -1:
                            continue
                except (ValueError, TypeError):
                    pass

            if not query_norm:
                results.append(sk)
            else:
                name_pt_norm = normalize_search_string(sk.get("name", ""))
                name_en_norm = normalize_search_string(sk.get("name_en", ""))
                type_name_norm = normalize_search_string(sk.get("type", ""))
                cls_name_norm = normalize_search_string(sk.get("class_name", ""))
                sid_str = str(sk["id"])

                if (query_norm in name_pt_norm or 
                    query_norm in name_en_norm or 
                    query_norm in type_name_norm or 
                    query_norm in cls_name_norm or 
                    query_norm == sid_str):
                    results.append(sk)

            if len(results) >= limit:
                break

        if query_norm.isdigit():
            sk_id = int(query_norm)
            if not any(s["id"] == sk_id for s in results):
                exact = self.get_skill_info(sk_id, realm_id)
                results.insert(0, exact)

        return results[:limit]

    def get_max_skills_for_class(self, class_id: int, realm_id: str = "realm_126") -> List[Dict[str, Any]]:
        """Retorna todas as habilidades da classe configuradas no nível máximo (10)"""
        skills_db = self.get_realm_skills(realm_id)
        max_skills = []
        for sk in skills_db.values():
            if sk["class_id"] == class_id or sk["class_id"] == -1:
                max_skills.append({
                    "skill_id": sk["id"],
                    "level": sk.get("max_lv", 10),
                    "progress": 0,
                    "name": sk["name"],
                    "type": sk["type"],
                    "icon": sk["icon"],
                    "icon_file": sk.get("icon_file", ""),
                    "icon_img": sk.get("icon_img")
                })
        return max_skills


# Instância Singleton
decoder_instance = ElementsDecoder()

