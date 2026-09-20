//! Os atributos por classe de personagem, da tabela `CHARRACTER_CLASS_CONFIG` do
//! `elements.data`. (O nome tem o erro de digitação do original — dois "R".)
//!
//! # Origem
//!
//! `player_template::__LoadDataFromDataMan` (`cgame/gs/playertemplate.cpp`), que percorre
//! `DT_CHARRACTER_CLASS_CONFIG` no `ID_SPACE_CONFIG` e preenche `_class_list` e
//! `_template_list`. Só entram aqui os campos que aquele laço lê.
//!
//! # Para que serve agora
//!
//! Duas linhas do cálculo de combate dependem desta tabela e de mais nada:
//!
//! ```text
//! GetBasicAttackRate(cls, agi) = agi_attack[cls] * agi   // a precisão
//! GetBasicArmor(cls, agi)      = agi_armor[cls]  * agi   // a evasão
//! ```
//!
//! Sem elas o lado do jogador do combate não tem entrada nenhuma — era o que fazia a
//! fórmula antiga inventar um número.
//!
//! # O que isto **não** é
//!
//! Não é o sistema de atributos do jogador. No original, `attack` e `armor` finais saem
//! de `player_template::UpdateAttack`/`UpdateDefense`, que somam equipamento
//! (`_cur_item`), pontos de encantamento (`_en_point`), percentuais (`_en_percent`) e
//! buffs por cima desta base. Nada disso existe do nosso lado, e este módulo não finge
//! que existe: ele entrega a **base**, e quem chamar sabe que é base.

use crate::generic_elements::{FieldValue, GenericElementsData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Quantas classes o 1.5.5 tem — `USER_CLASS_COUNT` em `cskill/skill/skill.h`.
pub const TOTAL_DE_CLASSES: usize = 12;

/// Os atributos de uma classe.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfigDeClasse {
    /// `character_class_id` — 0 a 11, na ordem de `USER_CLASS_*`.
    pub classe: i32,
    /// `agi_attack` → `GetBasicAttackRate`: precisão por ponto de agilidade.
    pub ataque_por_agilidade: i32,
    /// `agi_armor` → `GetBasicArmor`: evasão por ponto de agilidade.
    pub armadura_por_agilidade: i32,
    /// `crit_rate`, em pontos percentuais.
    pub chance_de_critico: i32,
    /// `vit_hp` / `eng_mp`: vida por vitalidade e mana por energia.
    pub vida_por_vitalidade: i32,
    pub mana_por_energia: i32,
    /// `lvlup_hp` … `lvlup_magicdefence`: o ganho por nível. São `float` no arquivo.
    pub vida_por_nivel: f32,
    pub mana_por_nivel: f32,
    pub dano_por_nivel: f32,
    pub dano_magico_por_nivel: f32,
    pub defesa_por_nivel: f32,
    pub resistencia_por_nivel: f32,
    /// `attack_speed` em segundos, e `attack_range` em metros. O original converte a
    /// velocidade para *ticks* de 50 ms (`(int)(attack_speed * 20)`, com piso de 30
    /// quando dá zero ou menos) — a conversão fica em [`Self::ataque_em_ticks`].
    pub velocidade_de_ataque: f32,
    pub alcance_de_ataque: f32,
    /// `angro_increase` — quanto de **chi** cada golpe normal acrescenta
    /// (`player_template::__LoadDataFromDataMan`: `_class_list[cls].ap_per_hit =
    /// config.angro_increase`, `gs/playertemplate.cpp:286`; o ganho em si está em
    /// `gplayer_imp::DoAttack`, `player.cpp:3091-3093`). Arqueiro: 5.
    pub chi_por_golpe: i32,
    pub regeneracao_de_vida: i32,
    pub regeneracao_de_mana: i32,
    pub velocidade_andando: f32,
    pub velocidade_correndo: f32,
    pub velocidade_nadando: f32,
    pub velocidade_voando: f32,
    /// `faction` / `enemy_faction`.
    pub faccao: i32,
    pub faccao_inimiga: i32,
}

impl ConfigDeClasse {
    /// `GetBasicAttackRate(cls, agi)` — a precisão base, antes de equipamento.
    pub fn precisao_base(&self, agilidade: i32) -> i32 {
        self.ataque_por_agilidade * agilidade
    }

    /// `GetBasicArmor(cls, agi)` — a evasão base, antes de equipamento.
    pub fn evasao_base(&self, agilidade: i32) -> i32 {
        self.armadura_por_agilidade * agilidade
    }

    /// `_template_list[cls].attack_speed = (int)(config.attack_speed * 20)`, com o piso
    /// de 30 que o original aplica quando o resultado sai zero ou negativo.
    pub fn ataque_em_ticks(&self) -> i32 {
        let t = (self.velocidade_de_ataque * 20.0) as i32;
        if t <= 0 {
            30
        } else {
            t
        }
    }
}

/// A tabela inteira, por id de classe.
#[derive(Debug, Clone, Default)]
pub struct TabelaDeClasses {
    pub classes: HashMap<i32, ConfigDeClasse>,
}

impl TabelaDeClasses {
    pub fn get(&self, classe: i32) -> Option<&ConfigDeClasse> {
        self.classes.get(&classe)
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

/// Lê `CHARRACTER_CLASS_CONFIG` do `elements.data` já decodificado.
///
/// Devolve tabela vazia quando o arquivo não tem a tabela — é o caso do 1.2.6/v7, que
/// ainda usa o leitor tipado antigo.
pub fn carregar(elements: &GenericElementsData) -> TabelaDeClasses {
    let registros = elements.get("CHARRACTER_CLASS_CONFIG");
    if registros.is_empty() {
        return TabelaDeClasses::default();
    }

    let mut tabela = TabelaDeClasses::default();

    for reg in registros {
        let i = |n: &str| reg.get(n).and_then(|v| v.as_i32()).unwrap_or(0);
        let f = |n: &str| match reg.get(n) {
            Some(FieldValue::Float(v)) => *v,
            Some(FieldValue::Int(v)) => *v as f32,
            _ => 0.0,
        };

        let classe = i("character_class_id");
        // O original recusa a tabela inteira com `ASSERT(false); return false;` quando o
        // id sai da faixa. Aqui a entrada é descartada e as outras continuam: um realm
        // com uma classe a mais não deve derrubar o carregamento das onze que prestam.
        if !(0..TOTAL_DE_CLASSES as i32).contains(&classe) {
            continue;
        }

        tabela.classes.insert(
            classe,
            ConfigDeClasse {
                classe,
                ataque_por_agilidade: i("agi_attack"),
                armadura_por_agilidade: i("agi_armor"),
                chance_de_critico: i("crit_rate"),
                vida_por_vitalidade: i("vit_hp"),
                mana_por_energia: i("eng_mp"),
                vida_por_nivel: f("lvlup_hp"),
                mana_por_nivel: f("lvlup_mp"),
                dano_por_nivel: f("lvlup_dmg"),
                dano_magico_por_nivel: f("lvlup_magic"),
                defesa_por_nivel: f("lvlup_defense"),
                resistencia_por_nivel: f("lvlup_magicdefence"),
                velocidade_de_ataque: f("attack_speed"),
                alcance_de_ataque: f("attack_range"),
                chi_por_golpe: i("angro_increase"),
                regeneracao_de_vida: i("hp_gen"),
                regeneracao_de_mana: i("mp_gen"),
                velocidade_andando: f("walk_speed"),
                velocidade_correndo: f("run_speed"),
                velocidade_nadando: f("swim_speed"),
                velocidade_voando: f("fly_speed"),
                faccao: i("faction"),
                faccao_inimiga: i("enemy_faction"),
            },
        );
    }

    info!(
        "CHARRACTER_CLASS_CONFIG: {} classes carregadas",
        tabela.classes.len()
    );
    tabela
}
