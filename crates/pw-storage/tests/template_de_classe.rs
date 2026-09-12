//! O molde de personagem novo do realm, conferido contra o código.
//!
//! # A falha que este arquivo tranca
//!
//! O `class_templates` do realm 155BR tinha **seis** das doze classes, e os ids não batiam
//! com os nomes: a linha `cls = 4` chamava-se "Feiticeira", e 4 é o Bárbaro. Como
//! `create_character` procura pelo **id**, o Bárbaro criado em 2026-09-11 recebeu o molde
//! da Feiticeira — uma arma de magia (Graveto de Madeira, 2867) no lugar do Porrete.
//!
//! Nada no código podia perceber isso: a tabela é dado, e dado errado atravessa compilador
//! e tipo sem esbarrar em nada. O que pega é conferir a tabela contra a única fonte que
//! **está** conferida — `CharacterClass::default_weapon_id`, que o teste
//! `armas_iniciais_batem_com_o_elements` do `pw-data-loader` casa com o `WEAPON_ESSENCE`.
//!
//! Sem `TEST_DATABASE_URL` o teste passa sem verificar nada e diz isso na saída.

use pw_core::CharacterClass;
use pw_storage::{PostgresPool, StorageConfig};

/// O realm que o docker de teste serve.
const REALM: &str = "realm_155BR";

async fn pool() -> Option<PostgresPool> {
    let url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("AVISO: TEST_DATABASE_URL não definida — este teste NÃO verificou nada.");
            return None;
        }
    };
    let cfg = StorageConfig {
        database_url: url,
        max_connections: 2,
        min_connections: 1,
        ..Default::default()
    };
    PostgresPool::new(&cfg).await.ok()
}

fn todas_as_classes() -> Vec<CharacterClass> {
    (0..12u8).filter_map(CharacterClass::from_u8).collect()
}

/// As doze classes estão no molde, e o `cls` de cada linha é o que o nome diz.
#[tokio::test]
async fn o_realm_tem_molde_para_as_doze_classes() {
    let Some(p) = pool().await else { return };

    let linhas: Vec<(i32, String)> =
        sqlx::query_as("SELECT cls, name FROM class_templates WHERE realm_id = $1 ORDER BY cls")
            .bind(REALM)
            .fetch_all(p.get_ref())
            .await
            .expect("ler o molde do realm");

    if linhas.is_empty() {
        eprintln!("pulado: o realm {REALM} não tem molde neste banco");
        return;
    }

    assert_eq!(
        linhas.len(),
        12,
        "o molde tem {} classes; faltam {:?}",
        linhas.len(),
        todas_as_classes()
            .iter()
            .map(|c| *c as i32)
            .filter(|c| !linhas.iter().any(|(id, _)| id == c))
            .collect::<Vec<_>>()
    );

    for (cls, nome) in &linhas {
        assert!(
            CharacterClass::from_u8(*cls as u8).is_some(),
            "a linha cls={cls} ('{nome}') não é uma classe do jogo"
        );
    }
}

/// A arma do molde é a que o código diz para aquela classe.
///
/// É esta conferência que teria impedido o Bárbaro de nascer com arma de mago: o molde
/// dizia 2867 e `default_weapon_id(Barbarian)` diz 2258.
#[tokio::test]
async fn a_arma_do_molde_e_a_da_classe() {
    let Some(p) = pool().await else { return };

    let armas: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT t.cls, i.item_id
           FROM class_templates t
           JOIN class_template_items i ON i.template_id = t.id
          WHERE t.realm_id = $1 AND i.container_type = 1 AND i.slot = 0
          ORDER BY t.cls",
    )
    .bind(REALM)
    .fetch_all(p.get_ref())
    .await
    .expect("ler as armas do molde");

    if armas.is_empty() {
        eprintln!("pulado: o realm {REALM} não tem molde neste banco");
        return;
    }

    for (cls, item_id) in &armas {
        let classe = CharacterClass::from_u8(*cls as u8).expect("cls válido");
        assert_eq!(
            *item_id,
            classe.default_weapon_id(),
            "a classe {classe:?} (cls={cls}) nasce com o item {item_id} no molde, e o \
             código diz {}",
            classe.default_weapon_id()
        );
    }

    assert_eq!(armas.len(), 12, "há classe sem arma no molde");
}

/// Duas classes da mesma raça nascem juntas: no mesmo mundo, a menos de 2 m uma da outra.
///
/// No Perfect World a vila inicial é da **raça**, não da classe — é como o
/// `ptemplate.conf` pareia as seções e como `CharacterClass::race` as agrupa. Uma linha
/// fora do par denuncia molde preenchido à mão, linha a linha, que foi como o anterior
/// ficou torto.
///
/// "A menos de 2 m", e não "no mesmo ponto": os moldes do `clsconfig` original foram
/// gravados um por personagem, e as duas classes de uma raça ficam a até 1,1 m uma da
/// outra (Abissais: (-651.09, -225.21) e (-651.82, -225.49)). Ver
/// `scripts/2026_09_12_nascimento_no_mapa_161_155br.sql`.
#[tokio::test]
async fn as_duas_classes_de_uma_raca_nascem_juntas() {
    let Some(p) = pool().await else { return };

    let pontos: Vec<(i32, i32, f32, f32, f32)> = sqlx::query_as(
        "SELECT cls, spawn_world_id, spawn_x, spawn_y, spawn_z FROM class_templates WHERE realm_id = $1",
    )
    .bind(REALM)
    .fetch_all(p.get_ref())
    .await
    .expect("ler os nascimentos do molde");

    if pontos.is_empty() {
        eprintln!("pulado: o realm {REALM} não tem molde neste banco");
        return;
    }

    let mut por_raca: std::collections::HashMap<i32, Vec<(i32, i32, f32, f32, f32)>> =
        Default::default();
    for ponto in &pontos {
        let classe = CharacterClass::from_u8(ponto.0 as u8).expect("cls válido");
        por_raca.entry(classe.race() as i32).or_default().push(*ponto);
    }

    for (raca, mut classes) in por_raca {
        classes.sort_by_key(|c| c.0);
        let (c0, mundo, x, y, z) = classes[0];
        for (cls, cmundo, cx, cy, cz) in &classes[1..] {
            let d = ((cx - x).powi(2) + (cy - y).powi(2) + (cz - z).powi(2)).sqrt();
            assert!(
                *cmundo == mundo && d < 2.0,
                "a raça {raca} nasce em dois lugares: cls {c0} no mundo {mundo} ({x}, {y}, {z})                  e cls {cls} no mundo {cmundo} ({cx}, {cy}, {cz})"
            );
        }
        assert_eq!(classes.len(), 2, "a raça {raca} tem {} classes no molde", classes.len());
    }
}

/// Nenhum item do kit inicial exige nível acima de 1.
///
/// A Poção Perfeita de Cura (1801) estava no kit e exige nível 30 no `elements.data` — o
/// personagem nascia com dez de um item que ele não pode usar. Aqui a lista de ids é curta
/// e explícita de propósito: o teste não abre o `elements.data`, ele cobra que o kit seja
/// **este**, e quem mudar o kit tem de vir aqui dizer por quê.
#[tokio::test]
async fn o_kit_de_bolsa_e_o_de_nivel_zero() {
    let Some(p) = pool().await else { return };

    /// `(item_id, require_level do elements.data)` — 1796 e 1804 são as poções pequenas,
    /// 2100 é o Portal da Cidade, que não tem exigência de nível.
    const KIT: &[i32] = &[2100, 1796, 1804];

    let itens: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT DISTINCT t.cls, i.item_id
           FROM class_templates t
           JOIN class_template_items i ON i.template_id = t.id
          WHERE t.realm_id = $1 AND i.container_type = 0",
    )
    .bind(REALM)
    .fetch_all(p.get_ref())
    .await
    .expect("ler o kit do molde");

    if itens.is_empty() {
        eprintln!("pulado: o realm {REALM} não tem molde neste banco");
        return;
    }

    for (cls, item_id) in &itens {
        assert!(
            KIT.contains(item_id),
            "a classe cls={cls} nasce com o item {item_id}, que não é do kit de nível 0 \
             ({KIT:?}). A Poção Perfeita de Cura (1801) exige nível 30."
        );
    }
}
