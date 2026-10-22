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

/// B345 — `AddAttributePoint` (`item_elf.cpp:451-475`): acima de 40 cada ponto custa dois; `AddGeniusPoint`
/// (`:496-514`): cada gênio até 8 contando as peças e a soma até o `total_genius`.
#[test]
fn atributos_e_genios_seguem_os_limites_do_original() {
    let mut d = Daimon::novo(&[]);
    d.total_de_atributos = 50;
    d.total_de_genios = 6;
    // 45 em força custa 45 + 45 - 40 = 50: cabe exato; 46 não.
    assert!(!d.somar_atributos([46, 0, 0, 0], true));
    assert!(d.somar_atributos([45, 0, 0, 0], false));
    assert_eq!(d.forca, 45);
    assert!(!d.somar_atributos([0, 0, 0, 0], true), "tudo zero é recusado");
    // Gênio: 3 + 3 = 6 cabe; o sétimo não. Com 7 vindo das peças, só cabe 1 naquele gênio.
    assert!(d.somar_genios([3, 3, 0, 0, 0], [0; 5], false));
    assert!(!d.somar_genios([1, 0, 0, 0, 0], [3, 3, 0, 0, 0], true), "passa do total_genius");
    let mut e = Daimon::novo(&[]);
    e.total_de_genios = 10;
    assert!(!e.somar_genios([0, 0, 2, 0, 0], [0, 0, 7, 0, 0], true), "7 das peças + 2 > 8");
    assert!(e.somar_genios([0, 0, 1, 0, 0], [0, 0, 7, 0, 0], true));
}

/// B345 — `UpdateEquipData`/`EquipElfItem`: um lugar por `equip_type`, nível exigido, e a soma das peças; as vagas
/// de habilidade pelo `ss_table` sobre os pontos sorteados.
#[test]
fn pecas_vagas_e_mascara_de_requisito() {
    use pw_data_loader::manager::EquipamentoDeDaimon;
    use pw_gs::entity::requisito_de_daimon_cumprido;
    let peca = |tipo: i32, nivel: i16| EquipamentoDeDaimon { tipo, nivel_exigido: nivel, atributos: [1, 2, 3, 4], genios: [5, 0, 0, 0, 1] };
    let mut d = Daimon::novo(&[]);
    d.nivel = 10;
    d.total_de_atributos = 10; // 10 - 10 + 1 = 1 sorteado → 4 vagas
    assert!(d.equipar(9001, &peca(0, 10), 0, false));
    assert!(!d.equipar(9002, &peca(0, 1), 0b1, true), "lugar 0 ocupado");
    assert!(!d.equipar(9003, &peca(1, 11), 0b1, true), "nível 11 exigido");
    d.genios = [4, 0, 0, 0, 0];
    let pecas = |id: u32| (id == 9001).then(|| peca(0, 10));
    let a = d.atuais([10, 10, 10, 10], pecas);
    assert_eq!(a.atributos, [11, 12, 13, 14]);
    assert_eq!(a.genios, [8, 0, 0, 0, 1], "4 + 5 = 9, teto 8");
    assert_eq!((a.mascara, a.vagas), (0b1, 4));
    d.total_de_atributos = 70; // 61 sorteados: o primeiro limiar >= 61 é o 70 (índice 1) → 4 + 1 = 5 vagas
    assert_eq!(d.atuais([0; 4], pecas).vagas, 5);
    // A máscara da habilidade 1008 nível 1: 1001111 → Daimon nível 11, gênios 4 ≥ 1, 3 ≥ 1, 0 ≥ 1.
    assert!(requisito_de_daimon_cumprido(1001111, 11, [1, 0, 0, 1, 1]));
    assert!(!requisito_de_daimon_cumprido(1001111, 10, [1, 0, 0, 1, 1]), "nível");
    assert!(!requisito_de_daimon_cumprido(1001111, 11, [0, 0, 0, 1, 1]), "gênio 0");
    // Aprender e esquecer.
    assert_eq!(d.proximo_nivel_de_habilidade(1008, 5), Some(1));
    d.aprender(1008, 1);
    d.aprender(1008, 2);
    assert_eq!(d.habilidades, vec![(1008, 2)]);
    assert_eq!(d.esquecer(1008, 3), None);
    assert_eq!(d.esquecer(1008, 1), Some(1));
    assert_eq!(d.esquecer(1008, 1), Some(0));
    assert!(d.habilidades.is_empty());
}

/// B345 — os dados do `realm_155`: as peças do Daimon (`GOBLIN_EQUIP_ESSENCE`), os atributos iniciais do modelo e os
/// NPCs que ensinam habilidade de Daimon (`id_goblin_skill_service`).
#[test]
fn o_realm_tem_pecas_e_professores_de_daimon() {
    let Some(d) = realm() else { return };
    let g = d.elements_generic.as_ref().expect("elements");
    let pecas: Vec<i32> = g.get("GOBLIN_EQUIP_ESSENCE").iter().filter_map(|r| r.get("ID").and_then(|v| v.as_i32())).collect();
    assert!(!pecas.is_empty());
    let p = d.equipamento_de_daimon(pecas[0] as u32).expect("peça");
    assert!((0..4).contains(&p.tipo), "equip_type 0..3: {p:?}");
    let modelo = g.get("GOBLIN_ESSENCE").iter().filter_map(|r| r.get("ID").and_then(|v| v.as_i32())).next().expect("um Daimon");
    assert!(d.atributos_iniciais_do_daimon(modelo as u32).is_some());
    let professores = d.servicos_de_npc.values().filter(|s| !s.habilidades_de_daimon.is_empty()).count();
    let esquecem = d.servicos_de_npc.values().filter(|s| s.esquecer_habilidade_de_daimon).count();
    eprintln!("realm_155: {} peças de Daimon, {professores} NPCs ensinam, {esquecem} fazem esquecer", pecas.len());
    assert!(professores > 0);
    for s in d.servicos_de_npc.values() {
        for h in &s.habilidades_de_daimon {
            assert_eq!(d.habilidades.get(*h).and_then(|x| x.cls), Some(258), "o NPC ensina ao Daimon a {h}, que não é de Daimon");
        }
    }
}

/// B345 — o `InsertExp` devolve o que consumiu (`old_exp - exp`), que o `ElfPlayerInsertExp` tira do jogador. No teto
/// (Daimon no nível do dono) ele para a um ponto de subir e consome `ceil((levelupexp - 1) / fator)` — antes do B345 o
/// nosso não descontava isso e dizia que nada tinha entrado.
#[test]
fn injetar_exp_conta_o_consumido_ate_no_teto() {
    let precisa = |_nivel: i16| 100u32;
    let mut d = DaimonVestido { slot: 23, item_id: 23752, fator_de_exp: 1.0, estado: Daimon::novo(&[]), sujo: false };
    // Dono de nível 1: o fator é 1 e o Daimon para em 99 de 100.
    let (consumida, subiu) = d.inserir_exp(500, 1, 1, precisa);
    assert_eq!((consumida, subiu, d.estado.exp, d.estado.nivel), (99, false, 99, 1));
    // De novo, já no teto: o original recusa (`GetLvlupExp <= exp + 1`).
    assert_eq!(d.inserir_exp(500, 1, 1, precisa), (0, false));
}
