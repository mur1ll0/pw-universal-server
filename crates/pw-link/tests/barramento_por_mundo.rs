//! `GS_BUS` com um servidor de mundo por mapa.

use pw_link::gateway::LinkGateway;

#[test]
fn um_endereco_so_e_o_padrao() {
    assert_eq!(
        LinkGateway::ler_barramentos("pw-world-155:29100"),
        vec![(None, "pw-world-155:29100".to_string())]
    );
}

#[test]
fn a_lista_separa_mundo_e_endereco() {
    assert_eq!(
        LinkGateway::ler_barramentos(" 1=pw-world-155:29100, 161=pw-world-155-161:29100 "),
        vec![
            (Some(1), "pw-world-155:29100".to_string()),
            (Some(161), "pw-world-155-161:29100".to_string()),
        ]
    );
}
