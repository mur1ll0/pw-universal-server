//! Um golpe de jogador contra monstros reais do realm, pela fórmula portada.
//!
//! `cargo run -p pw-gs --example simula_golpe -- data/realm_155/config`
use pw_data_loader::{aipolicy::AiPolicyData, classes, generic_elements::load_elements_data_auto, monstros};
use pw_gs::combat::{
    chance_de_acerto, reducao_por_defesa, resolver, Defesa, Golpe, Resultado, Rolagens,
    CLASSES_MAGICAS,
};

fn main() {
    let dir = std::env::args().nth(1).expect("uso: simula_golpe <pasta config>");
    let el = load_elements_data_auto(&std::fs::read(format!("{dir}/elements.data")).unwrap()).unwrap();
    let pol = AiPolicyData::load_from_bytes(&std::fs::read(format!("{dir}/aipolicy.data")).unwrap()).unwrap();
    let mobs = monstros::carregar(&el, Some(&pol));
    let cls = classes::carregar(&el);

    let guerreiro = cls.get(0).expect("classe 0");
    let precisao = guerreiro.precisao_base(40);
    let evasao = guerreiro.evasao_base(40);
    println!("guerreiro nv50, agi 40 -> precisão {precisao}, evasão {evasao}\n");
    println!(
        "{:<24} {:>3} {:>8} {:>7} {:>5} {:>8} {:>6} {:>8}",
        "monstro", "nv", "vida", "defesa", "arm", "acerto", "dano", "golpes"
    );

    for id in [986u32, 987, 988, 989, 990, 16] {
        let Some(m) = mobs.get(id) else { continue };
        let golpe = Golpe {
            nivel_do_atacante: 50,
            taxa_de_ataque: precisao,
            dano_fisico: 350,
            dano_magico: [0; CLASSES_MAGICAS],
            e_fisico: true,
            chance_de_critico: 5,
            bonus_de_dano_critico: 0,
            grau_de_ataque: 0,
            de_habilidade: false,
            fator_de_curta_distancia: 1.0,
            anti_defesa: 0,
            anti_resistencia: 0,
            atacante_e_jogador_ou_pet: true,
        };
        let def = Defesa::simples(m.armadura, m.defesa, m.resistencias, m.grau_de_defesa);
        let chance = chance_de_acerto(precisao, m.armadura);
        let dano = match resolver(&golpe, &def, 3.0, false, Rolagens { acerto: 0.0, critico: 99 }) {
            Resultado::Acertou { dano, .. } => dano,
            _ => 0,
        };
        println!(
            "{:<24} {:>3} {:>8} {:>7} {:>5} {:>7.1}% {:>6} {:>8}   (defesa corta {:.1}%)",
            m.nome, m.nivel, m.vida, m.defesa, m.armadura, chance * 100.0, dano,
            if dano > 0 { (m.vida + dano - 1) / dano } else { 0 },
            reducao_por_defesa(m.defesa, 50) * 100.0
        );
    }
}
