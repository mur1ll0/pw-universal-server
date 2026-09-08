use pw_core::CharacterDetails;
use pw_data_loader::{TabelaDeBase, TabelaDeClasses, TemplateDeMonstro};
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
    /// O jogador está voando.
    ///
    /// Não há coluna no banco para isso, e nem deveria: quem relogar entra no chão, que é
    /// o que o cliente também assume.
    pub voando: bool,
    /// O jogador está mostrando a roupa (moda) no lugar da armadura.
    ///
    /// É estado de aparência, e o cliente alterna com o `SWITCH_FASHION_MODE` (C2S 85).
    /// Vive só no mundo: não há coluna para ele no banco, então volta ao padrão a cada
    /// login. Trocar isso é mudança de esquema, não de código.
    pub modo_roupa: bool,
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
    /// `aggro_range` do `elements.data`: até onde o monstro persegue. Era `35.0` escrito
    /// no `ai.rs` para todo monstro do jogo.
    pub aggro_range: f32,
    /// `sight_range`: até onde ele enxerga. Ainda não decide nada — entra quando a IA
    /// deixar de depender só da tabela de ameaça e passar a procurar alvo sozinha.
    pub sight_range: i32,
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
    /// Monta o jogador que entra no mundo, a partir do personagem do banco mais as duas
    /// tabelas de classe.
    ///
    /// # As fórmulas, e de onde vêm
    ///
    /// O `ptemplate.conf` dá o ponto de partida do nível 1 e o `CHARRACTER_CLASS_CONFIG`
    /// dá o que escala (`player_template::__LoadData` e `__LevelUp`):
    ///
    /// ```text
    /// max_hp  = base.hp  + lvl_hp * (nível-1) + vit_hp * vitalidade
    /// max_mp  = base.mp  + lvl_mp * (nível-1) + eng_mp * energia
    /// dano    = 1 + (int)(nível * lvlup_dmg)      - (int)(lvlup_dmg)
    /// defesa  =     (int)(nível * lvlup_defense) - (int)(lvlup_defense)
    /// precisão = agi_attack * agilidade
    /// evasão   = agi_armor  * agilidade
    /// ```
    ///
    /// O `- (int)(x)` no fim das duas do meio não é enfeite: `__LevelUp` soma
    /// `(int)((l+1)*d) - (int)(l*d)` a cada nível, e a soma telescópica de 1 até N é
    /// `(int)(N*d) - (int)(1*d)`. O dano parte de 1 porque é o que
    /// `player_template::__LoadData` grava em `damage_low`/`damage_high` antes de
    /// qualquer nível.
    ///
    /// # O que este jogador **não** tem
    ///
    /// Equipamento. No original, `UpdateAttack`/`UpdateDefense` somam `_cur_item`,
    /// `_en_point` e `_en_percent` por cima de tudo isto — arma, armadura, encantamento,
    /// refino. Nada disso existe do nosso lado, então o que sai daqui é um personagem
    /// **pelado**: os números são os certos para nível, classe e atributos, e nada mais.
    /// É a diferença entre "aproximado" e "incompleto de um jeito conhecido".
    ///
    /// `base` é `None` quando o realm não trouxe o `ptemplate.conf`; nesse caso vida e
    /// mana máximas ficam iguais às que estão gravadas no banco, e o log de quem chamou
    /// deve dizer isso.
    pub fn do_personagem(
        p: &CharacterDetails,
        classes: &TabelaDeClasses,
        base: Option<&TabelaDeBase>,
    ) -> Self {
        let cls = p.cls as i32;
        let cfg = classes.get(cls);
        let base = base.and_then(|b| b.get(cls));
        // `__LevelUp` roda uma vez por nível ganho, ou seja `nível - 1` vezes.
        let niveis = (p.level - 1).max(0);

        let telescopica = |por_nivel: f32| -> i32 {
            (p.level as f32 * por_nivel) as i32 - por_nivel as i32
        };

        let (max_hp, max_mp) = match (base, cfg) {
            (Some(b), Some(c)) => (
                b.vida + c.vida_por_nivel as i32 * niveis + c.vida_por_vitalidade * p.vitality,
                b.mana + c.mana_por_nivel as i32 * niveis + c.mana_por_energia * p.energy,
            ),
            // Sem o `ptemplate.conf` não há ponto de partida: fica o que o banco guardou,
            // que ao menos não é inventado.
            _ => (p.hp, p.mp),
        };

        let dano = cfg.map(|c| 1 + telescopica(c.dano_por_nivel)).unwrap_or(1);
        let dano_magico = cfg.map(|c| 1 + telescopica(c.dano_magico_por_nivel)).unwrap_or(1);
        let defesa = cfg.map(|c| telescopica(c.defesa_por_nivel)).unwrap_or(0);
        let resistencia = cfg.map(|c| telescopica(c.resistencia_por_nivel)).unwrap_or(0);

        Self {
            role_id: p.id,
            name: p.name.clone(),
            race: p.race,
            cls: p.cls,
            gender: p.gender,
            level: p.level,
            cultivation: p.cultivation,
            // O banco guarda a vida corrente; ela não pode passar do máximo recém-calculado
            // (um personagem que subiu de nível offline, ou um `ptemplate.conf` trocado).
            hp: p.hp.min(max_hp).max(0),
            max_hp,
            mp: p.mp.min(max_mp).max(0),
            max_mp,
            exp: p.exp,
            sp: p.sp,
            money: p.money,
            strength: p.strength,
            agility: p.agility,
            vitality: p.vitality,
            energy: p.energy,
            def_phys: defesa,
            def_metal: resistencia,
            def_wood: resistencia,
            def_water: resistencia,
            def_fire: resistencia,
            def_earth: resistencia,
            attack_min: dano,
            attack_max: dano,
            magic_attack_min: dano_magico,
            magic_attack_max: dano_magico,
            armor: cfg.map(|c| c.evasao_base(p.agility)).unwrap_or(0),
            attack_rate: cfg.map(|c| c.precisao_base(p.agility)).unwrap_or(0),
            // Grau de ataque/defesa e bônus de dano crítico vêm de equipamento e passiva
            // no original. Zero é o valor neutro do cálculo, não um palpite.
            attack_degree: 0,
            defend_degree: 0,
            crit_damage_bonus: 0,
            attack_speed: base.map(|b| b.ataque_em_ticks as f32 / 20.0).unwrap_or(1.0),
            move_speed: base.map(|b| b.velocidade_correndo).unwrap_or(3.0),
            // `crit_rate` está em pontos percentuais na tabela e em fração na entidade.
            crit_rate: cfg.map(|c| c.chance_de_critico as f32 / 100.0).unwrap_or(0.0),
            position: p.position,
            target_id: None,
            buffs: Vec::new(),
            voando: false,
            // Todo mundo entra mostrando a armadura; o banco não guarda esta escolha.
            modo_roupa: false,
        }
    }

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
            aggro_range: modelo.raio_de_odio,
            sight_range: modelo.raio_de_visao,
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
            aggro_range: 15.0,
            sight_range: 20,
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
