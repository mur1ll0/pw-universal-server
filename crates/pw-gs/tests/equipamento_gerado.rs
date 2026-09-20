//! B53 — equipamento sorteado no drop contra o `elements.data` real do `realm_155`:
//! o conteúdo se relê, fecha no último byte, e as propriedades adicionais entram nos
//! atributos. Sem a pasta do realm o teste avisa e não verifica nada.

use pw_core::{ContainerType, ConteudoDeEquipamento, ItemRecord};
use pw_data_loader::GameDataManager;
use pw_gs::entity::{BonusDeAddons, Equipamento};
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

#[test]
fn o_drop_sorteia_addons_que_se_releem_e_somam_nos_atributos() {
    let Some(d) = realm() else { return };
    assert!(d.addons.por_id.len() > 1000, "addons carregados: {}", d.addons.por_id.len());
    assert!(!d.geracao.is_empty());

    let mut com_addons = 0;
    let mut somados = 0;
    let mut sem_porte = 0;
    let mut ids: Vec<u32> = d.geracao.keys().copied().collect();
    ids.sort();
    for tid in ids.iter().take(3000) {
        for _ in 0..3 {
            let Some(c) = pw_gs::geracao::gerar_equipamento(&d, *tid) else { continue };
            let b = c.escrever();
            let modelo = d.equipamentos.ficha(*tid).unwrap();
            let lido = ConteudoDeEquipamento::ler(&b, &modelo).unwrap_or_else(|| panic!("o conteúdo de {tid} não se relê"));
            assert_eq!(lido, c);
            if c.addons.is_empty() {
                continue;
            }
            com_addons += 1;
            let mut item = ItemRecord::new(1, ContainerType::Equipment, 1, *tid, 1);
            item.octets = b;
            let e = Equipamento::dos_itens_com_addons(&[item], &d.equipamentos, Some(&d.addons));
            sem_porte += e.addons_sem_porte.len();
            if e.addons != BonusDeAddons::default() {
                somados += 1;
            }
        }
    }
    eprintln!("{com_addons} peças com addons, {somados} com bônus somado, {sem_porte} addons sem porte");
    assert!(com_addons > 50, "quase nada saiu com addon: {com_addons}");
    assert!(somados > com_addons / 3, "os addons não entraram nos atributos: {somados} de {com_addons}");
}

#[test]
fn o_refino_soma_o_valor_gravado_no_addon() {
    let mut b = BonusDeAddons::default();
    assert!(b.somar("refine_damage", &[24, 3]));
    assert!(b.somar("refine_magic_damage", &[10, 1]));
    assert!(b.somar("enhance_str_addon", &[4]));
    assert!(b.somar("IA_EA_ESS<offsetof(armor_essence,defense)>", &[9]));
    assert!(!b.somar("item_skill_addon", &[1, 2]));
    assert_eq!((b.dano, b.dano_magico, b.forca, b.defesa), (34, 10, 4, 0));
}

/// B60 — o item que a missão dá entra na bolsa **gerado**, com bloco de dados e propriedades.
///
/// No original, todo item de prêmio de missão passa por `generate_item_for_drop`
/// (`DeliverCommonItem`, `task/taskman.cpp:281-303`) — a mesma geração do drop de monstro.
/// Nós criávamos o item seco: o cliente mostrava a faixa do modelo no tooltip
/// ("Destreza +1~2") e o servidor não somava nada (relato do set Halo, 2026-09-18).
#[test]
fn o_premio_de_missao_entra_na_bolsa_com_o_equipamento_gerado() {
    let Some(d) = realm() else { return };
    // Peitoral do set Halo, o do relato.
    const PEITORAL_HALO: u32 = 28790;
    assert!(d.geracao.contains_key(&PEITORAL_HALO), "o 28790 devia ter modelo de geração");

    let mut bolsa = pw_gs::economia::Bolsa::nova(1, ContainerType::Inventory, 32, Vec::new());
    let e = bolsa.empilhar_gerado(PEITORAL_HALO, 1, &d).expect("cabe na bolsa");
    let item = bolsa.slots[e.slot].clone().expect("o slot ficou com o item");
    assert_eq!(item.item_id, PEITORAL_HALO);
    assert!(!item.octets.is_empty(), "o item entrou sem bloco de dados");
    assert!(item.durability > 0, "sem durabilidade de fábrica");

    // O bloco se relê, e o que ele traz soma nos atributos do jogador.
    let ficha = d.equipamentos.ficha(PEITORAL_HALO).expect("ficha do 28790");
    let c = ConteudoDeEquipamento::ler(&item.octets, &ficha).expect("o bloco tem de se reler");
    assert!(!c.addons.is_empty(), "o peitoral saiu sem propriedade adicional");
    let eq = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert_ne!(eq.addons, BonusDeAddons::default(), "as propriedades não somaram nos atributos");
}

/// B61 — a durabilidade sai na escala interna, que é a que o cliente divide por 100.
///
/// O gerador do original multiplica os dois números pelo `DURABILITY_UNIT_COUNT` no fim
/// (`update_require_data`, `gs/item/item_addon.h:454-458`, chamado em
/// `generate_item_temp.h:367`), e o cliente divide arredondando para cima
/// (`EC_IvtrEquip.cpp:281`). Sem a multiplicação, o ★Arco Real (50 no `elements.data`)
/// aparecia como **1/1** em jogo (relato de 2026-09-18).
#[test]
fn a_durabilidade_gerada_vai_na_escala_interna() {
    let Some(d) = realm() else { return };
    const ARCO_REAL: u32 = 36121;
    let de_fabrica = d.durabilidade_de_fabrica(ARCO_REAL).expect("o arco tem durabilidade") as i32;
    assert_eq!(de_fabrica, 50, "o elements do realm_155 mudou");

    let c = pw_gs::geracao::gerar_equipamento(&d, ARCO_REAL).expect("o arco se gera");
    assert_eq!(c.durabilidade_maxima, de_fabrica * pw_core::ESCALA_DA_DURABILIDADE);
    assert_eq!(c.durabilidade, c.durabilidade_maxima, "durability_drop == durability no 36121");
    // O que o cliente mostra: `(v + 99) / 100`.
    let na_tela = (c.durabilidade + pw_core::ESCALA_DA_DURABILIDADE - 1) / pw_core::ESCALA_DA_DURABILIDADE;
    assert_eq!(na_tela, 50, "o cliente mostraria {na_tela}/50");

    // E o bloco gravado carrega o mesmo número, no lugar onde o cliente o lê.
    let b = c.escrever();
    let ficha = d.equipamentos.ficha(ARCO_REAL).expect("ficha do arco");
    assert_eq!(ConteudoDeEquipamento::ler(&b, &ficha).expect("relê").durabilidade, c.durabilidade);
}

/// B61 — peça acabada não conta: `equip_item::VerifyRequirement` exige `durability > 0`
/// (`gs/item/equip_item.cpp:60-80`).
#[test]
fn a_peca_com_durabilidade_zerada_nao_entra_nos_atributos() {
    let Some(d) = realm() else { return };
    const PEITORAL_HALO: u32 = 28790;
    let mut bolsa = pw_gs::economia::Bolsa::nova(1, ContainerType::Equipment, 32, Vec::new());
    let e = bolsa.empilhar_gerado(PEITORAL_HALO, 1, &d).expect("cabe");
    let mut item = bolsa.slots[e.slot].clone().expect("o slot ficou com o item");
    item.slot = 4;

    let inteiro = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert!(inteiro.defesa > 0, "o peitoral inteiro devia dar defesa");

    item.durability = 0;
    let quebrado = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert_eq!(quebrado.defesa, 0, "peça acabada ainda somava defesa");
    assert_eq!(quebrado.addons, BonusDeAddons::default(), "peça acabada ainda somava propriedade");
}

/// B67 — o amuleto de vida e o hierograma de mana têm conteúdo próprio: 8 bytes.
///
/// `amulet_essence { int point; float trigger_percent; }` (`gs/item/item_amulet.h:16-19`),
/// escrito sem cabeçalho de requisito (`generate_item_temp.h:2296-2310`). Sem ele o cliente
/// mostrava o item zerado (relato de 2026-09-19).
#[test]
fn o_amuleto_e_o_hierograma_saem_com_os_oito_bytes_da_essencia() {
    let Some(d) = realm() else { return };
    const AMULETO: u32 = 35370; // Amuleto do Guardião - 1 (AUTOHP_ESSENCE)
    const HIEROGRAMA: u32 = 35376; // Hierograma do Guardião - 1 (AUTOMP_ESSENCE)

    let a = d.conteudo_do_amuleto(AMULETO).expect("o 35370 é um AUTOHP_ESSENCE");
    assert_eq!(a.len(), 8, "o conteúdo do amuleto tem 8 bytes");
    assert_eq!(i32::from_le_bytes([a[0], a[1], a[2], a[3]]), 5400, "total_hp do elements");
    assert!((f32::from_le_bytes([a[4], a[5], a[6], a[7]]) - 0.5).abs() < 1e-6, "trigger_amount");

    let m = d.conteudo_do_amuleto(HIEROGRAMA).expect("o 35376 é um AUTOMP_ESSENCE");
    assert_eq!(i32::from_le_bytes([m[0], m[1], m[2], m[3]]), 18000, "total_mp do elements");
    assert!((f32::from_le_bytes([m[4], m[5], m[6], m[7]]) - 0.75).abs() < 1e-6);

    // E um item qualquer não é amuleto.
    assert!(d.conteudo_do_amuleto(1796).is_none(), "poção não tem essência de amuleto");
}

/// B67 — a poção restaura ao longo do tempo, e o `elements.data` diz em quanto.
#[test]
fn a_pocao_de_vida_traz_o_total_e_o_tempo() {
    let Some(d) = realm() else { return };
    // Poção Pequena de Cura: 25 de vida em 10 s, recarga de 15 s.
    let (hp, hp_s, mp, mp_s, recarga) = d.quanto_o_remedio_restaura_no_tempo(1796).expect("1796 é remédio");
    assert_eq!((hp, hp_s), (25, 10), "hp_add_total/hp_add_time do elements");
    assert_eq!((mp, mp_s), (0, 0));
    assert_eq!(recarga, 15000, "cool_time");
}
