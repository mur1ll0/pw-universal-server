//! B346 — as Cartas de General vestidas: liderança ocupada, atributos somados e o devorar, contra os dados do
//! `realm_155` (`POKER_ESSENCE`, `POKER_LEVELEXP_CONFIG`, `CHARRACTER_CLASS_CONFIG.poker_adjust`).

use pw_data_loader::cartas_de_general::EssenciaDaCarta;
use pw_data_loader::GameDataManager;
use pw_gs::cartas_de_general::{cartas_ativas, exp_a_devorar, pode_vestir, somar_cartas, ERRO_EXP_DA_CARTA};
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

fn item(slot: u16, tid: u32, ess: EssenciaDaCarta) -> pw_core::ItemRecord {
    pw_core::ItemRecord {
        id: None,
        character_id: 1,
        container_type: pw_core::ContainerType::Equipment,
        slot,
        item_id: tid,
        count: 1,
        max_count: 1,
        refine_level: 0,
        sockets_count: 0,
        sockets: vec![],
        durability: 0,
        max_durability: 0,
        bind_status: 0,
        octets: ess.octetos(),
        custom_attributes: serde_json::Value::Null,
    }
}

#[test]
fn cartas_do_realm_lideranca_e_atributos() {
    let Some(d) = realm() else { return };
    let c = &d.cartas_de_general;
    assert_eq!(c.curva.len(), 101, "o marcador 0 e exp_1..100");
    assert_eq!(c.ajuste_da_qualidade.len(), 5);
    assert!(c.ajuste_da_classe.len() >= 12, "as classes do CHARRACTER_CLASS_CONFIG: {}", c.ajuste_da_classe.len());
    // Uma carta do lugar 0 de qualidade baixa, com atributos.
    let (&tid, carta) = c.cartas.iter().filter(|(_, x)| x.tipo == 0 && x.base.dano > 0).min_by_key(|(id, _)| **id).expect("uma carta de dano");
    let ess = EssenciaDaCarta { tipo: 0, qualidade: carta.qualidade, nivel_exigido: 1, lideranca_exigida: 10, nivel_maximo: carta.nivel_maximo, nivel: 1, exp: 0, renascimentos: 0 };
    // Duas cartas que pedem 10 de liderança, com 15: só a primeira (slot 32) ativa.
    let itens = vec![item(33, tid, EssenciaDaCarta { tipo: 1, ..ess }), item(32, tid, ess)];
    let (ativas, ocupada) = cartas_ativas(&itens, &d, 100, 15);
    assert_eq!((ativas.len(), ativas[0].slot, ocupada), (1, 32, 10));
    assert!(!pode_vestir(&itens, &d, &ess.octetos(), 34, 100, 15), "5 livres, pede 10");
    assert!(pode_vestir(&itens, &d, &ess.octetos(), 32, 100, 15), "trocando a do 32, os 10 dela voltam");
    assert!(!pode_vestir(&itens, &d, &EssenciaDaCarta { nivel_exigido: 101, ..ess }.octetos(), 32, 100, 99), "nível");
    // No lugar do tipo dela (coluna 0), a carta soma o dano × o ajuste da classe 0.
    let mut e = pw_gs::entity::Equipamento::default();
    somar_cartas(&mut e, &ativas, &d, 0);
    let esperado = (carta.base.dano as f32 * c.ajuste(0, 0) + 0.5) as i32;
    assert_eq!(e.dano, esperado, "ajuste da classe 0 na coluna 0: {}", c.ajuste(0, 0));
    eprintln!("carta {tid}: dano base {} × ajuste {} = {}", carta.base.dano, c.ajuste(0, 0), e.dano);
    // Devorar a própria carta (mesma qualidade): a experiência do `swallow_exp` + a acumulada.
    let vestida = EssenciaDaCarta { nivel: 1, ..ess };
    let r = exp_a_devorar(&d, &vestida, tid, &ess.octetos(), 2);
    eprintln!("carta {tid}: qualidade {}, swallow_exp {}", carta.qualidade, carta.exp_ao_devorar);
    // `GENERALCARD_RANK_S` (3) ou acima não é devorada (`player.cpp:24727-24733`).
    if carta.exp_ao_devorar > 0 && vestida.nivel < vestida.nivel_maximo && carta.qualidade < 3 {
        assert_eq!(r, Ok(carta.exp_ao_devorar * 2));
    } else {
        assert_eq!(r, Err(ERRO_EXP_DA_CARTA));
    }
    // Quantas missões dão liderança (`m_iLeaderShip`, deslocamento 261).
    let com_lideranca = d.tasks.tasks.values().filter(|t| t.rewards.lideranca != 0).count();
    let mut valores: Vec<(u32, i32)> = d.tasks.tasks.values().filter(|t| t.rewards.lideranca != 0).map(|t| (t.id, t.rewards.lideranca)).collect();
    valores.sort();
    eprintln!("realm_155: {} cartas, {com_lideranca} missões dão liderança: {valores:?}", c.cartas.len());
    assert!(valores.iter().all(|(_, v)| (1..=1000).contains(v)), "liderança fora do plausível: {valores:?}");
    assert!(com_lideranca > 0, "nenhuma missão dá liderança — o deslocamento 261 está errado?");
}
