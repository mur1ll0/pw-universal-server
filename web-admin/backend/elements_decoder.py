"""
PW-ADMIN: Decoder de elements.data e Gerenciador de Ícones do Surfaces/Iconset
Permite decodificação binária de arquivos elements.data de múltiplos realms do servidor,
extraindo nomes, descrições, categorias, atributos de forja e ícones fiéis ao jogo
a partir dos arquivos surfaces.pck / iconset (iconlist_ivtrm.dds e iconlist_skill.dds).
"""

import os
import io
import json
import struct
import sys
import unicodedata
from typing import Dict, List, Any, Optional, Tuple

# Leitor genérico de `elements.data` (build v156 em diante), dirigido pelo catálogo de
# layouts em `specs/elements_layouts/` -- ver o README lá para a arquitetura completa
# (detecção de versão pelo cabeçalho, catálogo de JSON por build, overrides por realm).
# Import best-effort: em produção (imagem Docker do pw-admin-api), o build context hoje é
# só `web-admin/backend/`, então `specs/` pode não estar disponível -- nesse caso o
# decodificador cai de volta pro comportamento antigo (só v7/1.2.6) em vez de quebrar o
# módulo inteiro. Ver a seção "Empacotamento" no relatório desta sessão para a decisão
# pendente de como levar `specs/elements_layouts` pra dentro da imagem.
_ELEMENTS_LAYOUTS_CANDIDATES = [
    os.path.join(os.path.dirname(__file__), "specs", "elements_layouts"),
    os.path.join(os.path.dirname(__file__), "..", "..", "specs", "elements_layouts"),
    "/app/specs/elements_layouts",
]
HAS_GENERIC_READER = False
pw_elements_reader = None
for _candidate in _ELEMENTS_LAYOUTS_CANDIDATES:
    if os.path.isdir(_candidate):
        sys.path.insert(0, _candidate)
        try:
            import pw_elements_reader  # type: ignore
            HAS_GENERIC_READER = True
        except Exception as _e:
            print(f"[ElementsDecoder] achei {_candidate} mas falhei ao importar pw_elements_reader: {_e}")
        break

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

# Mesma ideia de TABLE_CATEGORIES, mas por NOME de tabela -- usado pelo caminho novo
# (build v156 em diante, via `pw_elements_reader`), que le por nome em vez de índice
# posicional (o índice de tabela deixou de ser estável entre builds a partir do momento em
# que passamos a usar o catálogo `specs/elements_layouts/vNNN.json`).
TABLE_CATEGORIES_POR_NOME = {
    "WEAPON_ESSENCE": "Arma",
    "ARMOR_ESSENCE": "Armadura",
    "DECORATION_ESSENCE": "Ornamento / Jóia",
    "MEDICINE_ESSENCE": "Poção / Medicamento",
    "MATERIAL_ESSENCE": "Material de Forja",
    "TASKMATTER_ESSENCE": "Item de Missão",
    "TOSSMATTER_ESSENCE": "Item de Missão",
    "TOWNSCROLL_ESSENCE": "Pergaminho / Retorno",
    "UNIONSCROLL_ESSENCE": "Pergaminho / Retorno",
    "REVIVESCROLL_ESSENCE": "Pergaminho / Retorno",
    "FLYSWORD_ESSENCE": "Voo / Montaria Alada",
    "WINGMANWING_ESSENCE": "Voo / Montaria Alada",
    "FASHION_ESSENCE": "Moda / Roupas",
    "PROJECTILE_ESSENCE": "Projétil / Flecha",
    "STONE_ESSENCE": "Pedra da Alma / Gema",
    "SKILLTOME_ESSENCE": "Livro Sagrado / Tomo",
    "DAMAGERUNE_ESSENCE": "Hierograma / Amuleto",
    "ARMORRUNE_ESSENCE": "Hierograma / Amuleto",
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
        durability: int = 1400,
        max_durability: int = 1400,
        creator_name: str = "",
        refine_level: int = 0,
        sockets_count: int = 0,
        socket_stones: Optional[List[int]] = None,
        dmg_low: int = 3,
        dmg_high: int = 3,
        magic_low: int = 0,
        magic_high: int = 0,
        def_phys: int = 10,
        def_magic: int = 5,
        dodge: int = 0,
        hp_enh: int = 0,
        mp_enh: int = 0,
        color: int = 0x00FFFFFF,
        weapon_type: int = 0,
        weapon_class: int = 292,
        weapon_level: int = 0,
        attack_speed: int = 16,
        attack_range: float = 3.0,
        short_range: float = 0.0,
        tag: int = 3
    ) -> bytes:
        """
        Constrói octets binários (item_content) de acordo com os structs C++ do cliente e servidor Perfect World.
        Layout:
          1. prerequisition (20 bytes): level(h), race(h), str(h), vit(h), agi(h), eng(h), dur(i), max_dur(i)
          2. essence_size (short) + tag (short) [ou tag_type=1 + len + name]
          3. essence payload (_weapon_essence, _armor_essence, _decoration_essence)
          4. sockets: hole_num(short), hole_adj(short), stones(int[])
          5. addons: addon_num(int), addon_data entries
        """
        cat = (category or "").lower()
        socket_stones = socket_stones or []
        buf = bytearray()

        dura = int(durability) if durability is not None and durability > 0 else 1400
        max_dura = int(max_durability) if max_durability is not None and max_durability > 0 else dura

        # 1. prerequisition (20 bytes)
        buf.extend(struct.pack(
            "<hhhhhhii",
            int(level),
            int(race_mask),
            int(str_req),
            int(vit_req),
            int(agi_req),
            int(eng_req),
            dura,
            max_dura
        ))

        # 2. Maker Tag e Essence Payload
        c_bytes = (creator_name or "").encode("gbk", errors="ignore")[:31]

        if "arma" in cat:
            ess_size = 44
            ess_bytes = struct.pack(
                "<hhiiiiiiiiff",
                int(weapon_type),   # short mode (0=melee, 1=ranged, 2=assassin)
                0,                  # short unused
                int(weapon_class),  # int major_type (ex: 292 para cajados/espadas mágicas)
                int(weapon_level),  # int grade/tier (0 para nível inicial)
                0,                  # int req_projectile
                int(dmg_low),       # int damage_low
                int(dmg_high),      # int damage_high
                int(magic_low),     # int magic_damage_low
                int(magic_high),    # int magic_damage_high
                int(attack_speed),  # int attack_speed
                float(attack_range),# float attack_range
                float(short_range)  # float attack_short_range
            )
        elif "armadura" in cat:
            ess_size = 36
            ess_bytes = struct.pack(
                "<iiiiiiiii",
                int(def_phys),
                int(dodge),
                int(mp_enh),
                int(hp_enh),
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
                int(dmg_low),
                int(magic_low),
                int(def_phys),
                int(dodge),
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

        # Header de tamanho da essência + Tag do Criador
        if len(c_bytes) > 0:
            # Tag customizado de criador
            buf.extend(struct.pack("<hhh", ess_size, 1, len(c_bytes)))
            buf.extend(c_bytes)
        else:
            # Tag padrão de NPC / Sistema (tag=3)
            buf.extend(struct.pack("<hh", ess_size, int(tag)))

        # Adiciona bytes da essência
        buf.extend(ess_bytes)

        # 3. Sockets (Slots de Pedras)
        sock_cnt = min(max(0, int(sockets_count)), 4)
        buf.extend(struct.pack("<hh", sock_cnt, 0))
        for i in range(sock_cnt):
            st_id = int(socket_stones[i]) if i < len(socket_stones) else 0
            buf.extend(struct.pack("<I", st_id))

        # 4. Addons e Refino
        addons = []
        if refine_level > 0:
            ref_val = int(refine_level) * (15 if "arma" in cat else 10)
            ref_addon_id = 0x0001 | (1 << 13)
            addons.append((ref_addon_id, [ref_val]))

        buf.extend(struct.pack("<i", len(addons)))
        for a_id, args in addons:
            buf.extend(struct.pack("<I", a_id))
            for arg in args:
                buf.extend(struct.pack("<i", int(arg)))

        return bytes(buf)

    @classmethod
    def parse_item_octets(cls, raw_data: Any) -> Dict[str, Any]:
        raw_bytes = cls.hex_to_bytes(raw_data) if isinstance(raw_data, str) else (raw_data or b"")
        if not raw_bytes or len(raw_bytes) < 20:
            return {
                "has_octets": False,
                "raw_hex": raw_bytes.hex() if raw_bytes else "",
                "level": 1,
                "durability": 1400,
                "max_durability": 1400,
                "refine_level": 0,
                "sockets_count": 0,
                "socket_stones": [],
                "creator_name": "",
                "addons": []
            }

        try:
            lvl, race, st, vit, agi, eng, dura, max_dura = struct.unpack("<hhhhhhii", raw_bytes[:20])
            offset = 20

            ess_size = struct.unpack_from("<h", raw_bytes, offset)[0]
            offset += 2

            tag_val = struct.unpack_from("<h", raw_bytes, offset)[0]
            offset += 2

            creator = ""
            if tag_val == 1:
                name_len = struct.unpack_from("<h", raw_bytes, offset)[0]
                offset += 2
                creator = raw_bytes[offset:offset+name_len].decode("gbk", errors="ignore").strip()
                offset += name_len

            essence_raw = raw_bytes[offset:offset+ess_size] if offset + ess_size <= len(raw_bytes) else b""
            offset += ess_size

            sockets = []
            sock_count = 0
            if offset + 4 <= len(raw_bytes):
                sock_count, _ = struct.unpack("<hh", raw_bytes[offset:offset+4])
                offset += 4
                for _ in range(sock_count):
                    if offset + 4 <= len(raw_bytes):
                        st_id = struct.unpack("<i", raw_bytes[offset:offset+4])[0]
                        sockets.append(st_id)
                        offset += 4

            addons = []
            refine_level = 0
            if offset + 4 <= len(raw_bytes):
                addon_cnt = struct.unpack("<i", raw_bytes[offset:offset+4])[0]
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
                                arg_val = struct.unpack("<i", raw_bytes[offset:offset+4])[0]
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
                "tag": tag_val,
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


# Base de Habilidades do Jogo Canônica extraída diretamente do código C++ da engine (CElementSkill) e skillstr.txt
CANON_SKILLS_PATH = os.path.join(os.path.dirname(__file__), "skills_canon.json")
PW_SKILLS_DATABASE: List[Dict[str, Any]] = []

if os.path.exists(CANON_SKILLS_PATH):
    try:
        with open(CANON_SKILLS_PATH, "r", encoding="utf-8") as f:
            PW_SKILLS_DATABASE = json.load(f)
    except Exception as e:
        print(f"Erro ao carregar skills_canon.json: {e}")

if not PW_SKILLS_DATABASE:
    PW_SKILLS_DATABASE = [
        {"id": 1, "name": "Golpe do Tigre", "name_en": "Tiger Maw", "name_cn": "虎击", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-burst", "icon_file": "虎击.dds", "max_lv": 10, "desc": "Golpe inicial frontal causando dano físico adicional."},
        {"id": 2, "name": "Corte Sangrento", "name_en": "Draw Blood", "name_cn": "寸力", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-burst", "icon_file": "寸力.dds", "max_lv": 10, "desc": "Corta pontos vitais causando sangramento contínuo no alvo."},
        {"id": 3, "name": "Lâmina Eólica", "name_en": "Aeolian Blade", "name_cn": "凌风", "class_id": 0, "class_name": "Guerreiro", "type": "Ataque Físico", "icon": "fa-solid fa-burst", "icon_file": "凌风.dds", "max_lv": 10, "desc": "Corta o ar lançando uma lâmina cortante que pode atordoar o alvo."},
        {"id": 4, "name": "Rugido do Leão", "name_en": "Roar of the Pride", "name_cn": "狮子吼", "class_id": 0, "class_name": "Guerreiro", "type": "Controle em Área", "icon": "fa-solid fa-burst", "icon_file": "狮子吼.dds", "max_lv": 10, "desc": "Rugido ensurdecedor que atordoa todos os inimigos ao redor."},
        {"id": 77, "name": "Sino Dourado (Golden Bell)", "name_en": "Aura of the Golden Bell", "name_cn": "金钟罩", "class_id": 0, "class_name": "Guerreiro", "type": "Buff em Grupo", "icon": "fa-solid fa-burst", "icon_file": "金钟罩.dds", "max_lv": 10, "desc": "Aumenta a defesa física de todos os membros do grupo."},
        {"id": 69, "name": "Dragão Voador (Heaven Flame)", "name_en": "Heavens Flame", "name_cn": "天火狂龙", "class_id": 0, "class_name": "Guerreiro", "type": "Ultimate em Área", "icon": "fa-solid fa-burst", "icon_file": "天火狂龙.dds", "max_lv": 10, "desc": "Invoca o Dragão Celestial causando dano massivo."},
        {"id": 113, "name": "Coração Puro", "name_en": "Purehearted Blessing", "name_cn": "清心咒", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Básica", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "清心咒.dds", "max_lv": 10, "desc": "Cura rápida de alvo único."},
        {"id": 114, "name": "Prece da Calmaria (Ironheart)", "name_en": "Ironheart Blessing", "name_cn": "静心咒", "class_id": 7, "class_name": "Sacerdote", "type": "Cura Contínua", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "静心符.dds", "max_lv": 10, "desc": "Cura contínua empilhável."},
        {"id": 125, "name": "Flecha de Pluma", "name_en": "Plume Shot", "name_cn": "羽箭", "class_id": 7, "class_name": "Sacerdote", "type": "Ataque Mágico / Metal", "icon": "fa-solid fa-wand-magic-sparkles", "icon_file": "羽箭.dds", "max_lv": 10, "desc": "Dispara penas mágicas afiadas causando dano de metal."},
        {"id": 167, "name": "Portal da Cidade (Town Portal)", "name_en": "Town Portal", "name_cn": "回城术", "class_id": -1, "class_name": "Comum", "type": "Teleporte de Retorno", "icon": "fa-solid fa-archway", "icon_file": "回城术.dds", "max_lv": 1, "desc": "Teleporta o personagem de volta para a cidade mais próxima."}
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

    def _get_overrides_path_for_realm(self, realm_id: str) -> Optional[str]:
        """Acha um `*_overrides.json` pro realm (ver specs/elements_layouts/README.md pro
        porque as correções de skip/count ficam separadas do layout de formato). Hoje só
        `realm_155` tem um verificado -- outros realms carregam sem overrides, o que é
        seguro (a busca gulosa padrão resolve a maioria das tabelas sozinha; as que
        precisarem de ajuste vão falhar alto em vez de dar dado errado silenciosamente, ver
        `ElementsFormatError` em `pw_elements_reader.load_elements_data`)."""
        suffix = realm_id.replace("realm_", "", 1) if realm_id.startswith("realm_") else realm_id
        candidates = [
            os.path.join(os.path.dirname(__file__), "specs", f"elements_{suffix}", f"realm_{suffix}_overrides.json"),
            os.path.join(os.path.dirname(__file__), "..", "..", "specs", f"elements_{suffix}", f"realm_{suffix}_overrides.json"),
        ]
        for p in candidates:
            if os.path.exists(p):
                return p
        return None

    def _load_realm_elements_generic(self, elements_file: str, realm_id: str, iconset_data: Optional[Dict[str, Any]]) -> Dict[int, Dict[str, Any]]:
        """Carrega `elements.data` (build v156 em diante) via `pw_elements_reader`, e achata
        as tabelas de item (`TABLE_CATEGORIES_POR_NOME`) num único dict por ID, no mesmo
        formato que o resto do backend já espera (compatível com o que o caminho antigo v7
        produzia)."""
        overrides_path = self._get_overrides_path_for_realm(realm_id)
        tables = pw_elements_reader.load_elements_data(elements_file, overrides_path=overrides_path)

        items_db: Dict[int, Dict[str, Any]] = {}
        for table_name, category in TABLE_CATEGORIES_POR_NOME.items():
            for rec in tables.get(table_name, []):
                item_id = rec.get("ID")
                name_raw = str(rec.get("Name", "")).strip()
                if not item_id or item_id in items_db or not name_raw:
                    continue

                icon_file = ""
                for value in rec.values():
                    if isinstance(value, (bytes, bytearray)) and b".dds" in value:
                        decoded = value.decode("gbk", errors="ignore").strip()
                        icon_file = decoded.replace("/", "\\").split("\\")[-1]
                        break

                quality = "normal"
                if "☆☆☆" in name_raw:
                    quality = "legendary"
                elif "☆☆" in name_raw:
                    quality = "rare"
                elif "☆" in name_raw:
                    quality = "magic"

                icon_img = None
                if icon_file and iconset_data and icon_file in iconset_data.get("items_map", {}):
                    icon_img = f"/api/elements/icon/{realm_id}/{item_id}.png"

                items_db[item_id] = {
                    "id": item_id,
                    "name": name_raw,
                    "type": category,
                    "category": category,
                    "level": 1,
                    "quality": quality,
                    "icon": "fa-solid fa-box",
                    "icon_file": icon_file,
                    "icon_img": icon_img,
                    "desc": f"Item {name_raw} registrado no elements.data do servidor (ID {item_id}).",
                }
        return items_db

    def load_realm_elements(self, realm_id: str) -> Dict[int, Dict[str, Any]]:
        """Carrega e decodifica todos os itens de elements.data de um Realm com cache em memória"""
        if realm_id in self.realms_cache:
            return self.realms_cache[realm_id]

        items_db = dict(self.popular_items_cache)
        elements_file = self._get_elements_path_for_realm(realm_id)

        # Pré-carrega iconset do realm se disponível
        iconset_data = self.icon_manager.load_realm_iconset(realm_id)

        if elements_file and os.path.exists(elements_file) and HAS_GENERIC_READER:
            try:
                generic_items = self._load_realm_elements_generic(elements_file, realm_id, iconset_data)
                items_db.update(generic_items)
                self.realms_cache[realm_id] = items_db
                return items_db
            except pw_elements_reader.UnsupportedVersionError as e:
                print(f"[ElementsDecoder] {realm_id}: {e} -- caindo pro decodificador antigo (só v7)")
            except Exception as e:
                print(f"[ElementsDecoder] {realm_id}: falha no leitor genérico ({e}) -- caindo pro decodificador antigo")

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
        """Gera automaticamente o payload binário de octets correspondente ao item respeitando as tabelas e regras C++ do PW"""
        item_info = self.get_item_info(realm_id, item_id)
        cat = item_info.get("category", "Geral")
        typ = item_info.get("type", "Item")
        level = item_info.get("level", 1)

        # Regras específicas de itens conhecidos / iniciais
        if item_id == 2251:  # Varinha Mágica (Arma inicial do Sacerdote)
            dura = durability if durability is not None and durability > 0 else 1400
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=1,
                race_mask=255,
                str_req=5,
                vit_req=0,
                agi_req=0,
                eng_req=3,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=3,
                dmg_high=3,
                magic_low=5,
                magic_high=6,
                attack_speed=16,
                attack_range=3.0,
                weapon_class=292,
                tag=3
            )
        elif item_id in [2097, 1]:  # Espada Curta de Madeira (Guerreiro inicial)
            dura = durability if durability is not None and durability > 0 else 1400
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=1,
                race_mask=255,
                str_req=5,
                vit_req=0,
                agi_req=3,
                eng_req=0,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=3,
                dmg_high=5,
                magic_low=0,
                magic_high=0,
                attack_speed=20,
                attack_range=3.5,
                weapon_class=1,
                tag=3
            )
        elif item_id == 2258:  # Machado de Ferro (Bárbaro inicial)
            dura = durability if durability is not None and durability > 0 else 1400
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=1,
                race_mask=255,
                str_req=6,
                vit_req=0,
                agi_req=2,
                eng_req=0,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=4,
                dmg_high=7,
                magic_low=0,
                magic_high=0,
                attack_speed=14,
                attack_range=3.5,
                weapon_class=2,
                tag=3
            )
        elif item_id == 2250:  # Arco de Madeira (Arqueiro inicial)
            dura = durability if durability is not None and durability > 0 else 1400
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=1,
                race_mask=255,
                str_req=3,
                vit_req=0,
                agi_req=5,
                eng_req=0,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=3,
                dmg_high=6,
                magic_low=0,
                magic_high=0,
                attack_speed=17,
                attack_range=20.0,
                weapon_type=1,
                weapon_class=5,
                tag=3
            )
        elif item_id == 2867:  # Cajado de Madeira (Mago/Feiticeira inicial)
            dura = durability if durability is not None and durability > 0 else 1400
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=1,
                race_mask=255,
                str_req=4,
                vit_req=0,
                agi_req=0,
                eng_req=4,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=3,
                dmg_high=4,
                magic_low=5,
                magic_high=7,
                attack_speed=16,
                attack_range=3.0,
                weapon_class=292,
                tag=3
            )

        # Regra genérica para armas
        if "arma" in cat.lower() or "arma" in typ.lower():
            dura = durability if durability is not None and durability > 0 else 2800
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Arma",
                level=level,
                race_mask=255,
                str_req=level * 2,
                vit_req=0,
                agi_req=level * 2,
                eng_req=0,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                dmg_low=level * 5 + 10,
                dmg_high=level * 8 + 20,
                magic_low=level * 5 + 10,
                magic_high=level * 8 + 20,
                attack_speed=16,
                attack_range=3.5,
                weapon_class=1,
                tag=3
            )

        # Regra genérica para armaduras
        if "armadura" in cat.lower() or "armadura" in typ.lower():
            dura = durability if durability is not None and durability > 0 else 2800
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Armadura",
                level=level,
                race_mask=255,
                str_req=level * 2,
                vit_req=0,
                agi_req=0,
                eng_req=0,
                durability=dura,
                max_durability=max_dura,
                creator_name=creator_name,
                refine_level=refine_level,
                sockets_count=sockets_count,
                socket_stones=socket_stones or [],
                def_phys=level * 10 + 20,
                def_magic=level * 8 + 15,
                dodge=level * 2,
                hp_enh=0,
                mp_enh=0,
                tag=3
            )

        # Regra genérica para ornamentos / jóias
        if any(k in cat.lower() for k in ["ornamento", "jóia", "joia"]):
            dura = durability if durability is not None and durability > 0 else 1800
            max_dura = max_durability if max_durability is not None and max_durability > 0 else dura
            return ItemOctetCodec.build_item_octets(
                category="Ornamento",
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
                dmg_low=0,
                magic_low=0,
                def_phys=level * 5 + 10,
                def_magic=level * 5 + 10,
                tag=3
            )

        # Itens consumíveis ou sem octets obrigatórios
        dura = durability if durability is not None and durability > 0 else 1000
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
            dmg_low=0,
            dmg_high=0,
            def_phys=0,
            def_magic=0,
            tag=3
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
            exact_match = None
            for idx, r in enumerate(results):
                if r["id"] == id_num:
                    exact_match = results.pop(idx)
                    break
            if not exact_match:
                exact_match = self.get_item_info(realm_id, id_num)
            results.insert(0, exact_match)

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
        """Pesquisa habilidades no catálogo do jogo por ID, nome (PT/EN/CN), tipo ou classe com normalização de acentos"""
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
                name_cn_norm = normalize_search_string(sk.get("name_cn", ""))
                type_name_norm = normalize_search_string(sk.get("type", ""))
                cls_name_norm = normalize_search_string(sk.get("class_name", ""))
                sid_str = str(sk["id"])

                if (query_norm in name_pt_norm or 
                    query_norm in name_en_norm or 
                    query_norm in name_cn_norm or
                    query_norm in type_name_norm or 
                    query_norm in cls_name_norm or 
                    query_norm == sid_str):
                    results.append(sk)

            if len(results) >= limit:
                break

        if query_norm.isdigit():
            sk_id = int(query_norm)
            exact_match = None
            for idx, r in enumerate(results):
                if r["id"] == sk_id:
                    exact_match = results.pop(idx)
                    break
            if not exact_match:
                exact_match = self.get_skill_info(sk_id, realm_id)
            results.insert(0, exact_match)

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

