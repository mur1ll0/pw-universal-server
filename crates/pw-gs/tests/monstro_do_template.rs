//! O monstro que entra no mundo carrega os atributos do `elements.data`.
//!
//! Antes, `WorldInstance::init_spawns` escrevia nível 1, 500 de vida e dano 20 a 35 para
//! **todo** monstro de **todo** mapa. Agora ele consulta a tabela de `MONSTER_ESSENCE`
//! (`pw_data_loader::monstros`) e só cai no genérico quando não há template.
//!
//! O teste ataca o mapeamento direto, pelos construtores, porque montar um
//! `WorldInstance` exige um `CharacterRepository` e portanto um banco — e o que interessa
//! aqui não tem nada a ver com banco.

use pw_core::Vector3;
use pw_data_loader::monstros::{FaixaDeDano, TemplateDeMonstro};
use pw_gs::entity::MonsterEntity;

/// Um template com todo campo num valor distinto, para um campo trocado por outro
/// aparecer como valor errado em vez de passar despercebido.
fn template() -> TemplateDeMonstro {
    TemplateDeMonstro {
        id: 4242,
        nome: "Tauroc Valorian".into(),
        nivel: 47,
        exp: 3110,
        pontos_de_skill: 12,
        vida: 6701,
        regeneracao_de_vida: 33,
        defesa: 468,
        armadura: 46,
        resistencias: [11, 22, 33, 44, 55],
        imunidades: 0,
        taxa_de_ataque: 2423,
        dano_fisico: FaixaDeDano { minimo: 311, maximo: 356 },
        dano_magico: FaixaDeDano { minimo: 7, maximo: 9 },
        dano_magico_por_classe: [FaixaDeDano::default(); 5],
        alcance_de_ataque: 13.2,
        ataque_em_ticks: 30,
        atraso_do_dano_em_ticks: 10,
        grau_de_ataque: 3,
        grau_de_defesa: 4,
        ataque_a_distancia: true,
        velocidade_andando: 1.5,
        velocidade_correndo: 4.5,
        velocidade_nadando: 2.0,
        velocidade_voando: 0.0,
        tamanho: 1.0,
        tipo_de_habitat: 0,
        patrulha: true,
        estrategia: 2,
        politica_de_ia: 60,
        agressivo: 1,
        raio_de_odio: 15.0,
        tempo_de_odio: 20,
        raio_de_visao: 30,
        faccao: 8,
        faccao_de_monstro: 3,
        depois_da_morte: 0,
        skills: Vec::new(),
        skills_com_75_de_vida: Vec::new(),
        skills_com_50_de_vida: Vec::new(),
        skills_com_25_de_vida: Vec::new(),
        estrategias_de_odio: Vec::new(),
        dinheiro_medio: 900,
        dinheiro_variacao: 100,
    }
}

#[test]
fn o_monstro_sai_com_os_atributos_do_elements_data() {
    let pos = Vector3::new(120.0, 20.0, -340.0);
    let m = MonsterEntity::do_template(900_001, &template(), pos, 45_000);

    assert_eq!(m.id, 900_001);
    assert_eq!(m.template_id, 4242);
    assert_eq!(m.name, "Tauroc Valorian");
    assert_eq!(m.level, 47);
    assert_eq!(m.hp, 6701);
    assert_eq!(m.max_hp, 6701);
    assert_eq!(m.def_phys, 468);
    // A primeira das cinco resistências (metal) — ver o comentário em `do_template`.
    assert_eq!(m.def_magic, 11);
    assert_eq!(m.attack_min, 311);
    assert_eq!(m.attack_max, 356);
    assert_eq!(m.attack_range, 13.2);
    assert_eq!(m.exp, 3110);
    assert_eq!(m.sp, 12);
    assert_eq!(m.aipolicy_id, 60);
    assert_eq!(m.move_speed, 4.5);
    assert_eq!(m.position, pos);
    assert_eq!(m.spawn_center, pos);
    assert_eq!(m.respawn_delay_ms, 45_000);
    assert!(!m.is_dead);
}

#[test]
fn a_mana_do_monstro_e_um_como_no_original() {
    // `nt.bp.mp = 1` e `nt.ep.max_mp = 1` em `npcgenerator.cpp`: o custo de habilidade de
    // monstro não sai de mana. Os 100 de antes eram invenção nossa.
    let m = MonsterEntity::do_template(1, &template(), Vector3::new(0.0, 0.0, 0.0), 0);
    assert_eq!(m.mp, 1);
    assert_eq!(m.max_mp, 1);
}

#[test]
fn o_ataque_nao_e_a_taxa_de_ataque() {
    // Erro fácil de cometer: `mob.attack` (2423 aqui) é o `attack_rate` da fórmula de
    // acerto, não o dano. O dano é `damage_min`/`damage_max`.
    let t = template();
    let m = MonsterEntity::do_template(1, &t, Vector3::new(0.0, 0.0, 0.0), 0);
    assert_ne!(m.attack_min, t.taxa_de_ataque);
    assert_ne!(m.attack_max, t.taxa_de_ataque);
    assert_eq!((m.attack_min, m.attack_max), (311, 356));
}

#[test]
fn o_placeholder_continua_disponivel_e_e_distinguivel() {
    // O genérico não sumiu: é o que o realm 1.2.6 usa, e o que aparece quando o
    // `npcgen.data` cita um monstro que o `elements.data` não tem. Tem de ser
    // reconhecível no log e na tela — não pode se passar por monstro de verdade.
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let m = MonsterEntity::placeholder(7, 555, pos, 30_000);
    assert_eq!(m.template_id, 555);
    assert_eq!(m.name, "Monstro");
    assert_eq!(m.level, 1);
    assert_eq!(m.aipolicy_id, 0);
    assert_eq!(m.position, pos);
    assert_eq!(m.respawn_delay_ms, 30_000);

    // E é diferente do que sai do template, campo a campo no que importa.
    let real = MonsterEntity::do_template(7, &template(), pos, 30_000);
    assert_ne!(m.name, real.name);
    assert_ne!(m.level, real.level);
    assert_ne!(m.max_hp, real.max_hp);
    assert_ne!(m.aipolicy_id, real.aipolicy_id);
}
