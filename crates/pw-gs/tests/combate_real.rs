//! As fórmulas de dano, conferidas contra o C++ original.
//!
//! Os números esperados aqui foram calculados **à mão a partir do fonte**
//! (`actobject.cpp::AttackJudgement`, o trecho seguinte de `HandleAttackMsg`, e os
//! auxiliares de `playertemplate.h`), não observados da nossa implementação. Um teste que
//! só registra o que o código faz hoje não pega porte errado.
//!
//! Nada aqui sorteia: [`resolver`] recebe as rolagens prontas, então cada caso é um
//! número fechado.

use pw_gs::combat::{
    chance_de_acerto, defesa_apos_penetracao, fator_de_reducao_longe, fator_de_reducao_perto,
    reducao_por_defesa, resolver, Defesa, Golpe, Resultado, Rolagens, CLASSES_MAGICAS,
    IMUNE_A_FISICO,
};

/// Um golpe físico simples, sem crítico nem penetração, para os testes mexerem num campo
/// de cada vez.
fn golpe() -> Golpe {
    Golpe {
        nivel_do_atacante: 50,
        taxa_de_ataque: 1000,
        dano_fisico: 1000,
        dano_magico: [0; CLASSES_MAGICAS],
        e_fisico: true,
        chance_de_critico: 0,
        bonus_de_dano_critico: 0,
        grau_de_ataque: 0,
        de_habilidade: false,
        fator_de_curta_distancia: 1.0,
        anti_defesa: 0,
        anti_resistencia: 0,
        atacante_e_jogador_ou_pet: false,
    }
}

fn defesa() -> Defesa {
    Defesa::simples(0, 0, [0; CLASSES_MAGICAS], 0)
}

/// Rolagens que sempre acertam e nunca dão crítico.
fn certeiro() -> Rolagens {
    Rolagens { acerto: 0.0, critico: 99 }
}

// ---------------------------------------------------------------------------------
// Os auxiliares, isolados
// ---------------------------------------------------------------------------------

#[test]
fn a_chance_de_acerto_e_taxa_sobre_taxa_mais_meia_armadura() {
    // `attack_pb = rate / (rate + (armor >> 1))`, com piso de 0,05 e **sem teto** — a
    // linha do teto de 0,95 está comentada no original.
    assert_eq!(chance_de_acerto(1000, 0), 1.0, "sem armadura o golpe sempre acerta");
    // 1000 / (1000 + 500) = 0,666…
    assert!((chance_de_acerto(1000, 1000) - 2.0 / 3.0).abs() < 1e-6);
    // O deslocamento é inteiro: 999>>1 = 499, não 499,5.
    assert!((chance_de_acerto(1000, 999) - 1000.0 / 1499.0).abs() < 1e-6);
    // Piso: 1 / (1 + 5000) seria 0,0002.
    assert_eq!(chance_de_acerto(1, 10_000), 0.05);
}

#[test]
fn a_reducao_por_defesa_usa_o_nivel_de_quem_ataca() {
    // `def / (def + 40*attacker_level - 25)`, teto de 0,95.
    // Nível 50, defesa 1975: 1975 / (1975 + 2000 - 25) = 1975/3950 = 0,5.
    assert!((reducao_por_defesa(1975, 50) - 0.5).abs() < 1e-6);
    // A mesma defesa contra um atacante de nível 100 segura menos:
    // 1975 / (1975 + 4000 - 25) = 1975/5950 ≈ 0,3319.
    let alto = reducao_por_defesa(1975, 100);
    assert!(alto < 0.34 && alto > 0.33, "reduziu {alto}");
    assert!(alto < reducao_por_defesa(1975, 50), "nível maior tem de reduzir menos");
    // Teto.
    assert_eq!(reducao_por_defesa(10_000_000, 1), 0.95);
    // Sem defesa nenhuma não há redução.
    assert_eq!(reducao_por_defesa(0, 50), 0.0);
}

#[test]
fn a_penetracao_tem_teto_de_35_por_cento() {
    // `anti_ratio = anti/(anti+10000)`, limitado a 0,35.
    assert_eq!(defesa_apos_penetracao(0, 1000), 1000, "sem penetração a defesa fica inteira");
    // 10000/(10000+10000) = 0,5 → limitado a 0,35 → 1000*0,65 = 650.
    assert_eq!(defesa_apos_penetracao(10_000, 1000), 650);
    // Muito acima do teto continua em 0,35.
    assert_eq!(defesa_apos_penetracao(1_000_000, 1000), 650);
    // Abaixo do teto: 1000/(11000) ≈ 0,0909 → 1000*0,909 = 909.
    assert_eq!(defesa_apos_penetracao(1000, 1000), 909);
}

#[test]
fn a_atenuacao_por_distancia_tem_os_limiares_do_original() {
    // Longe: nada abaixo de 8 m; acima disso `ratio * (d-8)/(d+2)`, com d limitado a 40.
    assert_eq!(fator_de_reducao_longe(8.0, 1.0), 0.0);
    assert!((fator_de_reducao_longe(18.0, 1.0) - 10.0 / 20.0).abs() < 1e-6);
    // Acima de 40 m o valor congela no de 40: (40-8)/(40+2) = 32/42.
    let em_40 = fator_de_reducao_longe(40.0, 1.0);
    assert!((fator_de_reducao_longe(100.0, 1.0) - em_40).abs() < 1e-6);

    // Perto: nada a partir de 8 m; abaixo disso é a razão inteira, limitada a 1.
    assert_eq!(fator_de_reducao_perto(8.0, 1.0), 0.0);
    assert_eq!(fator_de_reducao_perto(2.0, 0.3), 0.3);
    assert_eq!(fator_de_reducao_perto(2.0, 5.0), 1.0);
}

// ---------------------------------------------------------------------------------
// O golpe inteiro
// ---------------------------------------------------------------------------------

#[test]
fn errar_a_rolagem_de_acerto_encerra_o_golpe() {
    let mut d = defesa();
    d.armadura = 1000; // chance = 1000/1500 = 0,666…
    // A rolagem erra quando é MAIOR que a chance.
    assert_eq!(resolver(&golpe(), &d, 5.0, false, Rolagens { acerto: 0.7, critico: 0 }), Resultado::Errou);
    assert!(matches!(
        resolver(&golpe(), &d, 5.0, false, Rolagens { acerto: 0.6, critico: 99 }),
        Resultado::Acertou { .. }
    ));
}

#[test]
fn o_ataque_magico_nao_rola_acerto() {
    // `AttackJudgement` só testa acerto quando `attack_attr == PHYSIC_ATTACK`.
    let mut g = golpe();
    g.e_fisico = false;
    g.dano_fisico = 0;
    g.dano_magico[0] = 1000;
    let mut d = defesa();
    d.armadura = 1_000_000; // acerto impossível, se fosse testado

    let r = resolver(&g, &d, 5.0, false, Rolagens { acerto: 1.0, critico: 99 });
    assert!(matches!(r, Resultado::Acertou { .. }), "golpe mágico não deveria errar: {r:?}");
}

#[test]
fn o_dano_e_reduzido_pela_defesa_do_alvo() {
    // Dano 1000, defesa 1975, atacante nível 50 → redução 0,5 → 500.
    // Sem crítico e com graus iguais: (500 + 0,5) * 1,0 = 500.
    let mut d = defesa();
    d.defesa = 1975;
    assert_eq!(resolver(&golpe(), &d, 5.0, false, certeiro()).dano(), 500);
}

#[test]
fn cada_classe_magica_e_reduzida_pela_sua_resistencia() {
    // Duas classes com dano igual e resistências diferentes têm de sair diferentes.
    let mut g = golpe();
    g.dano_fisico = 0;
    g.dano_magico[0] = 1000; // metal
    g.dano_magico[3] = 1000; // fogo

    let mut d = defesa();
    d.resistencias[0] = 1975; // metade
    d.resistencias[3] = 0; // inteiro

    // 500 + 1000 = 1500, mais o 0,5 do arredondamento.
    assert_eq!(resolver(&g, &d, 5.0, false, certeiro()).dano(), 1500);
}

#[test]
fn a_imunidade_zera_a_classe_e_deixa_as_outras_passarem() {
    let mut g = golpe();
    g.dano_fisico = 1000;
    g.dano_magico[0] = 1000;

    let mut d = defesa();
    d.imunidades = IMUNE_A_FISICO;

    let r = resolver(&g, &d, 5.0, false, certeiro());
    assert_eq!(r.dano(), 1000, "só o mágico deveria ter passado");
    assert!(
        matches!(r, Resultado::Acertou { alguma_imunidade: true, .. }),
        "a imunidade tem de ser sinalizada: {r:?}"
    );

    // Imune a metal (bit 1), não ao físico.
    let mut d2 = defesa();
    d2.imunidades = 1 << 1;
    assert_eq!(resolver(&g, &d2, 5.0, false, certeiro()).dano(), 1000);
}

#[test]
fn imune_a_tudo_e_sem_efeito_e_nao_acerto() {
    let mut g = golpe();
    g.dano_magico[0] = 1000;
    let mut d = defesa();
    d.imunidades = IMUNE_A_FISICO | (1 << 1);
    assert_eq!(resolver(&g, &d, 5.0, false, certeiro()), Resultado::SemEfeito);
}

#[test]
fn golpe_sem_dano_nenhum_e_sem_efeito() {
    // `attacked` fica false quando nenhuma classe tinha dano — o original trata como
    // esquiva.
    let mut g = golpe();
    g.dano_fisico = 0;
    assert_eq!(resolver(&g, &defesa(), 5.0, false, certeiro()), Resultado::SemEfeito);
}

#[test]
fn o_critico_dobra_e_o_bonus_soma_por_cima() {
    // `damage_adjust *= CRIT_DAMAGE_BONUS + crit_damage_bonus*0,01 - crit_damage_reduce*0,01`
    let mut g = golpe();
    g.chance_de_critico = 50;

    // Rolagem 49 < 50 → crítico. 1000 * 2,0 = 2000 (mais o 0,5 do arredondamento).
    let r = resolver(&g, &defesa(), 5.0, false, Rolagens { acerto: 0.0, critico: 49 });
    assert_eq!(r.dano(), 2000);
    assert!(r.foi_critico());

    // Rolagem 50 não é menor que 50 → sem crítico.
    let r = resolver(&g, &defesa(), 5.0, false, Rolagens { acerto: 0.0, critico: 50 });
    assert_eq!(r.dano(), 1000);
    assert!(!r.foi_critico());

    // Bônus de 50% → 2,5×.
    g.bonus_de_dano_critico = 50;
    let r = resolver(&g, &defesa(), 5.0, false, Rolagens { acerto: 0.0, critico: 0 });
    assert_eq!(r.dano(), 2500);

    // A resistência a crítico do alvo desconta da chance: 50 - 50 = 0, nada é crítico.
    let mut d = defesa();
    d.resistencia_a_critico = 50;
    let r = resolver(&g, &d, 5.0, false, Rolagens { acerto: 0.0, critico: 0 });
    assert!(!r.foi_critico(), "resistência deveria ter anulado a chance");
}

#[test]
fn o_grau_de_ataque_e_de_defesa_tem_curvas_diferentes() {
    // Vantagem: `dano * (1 + grau*0,01)`. Desvantagem: `dano / (1 - grau*0,012)`.
    // As duas não são simétricas, e é assim no original.
    let mut g = golpe();

    g.grau_de_ataque = 100; // +100 → dobra
    assert_eq!(resolver(&g, &defesa(), 5.0, false, certeiro()).dano(), 2001);

    g.grau_de_ataque = 0;
    let mut d = defesa();
    d.grau_de_defesa = 100; // -100 → dividido por (1 + 1,2) = 2,2
    let esperado = (1000.5 / 2.2) as i32;
    assert_eq!(resolver(&g, &d, 5.0, false, certeiro()).dano(), esperado);
    assert_eq!(esperado, 454);
}

#[test]
fn golpe_que_acertou_nunca_faz_zero() {
    // `if (int_damage <= 0) int_damage = 1;`
    let mut g = golpe();
    g.dano_fisico = 1;
    let mut d = defesa();
    d.defesa = 10_000_000; // redução no teto, 0,95 → 0,05 de dano
    assert_eq!(resolver(&g, &d, 5.0, false, certeiro()).dano(), 1);
}

#[test]
fn a_curta_distancia_divide_o_golpe_normal_pela_metade() {
    // `if (is_short_range) { if (skill_id) {...} else physic_damage /= 2; }`
    let g = golpe();
    assert_eq!(resolver(&g, &defesa(), 1.0, true, certeiro()).dano(), 500);
    // E só o físico: o dano elemental passa inteiro no golpe normal.
    let mut g2 = golpe();
    g2.dano_magico[0] = 1000;
    assert_eq!(resolver(&g2, &defesa(), 1.0, true, certeiro()).dano(), 1500);
}

#[test]
fn a_curta_distancia_de_habilidade_usa_o_fator_proprio() {
    let mut g = golpe();
    g.de_habilidade = true;
    g.fator_de_curta_distancia = 0.25;
    g.dano_magico[0] = 1000;
    // Habilidade a curta distância multiplica físico E mágico pelo fator.
    assert_eq!(resolver(&g, &defesa(), 1.0, true, certeiro()).dano(), 500);
}

#[test]
fn a_distancia_so_atenua_quando_quem_ataca_e_jogador_ou_pet() {
    let mut d = defesa();
    d.reducao_longe_normal = 1.0;

    // Monstro atacando a 40 m: sem atenuação, porque não é jogador nem pet.
    let g = golpe();
    assert_eq!(resolver(&g, &d, 40.0, false, certeiro()).dano(), 1000);

    // Jogador a 40 m: (40-8)/(40+2) = 32/42 ≈ 0,7619 de redução → ~23,8% passa.
    let mut gj = golpe();
    gj.atacante_e_jogador_ou_pet = true;
    let r = resolver(&gj, &d, 40.0, false, certeiro()).dano();
    assert!((235..=240).contains(&r), "dano atenuado saiu {r}");

    // E perto (dentro de 8 m) a redução "longe" não vale.
    assert_eq!(resolver(&gj, &d, 5.0, false, certeiro()).dano(), 1000);
}

#[test]
fn a_penetracao_aumenta_o_dano_que_passa() {
    let mut d = defesa();
    d.defesa = 1975; // sem penetração: redução 0,5 → 500

    let mut g = golpe();
    g.anti_defesa = 10_000; // teto: defesa vira 650*... → 1975*0,65 = 1283

    let sem = resolver(&golpe(), &d, 5.0, false, certeiro()).dano();
    let com = resolver(&g, &d, 5.0, false, certeiro()).dano();
    assert!(com > sem, "penetração deveria aumentar o dano: {sem} contra {com}");
    // 1975*0,65 = 1283 (truncado) → 1283/(1283+1975) = 0,3937 → 606,3 → 606.
    assert_eq!(com, 606);
}

// ---------------------------------------------------------------------------------
// A ponte com as entidades
// ---------------------------------------------------------------------------------

use pw_core::{CharacterClass, Gender, Race, Vector3};
use pw_data_loader::classes::{ConfigDeClasse, TabelaDeClasses};
use pw_gs::combat::CombatEngine;
use pw_gs::entity::{MonsterEntity, PlayerEntity};

fn classe(id: i32, agi_attack: i32, agi_armor: i32) -> ConfigDeClasse {
    ConfigDeClasse {
        classe: id,
        ataque_por_agilidade: agi_attack,
        armadura_por_agilidade: agi_armor,
        chance_de_critico: 1,
        vida_por_vitalidade: 15,
        mana_por_energia: 9,
        vida_por_nivel: 5.0,
        mana_por_nivel: 3.0,
        dano_por_nivel: 1.0,
        dano_magico_por_nivel: 0.0,
        defesa_por_nivel: 1.0,
        resistencia_por_nivel: 0.0,
        velocidade_de_ataque: 0.8,
        alcance_de_ataque: 2.5,
        regeneracao_de_vida: 3,
        regeneracao_de_mana: 1,
        velocidade_andando: 1.5,
        velocidade_correndo: 4.0,
        velocidade_nadando: 2.2,
        velocidade_voando: 3.0,
        faccao: 0,
        faccao_inimiga: 0,
    }
}

fn jogador() -> PlayerEntity {
    PlayerEntity {
        role_id: 1,
        name: "Testador".into(),
        race: Race::Human,
        cls: CharacterClass::Blademaster,
        gender: Gender::Male,
        level: 50,
        cultivation: 0,
        hp: 3000,
        max_hp: 3000,
        mp: 500,
        max_mp: 500,
        exp: 0,
        sp: 0,
        money: 0,
        strength: 60,
        agility: 40,
        vitality: 50,
        energy: 10,
        def_phys: 500,
        def_metal: 100,
        def_wood: 100,
        def_water: 100,
        def_fire: 100,
        def_earth: 100,
        attack_min: 300,
        attack_max: 400,
        magic_attack_min: 0,
        magic_attack_max: 0,
        armor: 0,
        attack_rate: 0,
        attack_degree: 0,
        defend_degree: 0,
        crit_damage_bonus: 0,
        attack_speed: 1.0,
        move_speed: 4.0,
        crit_rate: 0.05,
        position: Vector3::new(0.0, 0.0, 0.0),
        target_id: None,
        buffs: Vec::new(),
        visiveis: std::collections::HashSet::new(),
        centro_do_stream: Vector3::new(0.0, 0.0, 0.0),
        voando: false,
        modo_roupa: false,
        sec_level: 0,
        habilidades: Default::default(),
    }
}

#[test]
fn a_precisao_e_a_evasao_do_jogador_vem_da_tabela_de_classes() {
    let mut t = TabelaDeClasses::default();
    // Guerreiro é a classe 0 e tem 10/10 no realm 155.
    t.classes.insert(0, classe(0, 10, 10));

    let mut p = jogador();
    assert_eq!((p.attack_rate, p.armor), (0, 0), "começa sem origem nenhuma");

    assert!(p.aplicar_atributos_de_classe(&t));
    // agilidade 40 × 10 = 400 nos dois.
    assert_eq!(p.attack_rate, 400);
    assert_eq!(p.armor, 400);
}

#[test]
fn sem_a_tabela_a_derivacao_avisa_em_vez_de_inventar() {
    let vazia = TabelaDeClasses::default();
    let mut p = jogador();
    p.attack_rate = 123;
    p.armor = 456;
    assert!(!p.aplicar_atributos_de_classe(&vazia), "deveria dizer que não conseguiu");
    assert_eq!((p.attack_rate, p.armor), (123, 456), "não pode mexer no que já estava lá");
}

/// Um monstro com os números que o `elements.data` do realm 155 dá para o
/// `Tauroc Valorian` (nível 47), para o cenário fim a fim não usar valor imaginário.
fn tauroc() -> MonsterEntity {
    let mut m = MonsterEntity::placeholder(900_001, 986, Vector3::new(3.0, 0.0, 0.0), 45_000);
    m.name = "Tauroc Valorian".into();
    m.level = 47;
    m.hp = 6701;
    m.max_hp = 6701;
    m.def_phys = 468;
    m.armor = 46;
    m.attack_rate = 2423;
    m.resistances = [468; 5];
    m.attack_min = 311;
    m.attack_max = 356;
    m.magic_attack = [(0, 0); 5];
    m.attack_degree = 0;
    m.defend_degree = 0;
    m
}

#[test]
fn um_golpe_realista_cai_na_faixa_que_a_formula_manda() {
    let mut t = TabelaDeClasses::default();
    t.classes.insert(0, classe(0, 10, 10));
    let mut p = jogador();
    p.aplicar_atributos_de_classe(&t);

    // Precisão 400 contra armadura 46: chance = 400/(400+23) ≈ 0,945. Alta, mas não 1 —
    // e é exatamente esse "não 1" que a fórmula antiga não tinha.
    let chance = chance_de_acerto(p.attack_rate, tauroc().armor);
    assert!((chance - 400.0 / 423.0).abs() < 1e-6, "chance saiu {chance}");

    // Dano: 300..400 uniforme, reduzido por defesa 468 contra atacante nível 50:
    // 468 / (468 + 2000 - 25) = 0,1916 → passa 80,8%.
    let reducao = reducao_por_defesa(tauroc().def_phys, p.level);
    let minimo = (300.0 * (1.0 - reducao)) as i32;
    let maximo = (400.0 * (1.0 - reducao) * 2.0) as i32 + 1; // margem para o crítico

    let m = tauroc();
    let mut acertos = 0;
    for _ in 0..200 {
        match CombatEngine::jogador_ataca_monstro(&p, &m, 3.0) {
            pw_gs::combat::Resultado::Acertou { dano, .. } => {
                assert!(
                    (minimo..=maximo).contains(&dano),
                    "dano {dano} fora da faixa {minimo}..={maximo}"
                );
                acertos += 1;
            }
            pw_gs::combat::Resultado::Errou => {}
            outro => panic!("resultado inesperado: {outro:?}"),
        }
    }
    // Com 94,5% de chance, 200 golpes sem nenhum acerto é impossível na prática.
    assert!(acertos > 150, "só {acertos} acertos em 200 golpes");
}

#[test]
fn o_monstro_devolve_dano_pela_mesma_formula() {
    let p = jogador();
    let m = tauroc();
    // Precisão 2423 contra evasão 0 do jogador de teste → sempre acerta.
    let mut vistos = Vec::new();
    for _ in 0..50 {
        vistos.push(CombatEngine::monstro_ataca_jogador(&m, &p, 3.0).dano());
    }
    assert!(vistos.iter().all(|d| *d > 0), "o monstro não deveria errar sem evasão");
    // Dano 311..356 contra defesa 500 e atacante nível 47:
    // 500 / (500 + 1880 - 25) = 0,2123 → passa ~78,8%, ou seja 245..280.
    let maior = *vistos.iter().max().unwrap();
    let menor = *vistos.iter().min().unwrap();
    assert!(menor >= 240 && maior <= 285, "dano do monstro em {menor}..{maior}");
}
