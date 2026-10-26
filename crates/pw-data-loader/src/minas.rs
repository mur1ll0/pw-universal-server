//! As minas do realm (`MINE_ESSENCE`), já com as regras de carga do servidor original.
//!
//! O `gs` não usa o registro cru: `npc_stubs_manager` monta um `mine_info` com limites e
//! recusas (`npcgenerator.cpp:1280-1365`). Este módulo faz a mesma montagem, para que o
//! mundo leia os números que o original usaria.

use crate::generic_elements::{FieldValue, GenericElementsData, Record};
use std::collections::HashMap;

/// Um material que a mina pode dar (`materials[i]`, só os de probabilidade positiva).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialDaMina {
    pub item: u32,
    pub probabilidade: f32,
    /// Validade em segundos (0 = sem validade).
    pub vida: i32,
}

/// `npc_template::__mine_info`.
#[derive(Debug, Clone, PartialEq)]
pub struct MinaDoRealm {
    pub id: u32,
    /// `num1` e `num2`, com `probability2` de sair o segundo.
    pub quantidade: u32,
    pub quantidade_bonus: u32,
    pub chance_bonus: f32,
    /// Segundos.
    pub tempo_minimo: u32,
    pub tempo_maximo: u32,
    /// `level_required`.
    pub nivel: i32,
    pub exp: i32,
    pub sp: i32,
    pub materiais: Vec<MaterialDaMina>,
    /// `id_equipment_required` — a ferramenta (0 = nenhuma).
    pub ferramenta: i32,
    pub missao_de_entrada: u32,
    pub missao_de_saida: u32,
    pub ininterrupta: bool,
    /// `permenent` — não some ao ser colhida.
    pub permanente: bool,
    pub gasta_ferramenta: bool,
    /// `gather_dist`, entre 4 e 20.
    pub distancia: f32,
    /// `max_gatherer`, entre 1 e 20.
    pub coletores: u32,
    /// `material_gain_ratio`.
    pub chance_de_sucesso: f32,
    /// `npcgen_1..4` do `MINE_ESSENCE`: os monstros que **nascem ao colher**
    /// (`(template, quantidade, raio, vida em segundos)`). É assim que a Flor de Safira
    /// (44566) solta o Guardião de Almas (44608) que guarda o Estame da missão 31779.
    pub monstros_ao_colher: Vec<(u32, u32, f32, i32)>,
    pub tipo: i32,
}

pub type TabelaDeMinas = HashMap<u32, MinaDoRealm>;

/// Lê `MINE_ESSENCE` com as recusas de `npcgenerator.cpp:1296-1365` (as minas que o GS
/// recusa ficam de fora; o motivo de cada uma sai de [`montar`]).
pub fn carregar(elements: &GenericElementsData) -> TabelaDeMinas {
    let mut tabela = TabelaDeMinas::new();
    for reg in elements.get("MINE_ESSENCE") {
        if let Ok(m) = montar(reg) {
            tabela.insert(m.id, m);
        }
    }
    tabela
}

/// Monta o `mine_info` de um registro de `MINE_ESSENCE` como `npc_stubs_manager` faz
/// (`npcgenerator.cpp:1280-1365`), ou devolve o motivo por que o GS recusa a mina (a mina
/// recusada não entra no `npc_stubs_manager` e a área de recurso que a cita não a faz nascer,
/// `npcgenerator.cpp:3913-3917`). No v7, `material_gain_ratio` ainda não está identificado;
/// ver a derivação abaixo.
pub fn montar(reg: &Record) -> Result<MinaDoRealm, String> {
    let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
    let f = |n: &str| match reg.get(n) {
        Some(FieldValue::Float(v)) => *v,
        Some(FieldValue::Int(v)) => *v as f32,
        _ => 0.0,
    };
    let id = i("ID");
    if id <= 0 {
        return Err("sem id".into());
    }
    let (quantidade, tempo_minimo, tempo_maximo) = (i("num1"), i("time_min"), i("time_max"));
    let (entrada, saida) = (i("task_in"), i("task_out"));
    // `npcgenerator.cpp:1297-1306`.
    if quantidade <= 0 && (entrada == 0 || saida == 0) {
        return Err("quantidade (num1) ≤ 0 e não é mina de missão (task_in e task_out) — npcgenerator.cpp:1297-1299".into());
    }
    if tempo_minimo > 1024 || tempo_maximo > 1024 || tempo_minimo > tempo_maximo || tempo_maximo <= 0 {
        return Err(format!(
            "tempo de coleta {tempo_minimo}–{tempo_maximo} s fora do que o GS aceita (máx. ≤ 1024, mín. ≤ máx., máx. > 0) — npcgenerator.cpp:1299-1303"
        ));
    }
    let mut materiais = Vec::new();
    let mut soma = 0.0f32;
    for k in 1..=16 {
        let p = f(&format!("materials_{k}_probability"));
        if p > 0.0 {
            materiais.push(MaterialDaMina {
                item: i(&format!("materials_{k}_id")) as u32,
                probabilidade: p,
                vida: i(&format!("materials_{k}_life")),
            });
            soma += p;
        }
    }
    // `npcgenerator.cpp:1308-1325`.
    if materiais.is_empty() {
        return Err("nenhum material com chance > 0 — npcgenerator.cpp:1323-1327".into());
    }
    if (soma - 1.0).abs() >= 1e-5 {
        return Err(format!("as chances dos materiais somam {soma} e o GS exige 1 — npcgenerator.cpp:1323-1327"));
    }
    if (entrada != 0) != (saida != 0) {
        return Err("mina de missão com task_in sem task_out (ou o contrário) — npcgenerator.cpp:1328-1332".into());
    }
    // O v7 termina antes de `material_gain_ratio`: o catálogo registra os
    // 48 B finais como opacos. Enquanto não há evidência do multiplicador
    // antigo, usa-se a soma das probabilidades do próprio registro (já
    // exigida como 1 acima), sem introduzir uma constante de jogo.
    // Evidência: elements.data v7 do realm_126, MINE_ESSENCE de 452 B;
    // `source_client_153/CCommon/ExpTypes.h:3597-3655` mostra os campos
    // que só a estrutura posterior nomeia.
    let chance_de_sucesso = if reg.contains_key("material_gain_ratio") { f("material_gain_ratio") } else { soma };
    if chance_de_sucesso <= 0.0 {
        return Err("chance de sucesso da coleta (material_gain_ratio) ≤ 0 — npcgenerator.cpp:1353-1357".into());
    }
    let tipo = i("mine_type");
    if !(0..=1).contains(&tipo) {
        return Err(format!("tipo de mina {tipo} fora de 0–1 — npcgenerator.cpp:1359-1363"));
    }
    Ok(MinaDoRealm {
        id: id as u32,
        quantidade: quantidade.max(0) as u32,
        quantidade_bonus: i("num2").max(0) as u32,
        chance_bonus: f("probability2"),
        tempo_minimo: tempo_minimo as u32,
        tempo_maximo: tempo_maximo as u32,
        nivel: i("level_required"),
        exp: i("exp"),
        sp: i("skillpoint"),
        materiais,
        ferramenta: i("id_equipment_required"),
        missao_de_entrada: entrada as u32,
        missao_de_saida: saida as u32,
        ininterrupta: i("uninterruptable") != 0,
        permanente: i("permenent") != 0,
        gasta_ferramenta: i("eliminate_tool") != 0,
        distancia: f("gather_dist").clamp(4.0, 20.0),
        coletores: i("max_gatherer").clamp(1, 20) as u32,
        chance_de_sucesso,
        tipo,
        monstros_ao_colher: (1..=4)
            .filter_map(|k| {
                let tid = i(&format!("npcgen_{k}_id_monster"));
                let num = i(&format!("npcgen_{k}_num"));
                (tid > 0 && num > 0).then(|| (tid as u32, num as u32, f(&format!("npcgen_{k}_radius")), i(&format!("npcgen_{k}_life_time"))))
            })
            .collect(),
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn registro(campos: &[(&str, FieldValue)]) -> Record {
        campos.iter().map(|(n, v)| (n.to_string(), v.clone())).collect()
    }

    #[test]
    fn diz_por_que_o_gs_recusa_a_mina() {
        let base = || {
            registro(&[
                ("ID", FieldValue::Int(9)),
                ("num1", FieldValue::Int(1)),
                ("time_min", FieldValue::Int(5)),
                ("time_max", FieldValue::Int(10)),
                ("materials_1_id", FieldValue::Int(795)),
                ("materials_1_probability", FieldValue::Float(1.0)),
                ("material_gain_ratio", FieldValue::Float(1.0)),
            ])
        };
        assert!(montar(&base()).is_ok());
        let mut r = base();
        r.insert("materials_1_probability".into(), FieldValue::Float(0.5));
        assert!(montar(&r).unwrap_err().contains("somam 0.5"));
        let mut r = base();
        r.insert("time_max".into(), FieldValue::Int(2000));
        assert!(montar(&r).unwrap_err().contains("1299-1303"));
        let mut r = base();
        r.insert("task_in".into(), FieldValue::Int(1));
        assert!(montar(&r).unwrap_err().contains("task_in"));
    }
}
