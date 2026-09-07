use pw_data_loader::{TabelaDeClasses, TemplateDeMonstro};
use pw_core::{CharacterClass, Gender, Race, RoleId, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveBuff {
    pub buff_id: u32,
    pub level: u8,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
    pub tick_interval_ms: u32,
    pub tick_elapsed_ms: u32,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerEntity {
    pub role_id: RoleId,
    pub name: String,
    pub race: Race,
    pub cls: CharacterClass,
    pub gender: Gender,
    pub level: i32,
    pub cultivation: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub sp: i64,
    pub money: i64,
    
    // Atributos de Combate
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub energy: i32,
    pub def_phys: i32,
    pub def_metal: i32,
    pub def_wood: i32,
    pub def_water: i32,
    pub def_fire: i32,
    pub def_earth: i32,
    pub attack_min: i32,
    pub attack_max: i32,
    pub magic_attack_min: i32,
    pub magic_attack_max: i32,
    /// `_cur_prop.armor` — a evasão. No original vem de
    /// `GetBasicArmor(classe, agilidade) = agi_armor[classe] * agilidade`, mais
    /// equipamento; ver [`crate::entity::PlayerEntity::armadura_base`].
    pub armor: i32,
    /// `_cur_prop.attack` — a precisão. Mesma origem, com `agi_attack`.
    pub attack_rate: i32,
    /// `attack_degree` / `_defend_degree`. No original vêm de equipamento e habilidade
    /// passiva; sem esses sistemas ficam em zero, que é o valor neutro do cálculo.
    pub attack_degree: i32,
    pub defend_degree: i32,
    /// `crit_damage_bonus`, em pontos percentuais somados ao dobro base do crítico.
    pub crit_damage_bonus: i32,
    pub attack_speed: f32,
    pub move_speed: f32,
    pub crit_rate: f32,
    
    pub position: Vector3,
    pub target_id: Option<i64>,
    pub buffs: Vec<ActiveBuff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonsterEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub level: i32,
    pub hp: i64,
    pub max_hp: i64,
    pub mp: i32,
    pub max_mp: i32,
    /// `_cur_prop.defense` — reduz o dano físico recebido.
    pub def_phys: i32,
    /// `_cur_prop.armor` — a evasão, que entra na chance de o golpe acertar
    /// (`taxa / (taxa + armadura/2)`). Sem isto o acerto não pode ser calculado.
    pub armor: i32,
    /// `_cur_prop.attack` — a **precisão** do monstro, não o dano dele.
    pub attack_rate: i32,
    /// `_cur_prop.resistance[0..4]`: metal, madeira, água, fogo, terra. Substituiu o
    /// `def_magic` de valor único, que não tinha correspondente no original.
    pub resistances: [i32; 5],
    /// `attack_degree` / `_defend_degree`, que ajustam o dano no fim do cálculo.
    pub attack_degree: i32,
    pub defend_degree: i32,
    /// `_cur_prop.damage_low`/`damage_high` — a faixa de dano físico.
    pub attack_min: i32,
    pub attack_max: i32,
    /// `_cur_prop.addon_damage[0..4]`, na mesma ordem de `resistances`: as parcelas de
    /// dano elemental que o golpe normal do monstro carrega.
    pub magic_attack: [(i32, i32); 5],
    pub attack_range: f32,
    pub exp: i64,
    pub sp: i64,
    pub aipolicy_id: u32,
    pub drop_table_id: u32,
    
    pub position: Vector3,
    pub spawn_center: Vector3,
    pub move_speed: f32,
    pub is_dead: bool,
    pub respawn_timer_ms: u32,
    pub respawn_delay_ms: u32,
    
    pub target_id: Option<i64>,
    pub buffs: Vec<ActiveBuff>,
}

impl PlayerEntity {
    /// A precisão e a evasão base do jogador, do `CHARRACTER_CLASS_CONFIG` do
    /// `elements.data`: `agi_attack * agilidade` e `agi_armor * agilidade`
    /// (`player_template::GetBasicAttackRate` / `GetBasicArmor`).
    ///
    /// É **base**: o original soma equipamento, pontos de encantamento e percentuais por
    /// cima (`UpdateAttack` / `UpdateDefense`), e nada disso existe do nosso lado. Sem
    /// esta função, porém, os dois campos não teriam origem nenhuma — que era o estado
    /// anterior, com a fórmula de dano inventando o número.
    ///
    /// `None` quando o realm não tem a tabela (1.2.6/v7) ou a classe não está nela.
    pub fn precisao_e_evasao_base(
        classes: &TabelaDeClasses,
        classe: CharacterClass,
        agilidade: i32,
    ) -> Option<(i32, i32)> {
        let c = classes.get(classe as i32)?;
        Some((c.precisao_base(agilidade), c.evasao_base(agilidade)))
    }

    /// Preenche `attack_rate` e `armor` a partir da tabela de classes, quando ela existir.
    /// Deixa os valores como estavam quando não existir — o chamador decide se isso é
    /// aceitável para o realm dele.
    pub fn aplicar_atributos_de_classe(&mut self, classes: &TabelaDeClasses) -> bool {
        match Self::precisao_e_evasao_base(classes, self.cls, self.agility) {
            Some((precisao, evasao)) => {
                self.attack_rate = precisao;
                self.armor = evasao;
                true
            }
            None => false,
        }
    }
}

impl MonsterEntity {
    /// Instancia um monstro a partir do template do `elements.data`
    /// (`MONSTER_ESSENCE`), como `npcgenerator.cpp` faz no servidor original.
    ///
    /// Só os campos que este `MonsterEntity` tem são preenchidos; o template carrega
    /// bastante coisa a mais (as cinco resistências, dano mágico por classe, habilidades,
    /// raio de ódio, grau de ataque e defesa) que entra quando o combate e a IA reais
    /// forem portados — ver `pw_data_loader::monstros`.
    pub fn do_template(
        id: i64,
        modelo: &TemplateDeMonstro,
        posicao: Vector3,
        respawn_delay_ms: u32,
    ) -> Self {
        Self {
            id,
            template_id: modelo.id,
            name: modelo.nome.clone(),
            level: modelo.nivel,
            hp: modelo.vida as i64,
            max_hp: modelo.vida as i64,
            // O original fixa mana em 1 para monstro (`nt.bp.mp = 1`, `nt.ep.max_mp = 1`):
            // o custo de habilidade de monstro não sai de mana.
            mp: 1,
            max_mp: 1,
            def_phys: modelo.defesa,
            armor: modelo.armadura,
            attack_rate: modelo.taxa_de_ataque,
            resistances: modelo.resistencias,
            attack_degree: modelo.grau_de_ataque,
            defend_degree: modelo.grau_de_defesa,
            attack_min: modelo.dano_fisico.minimo,
            attack_max: modelo.dano_fisico.maximo,
            magic_attack: modelo
                .dano_magico_por_classe
                .map(|f| (f.minimo, f.maximo)),
            attack_range: modelo.alcance_de_ataque,
            exp: modelo.exp as i64,
            sp: modelo.pontos_de_skill as i64,
            aipolicy_id: modelo.politica_de_ia,
            // `MONSTER_ESSENCE` não tem "id de tabela de drop": tem 20 pares
            // item/probabilidade. Fica zero até o sistema de drop existir.
            drop_table_id: 0,
            position: posicao,
            spawn_center: posicao,
            move_speed: modelo.velocidade_correndo,
            is_dead: false,
            respawn_timer_ms: 0,
            respawn_delay_ms,
            target_id: None,
            buffs: Vec::new(),
        }
    }

    /// O monstro genérico de antes do `elements.data` entrar no caminho.
    ///
    /// Continua existindo para dois casos honestos: o realm 1.2.6, cujo `elements.data`
    /// (v7) o leitor genérico ainda não cobre, e o `npcgen.data` que cita um monstro que o
    /// `elements.data` não tem. Some da tela sem explicação seria pior do que aparecer com
    /// atributo genérico e um aviso no log.
    pub fn placeholder(
        id: i64,
        template_id: u32,
        posicao: Vector3,
        respawn_delay_ms: u32,
    ) -> Self {
        Self {
            id,
            template_id,
            name: "Monstro".to_string(),
            level: 1,
            hp: 500,
            max_hp: 500,
            mp: 100,
            max_mp: 100,
            def_phys: 50,
            armor: 50,
            attack_rate: 100,
            resistances: [50; 5],
            attack_degree: 0,
            defend_degree: 0,
            attack_min: 20,
            attack_max: 35,
            magic_attack: [(0, 0); 5],
            attack_range: 2.5,
            exp: 100,
            sp: 20,
            aipolicy_id: 0,
            drop_table_id: 0,
            position: posicao,
            spawn_center: posicao,
            move_speed: 3.5,
            is_dead: false,
            respawn_timer_ms: 0,
            respawn_delay_ms,
            target_id: None,
            buffs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcEntity {
    pub id: i64,
    pub template_id: u32,
    pub name: String,
    pub position: Vector3,
    pub dialog_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDropEntity {
    pub id: i64,
    pub item_id: u32,
    pub count: u32,
    pub position: Vector3,
    pub owner_role_id: Option<RoleId>,
    pub protect_timer_ms: u32,
    pub despawn_timer_ms: u32,
}
