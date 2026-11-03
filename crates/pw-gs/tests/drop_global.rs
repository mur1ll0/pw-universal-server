//! O drop global (`extra_drops.sev`) entra antes do drop do `drop_matters`, como em `gnpc_imp::DropItem`
//! (`cgame/gs/npc.cpp:2721-2725`): tabela "soma" deixa o drop normal acontecer, "substitui" corta.
//!
//! Usa o `elements.data` e o `extra_drops.sev` do `realm_126`; sem os arquivos no disco, não verifica nada.

use pw_data_loader::drop_global::{DropGlobal, TabelaDeDropGlobal, TipoDeDropGlobal};
use pw_data_loader::generic_elements::load_elements_data_auto;
use pw_data_loader::progressao::TabelaDeProgressao;
use pw_data_loader::GameDataManager;
use pw_gs::economia::gerar_queda;
use pw_gs::taxas::Taxas;
use rand::{rngs::StdRng, SeedableRng};
use std::path::PathBuf;

fn dados_126() -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    let e = load_elements_data_auto(&std::fs::read(dir.join("elements.data")).ok()?).unwrap();
    let mut d = GameDataManager::default();
    d.monstros = pw_data_loader::monstros::carregar(&e, None);
    d.progressao = TabelaDeProgressao::neutra_para_teste();
    d.drop_global = DropGlobal::ler(&std::fs::read(dir.join("extra_drops.sev")).unwrap()).unwrap();
    Some(d)
}

const MORTES: usize = 40_000;

#[test]
fn tabela_soma_cai_junto_com_o_drop_normal() {
    let Some(d) = dados_126() else { return };
    // Um monstro da 1ª tabela (níveis 1-4, tipo 2) que também tenha `drop_matters`.
    let t = &d.drop_global.tabelas[0];
    assert_eq!(t.tipo, TipoDeDropGlobal::Soma);
    let m = t
        .monstros
        .iter()
        .filter_map(|id| d.monstros.get(*id))
        .find(|m| m.rodadas_de_drop > 0 && m.itens_de_drop.iter().any(|(i, c)| *i != 0 && *c > 0.0) && !m.itens_de_drop.iter().any(|(i, _)| t.itens.iter().any(|(g, _)| g == i)))
        .expect("monstro da tabela 0 com drop próprio")
        .clone();
    let da_tabela = |id: u32| t.itens.iter().any(|(g, _)| *g == id);
    let mut rng = StdRng::seed_from_u64(126);
    let (mut globais, mut normais) = (0usize, 0usize);
    for _ in 0..MORTES {
        for id in gerar_queda(&m, m.nivel, &d, &Taxas::default(), &mut rng).itens {
            if da_tabela(id) {
                globais += 1;
            } else {
                normais += 1;
            }
        }
    }
    // Itens por morte = Σ k·P(k) da tabela (0,033 na tabela 0 do 1.2.6).
    let esperado: f64 = t.chance_de_quantos.iter().enumerate().map(|(k, p)| k as f64 * *p as f64).sum::<f64>() * MORTES as f64;
    let desvio = (globais as f64 - esperado).abs() / esperado;
    assert!(desvio < 0.1, "globais {globais}, esperado {esperado:.0}");
    assert!(normais > 0, "o drop do drop_matters tem de continuar (tabela tipo 2)");
}

#[test]
fn tabela_substitui_corta_o_drop_normal() {
    let Some(mut d) = dados_126() else { return };
    let m = d
        .monstros
        .templates
        .values()
        .find(|m| m.rodadas_de_drop > 0 && m.itens_de_drop.iter().any(|(i, c)| *i != 0 && *c > 0.5) && m.dinheiro_medio > m.dinheiro_variacao)
        .expect("monstro com drop próprio e dinheiro")
        .clone();
    d.drop_global = DropGlobal::de_tabelas(vec![TabelaDeDropGlobal {
        indice: 0,
        nome: "teste".into(),
        tipo: TipoDeDropGlobal::Substitui,
        monstros: vec![m.id],
        itens: vec![(999_999, 1.0)],
        chance_de_quantos: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    }]);
    let mut rng = StdRng::seed_from_u64(155);
    let mut com_dinheiro = 0usize;
    for _ in 0..2_000 {
        let q = gerar_queda(&m, m.nivel, &d, &Taxas::default(), &mut rng);
        assert_eq!(q.itens, [999_999, 999_999]);
        assert!(q.montes_de_dinheiro.len() <= 1, "um sorteio de dinheiro só: {:?}", q.montes_de_dinheiro);
        com_dinheiro += q.montes_de_dinheiro.len();
    }
    // `MONEY_DROP_RATE` = 0,7 (`config.h:82`).
    assert!((1_250..1_550).contains(&com_dinheiro), "com dinheiro {com_dinheiro} de 2000");
}
