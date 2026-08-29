use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum AiPolicyError {
    #[error("Erro de I/O na leitura do aipolicy.data: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AiPolicyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiTriggerType {
    OnAggro,
    OnHPPercent(u8), // ex: 50% vida
    OnTimer(u32),    // segundos
    OnAttacked,
    OnTargetDie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiAction {
    CastSkill { skill_id: u32, level: u8 },
    SummonMinions { monster_id: u32, count: u32, radius: f32 },
    SayText { message: String },
    ChangeAggroToHighestDamage,
    Flee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTriggerAction {
    pub trigger: AiTriggerType,
    pub actions: Vec<AiAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPolicy {
    pub id: u32,
    pub name: String,
    pub rules: Vec<AiTriggerAction>,
}

#[derive(Debug, Clone, Default)]
pub struct AiPolicyData {
    pub policies: HashMap<u32, AiPolicy>,
}

impl AiPolicyData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        info!("Carregando aipolicy.data (Árvores de IA de Monstros/Chefes)...");

        let mut aipolicy_data = Self {
            policies: HashMap::new(),
        };

        aipolicy_data.parse_policies(&mut cursor)?;
        info!("aipolicy.data carregado: {} políticas de IA registradas", aipolicy_data.policies.len());

        Ok(aipolicy_data)
    }

    fn parse_policies(&mut self, _cursor: &mut Cursor<&[u8]>) -> Result<()> {
        Ok(())
    }

    pub fn get_policy(&self, policy_id: u32) -> Option<&AiPolicy> {
        self.policies.get(&policy_id)
    }
}
