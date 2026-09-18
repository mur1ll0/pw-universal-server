//! As armas do `elements.data` (`WEAPON_ESSENCE`), com o que o **cliente** precisa saber
//! de cada uma.
//!
//! # Por que isto existe
//!
//! O bloco de dados que acompanha cada item no `OWN_ITEM_INFO` (40) não é enfeite: é dele
//! que `CECIvtrEquip::SetItemInfo` (`EC_IvtrEquip.cpp:176-200`) tira **os requisitos e a
//! ficha da arma**, e é com eles que `CECHostPlayer::CanUseEquipment`
//! (`EC_HostPlayer.cpp:4894-4980`) decide se o jogador pode usar o que está equipado.
//! Item recusado é desenhado em `A3DCOLORRGB(192, 0, 0)` — vermelho.
//!
//! Até 2026-09-09 esse bloco era montado por uma **tabela chumbada de quatro itens** no
//! codificador, com um `_ =>` genérico para todo o resto. A Varinha do Sacerdote (2251)
//! caía no genérico, que declarava `weapon_type = 1` (`WEAPONTYPE_RANGE`); o cliente
//! concluía que era arma de munição, não achava flecha nenhuma e recusava — a "arma
//! vermelha" que apareceu em jogo por quatro sessões seguidas. O tooltip relatado batia
//! campo a campo com aquele genérico: alcance 3.50, sem linha de força, sem linha de
//! profissão.
//!
//! Agora o bloco sai do arquivo.

use crate::generic_elements::{FieldValue, GenericElementsData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// `WEAPONTYPE_MELEE` do `EC_IvtrTypes.h:166`.
pub const CORPO_A_CORPO: i16 = 0;
/// `WEAPONTYPE_RANGE` do `EC_IvtrTypes.h:167` — arma que consome munição.
pub const LONGO_ALCANCE: i16 = 1;

/// Uma arma, com os campos que viajam para o cliente.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TemplateDeArma {
    pub id: u32,
    /// `id_major_type` — o tipo maior (Espada, Magia, Longo Alcance…). É por ele que
    /// `ElementSkill::Condition` decide se a habilidade pode ser conjurada.
    pub tipo_maior: i32,
    /// `character_combo_id` — máscara de classes que podem equipar, um bit por classe.
    ///
    /// **Zero aqui recusa todo mundo**: `CanUseEquipment` faz
    /// `!(GetProfessionRequirement() & (1 << profissão))`.
    pub classes_permitidas: i32,
    pub nivel_exigido: i16,
    pub forca_exigida: i16,
    pub agilidade_exigida: i16,
    /// `require_tili` — a vitalidade. O nome no arquivo é a transliteração de 体力.
    pub vitalidade_exigida: i16,
    pub energia_exigida: i16,
    pub reputacao_exigida: i32,
    /// `require_projectile` — o id do tipo de munição, ou 0.
    pub municao_exigida: i32,
    /// `level` — o nível da arma (`weapon_level` no item).
    pub nivel: i32,
    /// `attack_speed` do item, em *ticks* de 50 ms: `(int)(subtipo.attack_speed*20 + 0.1) +
    /// (índice − 2)`, com o índice sorteado entre `probability_slowest..fastest`
    /// (`generate_item_temp.h:337-344`). Sem sorteio guardado no item, vale o índice 2
    /// ("normal", que é também o de probabilidade 1 nas armas iniciais). 0 sem subtipo.
    pub velocidade_em_ticks: i32,
    /// `short_range_mode`: 0 é arma de longo alcance. Ver [`Self::tipo_de_arma`].
    pub modo_de_alcance: i32,
    pub dano_minimo: i32,
    pub dano_maximo: i32,
    pub dano_magico_minimo: i32,
    pub dano_magico_maximo: i32,
    pub alcance: f32,
    /// `durability_min` — a durabilidade **de fábrica**, na escala do arquivo. O cliente
    /// multiplica por `ENDURANCE_SCALE` ao montar o item a partir do `elements.data`; o
    /// que viaja no bloco já vai na escala do cliente.
    pub durabilidade: i32,
}

impl TemplateDeArma {
    /// O `weapon_type` que o cliente espera: `WEAPONTYPE_RANGE` quando a arma é de longo
    /// alcance, `WEAPONTYPE_MELEE` no resto.
    ///
    /// # Como este mapeamento foi decidido
    ///
    /// `IsRangeWeapon()` é `m_Essence.weapon_type == WEAPONTYPE_RANGE`
    /// (`EC_IvtrWeapon.h:88`), e é **só** para arma de longo alcance que
    /// `CanUseEquipment` cobra munição. No `elements.data` do realm, entre as 2.741 armas:
    ///
    /// | `short_range_mode` | exige munição | quantas |
    /// | ---: | :--- | ---: |
    /// | 1 | não | 2.187 |
    /// | 0 | **sim** | 326 |
    /// | 2 | não | 222 |
    /// | 0 | não | 5 |
    /// | 1 | sim | 1 |
    ///
    /// Ou seja, `short_range_mode == 0` é a marca de longo alcance, e praticamente todas
    /// essas são do tipo maior 13 ("Longo Alcance"). As seis exceções vão como o arquivo
    /// diz: quem manda é o campo, não a nossa expectativa.
    pub fn tipo_de_arma(&self) -> i16 {
        if self.modo_de_alcance == 0 {
            LONGO_ALCANCE
        } else {
            CORPO_A_CORPO
        }
    }
}

/// A tabela inteira, por id de item.
pub type TabelaDeArmas = HashMap<u32, TemplateDeArma>;

/// Lê `WEAPON_ESSENCE` do `elements.data` já decodificado.
///
/// Tabela vazia quando o arquivo não tem a tabela — é o caso do 1.2.6/v7, que ainda usa o
/// leitor tipado antigo.
pub fn carregar(elements: &GenericElementsData) -> TabelaDeArmas {
    let registros = elements.get("WEAPON_ESSENCE");
    if registros.is_empty() {
        return TabelaDeArmas::default();
    }

    let mut tabela = TabelaDeArmas::default();
    let subtipos: HashMap<i32, f32> = elements
        .get("WEAPON_SUB_TYPE")
        .iter()
        .filter_map(|r| {
            let id = r.get("ID").and_then(|v| v.as_i32())?;
            let v = match r.get("attack_speed") {
                Some(FieldValue::Float(v)) => *v,
                Some(FieldValue::Int(v)) => *v as f32,
                _ => return None,
            };
            Some((id, v))
        })
        .collect();

    for reg in registros {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let f = |n: &str| match reg.get(n) {
            Some(FieldValue::Float(v)) => *v,
            Some(FieldValue::Int(v)) => *v as f32,
            _ => 0.0,
        };

        let id = i("ID");
        if id <= 0 {
            continue;
        }

        tabela.insert(
            id as u32,
            TemplateDeArma {
                id: id as u32,
                tipo_maior: i("id_major_type"),
                classes_permitidas: i("character_combo_id"),
                nivel_exigido: i("require_level") as i16,
                forca_exigida: i("require_strength") as i16,
                agilidade_exigida: i("require_agility") as i16,
                vitalidade_exigida: i("require_tili") as i16,
                energia_exigida: i("require_energy") as i16,
                reputacao_exigida: i("require_reputation"),
                municao_exigida: i("require_projectile"),
                nivel: i("level"),
                velocidade_em_ticks: subtipos.get(&i("id_sub_type")).map(|v| (v * 20.0 + 0.1) as i32).unwrap_or(0),
                modo_de_alcance: i("short_range_mode"),
                dano_minimo: i("damage_low"),
                // O arquivo tem `damage_high_min` e `damage_high_max` — o teto sorteado na
                // criação do item. Sem sistema de sorteio, vale o mínimo, que é o que o
                // item de loja traz.
                dano_maximo: i("damage_high_min"),
                dano_magico_minimo: i("magic_damage_low"),
                dano_magico_maximo: i("magic_damage_high_min"),
                alcance: f("attack_range"),
                durabilidade: i("durability_min"),
            },
        );
    }

    info!("armas: {} carregadas do WEAPON_ESSENCE", tabela.len());
    tabela
}

/// As munições do realm (`PROJECTILE_ESSENCE`), já na forma que viaja ao cliente.
pub type TabelaDeMunicoes = HashMap<u32, pw_core::FichaDaMunicao>;

/// Lê `PROJECTILE_ESSENCE`. Os campos são os que `generate_projectile` copia para a
/// essência do item (`generate_item_temp.h:622-626`).
pub fn carregar_municoes(elements: &GenericElementsData) -> TabelaDeMunicoes {
    let mut tabela = TabelaDeMunicoes::default();
    for reg in elements.get("PROJECTILE_ESSENCE") {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let id = i("ID");
        if id <= 0 {
            continue;
        }
        tabela.insert(
            id as u32,
            pw_core::FichaDaMunicao {
                tipo: i("type"),
                dano_extra: i("damage_enhance"),
                dano_extra_percentual: i("damage_scale_enhance"),
                nivel_minimo_da_arma: i("require_weapon_level_min"),
                nivel_maximo_da_arma: i("require_weapon_level_max"),
            },
        );
    }
    tabela
}

/// `QUIVER_ESSENCE`: a aljava é item de drop que vira munição — `generate_quiver`
/// (`generate_item_temp.h:650-667`) entrega o `id_projectile` com
/// `RandNormal(num_min, num_max)` unidades (uniforme na classe de sorteio normal,
/// `itemdataman.h:30`).
pub type TabelaDeAljavas = HashMap<u32, (u32, u32, u32)>;

pub fn carregar_aljavas(elements: &GenericElementsData) -> TabelaDeAljavas {
    elements
        .get("QUIVER_ESSENCE")
        .iter()
        .filter_map(|r| {
            let i = |n: &str| r.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
            (i("ID") > 0 && i("id_projectile") > 0).then(|| {
                (i("ID") as u32, (i("id_projectile") as u32, i("num_min").max(0) as u32, i("num_max").max(0) as u32))
            })
        })
        .collect()
}

/// A conversão para o que viaja ao cliente.
///
/// Fica aqui, e não no codificador, porque é conhecimento sobre o **arquivo**: quais
/// campos do `WEAPON_ESSENCE` correspondem a cada campo da ficha, e o que fazer com os
/// que não têm correspondência direta.
impl From<&TemplateDeArma> for pw_core::FichaDaArma {
    fn from(a: &TemplateDeArma) -> Self {
        Self {
            tipo_de_arma: a.tipo_de_arma(),
            classes_permitidas: a.classes_permitidas,
            nivel_exigido: a.nivel_exigido,
            forca_exigida: a.forca_exigida,
            vitalidade_exigida: a.vitalidade_exigida,
            agilidade_exigida: a.agilidade_exigida,
            energia_exigida: a.energia_exigida,
            municao_exigida: a.municao_exigida,
            tipo_maior: a.tipo_maior,
            nivel_da_arma: a.nivel,
            dano_minimo: a.dano_minimo,
            dano_maximo: a.dano_maximo,
            dano_magico_minimo: a.dano_magico_minimo,
            dano_magico_maximo: a.dano_magico_maximo,
            // O `WEAPON_ESSENCE` não tem velocidade de ataque por arma: quem a define é a
            // classe (`CHARRACTER_CLASS_CONFIG.attack_speed`). Zero aqui deixa o cliente
            // com a da classe, que é o comportamento certo enquanto não houver arma que
            // altere a cadência.
            velocidade_de_ataque: a.velocidade_em_ticks,
            alcance: a.alcance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_a_tabela_no_arquivo_a_carga_e_vazia() {
        let vazio = GenericElementsData {
            tables: HashMap::new(),
            version: 0,
        };
        assert!(carregar(&vazio).is_empty());
    }

    #[test]
    fn o_modo_de_alcance_zero_e_o_que_marca_arma_de_municao() {
        let mut arma = TemplateDeArma {
            id: 2251,
            tipo_maior: 292,
            classes_permitidas: 767,
            nivel_exigido: 1,
            forca_exigida: 5,
            agilidade_exigida: 0,
            vitalidade_exigida: 0,
            energia_exigida: 3,
            reputacao_exigida: 0,
            municao_exigida: 0,
            nivel: 1,
            velocidade_em_ticks: 0,
            modo_de_alcance: 1,
            dano_minimo: 3,
            dano_maximo: 3,
            dano_magico_minimo: 5,
            dano_magico_maximo: 5,
            alcance: 3.0,
            durabilidade: 14,
        };
        assert_eq!(
            arma.tipo_de_arma(),
            CORPO_A_CORPO,
            "a Varinha não pode ser de munição: era isso que a deixava vermelha"
        );

        arma.modo_de_alcance = 0;
        assert_eq!(arma.tipo_de_arma(), LONGO_ALCANCE);
    }
}
