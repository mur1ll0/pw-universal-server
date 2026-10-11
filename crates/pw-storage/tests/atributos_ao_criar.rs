//! Personagem novo nasce com o que o **template do realm** manda (E8, B217).
//!
//! # As falhas que este arquivo tranca
//!
//! 1. Até 2026-09-09 o `INSERT` do `create_character` não mencionava os atributos e todo
//!    personagem nascia com o `DEFAULT 10` do esquema.
//! 2. De 2026-09-16 ao B217 os atributos eram a constante 5 do `ptemplate.rs`, e as colunas
//!    `strength..energy` do template eram **ignoradas** — editar o template não mudava nada.
//!    No original, o `DBCreateRole` copia o `GRoleStatus` inteiro do molde da classe
//!    (`cnet/gamedbd/dbcreaterole.hrp:46-60`): nível, cultivo, EXP, SP, pontos, atributos,
//!    posição; e a bolsa, o corpo e as habilidades.
//! 3. O reparo da carga (`get_details`) dava o kit do código a quem não tinha habilidade ou
//!    item — e devolveria o que o administrador tirou do template.
//!
//! Vida e mana não são do template: são a conta da classe (`vida_e_mana_maximas`) sobre o
//! nível, a vitalidade e a energia de nascimento.
use pw_core::{CharacterClass, Gender, Race};
mod comum;

use pw_storage::{CharacterRepository, EstatisticasDoTemplate, PostgresPool, StorageConfig};

struct Cenario {
    repo: CharacterRepository,
    pool: PostgresPool,
    conta: i32,
    realm: String,
    marca: String,
}

async fn montar() -> Option<Cenario> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return None;
        }
    };
    let cfg = StorageConfig {
        database_url: url,
        max_connections: 2,
        min_connections: 1,
        ..Default::default()
    };
    let pool = PostgresPool::new(&cfg).await.expect("conexão com o banco");
    comum::limpar_sobras_de_teste(&pool).await;

    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let marca = format!(
        "{}_{}",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 1_000_000_000,
        SEQ.fetch_add(1, Ordering::Relaxed)
    );

    let realm = format!("t_at_{marca}");
    sqlx::query(
        "INSERT INTO realms (id, name, version, host, port, max_players, config)
         VALUES ($1, 'Teste Atributos', '1.2.6', '127.0.0.1', 29000, 10, '{}'::jsonb)",
    )
    .bind(&realm)
    .execute(pool.get_ref())
    .await
    .expect("criar realm");

    let conta: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, 'x') RETURNING id",
    )
    .bind(format!("at_{marca}"))
    .fetch_one(pool.get_ref())
    .await
    .expect("criar conta");

    let repo = CharacterRepository::new(pool.clone());
    // Semeia os templates do realm (as seis classes do 1.2.6), como na partida do link.
    repo.template_repo().ensure_default_templates(&realm).await.expect("semear templates");
    Some(Cenario { repo, pool, conta, realm, marca })
}

/// A conta de teste: 10 por nível acima do 1, 13 por vitalidade, 7 por energia.
fn conta(nivel: i32, vit: i32, eng: i32) -> (i32, i32) {
    (10 * (nivel - 1) + 13 * vit, 10 * (nivel - 1) + 7 * eng)
}

async fn criar(c: &Cenario, nome: &str, cls: CharacterClass) -> i32 {
    c.repo
        .create_character(
            c.conta,
            &c.realm,
            &format!("{nome}{}", c.marca),
            Race::Human,
            cls,
            Gender::Male,
            Vec::new(),
            Some(&conta),
        )
        .await
        .expect("criar personagem")
}

#[tokio::test]
async fn o_template_semeado_da_os_atributos_do_molde() {
    let Some(c) = montar().await else { return };
    let id = criar(&c, "Sem", CharacterClass::Blademaster).await;
    let d = c.repo.get_details_por_role(id).await.unwrap().expect("personagem");
    assert_eq!((d.strength, d.agility, d.vitality, d.energy), (5, 5, 5, 5));
    assert_eq!((d.hp, d.mp), conta(1, 5, 5));
    assert_eq!(c.repo.pontos_de_atributo(id).await.unwrap(), 0);
}

#[tokio::test]
async fn o_personagem_nasce_com_o_template_editado() {
    let Some(c) = montar().await else { return };
    let t = c.repo.template_repo();
    let e = EstatisticasDoTemplate {
        nivel: 3, cultivo: 1, exp: 1234, sp: 56, dinheiro: 7890, pontos: 11,
        forca: 6, agilidade: 7, vitalidade: 8, energia: 9,
        mapa: 1, x: 100.5, y: 220.0, z: -300.25,
    };
    assert!(t.definir_estatisticas(&c.realm, 0, &e).await.unwrap());
    // Bolsa: troca a poção do slot 0 e põe 3 de outra no 5; corpo: tira a arma.
    assert!(t.definir_item(&c.realm, 0, 0, 0, 1796, 7, 0).await.unwrap());
    assert!(t.definir_item(&c.realm, 0, 0, 5, 2100, 3, 0).await.unwrap());
    assert!(t.remover_item(&c.realm, 0, 1, 0).await.unwrap());
    // Habilidades: a 1 sobe ao nível 2 e a 167 sai.
    assert!(t.definir_habilidade(&c.realm, 0, 1, 2).await.unwrap());
    assert!(t.definir_habilidade(&c.realm, 0, 167, 0).await.unwrap());

    let id = criar(&c, "Ed", CharacterClass::Blademaster).await;
    let d = c.repo.get_details_por_role(id).await.unwrap().expect("personagem");
    assert_eq!((d.level, d.cultivation, d.exp, d.sp, d.money), (3, 1, 1234, 56, 7890));
    assert_eq!((d.strength, d.agility, d.vitality, d.energy), (6, 7, 8, 9));
    assert_eq!((d.hp, d.mp), conta(3, 8, 9), "vida/mana cheias pela conta, no nível e atributos do template");
    assert_eq!(d.world_id, 1);
    assert_eq!((d.position.x, d.position.y, d.position.z), (100.5, 220.0, -300.25));
    assert_eq!(c.repo.pontos_de_atributo(id).await.unwrap(), 11);
    let bolsa: Vec<(u16, u32, u32)> = d.inventory.iter().map(|i| (i.slot, i.item_id, i.count)).collect();
    assert!(bolsa.contains(&(0, 1796, 7)) && bolsa.contains(&(5, 2100, 3)), "bolsa: {bolsa:?}");
    assert!(d.equipment.iter().all(|i| i.slot != 0), "a arma saiu do template");
    let habilidades: Vec<(u32, u8)> = d.skills.iter().map(|h| (h.skill_id, h.level)).collect();
    assert_eq!(habilidades, vec![(1, 2)]);

    // Um personagem de outra classe não muda.
    let outro = criar(&c, "Ou", CharacterClass::Wizard).await;
    let o = c.repo.get_details_por_role(outro).await.unwrap().expect("personagem");
    assert_eq!((o.level, o.money, o.strength), (1, 0, 5));
}

#[tokio::test]
async fn template_sem_nada_nao_e_reparado_pelo_codigo() {
    let Some(c) = montar().await else { return };
    let t = c.repo.template_repo();
    let tpl = t.template_completo(&c.realm, 1).await.unwrap().expect("template do Mago");
    for i in &tpl.items {
        assert!(t.remover_item(&c.realm, 1, i.container_type, i.slot).await.unwrap());
    }
    for h in &tpl.skills {
        assert!(t.definir_habilidade(&c.realm, 1, h.skill_id, 0).await.unwrap());
    }
    let id = criar(&c, "Vaz", CharacterClass::Wizard).await;
    let d = c.repo.get_details_por_role(id).await.unwrap().expect("personagem");
    assert!(d.skills.is_empty(), "o reparo deu as habilidades do código: {:?}", d.skills.len());
    assert!(d.inventory.is_empty() && d.equipment.is_empty(), "o reparo deu o kit do código");
}

#[tokio::test]
async fn classe_sem_template_nasce_com_o_molde_e_o_reparo() {
    let Some(c) = montar().await else { return };
    // O Espiritualista (2) não está no semeio do 1.2.6.
    let id = criar(&c, "Esp", CharacterClass::Psychomancer).await;
    let d = c.repo.get_details_por_role(id).await.unwrap().expect("personagem");
    assert_eq!((d.level, d.strength, d.vitality), (1, 5, 5));
    assert!(!d.skills.is_empty(), "sem template, o reparo do código continua valendo");
    let _ = &c.pool;
}

#[tokio::test]
async fn mover_item_do_template_troca_os_slots() {
    let Some(c) = montar().await else { return };
    let t = c.repo.template_repo();
    // Guerreiro semeado: bolsa 0 = 2100, bolsa 1 = 1796.
    assert!(t.mover_item(&c.realm, 0, (0, 0), (0, 1)).await.unwrap());
    let tpl = t.template_completo(&c.realm, 0).await.unwrap().unwrap();
    let em = |s: i16| tpl.items.iter().find(|i| i.container_type == 0 && i.slot == s).map(|i| i.item_id);
    assert_eq!((em(0), em(1)), (Some(1796), Some(2100)));
    // Para um slot vazio: só move.
    assert!(t.mover_item(&c.realm, 0, (0, 0), (0, 20)).await.unwrap());
    let tpl = t.template_completo(&c.realm, 0).await.unwrap().unwrap();
    assert!(tpl.items.iter().any(|i| i.container_type == 0 && i.slot == 20 && i.item_id == 1796));
    assert!(!tpl.items.iter().any(|i| i.container_type == 0 && i.slot == 0));
    // De slot vazio: nada.
    assert!(!t.mover_item(&c.realm, 0, (0, 0), (0, 21)).await.unwrap());
}
