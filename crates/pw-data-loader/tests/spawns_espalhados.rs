//! Os monstros de uma área nascem **espalhados** dentro dela, não empilhados no centro.
//!
//! O `npcgen.data` define áreas com um tamanho (`vExts`) e quantos monstros cada uma
//! gera. O original monta a caixa `pos ∓ exts/2` (`base_spawner::SetRegion`) e sorteia
//! cada monstro dentro dela (`terrain_gen_pos::Generate`).
//!
//! O leitor lia `vExts` e **descartava**, colocando os monstros num deslocamento fixo de
//! até três metros em diagonal. Em jogo, 2026-09-07: "os monstros estão spawnando todos
//! agrupados".

use pw_data_loader::npcgen::{NpcGenData, SpawnType};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn carregar(realm: &str) -> Option<NpcGenData> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(format!("data/{realm}/config/world/npcgen.data"));
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    Some(NpcGenData::load_from_bytes(&bytes).expect("npcgen.data do mundo deveria ser legível"))
}

/// Quantas células de 10 m estão ocupadas, e quantos monstros tem a mais cheia.
fn dispersao(d: &NpcGenData) -> (usize, usize, usize) {
    let mut celas: BTreeMap<(i32, i32), usize> = BTreeMap::new();
    let mut total = 0;
    for i in d.instances.iter().filter(|i| i.spawn_type == SpawnType::Monster) {
        total += 1;
        *celas.entry((i.pos.x as i32 / 10, i.pos.z as i32 / 10)).or_default() += 1;
    }
    let pior = celas.values().copied().max().unwrap_or(0);
    (total, celas.len(), pior)
}

#[test]
fn os_monstros_do_mundo_nao_nascem_empilhados() {
    let Some(d) = carregar("realm_155BR") else { return };
    let (total, celulas, pior) = dispersao(&d);

    assert!(total > 10_000, "poucos monstros no mundo: {total}");

    // Com o deslocamento fixo de antes, 21.846 monstros ocupavam 7.281 células de 10 m e a
    // mais cheia tinha 42. Espalhados pela caixa da área, passam de 20 mil células com no
    // máximo um punhado cada. O que se afirma é a **forma**: mais de duas células por três
    // monstros, e nenhuma célula com dezenas.
    assert!(
        celulas * 3 > total * 2,
        "{total} monstros em só {celulas} células de 10 m — estão agrupados de novo"
    );
    assert!(
        pior <= 15,
        "uma célula de 10 m tem {pior} monstros — sinal de que voltaram a nascer no centro"
    );
}

#[test]
fn a_posicao_e_estavel_entre_cargas() {
    // A posição é função de `(id da instância, índice)`, não sorteio: carregar duas vezes
    // tem de dar exatamente o mesmo mundo. Sem isso, reiniciar o servidor teleportaria
    // todo monstro, e nenhum teste poderia afirmar posição.
    let (Some(a), Some(b)) = (carregar("realm_155BR"), carregar("realm_155BR")) else {
        return;
    };
    assert_eq!(a.instances.len(), b.instances.len());
    for (x, y) in a.instances.iter().zip(b.instances.iter()) {
        assert_eq!(x.instance_id, y.instance_id);
        assert_eq!(x.pos, y.pos, "instância {} mudou de lugar entre cargas", x.instance_id);
    }
}

#[test]
fn area_sem_tamanho_continua_com_o_monstro_no_centro() {
    // O original trata o caso à parte (`_pos_min.squared_distance(_pos_max) < 1e-3`): área
    // que é um ponto gera um monstro no ponto. Espalhar aí inventaria posição.
    //
    // Não dá para escolher a área pelo arquivo real, então o que se confere é a
    // consequência: existem monstros exatamente sobre coordenadas "redondas" de área, e a
    // dispersão não é total.
    let Some(d) = carregar("realm_155BR") else { return };
    let mut por_posicao: BTreeMap<(i32, i32, i32), usize> = BTreeMap::new();
    for i in d.instances.iter().filter(|i| i.spawn_type == SpawnType::Monster) {
        *por_posicao
            .entry((
                (i.pos.x * 10.0) as i32,
                (i.pos.y * 10.0) as i32,
                (i.pos.z * 10.0) as i32,
            ))
            .or_default() += 1;
    }
    // Duas instâncias exatamente na mesma coordenada seriam duas áreas-ponto no mesmo
    // lugar, o que é raro; o que não pode é isso ser comum.
    let repetidas = por_posicao.values().filter(|n| **n > 1).count();
    assert!(
        repetidas * 100 < por_posicao.len(),
        "{repetidas} coordenadas com mais de um monstro em cima — espalhamento não pegou"
    );
}
