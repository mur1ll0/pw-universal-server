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
        eprintln!(
            "AVISO: sem {} — este teste NÃO verificou nada.",
            dir.display()
        );
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
        modo_roupa: false,
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
        ap: 0,
        max_ap: 0,
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

    assert!(
        (j.attack_range - 20.3).abs() < 1e-3,
        "alcance {}",
        j.attack_range
    );
    assert!(
        (j.attack_speed - 1.5).abs() < 1e-3,
        "golpe a cada {} s",
        j.attack_speed
    );
    // Arco 5..8, base de nível do Arqueiro, × (100 + agilidade 10 × 100/150 → 7)%.
    assert!(
        j.attack_min > sem_arco.attack_min,
        "o arco não somou dano: {} contra {}",
        j.attack_min,
        sem_arco.attack_min
    );
    assert_eq!(j.bonus_de_dano_pct, 7);
    assert_eq!(
        j.dano_bruto.0 - sem_arco.dano_bruto.0,
        5,
        "damage_low do arco"
    );
    assert_eq!(
        j.dano_bruto.1 - sem_arco.dano_bruto.1,
        8,
        "damage_high_min do arco"
    );
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
        assert!(
            (minimo..=maximo).contains(&g.dano_fisico),
            "{} fora de {minimo}..{maximo}",
            g.dano_fisico
        );
        assert!(g.de_habilidade);
    }
    assert_eq!(
        h.get(235).unwrap().alcance(1, j.attack_range),
        Some(j.attack_range),
        "alcance = o da arma"
    );
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
    let cheia = CombatEngine::golpe_de_habilidade(&j, &d, 1, 1.0)
        .unwrap()
        .dano_fisico;
    let metade = CombatEngine::golpe_de_habilidade(&j, &d, 1, 0.5)
        .unwrap()
        .dano_fisico;
    // 100 × (100 + 7 + 165)% = 272; com metade: 100 × (100 + 7 + 82)% = 189.
    assert_eq!(cheia, 272);
    assert_eq!(metade, 189);
}

/// B73 — a Barreira de Asa (249) põe o `filter_Wingshield`: escudo que absorve dano e
/// injeta mana a cada 3 s, por 20 s (`cskill/skills/skill249.h:257-262`).
#[test]
fn a_barreira_de_asa_absorve_dano_e_devolve_mana() {
    let mut efeitos = pw_gs::efeitos::Efeitos::default();
    // Nível 1: `SetAmount(60 + 75 × 1)` = 135, `SetValue(4 + 6 × 1)` = 10, `SetTime(20000)`.
    efeitos.adicionar(pw_gs::efeitos::Filtro {
        efeito: pw_gs::efeitos::Efeito::Wingshield,
        restante_s: 20,
        razao: 0,
        fator: 0.0,
        por_segundo: 10,
        contador: 0,
        origem: 0,
        icone: true,
        absorve: 135.0,
        escala_defesa: 0,
        fator_de_habilidade: 0.0,
    });

    // Ícone 69 (HSTATE_WINGSHIELD) e estado visível 29 (VSTATE_WINGSHIELD).
    assert!(
        efeitos.icones().iter().any(|&(h, t)| h == 69 && t == 20),
        "sem o ícone do escudo"
    );
    assert_ne!(
        efeitos.estados_visiveis()[0] & (1 << 29),
        0,
        "sem o VSTATE_WINGSHIELD"
    );

    // `AdjustDamage`: um quinto do golpe (20) cabe no escudo (135), então passam 20 de 100 e
    // o escudo perde quatro vezes isso (`skillfilter.h:4168-4183`).
    assert_eq!(
        pw_gs::efeitos::dano_recebido(&mut efeitos, 100),
        20,
        "o escudo não absorveu"
    );
    // 135 − 80 = 55. O golpe seguinte de 500 não cabe: `r = 1 − 55/100` = 0,45.
    assert_eq!(
        pw_gs::efeitos::dano_recebido(&mut efeitos, 500),
        225,
        "o escudo não se esgotou direito"
    );
    // Zerado, o filtro se apaga (`_amount < 6`).
    let (_, algum_acabou) = efeitos.batida();
    assert!(algum_acabou, "o escudo gasto devia sumir");
    assert!(
        efeitos.icones().is_empty(),
        "o ícone do escudo ficou para trás"
    );

    // E o batimento devolve a mana de três em três segundos, o valor inteiro do `SetValue`.
    let mut efeitos = pw_gs::efeitos::Efeitos::default();
    efeitos.adicionar(pw_gs::efeitos::Filtro {
        efeito: pw_gs::efeitos::Efeito::Wingshield,
        restante_s: 20,
        razao: 0,
        fator: 0.0,
        por_segundo: 10,
        contador: 0,
        origem: 0,
        icone: true,
        absorve: 135.0,
        escala_defesa: 0,
        fator_de_habilidade: 0.0,
    });
    let mut manas = Vec::new();
    for _ in 0..6 {
        for t in efeitos.batida().0 {
            if let pw_gs::efeitos::Tique::Mana(v) = t {
                manas.push(v);
            }
        }
    }
    assert_eq!(
        manas,
        vec![10, 10],
        "seis segundos deviam render dois tiques de 10 de mana"
    );
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
        absorve: 0.0,
        escala_defesa: 0,
        fator_de_habilidade: 0.0,
    };
    j.efeitos.adicionar(filtro);

    // 1. Ícones contém o estado 70 (HSTATE_FIREARROW) com 600 segundos
    let icones = j.efeitos.icones();
    assert!(
        icones.iter().any(|&(h, t)| h == 70 && t == 600),
        "deve conter ícone 70: {:?}",
        icones
    );

    // 2. Estados visíveis contém o VSTATE_FIREARROW (30)
    let visiveis = j.efeitos.estados_visiveis();
    assert_ne!(
        visiveis[0] & (1 << 30),
        0,
        "deve ter o bit 30 ativo no primeiro DWORD de estados visíveis"
    );

    // 3. O golpe normal soma o dano de fogo em `dano_magico[3]`, e a conta é sobre o dano
    //    **da arma vestida** — `_ratio × 0,5 × (weapon.damage_low + weapon.damage_high)`
    //    (`filter_Firearrow::TranslateSendAttack`, `cskill/skill/skillfilter.h:4268-4275`),
    //    não sobre o dano total do personagem (B67).
    let golpe = CombatEngine::golpe_de_jogador(&j);
    let (baixo, alto) = j.equipamento.arma.expect("o arco está vestido").dano;
    let esperado_fogo = (0.13 * 0.5 * (baixo + alto) as f32) as i32;
    assert_eq!(
        golpe.dano_magico[3], esperado_fogo,
        "dano de fogo deve ser {esperado_fogo}"
    );
    // Com o Arco de Madeira (5..8) e razão 0,13 a conta trunca em zero — como no original,
    // que também faz `(int)(...)`. Com uma arma de verdade o bônus aparece:
    if let Some(arma) = j.equipamento.arma.as_mut() {
        arma.dano = (200, 300);
    }
    let golpe = CombatEngine::golpe_de_jogador(&j);
    assert_eq!(
        golpe.dano_magico[3],
        (0.13 * 0.5 * 500.0) as i32,
        "32 de fogo com arma 200..300"
    );

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
    assert!(
        firearrow.is_some(),
        "Firearrow deve ser aplicado mesmo sem SetProbability prévio!"
    );
    let fa = firearrow.unwrap();
    assert_eq!(fa.tempo_s, 600, "tempo deve ser 600s");
    assert!(
        (fa.razao - 0.13).abs() < 1e-4,
        "razão do nível 1 deve ser 0.13: {}",
        fa.razao
    );
    assert_eq!(fa.efeito, Some(pw_gs::efeitos::Efeito::Firearrow));
}

/// O ID do monstro invocado dinamicamente deve satisfazer a macro oficial do cliente:
/// `#define ISNPCID(id) (((id) & 0x80000000) && !((id) & 0x40000000))` (EC_GPDataType.h:26).
/// Se o bit 30 (0x40000000) for 1, o cliente o considera matéria/mina (ISMATTERID) e recusa seleção com clique.
#[test]
fn o_id_do_monstro_invocado_satisfaz_a_macro_is_npc_id_do_cliente() {
    // B117: fora do bit 29 (`PET_MASK`), que marca mascote.
    const PRIMEIRO: u32 = 0x9000_0000;
    let mut prox = PRIMEIRO;
    for _ in 0..100 {
        let nid = prox as i32;
        let is_npc_id = ((nid as u32 & 0x8000_0000) != 0) && ((nid as u32 & 0x4000_0000) == 0);
        let is_player_id = (nid != 0) && ((nid as u32 & 0x8000_0000) == 0);
        let is_matter_id = (nid as u32 & 0xC000_0000) == 0xC000_0000;

        assert!(
            is_npc_id,
            "ID {:#X} deve ser reconhecido como NPC pelo cliente!",
            nid as u32
        );
        assert!(!is_player_id, "ID {:#X} não deve ser jogador", nid as u32);
        assert!(
            !is_matter_id,
            "ID {:#X} não deve ser matéria (mina/drop)",
            nid as u32
        );
        assert!(
            !pw_gs::mascote::e_mascote(nid as i64),
            "ID {:#X} não pode ter o bit de mascote",
            nid as u32
        );

        prox = PRIMEIRO | ((prox.wrapping_add(1)) & 0x1FFF_FFFF);
    }
}

/// A Forma Sombria (2570) do Tormentador: o roteiro é `SetTime(16000 + 3000 × L)`,
/// `SetRatio(0,04 × L)`, `SetValue(0,6 × L)`, `SetFairyform(1)`
/// (`cskill/skills/skill2570.h:240-243`), e o `SetFairyform` não consulta o dado.
#[test]
fn o_roteiro_da_forma_sombria_aplica_fairyform_com_os_numeros_do_stub() {
    let h = TabelaDeHabilidades::do_155();
    let passos = h
        .get(2570)
        .expect("habilidade 2570")
        .no_alvo
        .clone()
        .expect("roteiro no alvo");
    let vars = |nome: &str| (nome == "L").then_some(1.0);
    let mut dado = || 99;
    let (aplicacoes, _) = pw_gs::efeitos::executar_roteiro(&passos, &vars, &mut dado);
    let f = aplicacoes
        .iter()
        .find(|a| a.nome == "Fairyform")
        .expect("Fairyform não saiu do roteiro");
    assert_eq!(
        f.efeito,
        Some(pw_gs::efeitos::Efeito::Fairyform),
        "Fairyform continua sem porte"
    );
    assert_eq!(f.tempo_s, 19, "16000 + 3000 ms = 19 s");
    assert!((f.razao - 0.04).abs() < 1e-5, "ratio {}", f.razao);
    assert!((f.valor - 0.6).abs() < 1e-5, "value {}", f.valor);
}

/// `filter_Fairyform` (`cskill/skill/skillfilter.h:16819-16875`): forma 65 (`1 | FORM_CLASS <<
/// 6`), equipamento trancado, `_speed`% de velocidade e `_defense`% de defesa, ícone 279
/// (`HSTATE_FAIRYFORM`), **sem** `REMOVE_ON_DEATH`, e tudo desfeito quando o tempo acaba.
#[test]
fn a_forma_sombria_transforma_tranca_o_equipamento_e_acaba_no_tempo() {
    use pw_gs::efeitos::{Efeito, Efeitos, Filtro};
    let mut e = Efeitos::default();
    assert_eq!(e.forma(), None);
    // Nível 1: (int)(100 × 0,04) = 4 e (int)(100 × 0,6) = 60, por 19 s.
    e.adicionar(Filtro {
        efeito: Efeito::Fairyform,
        restante_s: 19,
        razao: 4,
        fator: 0.04,
        por_segundo: 0,
        contador: 0,
        origem: 0,
        icone: true,
        absorve: 0.0,
        escala_defesa: 60,
        fator_de_habilidade: 0.0,
    });
    assert_eq!(e.forma(), Some((1, 1)));
    assert!(e.equipamento_travado());
    let r = e.realce();
    assert_eq!((r.velocidade, r.defesa), (4, 60));
    assert!(
        e.icones().iter().any(|&(h, t)| h == 279 && t == 19),
        "sem o ícone HSTATE_FAIRYFORM"
    );
    assert_eq!(e.estados_visiveis(), [0; 6], "o Fairyform não tem VSTATE");

    // `SetFairyform` recusa quem já está transformado; aqui, o WEAK descarta o segundo.
    let mut outro = e.filtros[0].clone();
    outro.restante_s = 99;
    assert!(!e.adicionar(outro));
    // Nem o Dispersar (bênção/maldição) nem a morte a tiram.
    assert!(!e.limpar(true) && !e.limpar(false));
    e.ao_morrer();
    assert_eq!(
        e.forma(),
        Some((1, 1)),
        "a forma não tem FILTER_MASK_REMOVE_ON_DEATH"
    );

    let mut acabou = false;
    for _ in 0..19 {
        acabou |= e.batida().1;
    }
    assert!(acabou, "a forma devia acabar em 19 s");
    assert_eq!(e.forma(), None);
    assert!(!e.equipamento_travado());
    assert_eq!(e.realce().velocidade, 0);
}

/// Muralha de Espinhos (306, B120): no 1.2.6 o `gs` faz `SetRetort` com `ratio` 0,05·L + 0,1
/// por 600 s (VA 0x837de86); no 1.5.5 o stub faz `SetRetort2` com o mesmo `ratio` e `value`
/// 0,02·L (`cskill/skills/skill306.h`). Os dois viram filtro — antes, "sem porte".
#[test]
fn a_muralha_de_espinhos_vira_retort_nas_duas_versoes() {
    for (tabela, nome, efeito) in [
        (
            TabelaDeHabilidades::do_126(),
            "Retort",
            pw_gs::efeitos::Efeito::Retort,
        ),
        (
            TabelaDeHabilidades::do_155(),
            "Retort2",
            pw_gs::efeitos::Efeito::Retort2,
        ),
    ] {
        let passos = tabela
            .get(306)
            .expect("306")
            .no_alvo
            .clone()
            .expect("roteiro no alvo");
        let vars = |n: &str| (n == "L").then_some(2.0);
        let mut dado = || 0;
        let (aplicacoes, _) = pw_gs::efeitos::executar_roteiro(&passos, &vars, &mut dado);
        let a = aplicacoes
            .iter()
            .find(|a| a.nome == nome)
            .unwrap_or_else(|| panic!("{nome} não saiu"));
        assert_eq!(a.efeito, Some(efeito));
        assert_eq!(a.tempo_s, 600);
        assert!((a.razao - 0.2).abs() < 1e-5, "ratio {}", a.razao);
    }
}

/// Chamado da Raposa (312, B120): as duas versões fazem `SetFoxform`, e o `allow_forms` separa as
/// habilidades de forma. No nível 1, `probability` (a precisão do `filter_Foxform`) é 1,0 no
/// 1.5.5 (`skill312.h:163-167`) e 1,5 no 1.2.6 (`0,5·L + 1` do `StateAttack` do `gs` 1.2.6, B122).
#[test]
fn o_chamado_da_raposa_e_as_formas_das_habilidades() {
    for (tabela, precisao) in [
        (TabelaDeHabilidades::do_126(), 1.5),
        (TabelaDeHabilidades::do_155(), 1.0),
    ] {
        let passos = tabela
            .get(312)
            .expect("312")
            .no_alvo
            .clone()
            .expect("roteiro no alvo");
        let vars = |n: &str| (n == "L").then_some(1.0);
        let mut dado = || 99;
        let (aplicacoes, _) = pw_gs::efeitos::executar_roteiro(&passos, &vars, &mut dado);
        let a = aplicacoes
            .iter()
            .find(|a| a.nome == "Foxform")
            .expect("Foxform não saiu");
        assert_eq!(a.efeito, Some(pw_gs::efeitos::Efeito::Foxform));
        assert!(
            (a.razao - 0.3).abs() < 1e-5
                && (a.quantia - 0.6).abs() < 1e-5
                && (a.probabilidade - precisao).abs() < 1e-5
        );
        assert!(
            tabela.get(312).unwrap().permitida_na_forma(0)
                && tabela.get(312).unwrap().permitida_na_forma(1)
        );
        for forma_de_raposa in 313..=318 {
            let h = tabela.get(forma_de_raposa).unwrap();
            assert!(
                !h.permitida_na_forma(0) && h.permitida_na_forma(1),
                "{forma_de_raposa} só na raposa"
            );
        }
        let h = tabela.get(299).unwrap();
        assert!(
            h.permitida_na_forma(0) && !h.permitida_na_forma(1),
            "299 só fora da raposa"
        );
    }
}

/// Roteiros do 1.2.6 gerados do `gs` 1.2.6 (B122, `specs/habilidades_126/roteiros_126.py`): os
/// 64 que o catálogo herdava errados do 1.5.5 agora são os do binário, e o 1.5.5 fica como era.
#[test]
fn os_roteiros_do_126_sao_os_do_gs_126() {
    let t126 = TabelaDeHabilidades::do_126();
    let t155 = TabelaDeHabilidades::do_155();
    let nomes = |t: &TabelaDeHabilidades, id: u32, em_si: bool| -> Vec<String> {
        let h = t.get(id).unwrap_or_else(|| panic!("{id}"));
        let r = if em_si { &h.em_si } else { &h.no_alvo };
        r.clone()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.1)
            .filter(|n| {
                !matches!(
                    n.as_str(),
                    "Probability" | "Time" | "Ratio" | "Amount" | "Value" | "Showicon"
                )
            })
            .collect()
    };
    let rodar =
        |t: &TabelaDeHabilidades, id: u32, em_si: bool, vars: &dyn Fn(&str) -> Option<f64>| {
            let h = t.get(id).unwrap();
            let passos = if em_si {
                h.em_si.clone()
            } else {
                h.no_alvo.clone()
            }
            .expect("roteiro");
            let mut dado = || 0;
            pw_gs::efeitos::executar_roteiro(&passos, vars, &mut dado).0
        };
    // 610: o 1.2.6 reduz ataque e magia (0,2 por 30 s); o 1.5.5, defesa e resistência.
    assert_eq!(
        nomes(&t126, 610, false),
        ["Eaglecurse", "Decattack", "Decmagic"]
    );
    assert_eq!(
        nomes(&t155, 610, false),
        ["Eaglecurse", "Decdefence", "Decresist"]
    );
    let a = rodar(&t126, 610, false, &|n| (n == "L").then_some(1.0));
    let dec = a.iter().find(|x| x.nome == "Decattack").unwrap();
    assert_eq!(dec.tempo_s, 30);
    assert!((dec.razao - 0.2).abs() < 1e-6);
    // 404: `BlessMe` dá chi (`SetAp`, 50 % de 100) no lugar do `Apgencont2` do 1.5.5.
    assert_eq!(nomes(&t126, 404, true), ["Ap"]);
    let ap = rodar(&t126, 404, true, &|n| (n == "L").then_some(1.0));
    assert_eq!((ap[0].nome.as_str(), ap[0].valor), ("Ap", 100.0));
    let passos = t126.get(404).unwrap().em_si.clone().unwrap();
    assert_eq!(passos[0].2, "1.0 * 50", "o dado de 50 % do `ThrowDice`");
    // 187: `SetProbability(11.0)` (0x41300000), tempo `rand() % 100 + 100` ms e `SetDizzy`
    // (VA 0x835ff10) — no 1.5.5 o roteiro é vazio.
    assert_eq!(nomes(&t126, 187, false), ["Dizzy"]);
    // 116: o `gs` guarda `S_Magicdamage × (0,2 + 0,03·L) + 630 + 60·L` num `int` antes do
    // `SetValue`: com 333 de dano mágico no nível 1, 766 e não 766,59.
    let cura = rodar(&t126, 116, false, &|n| match n {
        "L" => Some(1.0),
        "S_Magicdamage" => Some(333.0),
        _ => Some(0.0),
    });
    assert_eq!(cura.iter().find(|x| x.nome == "Heal").unwrap().valor, 766.0);
    // 167: o Portal da Cidade faz `SetReturntown` no `StateAttack` do 1.2.6.
    assert_eq!(nomes(&t126, 167, false), ["Returntown"]);
    // 58/59: sem `StateAttack` no 1.2.6.
    assert!(nomes(&t126, 58, false).is_empty() && nomes(&t126, 59, false).is_empty());
}

/// B122 — `EventChange`: as passivas `EVENT_CHANGE` (a 323 da raposa: `EnhanceSwimSpeed(50)`;
/// a 324: `EnhanceScaleDamage(20 + 10·L)`) só valem na forma de classe, e o nado passa pelo
/// `UpdateSpeed` com o teto de 15 (`playertemplate.h:1104-1108`, `config.h:115`).
#[test]
fn as_passivas_de_forma_valem_so_na_forma_de_classe() {
    let Some(r) = realm() else { return };
    let mut p = PlayerEntity::do_personagem(&arqueiro(), &r.classes, Some(&r.base));
    let (nado, ataque) = (p.swim_speed, p.attack_max);
    let mut conhecidas = std::collections::HashMap::new();
    conhecidas.insert(323u32, 1u8);
    conhecidas.insert(324u32, 1u8);
    p.passivas_de_forma =
        pw_gs::efeitos::passivas_de_forma(&TabelaDeHabilidades::do_126(), &conhecidas);
    assert_eq!(
        (p.passivas_de_forma.natacao, p.passivas_de_forma.dano),
        (50, 30)
    );
    p.recalcular_por_nivel(&r.classes, Some(&r.base));
    assert_eq!(
        (p.swim_speed, p.attack_max),
        (nado, ataque),
        "fora da forma não vale"
    );
    p.efeitos.adicionar(pw_gs::efeitos::Filtro {
        efeito: pw_gs::efeitos::Efeito::Foxform,
        restante_s: i32::MAX,
        razao: 0,
        fator: 0.0,
        por_segundo: 0,
        contador: 1,
        origem: 0,
        icone: true,
        absorve: 0.0,
        escala_defesa: 0,
        fator_de_habilidade: 0.0,
    });
    p.recalcular_por_nivel(&r.classes, Some(&r.base));
    assert!(
        (p.swim_speed - (nado * 1.5).min(15.0)).abs() < 1e-4,
        "nado {nado} → {}",
        p.swim_speed
    );
    assert!(p.attack_max > ataque, "dano {ataque} → {}", p.attack_max);
    p.efeitos.desfazer_raposa();
    p.recalcular_por_nivel(&r.classes, Some(&r.base));
    assert_eq!(
        (p.swim_speed, p.attack_max),
        (nado, ataque),
        "o UndoEffect tira as passivas"
    );
}

/// B144 — munição: `projectile_equip_item::VerifyRequirement` (`equip_item.cpp:1251-1271`) só
/// ativa a flecha com o tipo que a arma pede e o `weapon_level` na faixa dela; ativa, soma o
/// `enhance_damage` ao dano (`UpdateEssence` + `NormalEnhance`). O Arco de Madeira (2250, nível
/// 0, pede 8546) aceita a Flecha de Novato (43283, 0–17, +1) e recusa a de Iniciante (8543,
/// 1–17).
#[test]
fn a_flecha_so_ativa_na_faixa_da_arma_e_soma_o_dano() {
    let Some(r) = realm() else { return };
    let arco = ItemRecord::new(5491, ContainerType::Equipment, 0, 2250, 1);
    let novato = ItemRecord::new(5491, ContainerType::Equipment, 11, 43283, 100);
    let iniciante = ItemRecord::new(5491, ContainerType::Equipment, 11, 8543, 100);
    let sem = Equipamento::dos_itens(std::slice::from_ref(&arco), &r.equipamentos);
    let com = Equipamento::dos_itens(&[arco.clone(), novato], &r.equipamentos);
    let fora = Equipamento::dos_itens(&[arco, iniciante], &r.equipamentos);
    assert_eq!(sem.municao_ativa, None);
    assert_eq!(com.municao_ativa, Some(8546));
    assert_eq!(com.dano, sem.dano + 1, "o enhance_damage da flecha");
    assert_eq!(fora.municao_ativa, None, "8543 pede arma de nível 1 a 17");
    assert_eq!(fora.dano, sem.dano);
}
