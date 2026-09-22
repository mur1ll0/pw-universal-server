//! O Daimon (o `elf_item` do original), no slot 23 do equipamento.
//!
//! O estado dele **é** o bloco de dados do item, e a experiência vem de um décimo da que o
//! jogador ganha (`ElfReceiveExp(exp / 10)`, `gs/player.cpp:2921-2928`).

use pw_data_loader::GameDataManager;
use pw_gs::entity::{Daimon, DaimonVestido};
use std::path::PathBuf;

fn realm() -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

/// O bloco de um Daimon novo é o que `generate_elf` escreve
/// (`gs/template/generate_item_temp.h:2485-2524`): 38 bytes de `elf_essence` (empacotado em
/// 1 byte), a lista de equipamento vazia e a de habilidades com as iniciais no nível 1.
#[test]
fn o_bloco_do_daimon_novo_e_o_do_gerador() {
    let d = Daimon::novo(&[1001, 1002]);
    let b = d.bloco();
    assert_eq!(b.len(), 38 + 4 + 4 + 2 * 4, "o bloco não tem o tamanho do gerador");

    assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()), 0, "exp");
    assert_eq!(i16::from_le_bytes(b[4..6].try_into().unwrap()), 1, "level");
    assert_eq!(i16::from_le_bytes(b[6..8].try_into().unwrap()), 0, "total_attribute");
    assert_eq!(i16::from_le_bytes(b[16..18].try_into().unwrap()), 1, "total_genius");
    assert_eq!(i16::from_le_bytes(b[28..30].try_into().unwrap()), 0, "refine_level");
    assert_eq!(i32::from_le_bytes(b[30..34].try_into().unwrap()), 20_000, "stamina");
    assert_eq!(i32::from_le_bytes(b[34..38].try_into().unwrap()), 0, "status_value");
    assert_eq!(i32::from_le_bytes(b[38..42].try_into().unwrap()), 0, "sem equipamento");
    assert_eq!(i32::from_le_bytes(b[42..46].try_into().unwrap()), 2, "duas habilidades iniciais");
    assert_eq!(u16::from_le_bytes(b[46..48].try_into().unwrap()), 1001);
    assert_eq!(i16::from_le_bytes(b[48..50].try_into().unwrap()), 1, "habilidade no nível 1");

    // E o bloco se relê inteiro: é assim que o estado sobrevive entre sessões.
    assert_eq!(Daimon::ler(&b), Some(d));
    assert_eq!(Daimon::ler(&b[..20]), None, "bloco curto não pode virar Daimon");
}

/// `InsertExp` (`gs/item/item_elf.cpp:692-750`): o Daimon come a experiência do jogador,
/// sobe de nível quando fecha a conta e **para** a um ponto de alcançar o nível do dono.
#[test]
fn o_daimon_come_um_decimo_e_sobe_de_nivel() {
    // Curva de teste: 100 de experiência por nível, sem fator.
    let precisa = |_nivel: i16| 100u32;
    let mut d = DaimonVestido {
        slot: 23,
        item_id: 23752,
        fator_de_exp: 1.0,
        estado: Daimon::novo(&[]),
        sujo: false,
    };

    // Jogador de nível 5, Daimon de 1: o fator é 1/5, então de 50 de experiência entram 10.
    let (ganhou, subiu) = d.receber_exp(50, 5, 5, precisa);
    assert!(ganhou && !subiu, "devia ganhar sem subir");
    assert_eq!(d.estado.exp, 10, "1/5 de 50");
    assert!(d.sujo, "o bloco mudou e precisa ir ao banco");

    // Experiência bastante: o laço do original sobe **quantos níveis couberem**, e a cada
    // nível o fator melhora (1/5, 2/5, 3/5…). Com 1000 e a curva de 100, o Daimon vai do 1
    // ao 5 — o nível do jogador — e para lá, com o resto virando experiência.
    let (_, subiu) = d.receber_exp(1000, 5, 5, precisa);
    assert!(subiu, "não subiu de nível");
    assert_eq!(d.estado.nivel, 5, "devia subir até o nível do jogador");
    assert_eq!(d.estado.exp, 8, "o que sobrou depois do último nível");
    assert_eq!(d.estado.total_de_atributos, 4, "um ponto de atributo por nível");
    assert_eq!(d.estado.total_de_genios, 2, "um gênio a mais ao chegar no nível 5");

    // No nível do jogador, o Daimon para a um ponto de subir e não passa disso.
    let mut d = DaimonVestido {
        slot: 23,
        item_id: 23752,
        fator_de_exp: 1.0,
        estado: Daimon { nivel: 5, ..Daimon::novo(&[]) },
        sujo: false,
    };
    d.receber_exp(10_000, 5, 5, precisa);
    assert_eq!(d.estado.nivel, 5, "passou do nível do jogador");
    assert_eq!(d.estado.exp, 99, "devia parar a um ponto de subir");
    // E daí em diante não ganha mais nada.
    assert_eq!(d.receber_exp(10_000, 5, 5, precisa), (false, false));

    // Daimon acima do jogador não recebe nada (`player_level < ess.level`).
    let mut d = DaimonVestido {
        slot: 23,
        item_id: 23752,
        fator_de_exp: 1.0,
        estado: Daimon { nivel: 9, ..Daimon::novo(&[]) },
        sujo: false,
    };
    assert_eq!(d.receber_exp(1000, 5, 5, precisa), (false, false));
}

/// B76 — por que o Daimon nível 1 de um dono nível 16 parece não ganhar nada.
///
/// O fator de obtenção é a razão dos níveis com piso de 10 % (`GetExpObtainFactor`,
/// `gs/item/item_elf.cpp:754-773`), e o ganho é truncado para inteiro
/// (`can_obtain_exp = (unsigned int)(exp * obtain_factor + 0.00001)`, `:715`). Com um décimo
/// de 80 de experiência — um monstro de nível 16 do `realm_155` — sobram 8 × 0,1 = 0,8, que
/// vira **zero**. É assim no original; o Daimon precisa de monstros de 100 de experiência
/// para ganhar o primeiro ponto.
#[test]
fn o_ganho_trunca_para_zero_com_pouca_experiencia() {
    let precisa = |_n: i16| 1000u32;
    let mut d = DaimonVestido {
        slot: 23,
        item_id: 23752,
        fator_de_exp: 1.0,
        estado: Daimon::novo(&[]),
        sujo: false,
    };
    // Um décimo de 80 é 8; 8 × 0,1 = 0,8 → nada.
    assert_eq!(d.receber_exp(8, 16, 16, precisa), (false, false), "80 de experiência não podia dar nada");
    assert_eq!(d.estado.exp, 0);
    // Com 100 de experiência (um décimo = 10), entra 1.
    assert_eq!(d.receber_exp(10, 16, 16, precisa), (true, false));
    assert_eq!(d.estado.exp, 1);
}

/// O `exp_factor` e as habilidades iniciais vêm do `GOBLIN_ESSENCE` do realm.
#[test]
fn o_daimon_do_realm_traz_fator_e_habilidades() {
    let Some(dados) = realm() else { return };
    // O 23752 é o "Verão", o que o eaa usa (slot 23 do equipamento dele).
    let (fator, iniciais) = dados.dados_do_daimon(23752).expect("o 23752 é um GOBLIN_ESSENCE");
    assert!(fator > 0.0 && fator <= 2.45, "exp_factor fora da faixa do original: {fator}");
    assert!(!iniciais.is_empty(), "o Daimon nasce com pelo menos uma habilidade");
    assert!(iniciais.iter().all(|s| *s > 0));
    assert_eq!(dados.dados_do_daimon(1796), None, "poção não é Daimon");

    // E o bloco gerado para ele fecha com as habilidades iniciais.
    let b = Daimon::novo(&iniciais).bloco();
    assert_eq!(b.len(), 46 + iniciais.len() * 4);
    assert_eq!(Daimon::ler(&b).map(|d| d.habilidades.len()), Some(iniciais.len()));
}
