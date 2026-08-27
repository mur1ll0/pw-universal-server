"""
PW-ADMIN-WEB: Backend API do Painel de Administração Moderno
Substituto completo do pwAdmin arcaico com FastAPI, PostgreSQL 16 e DragonflyDB.
"""

import os
import asyncio
import json
from typing import Optional, List, Dict, Any
from fastapi import FastAPI, HTTPException, Depends, Query
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
import asyncpg
import redis.asyncio as aioredis
import hashlib

DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql://pw_admin:pw_secure_password_2026@localhost:5432/pw_database"
)
REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")

app = FastAPI(
    title="PW-Admin-Web API",
    version="1.0.0",
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
    "realm_153": [m.copy() for m in DEFAULT_MAPS]
}

@app.on_event("startup")
async def startup_event():
    global db_pool, redis_client
    try:
        db_pool = await asyncpg.create_pool(DATABASE_URL, min_size=2, max_size=10)
        redis_client = aioredis.from_url(REDIS_URL, decode_responses=True)
        print("Conectado com sucesso ao PostgreSQL e DragonflyDB!")
    except Exception as e:
        print(f"Aviso ao conectar banco: {e}")

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

class AddItemRequest(BaseModel):
    character_id: int
    container_type: int = Field(0, ge=0, le=2) # 0=Inv, 1=Equip, 2=Storehouse
    slot: int = Field(..., ge=0, le=127)
    item_id: int
    count: int = Field(1, ge=1)
    refine_level: int = Field(0, ge=0, le=12)
    sockets_count: int = Field(0, ge=0, le=4)
    socket_stones: List[int] = []

class EditItemRequest(BaseModel):
    item_instance_id: int
    refine_level: Optional[int] = Field(None, ge=0, le=12)
    count: Optional[int] = Field(None, ge=1)
    durability: Optional[int] = None
    bind_status: Optional[int] = None

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
        return {"status": "success", "account": dict(row)}
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
    return {"status": "success", "account": dict(row)}

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
    return {"accounts": [dict(r) for r in rows]}

# ==============================================================================
# ROTAS: INSPEÇÃO E EDIÇÃO GRANULAR DE PERSONAGENS, ATRIBUTOS E INVENTÁRIO
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
    return {"characters": [dict(r) for r in rows]}

@app.get("/api/characters/{char_id}")
async def get_character_details(char_id: int, conn: asyncpg.Connection = Depends(get_db)):
    char = await conn.fetchrow("SELECT * FROM characters WHERE id = $1", char_id)
    if not char:
        raise HTTPException(status_code=404, detail="Personagem não encontrado")
    
    items = await conn.fetch("SELECT * FROM character_items WHERE character_id = $1 ORDER BY container_type, slot", char_id)
    skills = await conn.fetch("SELECT * FROM character_skills WHERE character_id = $1 ORDER BY skill_id", char_id)
    quests = await conn.fetch("SELECT * FROM character_quests WHERE character_id = $1", char_id)

    return {
        "character": dict(char),
        "items": [dict(i) for i in items],
        "skills": [dict(s) for s in skills],
        "quests": [dict(q) for q in quests],
    }

@app.post("/api/characters/{char_id}/items/add")
async def add_item_to_character(char_id: int, req: AddItemRequest, conn: asyncpg.Connection = Depends(get_db)):
    # Insere ou atualiza item garantindo restrição de unicidade no slot
    try:
        row = await conn.fetchrow(
            """
            INSERT INTO character_items (character_id, container_type, slot, item_id, count, refine_level, sockets_count, socket_stones)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (character_id, container_type, slot) 
            DO UPDATE SET 
                item_id = EXCLUDED.item_id,
                count = EXCLUDED.count,
                refine_level = EXCLUDED.refine_level,
                sockets_count = EXCLUDED.sockets_count,
                socket_stones = EXCLUDED.socket_stones,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id, character_id, container_type, slot, item_id, count, refine_level
            """,
            char_id, req.container_type, req.slot, req.item_id, req.count, req.refine_level, req.sockets_count, req.socket_stones
        )
        return {"status": "success", "item": dict(row)}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

@app.delete("/api/items/{item_instance_id}")
async def delete_item(item_instance_id: int, conn: asyncpg.Connection = Depends(get_db)):
    res = await conn.execute("DELETE FROM character_items WHERE id = $1", item_instance_id)
    if res == "DELETE 0":
        raise HTTPException(status_code=404, detail="Item não encontrado")
    return {"status": "success", "message": "Item removido com sucesso!"}

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
    return {"status": "success", "character": dict(row)}

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
# ROTAS: GERENCIAMENTO DE MAPAS & INSTÂNCIAS (LIGAR / DESLIGAR)
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

# ==============================================================================
# ROTAS: CHANGELOG DE VERSÕES & DISTRIBUIÇÃO DE PATCHES CDN
# ==============================================================================

@app.get("/api/patches/changelog")
async def get_patch_changelog():
    # Retorna o histórico de versões geradas pelo pw-patch-tool
    manifest_path = "patch_manifest.json"
    if os.path.exists(manifest_path):
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
            
    # Retorno padrão demonstrativo
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
# ROTAS: MULTI-REALM, EVENTOS E MÉTRICAS
# ==============================================================================

@app.get("/api/realms/list")
async def list_realms(conn: asyncpg.Connection = Depends(get_db)):
    rows = await conn.fetch("SELECT * FROM realms ORDER BY id ASC")
    realms_list = []
    for r in rows:
        d = dict(r)
        online_count = 0
        if redis_client:
            online_count = await redis_client.scard(f"online:{d['id']}") or 0
        d["online_players"] = online_count
        realms_list.append(d)
    return {"realms": realms_list}

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
    
    online_126 = 0
    online_153 = 0
    if redis_client:
        online_126 = await redis_client.scard("online:realm_126") or 0
        online_153 = await redis_client.scard("online:realm_153") or 0
        
    return {
        "total_accounts": total_accounts,
        "total_characters": total_chars,
        "online_by_realm": {
            "realm_126": online_126,
            "realm_153": online_153,
        },
        "total_online": online_126 + online_153
    }
