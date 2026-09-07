//! O template de monstro montado do `MONSTER_ESSENCE`.
//!
//! Duas metades: as regras de recusa do original, testadas com registros construídos à
//! mão (determinístico, sem depender de arquivo), e a carga do `elements.data` real do
//! realm 155, que é onde um erro de nome de campo apareceria.

use pw_data_loader::aipolicy::AiPolicyData;
use pw_data_loader::generic_elements::{FieldValue, GenericElementsData, Record};
use pw_data_loader::monstros::{self, TemplateDeMonstro};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------
// Metade 1: as regras de recusa, com registros sintéticos
// ---------------------------------------------------------------------------------

/// Um `MONSTER_ESSENCE` mínimo que **passa** por todas as recusas, para os testes
/// mexerem num campo de cada vez.
fn registro_valido(id: i32) -> Record {
    let mut r: Record = HashMap::new();
    r.insert("ID".into(), FieldValue::Int(id));
    r.insert("Name".into(), FieldValue::Text("Bicho".into()));
    r.insert("level".into(), FieldValue::Int(10));
    r.insert("life".into(), FieldValue::Int(500));
    r.insert("attack".into(), FieldValue::Int(100));
    r.insert("damage_min".into(), FieldValue::Int(20));
    r.insert("damage_max".into(), FieldValue::Int(30));
    // 1,0 s de ataque = 20 ticks de 50 ms; 0,5 s de atraso = 10 ticks.
    r.insert("attack_speed".into(), FieldValue::Float(1.0));
    r.insert("damage_delay".into(), FieldValue::Float(0.5));
    r
}

fn tabela_de(registros: Vec<Record>) -> monstros::TabelaDeMonstros {
    let mut tables = HashMap::new();
    tables.insert("MONSTER_ESSENCE".to_string(), registros);
    let dados = GenericElementsData { version: 156, tables };
    monstros::carregar(&dados, None)
}

#[test]
fn converte_segundos_em_ticks_de_50ms() {
    // `nt.ep.attack_speed = (int)(mob.attack_speed * 20 + 0.5)` e
    // `nt.damage_delay = (int)(mob.damage_delay * 20)` — o primeiro arredonda, o segundo
    // trunca. A diferença é do original, não descuido.
    let t = tabela_de(vec![registro_valido(1)]);
    let m = t.get(1).expect("o registro válido deveria entrar");
    assert_eq!(m.ataque_em_ticks, 20);
    assert_eq!(m.atraso_do_dano_em_ticks, 10);
}

#[test]
fn recusa_modelo_de_ataque_invalido() {
    // `if (attack_speed <= 0 || damage_low <= 0 || attack <= 0) continue;`
    for (campo, valor) in [
        ("attack", FieldValue::Int(0)),
        ("damage_min", FieldValue::Int(0)),
        ("attack_speed", FieldValue::Float(0.0)),
    ] {
        let mut r = registro_valido(1);
        r.insert(campo.into(), valor);
        let t = tabela_de(vec![r]);
        assert!(t.is_empty(), "{campo} zerado deveria recusar o monstro");
        assert_eq!(t.recusas.modelo_de_ataque_invalido, 1, "recusa errada para {campo}");
    }
}

#[test]
fn recusa_ataque_acima_de_256_ticks() {
    // `if (nt.ep.attack_speed > 256) continue;` — 13 s × 20 = 260 ticks.
    let mut r = registro_valido(1);
    r.insert("attack_speed".into(), FieldValue::Float(13.0));
    let t = tabela_de(vec![r]);
    assert!(t.is_empty());
    assert_eq!(t.recusas.ataque_lento_demais, 1);
}

#[test]
fn recusa_atraso_de_dano_acima_de_256_ticks() {
    // `if (nt.damage_delay > 256) continue;` — 13 s × 20 = 260 ticks.
    let mut r = registro_valido(1);
    r.insert("damage_delay".into(), FieldValue::Float(13.0));
    let t = tabela_de(vec![r]);
    assert!(t.is_empty());
    assert_eq!(t.recusas.atraso_de_dano_grande_demais, 1);
}

#[test]
fn ignora_registro_de_preenchimento() {
    // O `elements.data` tem registros com id 0 que não são monstro.
    let t = tabela_de(vec![registro_valido(0)]);
    assert!(t.is_empty());
    assert_eq!(t.recusas.total(), 0, "id 0 não é recusa, é registro vazio");
}

#[test]
fn tempo_de_odio_tem_piso_de_um_segundo() {
    // `if (nt.aggro_time <= 0) nt.aggro_time = 1;`
    let t = tabela_de(vec![registro_valido(1)]);
    assert_eq!(t.get(1).unwrap().tempo_de_odio, 1);
}

#[test]
fn ataque_a_distancia_e_derivado_do_alcance() {
    // `if (mob.attack_range > 6.0f) nt.short_range_mode = 1;` — derivado, não lido do
    // campo `short_range_mode` que existe no arquivo.
    let mut curto = registro_valido(1);
    curto.insert("attack_range".into(), FieldValue::Float(2.0));
    curto.insert("short_range_mode".into(), FieldValue::Int(1));
    assert!(!tabela_de(vec![curto]).get(1).unwrap().ataque_a_distancia);

    let mut longo = registro_valido(1);
    longo.insert("attack_range".into(), FieldValue::Float(13.2));
    longo.insert("short_range_mode".into(), FieldValue::Int(0));
    assert!(tabela_de(vec![longo]).get(1).unwrap().ataque_a_distancia);
}

#[test]
fn politica_inexistente_e_zerada_e_registrada() {
    // O original imprime "a política %d do monstro %d não foi achada" e zera o campo.
    // Sem `aipolicy.data` (None), o id passa como veio — a checagem é do arquivo, não uma
    // regra nossa.
    let mut r = registro_valido(7);
    r.insert("common_strategy".into(), FieldValue::Int(999));

    let sem_checagem = tabela_de(vec![r.clone()]);
    assert_eq!(sem_checagem.get(7).unwrap().politica_de_ia, 999);
    assert!(sem_checagem.politicas_orfas.is_empty());

    let mut tables = HashMap::new();
    tables.insert("MONSTER_ESSENCE".to_string(), vec![r]);
    let dados = GenericElementsData { version: 156, tables };
    let vazio = AiPolicyData::default();
    let com_checagem = monstros::carregar(&dados, Some(&vazio));
    assert_eq!(com_checagem.get(7).unwrap().politica_de_ia, 0);
    assert_eq!(com_checagem.politicas_orfas, vec![(7, 999)]);
}

#[test]
fn sem_a_tabela_no_arquivo_a_carga_e_vazia_e_nao_falha() {
    // É o caso do 1.2.6/v7, que o leitor genérico não cobre: quem consulta tem de saber
    // lidar com a ausência do template, não receber um template inventado.
    let dados = GenericElementsData { version: 7, tables: HashMap::new() };
    let t = monstros::carregar(&dados, None);
    assert!(t.is_empty());
    assert_eq!(t.recusas.total(), 0);
}

// ---------------------------------------------------------------------------------
// Metade 2: o elements.data real do realm 155
// ---------------------------------------------------------------------------------

fn carregar_realm_155() -> Option<(monstros::TabelaDeMonstros, usize)> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join("data/realm_155/config");
    let Ok(elements) = std::fs::read(base.join("elements.data")) else {
        eprintln!("pulado: elements.data do realm_155 não existe");
        return None;
    };
    let politicas = std::fs::read(base.join("aipolicy.data"))
        .ok()
        .and_then(|b| AiPolicyData::load_from_bytes(&b).ok());
    let elements = pw_data_loader::generic_elements::load_elements_data_auto(&elements)
        .expect("elements.data do realm_155 deveria ser legível");
    let registros = elements.get("MONSTER_ESSENCE").len();
    Some((monstros::carregar(&elements, politicas.as_ref()), registros))
}

#[test]
fn carrega_os_monstros_do_realm_155() {
    let Some((t, registros)) = carregar_realm_155() else { return };

    // Cada registro do arquivo ou vira template, ou é recusado por uma das três regras do
    // original, ou é registro de preenchimento (id 0). Não há terceira saída silenciosa.
    assert!(t.len() > 8000, "poucos templates: {}", t.len());
    assert!(t.len() + t.recusas.total() <= registros);

    // As recusas existem e são poucas — o original recusaria os mesmos.
    assert!(t.recusas.total() < registros / 100, "recusas demais: {:?}", t.recusas);
}

#[test]
fn os_atributos_lidos_fazem_sentido() {
    let Some((t, _)) = carregar_realm_155() else { return };

    // Se o layout estivesse deslocado, estes campos independentes sairiam como ruído.
    // O que se afirma aqui é a forma da distribuição, não valor de monstro nenhum.
    // Cerca de 81% têm nome. Os outros ~1500 **não** são erro de leitura: são as
    // entidades controladoras invisíveis do 1.5.5 — vida em números redondos
    // (999999999, 10000000), dano 1..2, facção 0x40000000 — que existem só para rodar
    // uma política de IA. O que se afirma aqui é que os sem nome se concentram nesse
    // perfil: 87% deles têm política de IA, contra 48% dos que têm nome. Um layout
    // deslocado não produziria essa separação.
    let com_nome: Vec<&TemplateDeMonstro> =
        t.templates.values().filter(|m| !m.nome.is_empty()).collect();
    let sem_nome: Vec<&TemplateDeMonstro> =
        t.templates.values().filter(|m| m.nome.is_empty()).collect();
    assert!(com_nome.len() * 4 > t.len() * 3, "{} de {} monstros têm nome", com_nome.len(), t.len());
    assert!(!sem_nome.is_empty(), "nenhum monstro sem nome — o perfil mudou, reconferir");

    let fracao_com_ia = |v: &[&TemplateDeMonstro]| -> f64 {
        v.iter().filter(|m| m.politica_de_ia != 0).count() as f64 / v.len() as f64
    };
    assert!(
        fracao_com_ia(&sem_nome) > fracao_com_ia(&com_nome) * 1.5,
        "os monstros sem nome deveriam ser sobretudo controladores de IA: {:.0}% deles têm          política contra {:.0}% dos nomeados",
        fracao_com_ia(&sem_nome) * 100.0,
        fracao_com_ia(&com_nome) * 100.0
    );

    // Nível dentro do teto do 1.5.5 (150), e a grande maioria acima de zero.
    assert!(t.templates.values().all(|m| (0..=150).contains(&m.nivel)), "nível fora de 0..=150");
    let nivel_zero = t.templates.values().filter(|m| m.nivel <= 0).count();
    assert!(nivel_zero * 100 < t.len(), "{nivel_zero} monstros de nível 0");

    // Vida cresce com o nível. Duas precauções, e as duas são sobre o dado, não sobre o
    // leitor: só monstros **nomeados** entram (as controladoras têm vida em números
    // redondos gigantes em qualquer nível — há uma de nível 1 com 9.999.999), e a
    // comparação é por **mediana**, não média, para uma controladora que escape não
    // decidir o resultado. É a checagem que pega deslocamento de coluna, porque `level` e
    // `life` estão longe um do outro no registro.
    let mediana_de_vida = |faixa: std::ops::RangeInclusive<i32>| -> i64 {
        let mut v: Vec<i64> = t
            .templates
            .values()
            .filter(|m| !m.nome.is_empty() && faixa.contains(&m.nivel) && m.vida > 0)
            .map(|m| m.vida as i64)
            .collect();
        assert!(!v.is_empty(), "nenhum monstro nomeado na faixa {faixa:?}");
        v.sort_unstable();
        v[v.len() / 2]
    };
    let baixo = mediana_de_vida(1..=20);
    let alto = mediana_de_vida(100..=150);
    assert!(alto > baixo * 10, "vida não cresce com o nível: {baixo} contra {alto}");

    // Velocidade de corrida em metros por segundo, na faixa que o original espera (ele
    // avisa quando walk/run ficam abaixo de 0,1).
    let parados = t.templates.values().filter(|m| m.velocidade_correndo <= 0.1).count();
    assert!(parados * 10 < t.len(), "{parados} monstros sem velocidade de corrida");
}

#[test]
fn as_politicas_de_ia_dos_monstros_existem_no_aipolicy() {
    // Mesma prova cruzada de `aipolicy_tests.rs`, agora pelo caminho que o servidor vai
    // usar de verdade: se a política não existir, `carregar` zera o campo e registra.
    let Some((t, _)) = carregar_realm_155() else { return };

    let com_politica = t.templates.values().filter(|m| m.politica_de_ia != 0).count();
    assert!(com_politica > 4000, "poucos monstros com política de IA: {com_politica}");
    assert!(
        t.politicas_orfas.len() * 1000 < com_politica,
        "políticas órfãs demais: {:?}",
        t.politicas_orfas
    );
}

#[test]
fn as_habilidades_dos_monstros_sao_lidas() {
    let Some((t, _)) = carregar_realm_155() else { return };

    let com_skill = t.templates.values().filter(|m| !m.skills.is_empty()).count();
    assert!(com_skill > 1000, "poucos monstros com habilidade: {com_skill}");

    // Toda habilidade lida tem id positivo (o filtro é esse) e nível numa faixa sã. Um
    // deslocamento de coluna daria níveis absurdos em massa.
    let niveis: Vec<i32> = t
        .templates
        .values()
        .flat_map(|m: &TemplateDeMonstro| m.skills.iter().map(|s| s.nivel))
        .collect();
    let fora = niveis.iter().filter(|&&n| !(0..=50).contains(&n)).count();
    assert!(fora * 100 < niveis.len(), "{fora} de {} níveis de skill fora de 0..=50", niveis.len());

    // Habilidades por limiar de vida existem e vêm com probabilidade entre 0 e 1.
    let por_vida: Vec<f32> = t
        .templates
        .values()
        .flat_map(|m| {
            m.skills_com_75_de_vida
                .iter()
                .chain(&m.skills_com_50_de_vida)
                .chain(&m.skills_com_25_de_vida)
                .map(|s| s.probabilidade)
        })
        .collect();
    assert!(!por_vida.is_empty(), "nenhuma habilidade por limiar de vida");
    assert!(
        por_vida.iter().all(|p| (0.0..=1.0).contains(p)),
        "probabilidade fora de 0..1 em habilidade por limiar de vida"
    );
}
