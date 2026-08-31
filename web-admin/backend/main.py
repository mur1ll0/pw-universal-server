"""
PW-ADMIN-WEB: Backend API do Painel de Administração Moderno
Substituto completo do pwAdmin arcaico com FastAPI, PostgreSQL 16 e DragonflyDB.
"""

import os
import asyncio
import json
from typing import Optional, List, Dict, Any
from fastapi import FastAPI, HTTPException, Depends, Query, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field
import asyncpg
import redis.asyncio as aioredis
import hashlib

from elements_decoder import decoder_instance, ItemOctetCodec, SkillOctetCodec

DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql://pw_admin:pw_secure_password_2026@localhost:5432/pw_database"
)
REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")

app = FastAPI(
    title="PW-Admin-Web API",
    version="2.0.0",
    description="Painel de Controle e Gestão Multi-Realm para Perfect World"
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

db_pool: Optional[asyncpg.Pool] = None
redis_client: Optional[aioredis.Redis] = None

# Helper para conversão segura de tipos de banco (BYTEA, datetime, records) para JSON
def safe_json_convert(obj: Any) -> Any:
    if obj is None:
        return None
    if isinstance(obj, (bytes, bytearray)):
        return obj.hex()
    if isinstance(obj, asyncpg.Record):
        return {k: safe_json_convert(v) for k, v in dict(obj).items()}
    if isinstance(obj, dict):
        return {k: safe_json_convert(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple, set)):
        return [safe_json_convert(item) for item in obj]
    if hasattr(obj, "isoformat"):
        return obj.isoformat()
    return obj

# Lista de Mapas Padrão do Jogo
DEFAULT_MAPS = [
    {"tag": 1, "name": "Mapa-Múndi (Pan Gu)", "category": "Mundo Principal", "folder": "world", "enabled": True},
    {"tag": 101, "name": "Caverna de Fogo (FB19 Humano)", "category": "Dungeon Clássica", "folder": "a01", "enabled": True},
    {"tag": 102, "name": "Toca dos Lobos (FB19 Fera)", "category": "Dungeon Clássica", "folder": "a02", "enabled": True},
    {"tag": 103, "name": "Túmulo do Herói (FB19 Elfo)", "category": "Dungeon Clássica", "folder": "a03", "enabled": True},
    {"tag": 104, "name": "Templo das Orquídeas (FB29)", "category": "Dungeon Clássica", "folder": "a04", "enabled": True},
    {"tag": 105, "name": "Túmulo do Herói (FB39)", "category": "Dungeon Clássica", "folder": "a05", "enabled": True},
    {"tag": 106, "name": "Residência dos Hades (FB49)", "category": "Dungeon Clássica", "folder": "a06", "enabled": True},
    {"tag": 107, "name": "Vale do Desastre (FB59)", "category": "Dungeon Clássica", "folder": "a07", "enabled": True},
    {"tag": 108, "name": "Portão Desalmado (FB69)", "category": "Dungeon Clássica", "folder": "a08", "enabled": True},
    {"tag": 109, "name": "Caverna do Tesouro (FB79)", "category": "Dungeon Clássica", "folder": "a09", "enabled": True},
    {"tag": 110, "name": "Ilha dos Uivadores (FB89)", "category": "Dungeon Clássica", "folder": "a10", "enabled": True},
    {"tag": 111, "name": "Terra dos Imortais (FB89)", "category": "Dungeon Clássica", "folder": "a11", "enabled": True},
    {"tag": 112, "name": "Purgatório Celeste (FB99)", "category": "Dungeon Clássica", "folder": "a12", "enabled": True},
    {"tag": 201, "name": "Cidade do Gelo (Frostcovered)", "category": "Instância Especial", "folder": "b01", "enabled": True},
    {"tag": 230, "name": "Palácio dos Crepúsculos (Dusk / TT)", "category": "Instância Especial", "folder": "b30", "enabled": True},
    {"tag": 231, "name": "Vale da Lua (Valley of Reciprocity)", "category": "Instância Especial", "folder": "b31", "enabled": True},
    {"tag": 232, "name": "Vale Primordial (Cubo do Destino)", "category": "Instância Especial", "folder": "b32", "enabled": True},
]

# Cache em memória de status dos mapas por Realm
realm_maps_state: Dict[str, List[Dict[str, Any]]] = {
    "realm_126": [m.copy() for m in DEFAULT_MAPS],
    "realm_148": [m.copy() for m in DEFAULT_MAPS],
    "realm_153": [m.copy() for m in DEFAULT_MAPS]
}

@app.on_event("startup")
async def startup_event():
    global db_pool, redis_client
    try:
        db_pool = await asyncpg.create_pool(DATABASE_URL, min_size=2, max_size=10)
        redis_client = aioredis.from_url(REDIS_URL, decode_responses=True)
        print("Conectado com sucesso ao PostgreSQL e DragonflyDB!")

        # Garante que os templates de classes existam para todos os realms
        async with db_pool.acquire() as conn:
            for r_id in ["realm_126", "realm_148", "realm_153"]:
                count = await conn.fetchval("SELECT COUNT(*) FROM class_templates WHERE realm_id = $1", r_id)
                if count == 0:
                    await seed_default_class_templates(conn, r_id)
    except Exception as e:
        print(f"Aviso ao conectar banco: {e}")

async def seed_default_class_templates(conn: asyncpg.Connection, realm_id: str):
    all_classes = [
        (0, "Guerreiro", 15, 10, 20, 5, 976.0, 219.2, 4187.3, 2097, [1, 2, 3, 4, 77, 69, 167]),
        (1, "Mago", 10, 10, 10, 20, 976.0, 219.2, 4187.3, 2867, [7, 8, 81, 88, 96, 167]),
        (2, "Espiritualista", 10, 10, 10, 20, 976.0, 219.2, 4187.3, 2867, [1450, 1451, 1453, 167]),
        (3, "Bárbaro", 15, 5, 25, 5, -1445.6, 219.3, 2642.0, 2258, [102, 104, 112, 150, 167]),
        (4, "Feiticeira", 15, 5, 15, 15, -1445.6, 219.3, 2642.0, 2867, [299, 300, 306, 312, 167]),
        (5, "Mercenário", 10, 20, 10, 5, 976.0, 219.2, 4187.3, 6, [1400, 1401, 1403, 167]),
        (6, "Arqueiro", 5, 15, 8, 22, -696.3, 219.0, -1178.8, 2250, [234, 235, 236, 245, 167]),
        (7, "Sacerdote", 10, 10, 15, 15, -696.3, 219.0, -1178.8, 2251, [113, 114, 125, 15, 19, 167]),
        (8, "Arcano", 15, 12, 18, 5, 976.0, 219.2, 4187.3, 6, [1500, 1501, 1503, 167]),
        (9, "Místico", 10, 10, 12, 18, 976.0, 219.2, 4187.3, 2867, [1600, 1601, 1603, 167]),
        (10, "Retalhador", 10, 18, 12, 10, 976.0, 219.2, 4187.3, 6, [1700, 1701, 1703, 167]),
        (11, "Tormentador", 10, 10, 12, 18, 976.0, 219.2, 4187.3, 2867, [1750, 1751, 1753, 167]),
    ]

    REALM_ALLOWED_CLASSES = {
        "realm_126": {0, 1, 3, 4, 6, 7},
        "realm_148": {0, 1, 2, 3, 4, 5, 6, 7, 8, 9},
        "realm_153": {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11},
    }
    allowed = REALM_ALLOWED_CLASSES.get(realm_id, {0, 1, 3, 4, 6, 7})
    classes = [c for c in all_classes if c[0] in allowed]

    # Limpa templates não permitidos para o Realm
    await conn.execute("DELETE FROM class_templates WHERE realm_id = $1 AND cls != ALL($2::smallint[])", realm_id, list(allowed))

    for cls, name, str_pt, agi, vit, eng, sx, sy, sz, weapon_id, skills in classes:
        tpl_id = await conn.fetchval(
            """
            INSERT INTO class_templates (
                realm_id, cls, name, initial_level, initial_cultivation,
                initial_money, initial_sp, strength, agility, vitality, energy,
                spawn_world_id, spawn_x, spawn_y, spawn_z
            )
            VALUES ($1, $2, $3, 1, 0, 0, 0, $4, $5, $6, $7, 1, $8, $9, $10)
            ON CONFLICT (realm_id, cls) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            """,
            realm_id, cls, name, str_pt, agi, vit, eng, sx, sy, sz
        )
        if not tpl_id:
            tpl_id = await conn.fetchval("SELECT id FROM class_templates WHERE realm_id = $1 AND cls = $2", realm_id, cls)

        # Arma inicial no corpo (slot 0)
        await conn.execute(
            """
            INSERT INTO class_template_items (
                template_id, container_type, slot, item_id, count,
                durability, max_durability, refine_level, sockets_count, socket_stones
            )
            VALUES ($1, 1, 0, $2, 1, 1400, 1400, 0, 0, '{}')
            ON CONFLICT (template_id, container_type, slot) DO UPDATE SET item_id = EXCLUDED.item_id, durability = 1400, max_durability = 1400
            """,
            tpl_id, weapon_id
        )

        # Arqueiro: flechas no corpo (slot 11)
        if cls == 6:
            await conn.execute(
                """
                INSERT INTO class_template_items (
                    template_id, container_type, slot, item_id, count,
                    durability, max_durability, refine_level, sockets_count, socket_stones
                )
                VALUES ($1, 1, 11, 2271, 1000, 0, 0, 0, 0, '{}')
                ON CONFLICT (template_id, container_type, slot) DO UPDATE SET item_id = EXCLUDED.item_id
                """,
                tpl_id
            )

        # Consumíveis bolsa
        bag_items = [(0, 2100, 5), (1, 1796, 10), (2, 1801, 10)]
        if cls == 6:
            bag_items.append((3, 2271, 1000))

        for slot, itm_id, count in bag_items:
            await conn.execute(
                """
                INSERT INTO class_template_items (
                    template_id, container_type, slot, item_id, count,
                    durability, max_durability, refine_level, sockets_count, socket_stones
                )
                VALUES ($1, 0, $2, $3, $4, 0, 0, 0, 0, '{}')
                ON CONFLICT (template_id, container_type, slot) DO UPDATE SET item_id = EXCLUDED.item_id
                """,
                tpl_id, slot, itm_id, count
            )

        # Habilidades
        for sk_id in skills:
            await conn.execute(
                """
                INSERT INTO class_template_skills (template_id, skill_id, level)
                VALUES ($1, $2, 1)
                ON CONFLICT (template_id, skill_id) DO NOTHING
                """,
                tpl_id, sk_id
            )

@app.on_event("shutdown")
async def shutdown_event():
    if db_pool:
        await db_pool.close()
    if redis_client:
        await redis_client.close()

async def get_db():
    if not db_pool:
        raise HTTPException(status_code=500, detail="Banco de dados desconectado")
    async with db_pool.acquire() as conn:
        yield conn

# ==============================================================================
# SCHEMAS PYDANTIC
# ==============================================================================

class CreateAccountRequest(BaseModel):
    username: str = Field(..., min_length=3, max_length=32)
    password: str = Field(..., min_length=4)
    email: Optional[str] = None
    gm_privileges: int = Field(0, ge=0, le=32)
    initial_gold: int = Field(0, ge=0)

class ResetPasswordRequest(BaseModel):
    account_id: int
    new_password: str = Field(..., min_length=4)

class SetGmRequest(BaseModel):
    account_id: int
    gm_level: int = Field(..., ge=0, le=32)

class GrantGoldRequest(BaseModel):
    account_id: int
    amount: int
    reason: Optional[str] = "Admin manual grant"

class BanAccountRequest(BaseModel):
    account_id: int
    is_banned: bool
    reason: Optional[str] = None
    duration_hours: Optional[int] = None

class SetMultipliersRequest(BaseModel):
    realm_id: str
    exp: float = 1.0
    sp: float = 1.0
    drop: float = 1.0
    gold: float = 1.0

class ToggleMapRequest(BaseModel):
    realm_id: str
    map_tag: int
    enabled: bool

class ToggleAllMapsRequest(BaseModel):
    enabled: bool

class AddItemRequest(BaseModel):
    character_id: int
    container_type: int = Field(0, ge=0, le=3) # 0=Inv, 1=Equip, 2=Storehouse, 3=Fashion
    slot: int = Field(..., ge=0, le=127)
    item_id: int
    count: int = Field(1, ge=1)
    refine_level: int = Field(0, ge=0, le=12)
    sockets_count: int = Field(0, ge=0, le=4)
    socket_stones: List[int] = []
    durability: Optional[int] = None
    max_durability: Optional[int] = None
    creator_name: Optional[str] = ""
    extra_data: Optional[str] = None

class MoveItemRequest(BaseModel):
    item_instance_id: int
    target_container_type: int = Field(..., ge=0, le=3)
    target_slot: int = Field(..., ge=0, le=127)

class EditItemRequest(BaseModel):
    item_instance_id: int
    slot: Optional[int] = None
    container_type: Optional[int] = None
    refine_level: Optional[int] = Field(None, ge=0, le=12)
    count: Optional[int] = Field(None, ge=1)
    durability: Optional[int] = None
    max_durability: Optional[int] = None
    bind_status: Optional[int] = None
    sockets_count: Optional[int] = Field(None, ge=0, le=4)
    socket_stones: Optional[List[int]] = None
    creator_name: Optional[str] = None
    extra_data: Optional[str] = None

class EncodeOctetsRequest(BaseModel):
    realm_id: str = "realm_126"
    item_id: int
    refine_level: int = 0
    sockets_count: int = 0
    socket_stones: List[int] = []
    durability: Optional[int] = None
    max_durability: Optional[int] = None
    creator_name: str = ""

class DecodeOctetsRequest(BaseModel):
    octets_hex: str


class SkillItemModel(BaseModel):
    skill_id: int
    level: int = Field(1, ge=1, le=12)
    progress: int = 0

class EncodeSkillsRequest(BaseModel):
    skills: List[SkillItemModel]

class DecodeSkillsRequest(BaseModel):
    octets_hex: str
    realm_id: Optional[str] = "realm_126"

class ImportSkillsRequest(BaseModel):
    octets_hex: str

class LearnAllSkillsRequest(BaseModel):
    realm_id: Optional[str] = "realm_126"

class AddSkillRequest(BaseModel):
    skill_id: int
    level: int = Field(1, ge=1, le=12)

class EditSkillRequest(BaseModel):
    skill_id: int
    level: int = Field(1, ge=1, le=12)

class EditCharacterStatsRequest(BaseModel):
    character_id: int
    level: Optional[int] = Field(None, ge=1, le=150)
    cultivation: Optional[int] = Field(None, ge=0, le=32)
    exp: Optional[int] = None
    sp: Optional[int] = None
    money: Optional[int] = None

class SystemBroadcastRequest(BaseModel):
    realm_id: str
    message: str

# ==============================================================================
# ROTAS: GESTÃO DE CONTAS (pwAdmin)
# ==============================================================================

@app.post("/api/accounts/create")
async def create_account(req: CreateAccountRequest, conn: asyncpg.Connection = Depends(get_db)):
    pwd_hash = hashlib.md5(f"{req.username.lower()}{req.password}".encode()).hexdigest()
    try:
        row = await conn.fetchrow(
            """
            INSERT INTO accounts (username, password_hash, email, gm_privileges, gold_balance)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, username, email, gm_privileges, gold_balance, created_at
            """,
            req.username, pwd_hash, req.email, req.gm_privileges, req.initial_gold
        )
        return safe_json_convert({"status": "success", "account": dict(row)})
    except asyncpg.UniqueViolationError:
        raise HTTPException(status_code=400, detail="Nome de usuário já existe")

@app.post("/api/accounts/reset-password")
async def reset_password(req: ResetPasswordRequest, conn: asyncpg.Connection = Depends(get_db)):
    acc = await conn.fetchrow("SELECT username FROM accounts WHERE id = $1", req.account_id)
    if not acc:
        raise HTTPException(status_code=404, detail="Conta não encontrada")
    
    pwd_hash = hashlib.md5(f"{acc['username'].lower()}{req.new_password}".encode()).hexdigest()
    await conn.execute("UPDATE accounts SET password_hash = $1 WHERE id = $2", pwd_hash, req.account_id)
    return {"status": "success", "message": f"Senha da conta {acc['username']} atualizada com sucesso!"}

@app.post("/api/accounts/set-gm")
async def set_gm(req: SetGmRequest, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute("UPDATE accounts SET gm_privileges = $1 WHERE id = $2", req.gm_level, req.account_id)
    if res == "UPDATE 0":
        raise HTTPException(status_code=404, detail="Conta não encontrada")
    return {"status": "success", "message": f"Privilégios de GM atualizados para nível {req.gm_level}"}

@app.post("/api/accounts/grant-gold")
async def grant_gold(req: GrantGoldRequest, conn: asyncpg.Connection = Depends(get_db)):
    row = await conn.fetchrow(
        """
        UPDATE accounts 
        SET gold_balance = gold_balance + $1 
        WHERE id = $2 
        RETURNING id, username, gold_balance
        """,
        req.amount, req.account_id
    )
    if not row:
        raise HTTPException(status_code=404, detail="Conta não encontrada")
    
    await conn.execute(
        """
        INSERT INTO admin_audit_logs (action_type, target_account_id, details)
        VALUES ('GRANT_GOLD', $1, $2)
        """,
        req.account_id,
        json.dumps({"amount": req.amount, "reason": req.reason})
    )
    return safe_json_convert({"status": "success", "account": dict(row)})

@app.post("/api/accounts/ban")
async def ban_account(req: BanAccountRequest, conn: asyncpg.Connection = Depends(get_db)):
    expires_at = None
    if req.is_banned and req.duration_hours:
        expires_at = await conn.fetchval(
            "SELECT CURRENT_TIMESTAMP + INTERVAL '1 hour' * $1", req.duration_hours
        )
    
    await conn.execute(
        """
        UPDATE accounts 
        SET is_banned = $1, ban_reason = $2, ban_expires_at = $3 
        WHERE id = $4
        """,
        req.is_banned, req.reason, expires_at, req.account_id
    )
    return {"status": "success", "banned": req.is_banned}

@app.get("/api/accounts/list")
async def list_accounts(
    limit: int = Query(50, le=200),
    offset: int = Query(0, ge=0),
    search: Optional[str] = None,
    conn: asyncpg.Connection = Depends(get_db)
):
    if search:
        rows = await conn.fetch(
            """
            SELECT id, username, email, gold_balance, gm_privileges, is_banned, created_at, last_login_at
            FROM accounts 
            WHERE LOWER(username) LIKE LOWER($1)
            ORDER BY id DESC LIMIT $2 OFFSET $3
            """,
            f"%{search}%", limit, offset
        )
    else:
        rows = await conn.fetch(
            """
            SELECT id, username, email, gold_balance, gm_privileges, is_banned, created_at, last_login_at
            FROM accounts 
            ORDER BY id DESC LIMIT $1 OFFSET $2
            """,
            limit, offset
        )
    return safe_json_convert({"accounts": [dict(r) for r in rows]})

# ==============================================================================
# ROTAS: INSPEÇÃO E EDIÇÃO GRANULAR DE PERSONAGENS, ITENS & SKILLS (elements.data)
# ==============================================================================

@app.get("/api/characters/search")
async def search_characters(
    name: Optional[str] = None,
    realm_id: Optional[str] = None,
    conn: asyncpg.Connection = Depends(get_db)
):
    query = "SELECT id, account_id, realm_id, name, race, cls, level, cultivation, money, world_id, is_deleted FROM characters WHERE 1=1"
    params = []
    if name:
        params.append(f"%{name}%")
        query += f" AND LOWER(name) LIKE LOWER(${len(params)})"
    if realm_id:
        params.append(realm_id)
        query += f" AND realm_id = ${len(params)}"
    
    query += " ORDER BY level DESC LIMIT 50"
    rows = await conn.fetch(query, *params)
    return safe_json_convert({"characters": [dict(r) for r in rows]})

@app.get("/api/characters/{char_id}")
async def get_character_details(char_id: int, conn: asyncpg.Connection = Depends(get_db)):
    char = await conn.fetchrow("SELECT * FROM characters WHERE id = $1", char_id)
    if not char:
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    
    char_dict = dict(char)
    char_realm = char_dict.get("realm_id") or "realm_126"
    items_rows = await conn.fetch("SELECT * FROM character_items WHERE character_id = $1 ORDER BY container_type, slot", char_id)
    skills_rows = await conn.fetch("SELECT * FROM character_skills WHERE character_id = $1 ORDER BY skill_id", char_id)
    quests_rows = await conn.fetch("SELECT * FROM character_quests WHERE character_id = $1", char_id)

    # Decodifica e enriquece itens usando elements.data e surfaces/iconset do Realm
    enriched_items = []
    for itm in items_rows:
        d = dict(itm)
        info = decoder_instance.get_item_info(char_realm, d["item_id"])
        d["name"] = info.get("name", f"Item #{d['item_id']}")
        d["type"] = info.get("type", "Item")
        d["category"] = info.get("category", "Geral")
        d["quality"] = info.get("quality", "normal")
        d["icon"] = info.get("icon", "fa-solid fa-box")
        d["icon_file"] = info.get("icon_file", "")
        d["icon_img"] = info.get("icon_img")
        d["desc"] = info.get("desc", "")
        d["atk_phys"] = info.get("atk_phys")
        d["atk_magic"] = info.get("atk_magic")
        d["def_phys"] = info.get("def_phys")
        d["def_magic"] = info.get("def_magic")

        # Processamento e Decodificação de Octets
        oct_data = d.get("extra_data")
        parsed_oct = ItemOctetCodec.parse_item_octets(oct_data)
        d["octets_parsed"] = parsed_oct
        d["octets_hex"] = ItemOctetCodec.bytes_to_hex(oct_data) if oct_data else ""
        if not d["octets_hex"] and any(k in d["category"].lower() for k in ["arma", "armadura", "ornamento", "jóia", "joia", "voo", "moda", "livro", "tomo"]):
            gen_bytes = decoder_instance.generate_octets_for_item(
                char_realm, d["item_id"], d.get("refine_level", 0), d.get("sockets_count", 0),
                d.get("socket_stones", []), d.get("durability", 2800), d.get("max_durability", 2800)
            )
            d["octets_hex"] = gen_bytes.hex()
            d["octets_parsed"] = ItemOctetCodec.parse_item_octets(gen_bytes)

        enriched_items.append(d)

    # Decodifica e enriquece habilidades do personagem
    enriched_skills = []
    for sk in skills_rows:
        d = dict(sk)
        sk_info = decoder_instance.get_skill_info(d["skill_id"], char_realm)
        d["name"] = sk_info.get("name", f"Habilidade #{d['skill_id']}")
        d["name_en"] = sk_info.get("name_en", "")
        d["class_name"] = sk_info.get("class_name", "Comum")
        d["type"] = sk_info.get("type", "Skill")
        d["icon"] = sk_info.get("icon", "fa-solid fa-wand-magic-sparkles text-indigo-400")
        d["icon_file"] = sk_info.get("icon_file", "")
        d["icon_img"] = sk_info.get("icon_img")
        d["desc"] = sk_info.get("desc", "")
        d["max_lv"] = sk_info.get("max_lv", 10)
        enriched_skills.append(d)

    skills_octets = SkillOctetCodec.build_skills_octets(enriched_skills)
    skills_octets_hex = SkillOctetCodec.bytes_to_hex(skills_octets)

    response_payload = {
        "status": "success",
        "character": char_dict,
        "items": enriched_items,
        "skills": enriched_skills,
        "skills_octets_hex": skills_octets_hex,
        "quests": [dict(q) for q in quests_rows],
    }
    return safe_json_convert(response_payload)

@app.post("/api/characters/{char_id}/items/add")
async def add_item_to_character(char_id: int, req: AddItemRequest, conn: asyncpg.Connection = Depends(get_db)):
    try:
        char = await conn.fetchrow("SELECT realm_id FROM characters WHERE id = $1", char_id)
        realm_id = char["realm_id"] if char and char["realm_id"] else "realm_126"

        if req.extra_data:
            extra_bytes = ItemOctetCodec.hex_to_bytes(req.extra_data)
        else:
            extra_bytes = decoder_instance.generate_octets_for_item(
                realm_id, req.item_id, req.refine_level, req.sockets_count,
                req.socket_stones, req.durability, req.max_durability, req.creator_name or ""
            )

        row = await conn.fetchrow(
            """
            INSERT INTO character_items (
                character_id, container_type, slot, item_id, count,
                refine_level, sockets_count, socket_stones, durability, max_durability,
                extra_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (character_id, container_type, slot) 
            DO UPDATE SET 
                item_id = EXCLUDED.item_id,
                count = EXCLUDED.count,
                refine_level = EXCLUDED.refine_level,
                sockets_count = EXCLUDED.sockets_count,
                socket_stones = EXCLUDED.socket_stones,
                durability = EXCLUDED.durability,
                max_durability = EXCLUDED.max_durability,
                extra_data = EXCLUDED.extra_data,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id, character_id, container_type, slot, item_id, count, refine_level, sockets_count, extra_data
            """,
            char_id, req.container_type, req.slot, req.item_id, req.count,
            req.refine_level, req.sockets_count, req.socket_stones,
            req.durability or 2800, req.max_durability or 2800, extra_bytes
        )
        return safe_json_convert({"status": "success", "item": dict(row)})
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/api/characters/{char_id}/items/move")
async def move_character_item(char_id: int, req: MoveItemRequest, conn: asyncpg.Connection = Depends(get_db)):
    existing = await conn.fetchrow(
        "SELECT id FROM character_items WHERE character_id = $1 AND container_type = $2 AND slot = $3 AND id != $4",
        char_id, req.target_container_type, req.target_slot, req.item_instance_id
    )
    if existing:
        raise HTTPException(status_code=400, detail=f"O slot {req.target_slot} do container selecionado já está ocupado.")
    
    res = await conn.execute(
        "UPDATE character_items SET container_type = $1, slot = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3 AND character_id = $4",
        req.target_container_type, req.target_slot, req.item_instance_id, char_id
    )
    if res == "UPDATE 0":
        raise HTTPException(status_code=404, detail="Item não encontrado")
    return {"status": "success", "message": "Item movido com sucesso!"}

@app.post("/api/items/{item_instance_id}/edit")
async def edit_item(item_instance_id: int, req: EditItemRequest, conn: asyncpg.Connection = Depends(get_db)):
    item_curr = await conn.fetchrow(
        "SELECT ci.*, c.realm_id FROM character_items ci JOIN characters c ON c.id = ci.character_id WHERE ci.id = $1",
        item_instance_id
    )
    if not item_curr:
        raise HTTPException(status_code=404, detail="Item não encontrado")

    realm_id = item_curr["realm_id"] or "realm_126"
    updates = []
    params = []

    # Extra Data / Octets Processing
    if req.extra_data is not None:
        extra_bytes = ItemOctetCodec.hex_to_bytes(req.extra_data)
        parsed = ItemOctetCodec.parse_item_octets(extra_bytes)
        params.append(extra_bytes)
        updates.append(f"extra_data = ${len(params)}")
        if parsed.get("has_octets"):
            params.append(parsed.get("refine_level", 0))
            updates.append(f"refine_level = ${len(params)}")
            params.append(parsed.get("sockets_count", 0))
            updates.append(f"sockets_count = ${len(params)}")
            params.append(parsed.get("socket_stones", []))
            updates.append(f"socket_stones = ${len(params)}")
            params.append(parsed.get("durability", 2800))
            updates.append(f"durability = ${len(params)}")
            params.append(parsed.get("max_durability", 2800))
            updates.append(f"max_durability = ${len(params)}")
    else:
        # Se alterou propriedades de equipamento, regenera extra_data
        ref_lvl = req.refine_level if req.refine_level is not None else item_curr["refine_level"]
        sock_cnt = req.sockets_count if req.sockets_count is not None else item_curr["sockets_count"]
        stones = req.socket_stones if req.socket_stones is not None else (item_curr["socket_stones"] or [])
        dura = req.durability if req.durability is not None else item_curr["durability"]
        max_dura = req.max_durability if req.max_durability is not None else item_curr["max_durability"]
        creator = req.creator_name if req.creator_name is not None else ""

        if any(x is not None for x in [req.refine_level, req.sockets_count, req.socket_stones, req.durability, req.creator_name]):
            new_octets = decoder_instance.generate_octets_for_item(
                realm_id, item_curr["item_id"], ref_lvl, sock_cnt, stones, dura, max_dura, creator
            )
            params.append(new_octets)
            updates.append(f"extra_data = ${len(params)}")

    if req.count is not None:
        params.append(req.count)
        updates.append(f"count = ${len(params)}")
    if req.refine_level is not None and req.extra_data is None:
        params.append(req.refine_level)
        updates.append(f"refine_level = ${len(params)}")
    if req.sockets_count is not None and req.extra_data is None:
        params.append(req.sockets_count)
        updates.append(f"sockets_count = ${len(params)}")
    if req.socket_stones is not None and req.extra_data is None:
        params.append(req.socket_stones)
        updates.append(f"socket_stones = ${len(params)}")
    if req.durability is not None and req.extra_data is None:
        params.append(req.durability)
        updates.append(f"durability = ${len(params)}")
    if req.max_durability is not None and req.extra_data is None:
        params.append(req.max_durability)
        updates.append(f"max_durability = ${len(params)}")
    if req.bind_status is not None:
        params.append(req.bind_status)
        updates.append(f"bind_status = ${len(params)}")
    if req.slot is not None:
        params.append(req.slot)
        updates.append(f"slot = ${len(params)}")
    if req.container_type is not None:
        params.append(req.container_type)
        updates.append(f"container_type = ${len(params)}")
        
    if not updates:
        raise HTTPException(status_code=400, detail="Nenhum campo fornecido para atualização")
        
    params.append(item_instance_id)
    query = f"UPDATE character_items SET {', '.join(updates)}, updated_at = CURRENT_TIMESTAMP WHERE id = ${len(params)} RETURNING *"
    try:
        row = await conn.fetchrow(query, *params)
        if not row:
            raise HTTPException(status_code=404, detail="Item não encontrado")
        return safe_json_convert({"status": "success", "item": dict(row)})
    except asyncpg.UniqueViolationError:
        raise HTTPException(status_code=400, detail="Conflito: Já existe outro item ocupando este slot.")


@app.delete("/api/items/{item_instance_id}")
async def delete_item(item_instance_id: int, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute("DELETE FROM character_items WHERE id = $1", item_instance_id)
    if res == "DELETE 0":
        raise HTTPException(status_code=404, detail="Item não encontrado")
    return {"status": "success", "message": "Item removido com sucesso!"}

# Rotas de Habilidades do Personagem
@app.post("/api/characters/{char_id}/skills/add")
async def add_character_skill(char_id: int, req: AddSkillRequest, conn: asyncpg.Connection = Depends(get_db)):
    await conn.execute(
        """
        INSERT INTO character_skills (character_id, skill_id, level)
        VALUES ($1, $2, $3)
        ON CONFLICT (character_id, skill_id) DO UPDATE SET level = EXCLUDED.level
        """,
        char_id, req.skill_id, req.level
    )
    return {"status": "success", "message": f"Habilidade {req.skill_id} salva no nível {req.level}!"}

@app.post("/api/characters/{char_id}/skills/edit")
async def edit_character_skill(char_id: int, req: EditSkillRequest, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute(
        "UPDATE character_skills SET level = $1 WHERE character_id = $2 AND skill_id = $3",
        req.level, char_id, req.skill_id
    )
    if res == "UPDATE 0":
        raise HTTPException(status_code=404, detail="Habilidade não encontrada no personagem")
    return {"status": "success", "message": f"Nível da habilidade atualizado para {req.level}!"}

@app.delete("/api/characters/{char_id}/skills/{skill_id}")
async def delete_character_skill(char_id: int, skill_id: int, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute(
        "DELETE FROM character_skills WHERE character_id = $1 AND skill_id = $2",
        char_id, skill_id
    )
    if res == "DELETE 0":
        raise HTTPException(status_code=404, detail="Habilidade não encontrada")
    return {"status": "success", "message": "Habilidade removida com sucesso!"}

@app.post("/api/characters/{char_id}/skills/learn-all")
async def learn_all_character_skills(char_id: int, req: Optional[LearnAllSkillsRequest] = None, conn: asyncpg.Connection = Depends(get_db)):
    """Aprende todas as habilidades da classe e comuns no nível máximo (10) para o personagem"""
    char = await conn.fetchrow("SELECT realm_id, cls FROM characters WHERE id = $1", char_id)
    if not char:
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    
    realm_id = req.realm_id if req and req.realm_id else (char["realm_id"] or "realm_126")
    class_id = char["cls"]
    
    max_skills = decoder_instance.get_max_skills_for_class(class_id, realm_id)
    for sk in max_skills:
        await conn.execute(
            """
            INSERT INTO character_skills (character_id, skill_id, level)
            VALUES ($1, $2, $3)
            ON CONFLICT (character_id, skill_id) DO UPDATE SET level = EXCLUDED.level
            """,
            char_id, sk["skill_id"], sk["level"]
        )
    return {
        "status": "success",
        "message": f"Todas as {len(max_skills)} habilidades da classe foram adicionadas/atualizadas no nível máximo!",
        "count": len(max_skills)
    }

@app.post("/api/characters/{char_id}/skills/import-hex")
async def import_character_skills_hex(char_id: int, req: ImportSkillsRequest, conn: asyncpg.Connection = Depends(get_db)):
    """Importa um payload hexadecimal bruto do Skill HexGen diretamente no personagem"""
    char = await conn.fetchrow("SELECT id FROM characters WHERE id = $1", char_id)
    if not char:
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    
    parsed = SkillOctetCodec.parse_skills_octets(req.octets_hex)
    if not parsed:
        raise HTTPException(status_code=400, detail="Formato de Octets de Habilidades inválido ou vazio")
        
    for sk in parsed:
        await conn.execute(
            """
            INSERT INTO character_skills (character_id, skill_id, level)
            VALUES ($1, $2, $3)
            ON CONFLICT (character_id, skill_id) DO UPDATE SET level = EXCLUDED.level
            """,
            char_id, sk["skill_id"], sk["level"]
        )
    return {
        "status": "success",
        "message": f"{len(parsed)} habilidades importadas com sucesso!",
        "count": len(parsed)
    }

@app.post("/api/characters/{char_id}/edit-stats")
async def edit_character_stats(char_id: int, req: EditCharacterStatsRequest, conn: asyncpg.Connection = Depends(get_db)):
    updates = []
    params = []
    
    if req.level is not None:
        params.append(req.level)
        updates.append(f"level = ${len(params)}")
    if req.cultivation is not None:
        params.append(req.cultivation)
        updates.append(f"cultivation = ${len(params)}")
    if req.exp is not None:
        params.append(req.exp)
        updates.append(f"exp = ${len(params)}")
    if req.sp is not None:
        params.append(req.sp)
        updates.append(f"sp = ${len(params)}")
    if req.money is not None:
        params.append(req.money)
        updates.append(f"money = ${len(params)}")
        
    if not updates:
        raise HTTPException(status_code=400, detail="Nenhum atributo informado")
        
    params.append(char_id)
    query = f"UPDATE characters SET {', '.join(updates)}, updated_at = CURRENT_TIMESTAMP WHERE id = ${len(params)} RETURNING id, name, level, cultivation, exp, sp, money"
    
    row = await conn.fetchrow(query, *params)
    if not row:
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    return safe_json_convert({"status": "success", "character": dict(row)})

class UnequipItemRequest(BaseModel):
    item_instance_id: int

class TeleportCharacterRequest(BaseModel):
    world_id: int = 1
    pos_x: float
    pos_y: float
    pos_z: float

@app.post("/api/characters/{char_id}/items/unequip")
async def unequip_character_item(char_id: int, req: UnequipItemRequest, conn: asyncpg.Connection = Depends(get_db)):
    item = await conn.fetchrow("SELECT id, container_type, slot FROM character_items WHERE id = $1 AND character_id = $2", req.item_instance_id, char_id)
    if not item:
        raise HTTPException(status_code=404, detail="Item não encontrado")
    
    if item["container_type"] == 0:
        return {"status": "success", "message": "O item já está na bolsa!", "target_slot": item["slot"]}

    occupied_rows = await conn.fetch("SELECT slot FROM character_items WHERE character_id = $1 AND container_type = 0", char_id)
    occupied_slots = {r["slot"] for r in occupied_rows}

    target_slot = None
    for s in range(32):
        if s not in occupied_slots:
            target_slot = s
            break
    
    if target_slot is None:
        raise HTTPException(status_code=400, detail="A bolsa do personagem está cheia! Libere espaço antes de desequipar.")

    await conn.execute(
        "UPDATE character_items SET container_type = 0, slot = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND character_id = $3",
        target_slot, req.item_instance_id, char_id
    )
    return {"status": "success", "message": f"Item movido para o slot {target_slot} da bolsa!", "target_slot": target_slot}

@app.post("/api/characters/{char_id}/teleport")
async def teleport_character(char_id: int, req: TeleportCharacterRequest, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute(
        """
        UPDATE characters 
        SET world_id = $1, pos_x = $2, pos_y = $3, pos_z = $4, updated_at = CURRENT_TIMESTAMP 
        WHERE id = $5
        """,
        req.world_id, req.pos_x, req.pos_y, req.pos_z, char_id
    )
    if res == "UPDATE 0":
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    return {"status": "success", "message": f"Personagem teletransportado com sucesso para ({req.pos_x:.1f}, {req.pos_y:.1f}, {req.pos_z:.1f}) no Mapa {req.world_id}!"}

@app.post("/api/characters/{char_id}/teleport-cdd")
async def teleport_to_dragon_city(char_id: int, conn: asyncpg.Connection = Depends(get_db)):
    await conn.execute(
        """
        UPDATE characters 
        SET world_id = 1, pos_x = 550.0, pos_y = 200.0, pos_z = 650.0 
        WHERE id = $1
        """,
        char_id
    )
    return {"status": "success", "message": "Personagem teletransportado com segurança para a Cidade do Dragão!"}

# ==============================================================================
# ROTAS: GERENCIAMENTO DE MAPAS & INSTÂNCIAS (LIGAR / DESLIGAR / LIGAR TODOS)
# ==============================================================================

@app.get("/api/realms/{realm_id}/maps")
async def get_realm_maps(realm_id: str):
    maps = realm_maps_state.get(realm_id, DEFAULT_MAPS)
    return {"realm_id": realm_id, "maps": maps}

@app.post("/api/realms/{realm_id}/maps/toggle")
async def toggle_realm_map(req: ToggleMapRequest):
    realm_maps = realm_maps_state.setdefault(req.realm_id, [m.copy() for m in DEFAULT_MAPS])
    for m in realm_maps:
        if m["tag"] == req.map_tag:
            m["enabled"] = req.enabled
            status_text = "LIGADO" if req.enabled else "DESLIGADO"
            return {"status": "success", "message": f"Mapa '{m['name']}' (Tag {m['tag']}) {status_text} com sucesso no {req.realm_id}!"}
    raise HTTPException(status_code=404, detail="Mapa não encontrado")

@app.post("/api/realms/{realm_id}/maps/toggle-all")
async def toggle_all_realm_maps(realm_id: str, req: ToggleAllMapsRequest):
    realm_maps = realm_maps_state.setdefault(realm_id, [m.copy() for m in DEFAULT_MAPS])
    for m in realm_maps:
        m["enabled"] = req.enabled
    status_text = "LIGADOS" if req.enabled else "DESLIGADOS"
    return {"status": "success", "message": f"Todos os mapas foram {status_text} com sucesso no {realm_id}!"}

# ==============================================================================
# ROTAS: BUSCA NO ELEMENTS.DATA & CATÁLOGO DE HABILIDADES
# ==============================================================================

@app.get("/api/elements/search-items")
async def search_elements_items(
    q: str = Query("", min_length=0),
    category: str = Query(""),
    container_type: Optional[int] = Query(None),
    slot_filter: Optional[int] = Query(None),
    class_id: Optional[int] = Query(None),
    realm_id: str = Query("realm_126"),
    limit: int = Query(50, le=200)
):
    results = decoder_instance.search_items(
        realm_id=realm_id,
        query=q,
        category=category,
        container_type=container_type,
        slot_filter=slot_filter,
        class_id=class_id,
        limit=limit
    )
    return {"status": "success", "results": results}

@app.get("/api/elements/item/{item_id}")
async def get_element_item(item_id: int, realm_id: str = Query("realm_126")):
    info = decoder_instance.get_item_info(realm_id=realm_id, item_id=item_id)
    return {"status": "success", "item": info}

@app.get("/api/elements/search-skills")
async def search_elements_skills(
    q: str = Query("", min_length=0),
    class_id: Optional[int] = Query(None),
    realm_id: str = Query("realm_126"),
    limit: int = Query(50, le=200)
):
    results = decoder_instance.search_skills(query=q, class_id=class_id, realm_id=realm_id, limit=limit)
    return {"status": "success", "results": results}

@app.get("/api/elements/icon/{realm_id}/{item_id}.png")
async def get_item_icon_png(realm_id: str, item_id: int):
    """Serve a imagem PNG 32x32 do item recortada do atlas DDS em tempo real com cache HTTP"""
    info = decoder_instance.get_item_info(realm_id=realm_id, item_id=item_id)
    icon_file = info.get("icon_file")
    if not icon_file:
        raise HTTPException(status_code=404, detail="Ícone não definido para este item")

    png_bytes = decoder_instance.icon_manager.get_item_icon_png(realm_id, icon_file)
    if not png_bytes:
        raise HTTPException(status_code=404, detail="Ícone não encontrado no iconset do realm")

    return Response(
        content=png_bytes,
        media_type="image/png",
        headers={"Cache-Control": "public, max-age=86400"}
    )

@app.get("/api/elements/skill-icon/{realm_id}/{skill_id}.png")
async def get_skill_icon_png(realm_id: str, skill_id: int):
    """Serve a imagem PNG 64x64 da habilidade recortada do atlas de skills em tempo real com cache HTTP"""
    info = decoder_instance.get_skill_info(skill_id=skill_id, realm_id=realm_id)
    icon_file = info.get("icon_file") or "unknown.dds"

    png_bytes = decoder_instance.icon_manager.get_skill_icon_png(realm_id, icon_file)
    if not png_bytes:
        # Fallback para o ícone padrão desconhecido
        png_bytes = decoder_instance.icon_manager.get_skill_icon_png(realm_id, "unknown.dds")
        if not png_bytes:
            png_bytes = decoder_instance.icon_manager.get_item_icon_png(realm_id, "unknown.dds")

    if not png_bytes:
        raise HTTPException(status_code=404, detail="Ícone de skill não encontrado no iconset do realm")

    return Response(
        content=png_bytes,
        media_type="image/png",
        headers={"Cache-Control": "public, max-age=86400"}
    )

@app.get("/api/elements/raw-icon/{realm_id}/{icon_filename}")
async def get_raw_icon_png(realm_id: str, icon_filename: str):
    """Serve a imagem PNG 32x32 a partir do nome bruto do arquivo .dds"""
    png_bytes = decoder_instance.icon_manager.get_item_icon_png(realm_id, icon_filename)
    if not png_bytes:
        png_bytes = decoder_instance.icon_manager.get_skill_icon_png(realm_id, icon_filename)
    
    if not png_bytes:
        raise HTTPException(status_code=404, detail="Ícone não encontrado no iconset")

    return Response(
        content=png_bytes,
        media_type="image/png",
        headers={"Cache-Control": "public, max-age=86400"}
    )

# ==============================================================================
# ROTAS: MULTI-REALM, EVENTOS E MÉTRICAS
# ==============================================================================

async def is_realm_active(host: str, port: int) -> bool:
    hosts = [host, f"pw-{host}", "127.0.0.1", "localhost"]
    for h in hosts:
        try:
            _, writer = await asyncio.wait_for(asyncio.open_connection(h, port), timeout=0.25)
            writer.close()
            await writer.wait_closed()
            return True
        except Exception:
            continue
    return False

@app.get("/api/realms/list")
async def list_realms(conn: asyncpg.Connection = Depends(get_db)):
    rows = await conn.fetch("SELECT * FROM realms ORDER BY id ASC")
    realms_list = []
    for r in rows:
        d = dict(r)
        realm_id = d["id"]
        port = d["port"]
        active = await is_realm_active(realm_id.replace("_", "-"), port)
        d["is_online"] = active
        await conn.execute("UPDATE realms SET is_online = $1 WHERE id = $2", active, realm_id)
        
        online_count = 0
        if redis_client:
            online_count = await redis_client.scard(f"online:{realm_id}") or 0
        d["online_players"] = online_count
        d["double_exp_multiplier"] = float(d["double_exp_multiplier"])
        d["double_sp_multiplier"] = float(d["double_sp_multiplier"])
        d["double_drop_multiplier"] = float(d["double_drop_multiplier"])
        d["double_gold_multiplier"] = float(d["double_gold_multiplier"])
        realms_list.append(d)
    return safe_json_convert({"realms": realms_list})

@app.post("/api/realms/set-multipliers")
async def set_multipliers(req: SetMultipliersRequest, conn: asyncpg.Connection = Depends(get_db)):
    await conn.execute(
        """
        UPDATE realms 
        SET double_exp_multiplier = $1,
            double_sp_multiplier = $2,
            double_drop_multiplier = $3,
            double_gold_multiplier = $4
        WHERE id = $5
        """,
        req.exp, req.sp, req.drop, req.gold, req.realm_id
    )
    return {"status": "success", "message": f"Multiplicadores do Realm {req.realm_id} atualizados!"}

@app.post("/api/realms/broadcast")
async def broadcast_announcement(req: SystemBroadcastRequest):
    if redis_client:
        msg_payload = {
            "realm_id": req.realm_id,
            "channel": 5,
            "sender_id": 0,
            "sender_name": "SISTEMA",
            "content": req.message,
            "timestamp": int(asyncio.get_event_loop().time())
        }
        await redis_client.publish(f"chat:{req.realm_id}:world", json.dumps(msg_payload))
        return {"status": "success", "message": "Anúncio transmitido com sucesso!"}
    raise HTTPException(status_code=500, detail="Redis desconectado")

@app.get("/api/metrics")
async def get_system_metrics(conn: asyncpg.Connection = Depends(get_db)):
    total_accounts = await conn.fetchval("SELECT COUNT(*) FROM accounts")
    total_chars = await conn.fetchval("SELECT COUNT(*) FROM characters WHERE is_deleted = FALSE")
    
    rows = await conn.fetch("SELECT * FROM realms ORDER BY id ASC")
    realms_list = []
    total_online = 0
    online_by_realm = {}
    
    for r in rows:
        d = dict(r)
        realm_id = d["id"]
        port = d["port"]
        active = await is_realm_active(realm_id.replace("_", "-"), port)
        d["is_online"] = active
        await conn.execute("UPDATE realms SET is_online = $1 WHERE id = $2", active, realm_id)
        
        count = 0
        if redis_client:
            count = await redis_client.scard(f"online:{realm_id}") or 0
        
        online_by_realm[realm_id] = count
        if active:
            total_online += count
            
        d["online_players"] = count
        d["double_exp_multiplier"] = float(d["double_exp_multiplier"])
        d["double_sp_multiplier"] = float(d["double_sp_multiplier"])
        d["double_drop_multiplier"] = float(d["double_drop_multiplier"])
        d["double_gold_multiplier"] = float(d["double_gold_multiplier"])
        realms_list.append(d)
        
    return safe_json_convert({
        "total_accounts": total_accounts,
        "total_characters": total_chars,
        "online_by_realm": online_by_realm,
        "total_online": total_online,
        "realms": realms_list
    })

# ==============================================================================
# ROTAS: GESTÃO DE TEMPLATES DE CLASSES (pwAdmin Class Config)
# ==============================================================================

class SaveTemplateItem(BaseModel):
    container_type: int = Field(0, description="1: Equipado, 0: Inventário")
    slot: int
    item_id: int
    count: int = Field(1, ge=1)
    durability: int = 10000
    max_durability: int = 10000
    refine_level: int = Field(0, ge=0, le=12)
    sockets_count: int = Field(0, ge=0, le=4)
    socket_stones: List[int] = []

class SaveTemplateSkill(BaseModel):
    skill_id: int
    level: int = Field(1, ge=1, le=10)

class SaveClassTemplateRequest(BaseModel):
    name: Optional[str] = None
    initial_level: int = Field(1, ge=1, le=150)
    initial_cultivation: int = Field(0, ge=0, le=32)
    initial_money: int = Field(0, ge=0)
    initial_sp: int = Field(0, ge=0)
    strength: int = Field(10, ge=1)
    agility: int = Field(10, ge=1)
    vitality: int = Field(10, ge=1)
    energy: int = Field(10, ge=1)
    spawn_world_id: int = 1
    spawn_x: float
    spawn_y: float
    spawn_z: float
    items: List[SaveTemplateItem] = []
    skills: List[SaveTemplateSkill] = []

@app.get("/api/templates/list")
async def list_class_templates(
    realm_id: str = Query("realm_126"),
    conn: asyncpg.Connection = Depends(get_db)
):
    rows = await conn.fetch(
        """
        SELECT * FROM class_templates 
        WHERE realm_id = $1 
        ORDER BY cls ASC
        """,
        realm_id
    )
    return safe_json_convert({"status": "success", "templates": [dict(r) for r in rows]})

@app.get("/api/templates/{realm_id}/{cls}")
async def get_class_template(
    realm_id: str,
    cls: int,
    conn: asyncpg.Connection = Depends(get_db)
):
    tpl = await conn.fetchrow(
        "SELECT * FROM class_templates WHERE realm_id = $1 AND cls = $2",
        realm_id, cls
    )
    if not tpl:
        raise HTTPException(status_code=404, detail="Template de classe não encontrado")

    items = await conn.fetch(
        """
        SELECT * FROM class_template_items 
        WHERE template_id = $1 
        ORDER BY container_type DESC, slot ASC
        """,
        tpl["id"]
    )
    skills = await conn.fetch(
        "SELECT * FROM class_template_skills WHERE template_id = $1 ORDER BY skill_id ASC",
        tpl["id"]
    )

    enriched_items = []
    for itm in items:
        d = dict(itm)
        info = decoder_instance.get_item_info(realm_id, d["item_id"])
        d["name"] = info.get("name", f"Item #{d['item_id']}")
        d["type"] = info.get("type", "Item")
        d["category"] = info.get("category", "Geral")
        d["quality"] = info.get("quality", "normal")
        d["icon"] = info.get("icon", "fa-solid fa-box")
        enriched_items.append(d)

    enriched_skills = []
    for sk in skills:
        d = dict(sk)
        sk_info = decoder_instance.get_skill_info(d["skill_id"])
        d["name"] = sk_info.get("name", f"Habilidade #{d['skill_id']}")
        d["class_name"] = sk_info.get("class_name", "Comum")
        d["type"] = sk_info.get("type", "Skill")
        d["icon"] = sk_info.get("icon", "fa-solid fa-wand-magic-sparkles text-indigo-400")
        enriched_skills.append(d)

    return safe_json_convert({
        "status": "success",
        "template": dict(tpl),
        "items": enriched_items,
        "skills": enriched_skills
    })

@app.post("/api/templates/{realm_id}/{cls}/save")
async def save_class_template(
    realm_id: str,
    cls: int,
    req: SaveClassTemplateRequest,
    conn: asyncpg.Connection = Depends(get_db)
):
    tpl_name = req.name or f"Classe {cls}"
    async with conn.transaction():
        tpl_id = await conn.fetchval(
            """
            INSERT INTO class_templates (
                realm_id, cls, name, initial_level, initial_cultivation,
                initial_money, initial_sp, strength, agility, vitality, energy,
                spawn_world_id, spawn_x, spawn_y, spawn_z, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, CURRENT_TIMESTAMP)
            ON CONFLICT (realm_id, cls) DO UPDATE SET
                name = EXCLUDED.name,
                initial_level = EXCLUDED.initial_level,
                initial_cultivation = EXCLUDED.initial_cultivation,
                initial_money = EXCLUDED.initial_money,
                initial_sp = EXCLUDED.initial_sp,
                strength = EXCLUDED.strength,
                agility = EXCLUDED.agility,
                vitality = EXCLUDED.vitality,
                energy = EXCLUDED.energy,
                spawn_world_id = EXCLUDED.spawn_world_id,
                spawn_x = EXCLUDED.spawn_x,
                spawn_y = EXCLUDED.spawn_y,
                spawn_z = EXCLUDED.spawn_z,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            """,
            realm_id, cls, tpl_name, req.initial_level, req.initial_cultivation,
            req.initial_money, req.initial_sp, req.strength, req.agility,
            req.vitality, req.energy, req.spawn_world_id, req.spawn_x, req.spawn_y, req.spawn_z
        )

        # Deleta itens antigos e insere os novos
        await conn.execute("DELETE FROM class_template_items WHERE template_id = $1", tpl_id)
        for itm in req.items:
            await conn.execute(
                """
                INSERT INTO class_template_items (
                    template_id, container_type, slot, item_id, count,
                    durability, max_durability, refine_level, sockets_count, socket_stones
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                """,
                tpl_id, itm.container_type, itm.slot, itm.item_id, itm.count,
                itm.durability, itm.max_durability, itm.refine_level, itm.sockets_count, itm.socket_stones
            )

        # Deleta skills antigas e insere as novas
        await conn.execute("DELETE FROM class_template_skills WHERE template_id = $1", tpl_id)
        for sk in req.skills:
            await conn.execute(
                """
                INSERT INTO class_template_skills (template_id, skill_id, level)
                VALUES ($1, $2, $3)
                ON CONFLICT (template_id, skill_id) DO UPDATE SET level = EXCLUDED.level
                """,
                tpl_id, sk.skill_id, sk.level
            )

    return {"status": "success", "message": f"Template da classe {cls} salvo com sucesso no Realm {realm_id}!"}

# ==============================================================================
# ROTAS: UTILITÁRIOS DE ENCODE E DECODE DE OCTETS (PW ENGINE)
# ==============================================================================

@app.post("/api/elements/encode-octets")
async def api_encode_octets(req: EncodeOctetsRequest):
    raw_bytes = decoder_instance.generate_octets_for_item(
        realm_id=req.realm_id,
        item_id=req.item_id,
        refine_level=req.refine_level,
        sockets_count=req.sockets_count,
        socket_stones=req.socket_stones,
        durability=req.durability,
        max_durability=req.max_durability,
        creator_name=req.creator_name
    )
    parsed = ItemOctetCodec.parse_item_octets(raw_bytes)
    return safe_json_convert({
        "status": "success",
        "octets_hex": raw_bytes.hex(),
        "octets_len": len(raw_bytes),
        "parsed": parsed
    })

@app.post("/api/elements/decode-octets")
async def api_decode_octets(req: DecodeOctetsRequest):
    parsed = ItemOctetCodec.parse_item_octets(req.octets_hex)
    return safe_json_convert({
        "status": "success",
        "parsed": parsed
    })

@app.post("/api/skills/encode-octets")
async def api_encode_skills(req: EncodeSkillsRequest):
    try:
        skills_dicts = [s.dict() for s in req.skills]
        raw_bytes = SkillOctetCodec.build_skills_octets(skills_dicts)
        return safe_json_convert({
            "status": "success",
            "count": len(skills_dicts),
            "octets_hex": raw_bytes.hex(),
            "bytes_length": len(raw_bytes)
        })
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

@app.post("/api/skills/decode-octets")
async def api_decode_skills(req: DecodeSkillsRequest):
    try:
        parsed_list = SkillOctetCodec.parse_skills_octets(req.octets_hex)
        realm_id = req.realm_id or "realm_126"
        enriched = []
        for sk in parsed_list:
            info = decoder_instance.get_skill_info(sk["skill_id"], realm_id)
            item = dict(info)
            item["level"] = sk["level"]
            item["progress"] = sk.get("progress", 0)
            enriched.append(item)
        return safe_json_convert({
            "status": "success",
            "count": len(parsed_list),
            "skills": enriched
        })
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


# ==============================================================================
# ROTAS: CHANGELOG DE VERSÕES & DISTRIBUIÇÃO DE PATCHES CDN
# ==============================================================================

@app.get("/api/patches/changelog")
async def get_patch_changelog():
    manifest_path = "patch_manifest.json"
    if os.path.exists(manifest_path):
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
            
    return {
        "current_version": 10,
        "latest_version": 12,
        "cdn_base_url": "https://patch.seuservidor.com/updates/",
        "patches": [
            {
                "from_version": 11,
                "to_version": 12,
                "release_date": "2026-08-27T13:15:00Z",
                "release_notes": "Adicionada nova rota de forja na Cidade do Dragão, novas roupas de evento e balanceamento de habilidades.",
                "package_file": "ec_patch_11-12.cup",
                "package_size_mb": 18.2,
                "package_sha256": "4b825dc642cb6eb9a060e54b210a691667b2d5e317bbbc7ef35492ff5e13d964",
                "changed_files_count": 6
            },
            {
                "from_version": 10,
                "to_version": 11,
                "release_date": "2026-08-20T10:00:00Z",
                "release_notes": "Correção de bugs visuais em montarias e atualização das promoções do GShop.",
                "package_file": "ec_patch_10-11.cup",
                "package_size_mb": 12.0,
                "package_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "changed_files_count": 4
            }
        ]
    }

# ==============================================================================
# ROTAS ESTÁTICAS E SPA
# ==============================================================================

@app.get("/", response_class=HTMLResponse)
async def serve_index():
    index_file = os.path.join(os.path.dirname(__file__), "static", "index.html")
    if os.path.exists(index_file):
        with open(index_file, "r", encoding="utf-8") as f:
            return HTMLResponse(content=f.read())
    fallback_file = os.path.join(os.path.dirname(__file__), "..", "frontend", "index.html")
    if os.path.exists(fallback_file):
        with open(fallback_file, "r", encoding="utf-8") as f:
            return HTMLResponse(content=f.read())
    return HTMLResponse("<h1>PW-Admin Web API está online! Acesse /docs para a documentação interativa.</h1>")

static_dir = os.path.join(os.path.dirname(__file__), "static")
if os.path.exists(static_dir):
    app.mount("/static", StaticFiles(directory=static_dir), name="static")
