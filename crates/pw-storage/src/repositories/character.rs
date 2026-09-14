use crate::error::{Result, StorageError};
use crate::postgres::PostgresPool;
use crate::repositories::item::ItemRepository;
use crate::repositories::quest::QuestRepository;
use crate::repositories::skill::SkillRepository;
use crate::repositories::template::TemplateRepository;
use chrono::{DateTime, Utc};
use pw_core::{
    AccountId, CharacterClass, CharacterDetails, CharacterSummary, ContainerType, Gender, Race,
    RealmId, RoleId, Vector3, WorldId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CharacterRecord {
    pub id: RoleId,
    pub account_id: AccountId,
    pub realm_id: RealmId,
    pub name: String,
    pub race: i32,
    pub cls: i32,
    pub gender: i16,
    pub level: i32,
    pub cultivation: i32,
    pub exp: i64,
    pub sp: i64,
    pub hp: i32,
    pub mp: i32,
    pub money: i64,
    pub world_id: WorldId,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    /// Os quatro atributos distribuíveis. Estão no schema desde o começo
    /// (`specs/01_DATABASE_SCHEMA_POSTGRES.sql`) e **nunca eram lidos**: o `SELECT *`
    /// trazia as colunas e o `FromRow` as descartava por não existirem aqui. São eles que
    /// alimentam vida/mana máxima, precisão e evasão do jogador no mundo.
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    pub custom_data: Option<Vec<u8>>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Quando entrou no mundo pela última vez. Vira o `lastlogin_time` do `RoleInfo`.
    pub last_login_at: Option<DateTime<Utc>>,
}

/// O suficiente pra descrever um personagem a OUTRO jogador — ver
/// `CharacterRepository::get_public_info`.
#[derive(Debug, Clone)]
pub struct CharacterPublicInfo {
    pub id: RoleId,
    pub name: String,
    pub race: i32,
    pub cls: i32,
    pub gender: u8,
    pub custom_data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct CharacterRepository {
    pool: PostgresPool,
    item_repo: ItemRepository,
    skill_repo: SkillRepository,
    quest_repo: QuestRepository,
    template_repo: TemplateRepository,
}

impl CharacterRepository {
    /// O pool por baixo deste repositório.
    ///
    /// Existe para que testes possam conferir uma coluna sem que o repositório ganhe um
    /// método de leitura por asserção — a API de produção não deve crescer por causa de
    /// teste.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    pub fn new(pool: PostgresPool) -> Self {
        let item_repo = ItemRepository::new(pool.clone());
        let skill_repo = SkillRepository::new(pool.clone());
        let quest_repo = QuestRepository::new(pool.clone());
        let template_repo = TemplateRepository::new(pool.clone());
        Self {
            pool,
            item_repo,
            skill_repo,
            quest_repo,
            template_repo,
        }
    }

    pub fn item_repo(&self) -> &ItemRepository {
        &self.item_repo
    }

    pub fn skill_repo(&self) -> &SkillRepository {
        &self.skill_repo
    }

    pub fn quest_repo(&self) -> &QuestRepository {
        &self.quest_repo
    }

    pub fn template_repo(&self) -> &TemplateRepository {
        &self.template_repo
    }

    /// Cria um novo personagem buscando dados da tabela class_templates e insere itens e habilidades
    pub async fn create_character(
        &self,
        account_id: AccountId,
        realm_id: &str,
        name: &str,
        // `_race`: a raça que o cliente mandou. **Não é usada** — ela sai da classe, ver o
        // `.bind` mais abaixo. Continua no parâmetro para o chamador não mudar, e para o
        // dia em que o campo `race` do `RoleInfo` for entendido de verdade.
        _race: Race,
        cls: CharacterClass,
        gender: Gender,
        custom_data: Vec<u8>,
        atributos: Option<pw_core::AtributosIniciais>,
    ) -> Result<RoleId> {
        // Busca template da classe no banco de dados
        let tpl_opt = self.template_repo.get_template_for_class(realm_id, cls as i32).await.unwrap_or(None);

        let (spawn_x, spawn_y, spawn_z, init_lvl, init_cult, init_money, init_sp, world_id) = if let Some(ref tpl) = tpl_opt {
            (
                tpl.template.spawn_x,
                tpl.template.spawn_y,
                tpl.template.spawn_z,
                tpl.template.initial_level,
                tpl.template.initial_cultivation,
                tpl.template.initial_money,
                tpl.template.initial_sp,
                tpl.template.spawn_world_id,
            )
        } else {
            let (sx, sy, sz) = cls.default_spawn_position();
            (sx, sy, sz, 1, 0, 0, 0, 1)
        };

        // Os quatro atributos vêm do `ptemplate.conf`, por classe — o Guerreiro nasce com
        // vitalidade 20, força 15, agilidade 10, energia 5; o Mago com energia 20 e força
        // 5. Até 2026-09-09 esta consulta não mencionava as colunas e todo personagem
        // nascia com o `DEFAULT` do esquema, **10/10/10/10**, qualquer que fosse a classe.
        //
        // Sem o arquivo (realm que não o trouxe), continua valendo o padrão da coluna: é
        // menos errado do que inventar um número por classe aqui.
        let atr = atributos.unwrap_or_else(|| {
            // Sem o `ptemplate.conf`, o padrão da coluna para os atributos e a tabela por
            // classe para a vida — o mesmo que valia antes de 2026-09-11.
            let (vida, mana) = cls.default_hp_mp();
            pw_core::AtributosIniciais {
                forca: 10,
                agilidade: 10,
                vitalidade: 10,
                energia: 10,
                vida,
                mana,
            }
        });
        let (init_hp, init_mp) = (atr.vida, atr.mana);

        let role_id = sqlx::query_scalar::<_, RoleId>(
            r#"
            INSERT INTO characters (
                account_id, realm_id, name, race, cls, gender, custom_data,
                level, cultivation, money, sp, world_id,
                pos_x, pos_y, pos_z, hp, mp,
                strength, agility, vitality, energy
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                    $18, $19, $20, $21)
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(realm_id)
        .bind(name)
        // **A raça sai da classe**, e não do que o cliente mandou. O campo `race` do
        // `RoleInfo` não é a raça: no original, o campo de mesmo nome do `GRoleBase` guarda
        // `classe | 0x80000000` para mulher (`gs/player_imp.h:1884-1895`). Acreditar nele
        // gravou um Bárbaro como Humano em 2026-09-11. Ver `CharacterClass::race`.
        .bind(cls.race() as i32)
        .bind(cls as i32)
        .bind(gender as i16)
        .bind(custom_data)
        .bind(init_lvl)
        .bind(init_cult)
        .bind(init_money)
        .bind(init_sp)
        .bind(world_id)
        .bind(spawn_x)
        .bind(spawn_y)
        .bind(spawn_z)
        .bind(init_hp)
        .bind(init_mp)
        .bind(atr.forca)
        .bind(atr.agilidade)
        .bind(atr.vitalidade)
        .bind(atr.energia)
        .fetch_one(self.pool.get_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref dbe) if dbe.is_unique_violation() => {
                StorageError::Duplicate(format!("Nome '{}' já está em uso neste Realm", name))
            }
            _ => StorageError::Database(e),
        })?;

        if let Some(tpl) = tpl_opt {
            // 1. Grava itens configurados no template (Equipamentos container_type=1 e Inventário container_type=0)
            for item in tpl.items {
                let ctype = ContainerType::from_i16(item.container_type);
                let record = pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ctype,
                    slot: item.slot as u16,
                    item_id: item.item_id as u32,
                    count: item.count as u32,
                    max_count: 100,
                    refine_level: item.refine_level as u8,
                    sockets_count: item.sockets_count as u8,
                    sockets: item.socket_stones.into_iter().map(|s| s as u32).collect(),
                    durability: item.durability as u32,
                    max_durability: item.max_durability as u32,
                    bind_status: 0,
                    octets: Vec::new(),
                    custom_attributes: serde_json::json!({}),
                };
                let _ = self.item_repo.upsert_item(&record).await;
            }

            // 2. Grava habilidades configuradas no template
            for sk in tpl.skills {
                let _ = self.skill_repo.learn_or_upgrade(role_id, sk.skill_id as u32, sk.level as u8).await;
            }
        } else {
            // Fallback: arma inicial equipada no slot 0
            let weapon_id = cls.default_weapon_id() as u32;
            let equip_weapon = pw_core::ItemRecord {
                id: None,
                character_id: role_id,
                container_type: ContainerType::Equipment,
                slot: 0,
                item_id: weapon_id,
                count: 1,
                max_count: 1,
                refine_level: 0,
                sockets_count: 0,
                sockets: vec![],
                durability: 10000,
                max_durability: 10000,
                bind_status: 0,
                octets: Vec::new(),
                custom_attributes: serde_json::json!({}),
            };
            let _ = self.item_repo.upsert_item(&equip_weapon).await;

            for (skill_id, level, _) in cls.default_skills() {
                let _ = self.skill_repo.learn_or_upgrade(role_id, skill_id as u32, level).await;
            }
        }

        Ok(role_id)
    }

    /// Lista os resumos dos personagens para a tela de seleção de personagens do cliente
    pub async fn list_by_account_and_realm(
        &self,
        account_id: AccountId,
        realm_id: &str,
    ) -> Result<Vec<CharacterSummary>> {
        let recs = sqlx::query_as::<_, CharacterRecord>(
            r#"
            SELECT * FROM characters 
            WHERE account_id = $1 AND realm_id = $2 AND is_deleted = FALSE 
            ORDER BY id ASC
            "#,
        )
        .bind(account_id)
        .bind(realm_id)
        .fetch_all(self.pool.get_ref())
        .await?;

        let mut summaries = Vec::with_capacity(recs.len());

        for r in recs {
            let equipment = self
                .item_repo
                .list_by_container(r.id, ContainerType::Equipment)
                .await?;

            let appearance_val = match &r.custom_data {
                Some(bytes) => serde_json::json!({ "raw": hex::encode(bytes) }),
                None => serde_json::json!({}),
            };

            summaries.push(CharacterSummary {
                id: r.id,
                account_id: r.account_id,
                realm_id: r.realm_id,
                name: r.name,
                race: Race::from_u8(r.race as u8).unwrap_or(Race::Human),
                cls: CharacterClass::from_u8(r.cls as u8).unwrap_or(CharacterClass::Blademaster),
                gender: Gender::from_u8(r.gender as u8),
                level: r.level,
                cultivation: r.cultivation,
                world_id: r.world_id,
                position: Vector3::new(r.pos_x, r.pos_y, r.pos_z),
                equipment,
                custom_appearance: appearance_val,
                is_deleted: r.is_deleted,
                delete_time: r.deleted_at,
                last_login_at: r.last_login_at,
            });
        }

        Ok(summaries)
    }

    /// Carrega todos os detalhes do personagem, **conferindo que ele é desta conta neste
    /// realm**.
    ///
    /// # Por que a conta e o realm são obrigatórios
    ///
    /// O `role_id` chega num pacote do cliente (`SelectRole`, `EnterWorld`) e é um
    /// inteiro sequencial — trivial de adivinhar. Enquanto esta consulta era
    /// `WHERE id = $1`, um cliente autenticado entrava no mundo como **qualquer
    /// personagem do servidor**, bastando mandar outro número.
    ///
    /// O realm entra junto porque um banco só atende todos os realms: sem ele, dois
    /// realms da mesma versão vazariam um no outro para quem tem personagem nos dois.
    ///
    /// Nenhuma variante sem escopo existe de propósito. Se um dia o servidor de mundo
    /// precisar carregar um personagem sem ter a conta em mãos, o certo é passar a
    /// identidade adiante pelo barramento, e não reabrir esta porta.
    /// O personagem, **sem** checar dono nem realm.
    ///
    /// Existe para o servidor de mundo, que recebe um `EnterWorld` do barramento com
    /// `roleid` e nada mais — nem conta, nem realm — e precisa montar a entidade do
    /// jogador. A autorização já aconteceu antes, no `pw-link`: `SelectRole` e
    /// `EnterWorld` conferem que o personagem é da conta autenticada
    /// (`docs/ESTADO_E_RETOMADA.md`, item 29). Repetir a checagem aqui exigiria carregar
    /// a conta pelo barramento só para reprovar o que já foi aprovado.
    ///
    /// **Não use isto num caminho que fale direto com o cliente** — ali a checagem de
    /// dono é obrigatória, e é para isso que existe [`Self::get_details`].
    pub async fn get_details_por_role(&self, role_id: RoleId) -> Result<Option<CharacterDetails>> {
        let dono: Option<(i32, String)> = sqlx::query_as(
            "SELECT account_id, realm_id FROM characters WHERE id = $1 AND is_deleted = false",
        )
        .bind(role_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        let Some((account_id, realm_id)) = dono else {
            return Ok(None);
        };
        self.get_details(role_id, account_id, &realm_id).await
    }

    pub async fn get_details(
        &self,
        role_id: RoleId,
        account_id: AccountId,
        realm_id: &str,
    ) -> Result<Option<CharacterDetails>> {
        let rec = sqlx::query_as::<_, CharacterRecord>(
            r#"
            SELECT * FROM characters
            WHERE id = $1 AND account_id = $2 AND realm_id = $3
            "#,
        )
        .bind(role_id)
        .bind(account_id)
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        let r = match rec {
            Some(row) => row,
            None => return Ok(None),
        };

        let mut inventory = self.item_repo.list_by_container(role_id, ContainerType::Inventory).await?;
        let equipment = self.item_repo.list_by_container(role_id, ContainerType::Equipment).await?;
        let storehouse = self.item_repo.list_by_container(role_id, ContainerType::Storehouse).await?;
        let mut skills = self.skill_repo.list_skills(role_id).await?;
        let quests = self.quest_repo.list_quests(role_id).await?;

        let cls_enum = CharacterClass::from_u8(r.cls as u8).unwrap_or(CharacterClass::Blademaster);

        // Se o personagem já existia no banco sem skills salvas, popula na tabela character_skills
        if skills.is_empty() {
            for (skill_id, level, _) in cls_enum.default_skills() {
                let _ = self.skill_repo.learn_or_upgrade(role_id, skill_id as u32, level).await;
            }
            skills = self.skill_repo.list_skills(role_id).await?;
        }

        // Se o personagem já existia no banco sem itens salvos, popula na tabela
        // character_items.
        //
        // É caminho de **reparo**, não de criação: quem cria é `create_character`, com o
        // molde do realm (`class_templates`). Os dois davam kits diferentes até
        // 2026-09-11 — aqui a arma ia para a **bolsa** em vez do slot de equipamento, e o
        // kit tinha a Poção Perfeita de Cura (1801), que exige nível 30 no `elements.data`
        // e não serve a personagem nenhum recém-criado.
        if inventory.is_empty() && equipment.is_empty() {
            let weapon_id = cls_enum.default_weapon_id() as u32;
            let starter_items = vec![
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Equipment,
                    slot: 0,
                    item_id: weapon_id,
                    count: 1,
                    max_count: 1,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    octets: Vec::new(),
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 1,
                    item_id: 2100,
                    count: 5,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    octets: Vec::new(),
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 2,
                    item_id: 1796,
                    count: 10,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    octets: Vec::new(),
                    custom_attributes: serde_json::json!({}),
                },
                pw_core::ItemRecord {
                    id: None,
                    character_id: role_id,
                    container_type: ContainerType::Inventory,
                    slot: 3,
                    // Poção Pequena do Espírito, `require_level = 0`. Era a Poção
                    // Perfeita de Cura (1801), que exige nível 30.
                    item_id: 1804,
                    count: 10,
                    max_count: 100,
                    refine_level: 0,
                    sockets_count: 0,
                    sockets: vec![],
                    durability: 10000,
                    max_durability: 10000,
                    bind_status: 0,
                    octets: Vec::new(),
                    custom_attributes: serde_json::json!({}),
                },
            ];
            for item in starter_items {
                let _ = self.item_repo.upsert_item(&item).await;
            }
            inventory = self.item_repo.list_by_container(role_id, ContainerType::Inventory).await?;
        }

        let details = CharacterDetails {
            id: r.id,
            account_id: r.account_id,
            realm_id: r.realm_id,
            name: r.name,
            race: Race::from_u8(r.race as u8).unwrap_or(Race::Human),
            cls: cls_enum,
            gender: Gender::from_u8(r.gender as u8),
            level: r.level,
            cultivation: r.cultivation,
            exp: r.exp,
            sp: r.sp,
            hp: r.hp,
            mp: r.mp,
            money: r.money,
            reputation: 0,
            world_id: r.world_id,
            position: Vector3::new(r.pos_x, r.pos_y, r.pos_z),
            strength: r.strength,
            agility: r.agility,
            vitality: r.vitality,
            energy: r.energy,
            inventory_size: 64,
            storehouse_size: 32,
            inventory,
            equipment,
            storehouse,
            skills,
            quests,
            custom_appearance: match &r.custom_data {
                Some(bytes) => serde_json::json!({ "raw": hex::encode(bytes) }),
                None => serde_json::json!({}),
            },
            version_data: serde_json::json!({}),
            created_at: r.created_at,
            last_login_at: Some(r.updated_at),
        };

        Ok(Some(details))
    }

    /// O que basta pra outro jogador saber quem é este personagem, pra
    /// `PlayerBaseInfo`/`GetCustomData` (visibilidade entre jogadores — ver
    /// `docs/ESTADO_E_RETOMADA.md`, item 17/19).
    ///
    /// De propósito **não** é `get_details`: aquele filtra por `account_id` (é sempre o
    /// próprio dono olhando o próprio personagem) e carrega inventário, equipamento,
    /// skills e missões, criando os padrões de personagem novo se faltarem — coisas que
    /// fazem sentido pra "eu entrando no jogo", não pra "alguém pediu pra ver o avatar
    /// de outro jogador que passou por perto". Uma consulta direta, sem efeito
    /// colateral nenhum.
    pub async fn get_public_info(
        &self,
        role_id: RoleId,
        realm_id: &str,
    ) -> Result<Option<CharacterPublicInfo>> {
        let rec = sqlx::query_as::<_, CharacterRecord>(
            r#"
            SELECT * FROM characters
            WHERE id = $1 AND realm_id = $2 AND is_deleted = false
            "#,
        )
        .bind(role_id)
        .bind(realm_id)
        .fetch_optional(self.pool.get_ref())
        .await?;

        Ok(rec.map(|r| CharacterPublicInfo {
            id: r.id,
            name: r.name,
            race: r.race,
            cls: r.cls,
            gender: r.gender as u8,
            custom_data: r.custom_data.unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    /// Marca que o personagem acabou de entrar no mundo.
    ///
    /// É este carimbo que vira o `lastlogin_time` do `RoleInfo` e faz o cliente vir com o
    /// último personagem jogado selecionado. Escrito na entrada, e não na saída, porque é
    /// "quando jogou pela última vez" que interessa — e uma queda de conexão não pode
    /// apagar o registro.
    pub async fn marcar_entrada_no_mundo(&self, role_id: RoleId) -> Result<()> {
        sqlx::query("UPDATE characters SET last_login_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(role_id)
            .execute(self.pool.get_ref())
            .await?;
        Ok(())
    }

    /// O nível de GM da conta dona do personagem.
    ///
    /// O mundo não recebe o `sec_level` da sessão (o `BusMessage::EnterWorld` não o
    /// carrega), e é ele que decide quem pode usar comando de GM — o teleporte por
    /// Ctrl+clique, por exemplo. Ler do banco evita mudar o formato da mensagem do
    /// barramento só para isso.
    ///
    /// Zero quando o personagem não existe: negar é a resposta segura.
    pub async fn nivel_de_gm(&self, role_id: RoleId) -> i32 {
        sqlx::query_scalar::<_, i32>(
            r#"
            SELECT a.gm_privileges
            FROM characters c JOIN accounts a ON a.id = c.account_id
            WHERE c.id = $1
            "#,
        )
        .bind(role_id)
        .fetch_optional(self.pool.get_ref())
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
    }

    /// Em que mapa o personagem está gravado.
    ///
    /// É a pergunta que o servidor de mundo faz a cada `EnterWorld` para decidir qual dos
    /// mapas do processo recebe o jogador (`pw_gs::mapas`) — uma coluna, e não o
    /// `get_details_por_role` inteiro com itens e habilidades, que o mapa escolhido lê logo
    /// depois de qualquer jeito.
    ///
    /// `None` quando o personagem não existe.
    pub async fn mundo_do_personagem(&self, role_id: RoleId) -> Result<Option<i32>> {
        let mundo = sqlx::query_scalar::<_, i32>("SELECT world_id FROM characters WHERE id = $1")
            .bind(role_id)
            .fetch_optional(self.pool.get_ref())
            .await?;
        Ok(mundo)
    }

    /// Salva o estado básico do personagem
    pub async fn save_status(
        &self,
        role_id: RoleId,
        level: i32,
        cultivation: i32,
        exp: i64,
        sp: i64,
        hp: i32,
        mp: i32,
        money: i64,
        world_id: WorldId,
        pos: &Vector3,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET level = $1, cultivation = $2, exp = $3, sp = $4,
                hp = $5, mp = $6, money = $7, world_id = $8,
                pos_x = $9, pos_y = $10, pos_z = $11,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $12
            "#,
        )
        .bind(level)
        .bind(cultivation)
        .bind(exp)
        .bind(sp)
        .bind(hp)
        .bind(mp)
        .bind(money)
        .bind(world_id)
        .bind(pos.x)
        .bind(pos.y)
        .bind(pos.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Atualiza as coordenadas do personagem no banco de dados
    pub async fn update_position(&self, role_id: RoleId, pos: &Vector3) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET pos_x = $1, pos_y = $2, pos_z = $3,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $4
            "#,
        )
        .bind(pos.x)
        .bind(pos.y)
        .bind(pos.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Teletransporte de emergência para a Cidade do Dragão (CDD)
    pub async fn teleport_to_dragon_city(&self, role_id: RoleId) -> Result<()> {
        let cdd = Vector3::dragon_city();
        sqlx::query(
            r#"
            UPDATE characters 
            SET world_id = 1, pos_x = $1, pos_y = $2, pos_z = $3 
            WHERE id = $4
            "#,
        )
        .bind(cdd.x)
        .bind(cdd.y)
        .bind(cdd.z)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Marca o personagem como excluído (soft delete).
    ///
    /// Exige a conta **e** o realm: o `role_id` vem de um pacote do cliente, que pode
    /// dizer qualquer número. Sem o escopo na cláusula `WHERE`, isto apagava o
    /// personagem de qualquer jogador do servidor — ver [`Self::get_details`].
    ///
    /// Devolve `false` quando nada foi apagado, o que quer dizer: não existe, já estava
    /// apagado, ou **não é desta conta neste realm**. O chamador não deve distinguir os
    /// três casos para o cliente, senão a resposta vira um oráculo que diz quais
    /// `role_id` existem.
    pub async fn delete_character(
        &self,
        role_id: RoleId,
        account_id: AccountId,
        realm_id: &str,
    ) -> Result<bool> {
        let r = sqlx::query(
            r#"
            UPDATE characters
            SET is_deleted = TRUE, deleted_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND account_id = $2 AND realm_id = $3 AND is_deleted = FALSE
            "#,
        )
        .bind(role_id)
        .bind(account_id)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(r.rows_affected() > 0)
    }

    /// Restaura um personagem marcado para exclusão (undo delete).
    ///
    /// Mesmo escopo obrigatório do [`Self::delete_character`]. Aqui a linha procurada
    /// está com `is_deleted = TRUE`, que é o estado oposto — restaurar o que não estava
    /// apagado não é operação válida.
    pub async fn restore_character(
        &self,
        role_id: RoleId,
        account_id: AccountId,
        realm_id: &str,
    ) -> Result<bool> {
        let r = sqlx::query(
            r#"
            UPDATE characters
            SET is_deleted = FALSE, deleted_at = NULL
            WHERE id = $1 AND account_id = $2 AND realm_id = $3 AND is_deleted = TRUE
            "#,
        )
        .bind(role_id)
        .bind(account_id)
        .bind(realm_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(r.rows_affected() > 0)
    }

    /// Adiciona moedas (Coins) ao personagem
    pub async fn add_money(&self, role_id: RoleId, amount: i64) -> Result<i64> {
        let new_balance = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE characters 
            SET money = money + $1, updated_at = CURRENT_TIMESTAMP 
            WHERE id = $2
            RETURNING money
            "#,
        )
        .bind(amount)
        .bind(role_id)
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(new_balance)
    }

    /// Deduz moedas (Coins) com verificação de saldo
    pub async fn deduct_money(&self, role_id: RoleId, amount: i64) -> Result<bool> {
        let rows = sqlx::query(
            r#"
            UPDATE characters 
            SET money = money - $1, updated_at = CURRENT_TIMESTAMP 
            WHERE id = $2 AND money >= $1
            "#,
        )
        .bind(amount)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(rows.rows_affected() > 0)
    }

    /// Adiciona EXP e Alma (SP) ao personagem
    pub async fn add_exp_sp(&self, role_id: RoleId, exp_gain: i64, sp_gain: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE characters 
            SET exp = exp + $1, sp = sp + $2, updated_at = CURRENT_TIMESTAMP 
            WHERE id = $3
            "#,
        )
        .bind(exp_gain)
        .bind(sp_gain)
        .bind(role_id)
        .execute(self.pool.get_ref())
        .await?;

        Ok(())
    }

    /// Consulta saldo de moedas do personagem
    pub async fn get_money(&self, role_id: RoleId) -> Result<i64> {
        let m = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT money FROM characters WHERE id = $1
            "#,
        )
        .bind(role_id)
        .fetch_one(self.pool.get_ref())
        .await?;

        Ok(m)
    }
}
