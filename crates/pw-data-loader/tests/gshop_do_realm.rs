//! O `gshop.data` dos realms, lido pelo layout que fecha o arquivo no último byte.
//!
//! Números medidos nos arquivos (`docs/evidencias/LOJA_GOLD_DIAGNOSTICO.md`).

use pw_data_loader::{FormatoDoGshop, GShopData};
use std::path::PathBuf;

fn ler(realm: &str, nome: &str) -> Option<GShopData> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config").join(nome);
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    Some(GShopData::load_from_bytes(&bytes).expect("o carimbo sempre se lê"))
}

#[test]
fn o_gshop_do_155_fecha_com_o_layout_vip() {
    let Some(g) = ler("realm_155", "gshop.data") else { return };
    assert_eq!(g.formato, Some(FormatoDoGshop::V155Vip));
    assert_eq!(g.timestamp, 1_478_590_602);
    assert_eq!(g.ofertas.len(), 1726);
    // Índice 489: "Casaco da União", item 4276, 30000 de cash na primeira opção.
    let o = &g.ofertas[489];
    assert_eq!((o.item_id, o.quantidade, o.opcoes[0].preco), (4276, 1, 30000));
    assert!(o.nome.starts_with("Casaco da Uni"), "{}", o.nome);
    // Nenhuma opção com período de venda nem grupo; 55 ofertas com brinde; nenhuma com dono.
    let opcoes: Vec<_> = g.ofertas.iter().flat_map(|o| o.opcoes.iter()).filter(|c| c.preco > 0).collect();
    assert_eq!(opcoes.len(), 975);
    assert!(opcoes.iter().all(|c| c.tipo_de_periodo == -1 && c.grupo == 0));
    assert_eq!(opcoes.iter().filter(|c| c.vip_minimo > 0).count(), 38);
    assert_eq!(g.ofertas.iter().filter(|o| o.brinde_id > 0).count(), 55);
    assert!(g.ofertas.iter().all(|o| o.donos == [0; 8]));
    assert_eq!(g.ofertas.iter().filter(|o| o.modo_do_limite != 0).count(), 20);

    let g1 = ler("realm_155", "gshop1.data").unwrap();
    assert_eq!((g1.formato, g1.ofertas.len()), (Some(FormatoDoGshop::V155Vip), 205));
}

#[test]
fn o_gshop_do_126_fecha_com_o_layout_de_12_bytes_por_opcao() {
    let Some(g) = ler("realm_126", "gshop.data") else { return };
    assert_eq!(g.formato, Some(FormatoDoGshop::V126));
    assert_eq!(g.timestamp, 1_206_433_535);
    assert_eq!(g.ofertas.len(), 668);
    let o = &g.ofertas[0];
    assert_eq!((o.item_id, o.quantidade, o.opcoes[0].preco), (4276, 1, 10));
    // Uma opção só em todas as ofertas; 3 com validade de 7 dias; nenhuma data absoluta.
    assert!(g.ofertas.iter().all(|o| o.opcoes[0].preco > 0 && o.opcoes[1..].iter().all(|c| c.preco == 0)));
    assert_eq!(g.ofertas.iter().filter(|o| o.opcoes[0].validade_s == 604_800).count(), 3);
    assert!(g.ofertas.iter().all(|o| o.opcoes[0].validade_absoluta == 0 && o.brinde_id == 0));
}

#[test]
fn o_gshopsev_do_servidor_so_da_o_carimbo() {
    let Some(g) = ler("realm_155", "gshopsev.data") else { return };
    assert_eq!(g.timestamp, 1_682_369_053);
    assert_eq!(g.formato, None);
    assert!(g.ofertas.is_empty());
}
