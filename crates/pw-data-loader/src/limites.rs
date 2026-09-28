//! Os limites de cada mapa: a chave `limit` do `gs.conf` original (`world_manager::
//! InitWorldLimit`, `cgame/gs/worldmanager.cpp:108-160`), extraída por
//! `specs/mapas/gerar_limites.py` para `specs/mapas/limites_126.json` (do `gs.conf` do 1.2.6) e
//! `limites_155.json` (do `pwserver_155v156`). Pela `tag` do mapa (o `world_id`).
//!
//! Hoje só o `nofly` tem consumidor: sem voo (`item_flysword.cpp:66`), sem mascote de ar
//! (`petman.cpp:104/140/270`) e o voo cai ao entrar (`player.cpp:11994`).
use serde::Deserialize;
use std::sync::OnceLock;

const LIMITES_126: &str = include_str!("../../../specs/mapas/limites_126.json");
const LIMITES_155: &str = include_str!("../../../specs/mapas/limites_155.json");

/// De qual `gs.conf` vêm os limites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogoDeLimites {
    V126,
    V155,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitesDoMapa {
    pub world_id: i32,
    pub secao: String,
    pub limites: Vec<String>,
    pub altura_maxima: f32,
}

#[derive(Debug, Deserialize)]
struct Arquivo {
    mapas: Vec<LimitesDoMapa>,
}

fn catalogo(c: CatalogoDeLimites) -> &'static [LimitesDoMapa] {
    static C126: OnceLock<Vec<LimitesDoMapa>> = OnceLock::new();
    static C155: OnceLock<Vec<LimitesDoMapa>> = OnceLock::new();
    let (celula, texto) = match c {
        CatalogoDeLimites::V126 => (&C126, LIMITES_126),
        CatalogoDeLimites::V155 => (&C155, LIMITES_155),
    };
    celula.get_or_init(|| {
        serde_json::from_str::<Arquivo>(texto).map(|a| a.mapas).unwrap_or_default()
    })
}

/// Os limites de um mapa; `None` para mapa fora do `gs.conf`.
pub fn do_mapa(c: CatalogoDeLimites, world_id: i32) -> Option<&'static LimitesDoMapa> {
    catalogo(c).iter().find(|m| m.world_id == world_id)
}

/// `_world_limit.nofly`.
pub fn sem_voo(c: CatalogoDeLimites, world_id: i32) -> bool {
    do_mapa(c, world_id).is_some_and(|m| m.limites.iter().any(|t| t == "nofly"))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_mapa_1_voa_e_as_instancias_nao() {
        for c in [CatalogoDeLimites::V126, CatalogoDeLimites::V155] {
            assert!(do_mapa(c, 1).is_some(), "{c:?}");
            assert!(!sem_voo(c, 1), "{c:?}: o mundo 1 deixa voar");
        }
        // 1.2.6: [World_is01] `limit = nothrow;nofly;allow-root;nomount;use-save-point;`.
        let is01 = catalogo(CatalogoDeLimites::V126).iter().find(|m| m.secao == "World_is01").unwrap();
        assert!(sem_voo(CatalogoDeLimites::V126, is01.world_id));
        assert_eq!(catalogo(CatalogoDeLimites::V126).len(), 43);
        assert_eq!(catalogo(CatalogoDeLimites::V155).len(), 79);
    }
}
