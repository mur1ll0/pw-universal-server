//! O jogador que entra no mundo carrega os números do personagem, não constantes.
//!
//! `world.players` nunca era populado — `PlayerEntity` só existia em teste. Este arquivo
//! cobre o construtor, que é puro; a inserção no mundo depende de banco e está em
//! `subcomandos_no_mundo.rs`.
//!
//! As fórmulas conferidas aqui são de `player_template::__LoadData` (o ponto de partida,
//! do `ptemplate.conf`) e `__LevelUp` (o que escala, do `CHARRACTER_CLASS_CONFIG`).

use chrono::Utc;
use pw_core::{CharacterClass, CharacterDetails, Gender, Race, Vector3};
use pw_data_loader::classes::{ConfigDeClasse, TabelaDeClasses};
use pw_data_loader::ptemplate::{BaseDaClasse, TabelaDeBase};
use pw_gs::entity::PlayerEntity;

/// Guerreiro com os números reais do realm 155 (`agi_attack`/`agi_armor` = 10/10) e
/// ganhos por nível redondos, para a conta ser conferível à mão.
fn config() -> TabelaDeClasses {
    let mut t = TabelaDeClasses::default();
    t.classes.insert(
        0,
        ConfigDeClasse {
            classe: 0,
            ataque_por_agilidade: 10,
            armadura_por_agilidade: 10,
            chance_de_critico: 6,
            vida_por_vitalidade: 4,
            mana_por_energia: 2,
            vida_por_nivel: 5.0,
            mana_por_nivel: 3.0,
            dano_por_nivel: 1.0,
            dano_magico_por_nivel: 0.0,
            defesa_por_nivel: 1.0,
            resistencia_por_nivel: 0.0,
            velocidade_de_ataque: 1.5,
            alcance_de_ataque: 1.4,
            regeneracao_de_vida: 3,
            regeneracao_de_mana: 1,
            velocidade_andando: 1.5,
            velocidade_correndo: 3.0,
            velocidade_nadando: 2.2,
            velocidade_voando: 3.0,
            faccao: 0,
            faccao_inimiga: 0,
        },
    );
    t
}

/// O `[SWORDSMAN]` do `ptemplate.conf` do pacote 1.5.5.
fn base() -> TabelaDeBase {
    let mut t = TabelaDeBase::default();
    t.classes.insert(
        0,
        BaseDaClasse {
            classe: 0,
            vida: 60,
            mana: 20,
            vitalidade: 20,
            energia: 5,
            forca: 15,
            agilidade: 10,
            ataque_em_ticks: 30,
            alcance_de_ataque: 1.4,
            regeneracao_de_vida: 3,
            regeneracao_de_mana: 1,
            velocidade_andando: 1.5,
            velocidade_correndo: 3.0,
            velocidade_nadando: 2.2,
            velocidade_voando: 3.0,
        },
    );
    t
}

fn personagem(level: i32, vitality: i32, energy: i32, agility: i32) -> CharacterDetails {
    CharacterDetails {
        id: 42,
        account_id: 7,
        realm_id: "realm_155".into(),
        name: "Testador".into(),
        race: Race::Human,
        cls: CharacterClass::Blademaster,
        gender: Gender::Male,
        level,
        cultivation: 3,
        exp: 12345,
        sp: 67,
        hp: 100,
        mp: 50,
        money: 999,
        reputation: 0,
        world_id: 1,
        position: Vector3::new(10.0, 20.0, 30.0),
        strength: 15,
        agility,
        vitality,
        energy,
        inventory_size: 64,
        storehouse_size: 32,
        inventory: Vec::new(),
        equipment: Vec::new(),
        storehouse: Vec::new(),
        skills: Vec::new(),
        quests: Vec::new(),
        custom_appearance: serde_json::Value::Null,
        version_data: serde_json::Value::Null,
        created_at: Utc::now(),
        last_login_at: None,
    }
}

#[test]
fn a_identidade_e_o_estado_vem_do_personagem() {
    let p = personagem(10, 20, 5, 10);
    let j = PlayerEntity::do_personagem(&p, &config(), Some(&base()));

    assert_eq!(j.role_id, 42);
    assert_eq!(j.name, "Testador");
    assert_eq!(j.level, 10);
    assert_eq!(j.cultivation, 3);
    assert_eq!(j.exp, 12345);
    assert_eq!(j.sp, 67);
    assert_eq!(j.money, 999);
    assert_eq!(j.position, Vector3::new(10.0, 20.0, 30.0));
    assert_eq!((j.strength, j.agility, j.vitality, j.energy), (15, 10, 20, 5));
    assert_eq!(j.target_id, None);
}

#[test]
fn a_vida_maxima_soma_base_mais_nivel_mais_vitalidade() {
    // max_hp = base.hp + lvl_hp*(nível-1) + vit_hp*vitalidade
    //        = 60 + 5*9 + 4*20 = 60 + 45 + 80 = 185
    let j = PlayerEntity::do_personagem(&personagem(10, 20, 5, 10), &config(), Some(&base()));
    assert_eq!(j.max_hp, 185);
    // max_mp = 20 + 3*9 + 2*5 = 20 + 27 + 10 = 57
    assert_eq!(j.max_mp, 57);

    // Nível 1 é só a base mais os atributos: 60 + 0 + 80 = 140.
    let j1 = PlayerEntity::do_personagem(&personagem(1, 20, 5, 10), &config(), Some(&base()));
    assert_eq!(j1.max_hp, 140);
    assert_eq!(j1.max_mp, 30);
}

#[test]
fn a_vida_corrente_nao_passa_da_maxima() {
    // O banco guarda 100 de vida; num personagem de nível 1 sem vitalidade o máximo é
    // menor que isso, e a entidade não pode entrar com vida acima do teto.
    let mut p = personagem(1, 0, 0, 10);
    p.hp = 100;
    let j = PlayerEntity::do_personagem(&p, &config(), Some(&base()));
    assert_eq!(j.max_hp, 60);
    assert_eq!(j.hp, 60, "a vida corrente devia ter sido limitada ao máximo");
}

#[test]
fn o_dano_e_a_defesa_escalam_com_o_nivel() {
    // O `__LevelUp` soma `(int)((l+1)*d) - (int)(l*d)` por nível; a soma de 1 até N é
    // `(int)(N*d) - (int)(1*d)`. Com d = 1,0 e nível 10: 10 - 1 = 9, mais o 1 de base.
    let j = PlayerEntity::do_personagem(&personagem(10, 20, 5, 10), &config(), Some(&base()));
    assert_eq!(j.attack_min, 10);
    assert_eq!(j.attack_max, 10);
    assert_eq!(j.def_phys, 9);

    // Nível 1 não ganhou nada ainda: dano fica no 1 que o original grava de base.
    let j1 = PlayerEntity::do_personagem(&personagem(1, 20, 5, 10), &config(), Some(&base()));
    assert_eq!(j1.attack_min, 1);
    assert_eq!(j1.def_phys, 0);
}

#[test]
fn a_precisao_e_a_evasao_saem_da_agilidade() {
    // agi_attack e agi_armor são 10 no guerreiro: 30 de agilidade dá 300 nos dois.
    let j = PlayerEntity::do_personagem(&personagem(10, 20, 5, 30), &config(), Some(&base()));
    assert_eq!(j.attack_rate, 300);
    assert_eq!(j.armor, 300);
}

#[test]
fn a_chance_de_critico_vira_fracao() {
    // 6 pontos percentuais na tabela viram 0,06 na entidade.
    let j = PlayerEntity::do_personagem(&personagem(10, 20, 5, 10), &config(), Some(&base()));
    assert!((j.crit_rate - 0.06).abs() < 1e-6, "crit_rate saiu {}", j.crit_rate);
}

#[test]
fn sem_o_ptemplate_a_vida_maxima_e_a_do_banco() {
    // Sem o ponto de partida não dá para calcular o teto: o honesto é usar o que está
    // gravado, e não um número inventado.
    let mut p = personagem(10, 20, 5, 10);
    p.hp = 777;
    p.mp = 333;
    let j = PlayerEntity::do_personagem(&p, &config(), None);
    assert_eq!((j.max_hp, j.hp), (777, 777));
    assert_eq!((j.max_mp, j.mp), (333, 333));
    // O que não depende do ptemplate continua saindo certo.
    assert_eq!(j.attack_rate, 100);
    assert_eq!(j.attack_min, 10);
}

#[test]
fn sem_a_tabela_de_classes_os_atributos_de_combate_ficam_neutros() {
    // Realm sem `CHARRACTER_CLASS_CONFIG` (1.2.6/v7): nada de precisão e evasão
    // inventadas. Zero é neutro no cálculo e visível no log.
    let vazia = TabelaDeClasses::default();
    let j = PlayerEntity::do_personagem(&personagem(10, 20, 5, 10), &vazia, Some(&base()));
    assert_eq!((j.attack_rate, j.armor), (0, 0));
    assert_eq!(j.crit_rate, 0.0);
    // E a vida máxima também não sai, porque depende das duas tabelas juntas.
    assert_eq!(j.max_hp, j.hp);
}

#[test]
fn o_grau_de_ataque_e_defesa_fica_em_zero_de_proposito() {
    // Vêm de equipamento e passiva no original; nenhum dos dois existe aqui. Zero é o
    // valor neutro do cálculo de dano, não um palpite.
    let j = PlayerEntity::do_personagem(&personagem(50, 40, 20, 40), &config(), Some(&base()));
    assert_eq!((j.attack_degree, j.defend_degree, j.crit_damage_bonus), (0, 0, 0));
}
