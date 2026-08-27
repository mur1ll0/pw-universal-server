use pw_core::RoleId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootRule {
    FreeForAll = 0, // Livre para todos
    RoundRobin = 1, // Rotativo / Alternado
    Random = 2,     // Aleatório
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    pub role_id: RoleId,
    pub name: String,
    pub level: i32,
    pub cls: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub world_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: u64,
    pub leader_id: RoleId,
    pub members: Vec<PartyMember>,
    pub loot_rule: LootRule,
}

#[derive(Clone, Default)]
pub struct PartyManager {
    parties: Arc<RwLock<HashMap<u64, Party>>>,
    player_party_map: Arc<RwLock<HashMap<RoleId, u64>>>,
    party_counter: Arc<RwLock<u64>>,
}

impl PartyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria um novo grupo com o líder
    pub async fn create_party(&self, leader: PartyMember) -> u64 {
        let mut counter = self.party_counter.write().await;
        *counter += 1;
        let party_id = *counter;

        let leader_id = leader.role_id;
        let party = Party {
            id: party_id,
            leader_id,
            members: vec![leader],
            loot_rule: LootRule::FreeForAll,
        };

        self.parties.write().await.insert(party_id, party);
        self.player_party_map.write().await.insert(leader_id, party_id);

        party_id
    }

    /// Adiciona um membro ao grupo (máximo 6 jogadores)
    pub async fn add_member(&self, party_id: u64, member: PartyMember) -> anyhow::Result<bool> {
        let mut parties = self.parties.write().await;
        if let Some(party) = parties.get_mut(&party_id) {
            if party.members.len() >= 6 {
                anyhow::bail!("Grupo já está cheio (máximo 6 jogadores)");
            }
            let member_id = member.role_id;
            party.members.push(member);
            self.player_party_map.write().await.insert(member_id, party_id);
            return Ok(true);
        }
        Ok(false)
    }

    /// Remove um jogador do grupo
    pub async fn remove_member(&self, role_id: RoleId) -> Option<u64> {
        let party_id = self.player_party_map.write().await.remove(&role_id)?;
        let mut parties = self.parties.write().await;

        if let Some(party) = parties.get_mut(&party_id) {
            party.members.retain(|m| m.role_id != role_id);

            if party.members.is_empty() {
                // Grupo desfeito
                parties.remove(&party_id);
            } else if party.leader_id == role_id {
                // Transfere liderança para o próximo membro
                party.leader_id = party.members[0].role_id;
            }
        }

        Some(party_id)
    }

    /// Busca o grupo de um jogador
    pub async fn get_party_by_player(&self, role_id: RoleId) -> Option<Party> {
        let party_id = *self.player_party_map.read().await.get(&role_id)?;
        self.parties.read().await.get(&party_id).cloned()
    }
}
