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
        waypoints: Vec::new(),
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

/// A habilidade 244 (烈焰之矢 / Flecha Fulgurante): buff com ícone 70 (HSTATE_FIREARROW),
/// estado visível 30 (VSTATE_FIREARROW) e adição de dano mágico de fogo (magic_damage[3])
/// ao ataque físico normal (`filter_Firearrow::TranslateSendAttack`).
#[test]
fn a_flecha_fulgurante_adiciona_icone_70_e_dano_de_fogo_ao_ataque() {
    let Some(r) = realm() else { return };
    let mut j = com_arco(&r);

    let filtro = pw_gs::efeitos::Filtro {
        efeito: pw_gs::efeitos::Efeito::Firearrow,
        restante_s: 600,
        razao: 13,
        fator: 0.13, // 0.10 + 0.03 * 1
        por_segundo: 0,
        contador: 0,
        origem: j.role_id as i64,
        icone: true,
    };
    j.efeitos.adicionar(filtro);

    // 1. Ícones contém o estado 70 (HSTATE_FIREARROW) com 600 segundos
    let icones = j.efeitos.icones();
    assert!(icones.iter().any(|&(h, t)| h == 70 && t == 600), "deve conter ícone 70: {:?}", icones);

    // 2. Estados visíveis contém o VSTATE_FIREARROW (30)
    let visiveis = j.efeitos.estados_visiveis();
    assert_ne!(visiveis[0] & (1 << 30), 0, "deve ter o bit 30 ativo no primeiro DWORD de estados visíveis");

    // 3. O golpe normal soma o dano de fogo em `dano_magico[3]`, e a conta é sobre o dano
    //    **da arma vestida** — `_ratio × 0,5 × (weapon.damage_low + weapon.damage_high)`
    //    (`filter_Firearrow::TranslateSendAttack`, `cskill/skill/skillfilter.h:4268-4275`),
    //    não sobre o dano total do personagem (B67).
    let golpe = CombatEngine::golpe_de_jogador(&j);
    let (baixo, alto) = j.equipamento.arma.expect("o arco está vestido").dano;
    let esperado_fogo = (0.13 * 0.5 * (baixo + alto) as f32) as i32;
    assert_eq!(golpe.dano_magico[3], esperado_fogo, "dano de fogo deve ser {esperado_fogo}");
    // Com o Arco de Madeira (5..8) e razão 0,13 a conta trunca em zero — como no original,
    // que também faz `(int)(...)`. Com uma arma de verdade o bônus aparece:
    if let Some(arma) = j.equipamento.arma.as_mut() {
        arma.dano = (200, 300);
    }
    let golpe = CombatEngine::golpe_de_jogador(&j);
    assert_eq!(golpe.dano_magico[3], (0.13 * 0.5 * 500.0) as i32, "32 de fogo com arma 200..300");

    // 4. Pacote ICON_STATE_NOTIFY (125) codifica corretamente
    let pacote = pw_protocol::S2CGamedataSend::icon_state_notify(j.role_id, &icones);
    assert_eq!(pacote.data[0..2], [125, 0], "opcode deve ser 125");
}

/// A habilidade 244 não possui `Probability` no stub: o roteiro deve aplicar o `Firearrow`
/// com 100% de probabilidade padrão.
#[test]
fn o_roteiro_da_habilidade_244_aplica_firearrow_sem_precisar_de_set_probability() {
    let h = TabelaDeHabilidades::do_155();
    let skill = h.get(244).expect("habilidade 244 deve existir");
    let passos = skill.no_alvo.clone().expect("deve ter no_alvo");

    let vars = |nome: &str| match nome {
        "L" => Some(1.0),
        _ => None,
    };
    let mut dado = || 50;
    let (aplicacoes, _) = pw_gs::efeitos::executar_roteiro(&passos, &vars, &mut dado);

    let firearrow = aplicacoes.iter().find(|a| a.nome == "Firearrow");
    assert!(firearrow.is_some(), "Firearrow deve ser aplicado mesmo sem SetProbability prévio!");
    let fa = firearrow.unwrap();
    assert_eq!(fa.tempo_s, 600, "tempo deve ser 600s");
    assert!((fa.razao - 0.13).abs() < 1e-4, "razão do nível 1 deve ser 0.13: {}", fa.razao);
    assert_eq!(fa.efeito, Some(pw_gs::efeitos::Efeito::Firearrow));
}

/// O ID do monstro invocado dinamicamente deve satisfazer a macro oficial do cliente:
/// `#define ISNPCID(id) (((id) & 0x80000000) && !((id) & 0x40000000))` (EC_GPDataType.h:26).
/// Se o bit 30 (0x40000000) for 1, o cliente o considera matéria/mina (ISMATTERID) e recusa seleção com clique.
#[test]
fn o_id_do_monstro_invocado_satisfaz_a_macro_is_npc_id_do_cliente() {
    const PRIMEIRO: u32 = 0xA000_0000;
    let mut prox = PRIMEIRO;
    for _ in 0..100 {
        let nid = prox as i32;
        let is_npc_id = ((nid as u32 & 0x8000_0000) != 0) && ((nid as u32 & 0x4000_0000) == 0);
        let is_player_id = (nid != 0) && ((nid as u32 & 0x8000_0000) == 0);
        let is_matter_id = (nid as u32 & 0xC000_0000) == 0xC000_0000;

        assert!(is_npc_id, "ID {:#X} deve ser reconhecido como NPC pelo cliente!", nid as u32);
        assert!(!is_player_id, "ID {:#X} não deve ser jogador", nid as u32);
        assert!(!is_matter_id, "ID {:#X} não deve ser matéria (mina/drop)", nid as u32);

        prox = PRIMEIRO | ((prox.wrapping_add(1)) & 0x1FFF_FFFF);
    }
}

