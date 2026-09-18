//! O Arqueiro do teste em jogo de 2026-09-16 contra os dados reais do `realm_155`:
//! alcance, cadência e dano vindos do arco, e o dano das habilidades pelos stubs do servidor.
//!
//! Sem a pasta do realm o teste avisa e não verifica nada.

use chrono::Utc;
use pw_core::{CharacterClass, CharacterDetails, ContainerType, Gender, ItemRecord, Race, Vector3};
use pw_data_loader::armaduras::TabelasDeEquipamento;
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::habilidades::TabelaDeHabilidades;
use pw_data_loader::{classes, ptemplate, TabelaDeBase, TabelaDeClasses};
use pw_gs::combat::CombatEngine;
use pw_gs::entity::{Equipamento, PlayerEntity};
use std::path::PathBuf;

struct Realm {
    classes: TabelaDeClasses,
    base: TabelaDeBase,
    equipamentos: TabelasDeEquipamento,
}

fn realm() -> Option<Realm> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    let Ok(bytes) = std::fs::read(dir.join("elements.data")) else {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    };
    let e = load_elements_data_auto(&bytes).unwrap();
    Some(Realm {
        classes: classes::carregar(&e),
        base: ptemplate::ler_da_pasta(&dir)?,
        equipamentos: TabelasDeEquipamento::carregar(&e),
    })
}

/// O eaa do teste: Arqueiro nível 3, 5/5/5/10 (5 pontos em agilidade).
fn arqueiro() -> CharacterDetails {
    CharacterDetails {
        id: 5491,
        account_id: 1,
        realm_id: "realm_155".into(),
        name: "eaa".into(),
        race: Race::WingedElf,
        cls: CharacterClass::Archer,
        gender: Gender::Male,
        level: 3,
        cultivation: 0,
        exp: 0,
        sp: 0,
        hp: 1000,
        mp: 1000,
        money: 0,
        reputation: 0,
        world_id: 161,
        position: Vector3::new(0.0, 0.0, 0.0),
        strength: 5,
        agility: 10,
        vitality: 5,
        energy: 5,
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

fn com_arco(r: &Realm) -> PlayerEntity {
    let mut j = PlayerEntity::do_personagem(&arqueiro(), &r.classes, Some(&r.base));
    let arco = ItemRecord::new(5491, ContainerType::Equipment, 0, 2250, 1);
    let e = Equipamento::dos_itens(&[arco], &r.equipamentos);
    j.vestir(e, &r.classes, Some(&r.base));
    j
}

/// `UpdateAttack` (`playertemplate.h:916-990`): com o Arco de Madeira o alcance é o do arco
/// (20) mais o corpo (0,3), a cadência é a do subtipo "Arco" (1,5 s × 20 = 30 ticks) e o
/// dano da arma cresce com a agilidade. Antes o alcance era o da classe (2,5) e o
/// personagem chegava perto para atirar.
#[test]
fn o_arco_de_madeira_da_alcance_cadencia_e_dano_ao_arqueiro() {
    let Some(r) = realm() else { return };
    let sem_arco = PlayerEntity::do_personagem(&arqueiro(), &r.classes, Some(&r.base));
    let j = com_arco(&r);

    assert!((j.attack_range - 20.3).abs() < 1e-3, "alcance {}", j.attack_range);
    assert!((j.attack_speed - 1.5).abs() < 1e-3, "golpe a cada {} s", j.attack_speed);
    // Arco 5..8, base de nível do Arqueiro, × (100 + agilidade 10 × 100/150 → 7)%.
    assert!(j.attack_min > sem_arco.attack_min, "o arco não somou dano: {} contra {}", j.attack_min, sem_arco.attack_min);
    assert_eq!(j.bonus_de_dano_pct, 7);
    assert_eq!(j.dano_bruto.0 - sem_arco.dano_bruto.0, 5, "damage_low do arco");
    assert_eq!(j.dano_bruto.1 - sem_arco.dano_bruto.1, 8, "damage_high_min do arco");
}

/// A 235 (连射) no nível 1: `SetRatio(0)`, `SetPlus(2,3 + 63,2 + 46,4 = 111,9)` e
/// `SetDamage(GetAttack())` — o tooltip do cliente mostra os mesmos 112
/// (`ElementSkill/skill235.h:226-231`). O dano é o do arco × bônus **mais** 111.
#[test]
fn a_habilidade_235_soma_o_plus_ao_dano_do_arco() {
    let Some(r) = realm() else { return };
    let j = com_arco(&r);
    let h = TabelaDeHabilidades::do_155();
    let d = h.get(235).unwrap().dano.clone().unwrap();
    for _ in 0..50 {
        let g = CombatEngine::golpe_de_habilidade(&j, &d, 1, 1.0).unwrap();
        let minimo = (j.dano_bruto.0 as f32 * 1.07) as i32 + 111;
        let maximo = (j.dano_bruto.1 as f32 * 1.07) as i32 + 111;
        assert!((minimo..=maximo).contains(&g.dano_fisico), "{} fora de {minimo}..{maximo}", g.dano_fisico);
        assert!(g.de_habilidade);
    }
    assert_eq!(h.get(235).unwrap().alcance(1, j.attack_range), Some(j.attack_range), "alcance = o da arma");
}

/// A 234 (引而不发) é de carga (`time_type = 3`): `ratio = (1,5 + 0,15 × nível) ×
/// GetCharging() / (5200 − 200 × nível)`. Com metade da carga, metade do ratio.
#[test]
fn a_mira_solta_antes_da_carga_cheia_tira_menos() {
    let Some(r) = realm() else { return };
    let mut j = com_arco(&r);
    j.dano_bruto = (100, 100);
    let h = TabelaDeHabilidades::do_155();
    let mira = h.get(234).unwrap();
    assert!(mira.e_de_carga());
    let d = mira.dano.clone().unwrap();
    let cheia = CombatEngine::golpe_de_habilidade(&j, &d, 1, 1.0).unwrap().dano_fisico;
    let metade = CombatEngine::golpe_de_habilidade(&j, &d, 1, 0.5).unwrap().dano_fisico;
    // 100 × (100 + 7 + 165)% = 272; com metade: 100 × (100 + 7 + 82)% = 189.
    assert_eq!(cheia, 272);
    assert_eq!(metade, 189);
}
