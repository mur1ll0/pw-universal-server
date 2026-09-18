//! As armaduras (`ARMOR_ESSENCE`) e os acessórios (`DECORATION_ESSENCE`) do
//! `elements.data`, com o que o **cliente** precisa saber de cada um.
//!
//! # Por que isto existe
//!
//! É o mesmo defeito da arma vermelha (item 38), esperando a primeira peça de armadura.
//!
//! Até agora só **arma** ganhava bloco de dados no `OWN_ITEM_INFO` (40): a ficha saía do
//! [`crate::armas`], e todo o resto ia sem bloco. Sem bloco, `CECIvtrEquip::SetItemInfo`
//! (`EC_IvtrEquip.cpp:178-181`) retorna na primeira linha e **nada** é preenchido — o
//! `m_iProfReq` fica no zero do construtor (`EC_IvtrEquip.cpp:74`). E `CanUseEquipment`
//! (`EC_HostPlayer.cpp:4953-4959`) faz, para `ICID_ARMOR` e `ICID_DECORATION`:
//!
//! ```cpp
//! if (!(pEquip->GetProfessionRequirement() & (1 << m_iProfession)))
//!     iReason = 3;
//! ```
//!
//! Zero recusa todas as classes, e item recusado é desenhado em `A3DCOLORRGB(192, 0, 0)`.
//! A primeira peça de armadura que o Murillo equipasse apareceria vermelha.
//!
//! ## Uma correção à fila de trabalho do item 40
//!
//! A fila dizia que o cliente cairia no `CECIvtrArmor::DefaultInfo`
//! (`EC_IvtrArmor.cpp:184-192`) e que o problema era aquele método não preencher
//! `m_iProfReq`. Está mais perto do que se pensava: no cliente 1.5.5, **`DefaultInfo()`
//! não é chamado em lugar nenhum** para armadura — o único `DefaultInfo()` do
//! `ElementClient` inteiro está em `EC_IvtrFashion.cpp:83`. Sem bloco, portanto, não é só
//! a profissão que fica zerada: nível, força, reputação e durabilidade também. A
//! conclusão prática é a mesma, e a correção também.
//!
//! Ela dizia também que o `id_sub_type` precisava viajar, "para o cliente saber em que
//! slot a peça entra". Não precisa, e não há onde: o cliente lê o subtipo do
//! `elements.data` **dele**, pelo id do item, no próprio construtor
//! (`EC_IvtrArmor.cpp:66-73`: `m_pDBSubType = get_data_ptr(m_pDBEssence->id_sub_type)`,
//! e daí `m_i64EquipMask = m_pDBSubType->equip_mask`). O `equip_mask` que o servidor
//! original monta em `generate_armor` fica no `item_data`, que é registro interno do
//! servidor e não sai na rede. A `IVTR_ESSENCE_ARMOR` não tem campo de subtipo.
//!
//! # A autoridade do layout
//!
//! `generate_armor` e `generate_decoration`
//! (`EvolvedPWServer/cgame/gs/template/generate_item_temp.h:490-556` e `772-830`) montam
//! byte a byte o que este módulo reproduz — é o gerador do servidor original, o outro
//! lado do `SetItemInfo` do cliente.

use crate::generic_elements::{GenericElementsData, Record};
use pw_core::ESCOLAS_MAGICAS;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// O cabeçalho comum a todo equipamento: a `prerequisition` do servidor original
/// (`gs/item/equip_item.h:230-238`), que o cliente lê em `CECIvtrEquip::SetItemInfo`
/// (`EC_IvtrEquip.cpp:186-192`) nesta ordem exata.
///
/// A ordem tem uma pegadinha: **vitalidade vem antes de agilidade**. O nome no arquivo é
/// `require_tili`, transliteração de 体力.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RequisitosDoEquipamento {
    /// `character_combo_id` — máscara de classes, um bit por classe. **Zero recusa todo
    /// mundo.**
    pub classes_permitidas: i32,
    pub nivel_exigido: i16,
    pub forca_exigida: i16,
    pub vitalidade_exigida: i16,
    pub agilidade_exigida: i16,
    pub energia_exigida: i16,
    pub reputacao_exigida: i32,
    /// `durability_min` — a durabilidade de fábrica, na escala do arquivo.
    pub durabilidade: i32,
}

impl RequisitosDoEquipamento {
    fn do_registro(reg: &Record) -> Self {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        Self {
            classes_permitidas: i("character_combo_id"),
            nivel_exigido: i("require_level") as i16,
            forca_exigida: i("require_strength") as i16,
            vitalidade_exigida: i("require_tili") as i16,
            agilidade_exigida: i("require_agility") as i16,
            energia_exigida: i("require_energy") as i16,
            reputacao_exigida: i("require_reputation"),
            durabilidade: i("durability_min"),
        }
    }
}

/// Uma armadura, com os campos que viajam para o cliente.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TemplateDeArmadura {
    pub id: u32,
    pub tipo_maior: i32,
    /// `id_sub_type` — a peça (elmo, peito, calça, botas…). **Não viaja**: o cliente lê o
    /// subtipo do `elements.data` dele. Fica aqui porque é o que responde "em que slot
    /// esta peça entra" quando o *servidor* precisa saber.
    pub tipo_menor: i32,
    pub requisitos: RequisitosDoEquipamento,
    /// `defence_low` — a defesa física.
    pub defesa: i32,
    /// `armor_enhance_low` — a evasão que a peça acrescenta.
    pub evasao: i32,
    pub mp_extra: i32,
    pub hp_extra: i32,
    /// `magic_defences[5].low`, na ordem Metal, Madeira, Água, Fogo, Terra.
    pub resistencias: [i32; ESCOLAS_MAGICAS],
}

/// Um acessório: anel, colar, cinto, patuá.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TemplateDeDecoracao {
    pub id: u32,
    pub tipo_maior: i32,
    pub tipo_menor: i32,
    pub requisitos: RequisitosDoEquipamento,
    pub dano: i32,
    pub dano_magico: i32,
    pub defesa: i32,
    pub evasao: i32,
    pub resistencias: [i32; ESCOLAS_MAGICAS],
}

pub type TabelaDeArmaduras = HashMap<u32, TemplateDeArmadura>;
pub type TabelaDeDecoracoes = HashMap<u32, TemplateDeDecoracao>;

/// As cinco resistências mágicas de um registro.
///
/// O original sorteia entre `low` e `high` (`generate_magic_defense`); sem sistema de
/// sorteio vale o mínimo, pela mesma razão que o [`crate::armas`] usa o mínimo do dano:
/// é o valor que o item de loja traz, e errar para baixo não tranca nada.
fn resistencias(reg: &Record) -> [i32; ESCOLAS_MAGICAS] {
    let mut r = [0i32; ESCOLAS_MAGICAS];
    for (n, destino) in r.iter_mut().enumerate() {
        *destino = reg
            .get(&format!("magic_defences_{}_low", n + 1))
            .and_then(|v| v.as_i32())
            .unwrap_or(0);
    }
    r
}

/// Lê `ARMOR_ESSENCE` do `elements.data` já decodificado.
///
/// Tabela vazia quando o arquivo não tem a tabela — é o caso do 1.2.6/v7, que ainda usa o
/// leitor tipado antigo.
pub fn carregar_armaduras(elements: &GenericElementsData) -> TabelaDeArmaduras {
    let mut tabela = TabelaDeArmaduras::default();

    for reg in elements.get("ARMOR_ESSENCE") {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let id = i("ID");
        if id <= 0 {
            continue;
        }

        tabela.insert(
            id as u32,
            TemplateDeArmadura {
                id: id as u32,
                tipo_maior: i("id_major_type"),
                tipo_menor: i("id_sub_type"),
                requisitos: RequisitosDoEquipamento::do_registro(reg),
                defesa: i("defence_low"),
                evasao: i("armor_enhance_low"),
                mp_extra: i("mp_enhance_low"),
                hp_extra: i("hp_enhance_low"),
                resistencias: resistencias(reg),
            },
        );
    }

    if !tabela.is_empty() {
        info!("armaduras: {} carregadas do ARMOR_ESSENCE", tabela.len());
    }
    tabela
}

/// Lê `DECORATION_ESSENCE` do `elements.data` já decodificado.
pub fn carregar_decoracoes(elements: &GenericElementsData) -> TabelaDeDecoracoes {
    let mut tabela = TabelaDeDecoracoes::default();

    for reg in elements.get("DECORATION_ESSENCE") {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let id = i("ID");
        if id <= 0 {
            continue;
        }

        tabela.insert(
            id as u32,
            TemplateDeDecoracao {
                id: id as u32,
                tipo_maior: i("id_major_type"),
                tipo_menor: i("id_sub_type"),
                requisitos: RequisitosDoEquipamento::do_registro(reg),
                dano: i("damage_low"),
                dano_magico: i("magic_damage_low"),
                defesa: i("defence_low"),
                evasao: i("armor_enhance_low"),
                resistencias: resistencias(reg),
            },
        );
    }

    if !tabela.is_empty() {
        info!(
            "decorações: {} carregadas do DECORATION_ESSENCE",
            tabela.len()
        );
    }
    tabela
}

impl From<&TemplateDeArmadura> for pw_core::FichaDaArmadura {
    fn from(a: &TemplateDeArmadura) -> Self {
        Self {
            classes_permitidas: a.requisitos.classes_permitidas,
            nivel_exigido: a.requisitos.nivel_exigido,
            forca_exigida: a.requisitos.forca_exigida,
            vitalidade_exigida: a.requisitos.vitalidade_exigida,
            agilidade_exigida: a.requisitos.agilidade_exigida,
            energia_exigida: a.requisitos.energia_exigida,
            defesa: a.defesa,
            evasao: a.evasao,
            mp_extra: a.mp_extra,
            hp_extra: a.hp_extra,
            resistencias: a.resistencias,
        }
    }
}

impl From<&TemplateDeDecoracao> for pw_core::FichaDeDecoracao {
    fn from(d: &TemplateDeDecoracao) -> Self {
        Self {
            classes_permitidas: d.requisitos.classes_permitidas,
            nivel_exigido: d.requisitos.nivel_exigido,
            forca_exigida: d.requisitos.forca_exigida,
            vitalidade_exigida: d.requisitos.vitalidade_exigida,
            agilidade_exigida: d.requisitos.agilidade_exigida,
            energia_exigida: d.requisitos.energia_exigida,
            dano: d.dano,
            dano_magico: d.dano_magico,
            defesa: d.defesa,
            evasao: d.evasao,
            resistencias: d.resistencias,
        }
    }
}

/// As três tabelas de equipamento do realm, e a busca que decide qual delas responde por
/// um item.
///
/// Existe porque quem monta o `OWN_ITEM_INFO` tem um id de item na mão e precisa da ficha
/// certa — e "qual família" é conhecimento sobre o `elements.data`, não sobre o formato de
/// rede. Um id vive em **uma** dessas tabelas: são espaços de essência distintos no
/// arquivo.
#[derive(Debug, Clone, Default)]
pub struct TabelasDeEquipamento {
    pub armas: crate::armas::TabelaDeArmas,
    pub armaduras: TabelaDeArmaduras,
    pub decoracoes: TabelaDeDecoracoes,
    pub municoes: crate::armas::TabelaDeMunicoes,
}

impl TabelasDeEquipamento {
    pub fn carregar(elements: &GenericElementsData) -> Self {
        Self {
            armas: crate::armas::carregar(elements),
            armaduras: carregar_armaduras(elements),
            decoracoes: carregar_decoracoes(elements),
            municoes: crate::armas::carregar_municoes(elements),
        }
    }

    /// A ficha que viaja com o item, ou `None` quando o id não é de equipamento — ou
    /// quando o realm não tem as tabelas (1.2.6/v7). `None` faz o comando ir sem bloco.
    pub fn ficha(&self, item_id: u32) -> Option<pw_core::FichaDoEquipamento> {
        use pw_core::FichaDoEquipamento as F;

        if let Some(a) = self.armas.get(&item_id) {
            return Some(F::Arma(a.into()));
        }
        if let Some(a) = self.armaduras.get(&item_id) {
            return Some(F::Armadura(a.into()));
        }
        if let Some(d) = self.decoracoes.get(&item_id) {
            return Some(F::Decoracao(d.into()));
        }
        self.municoes.get(&item_id).map(|m| F::Municao(*m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_as_tabelas_no_arquivo_a_carga_e_vazia() {
        let vazio = GenericElementsData {
            tables: HashMap::new(),
            version: 0,
        };
        let t = TabelasDeEquipamento::carregar(&vazio);
        assert!(t.armaduras.is_empty());
        assert!(t.decoracoes.is_empty());
        assert!(t.ficha(2251).is_none(), "sem tabela, o item vai sem bloco");
    }
}
