use crate::manager::GameDataManager;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct IntegrityIssue {
    pub category: &'static str,
    pub source_file: String,
    pub message: String,
    pub remediation: String,
}

pub struct DataValidator;

impl DataValidator {
    /// Executa uma varredura completa de integridade cruzada entre todos os arquivos .data carregados
    pub fn validate_all(data: &GameDataManager) -> Vec<IntegrityIssue> {
        info!("Iniciando Diagnóstico de Integridade Cruzada entre arquivos .data...");
        let mut issues = Vec::new();

        // 1. Valida se os monstros e NPCs nos arquivos npcgen.data de cada mapa existem no elements.data
        for (&world_id, spawns) in &data.map_spawns {
            for area in &spawns.areas {
                match area.spawn_type {
                    crate::npcgen::SpawnType::Monster => {
                        if !data.elements.monsters.contains_key(&area.template_id) {
                            issues.push(IntegrityIssue {
                                category: "NPCGEN_ORPHAN_MONSTER",
                                source_file: format!("world_id_{}/npcgen.data", world_id),
                                message: format!(
                                    "Região '{}' referencia Monstro ID {} que NÃO existe no elements.data!",
                                    area.area_name, area.template_id
                                ),
                                remediation: format!(
                                    "Adicione o monstro ID {} no elements.data ou corrija a área no npcgen.data.",
                                    area.template_id
                                ),
                            });
                        }
                    }
                    crate::npcgen::SpawnType::Npc => {
                        if !data.elements.npcs.contains_key(&area.template_id) {
                            issues.push(IntegrityIssue {
                                category: "NPCGEN_ORPHAN_NPC",
                                source_file: format!("world_id_{}/npcgen.data", world_id),
                                message: format!(
                                    "NPC de diálogo ID {} na região '{}' NÃO existe no elements.data!",
                                    area.template_id, area.area_name
                                ),
                                remediation: format!(
                                    "Cadastre o NPC {} no elements.data ou remova a entrada em npcgen.data.",
                                    area.template_id
                                ),
                            });
                        }
                    }
                    crate::npcgen::SpawnType::ResourceMine => {}
                }
            }
        }

        // 2. Valida se as ofertas do gshop.data apontam para itens reais do elements.data
        for (&shop_id, item) in &data.gshop.items {
            if !data.elements.is_valid_item_id(item.item_id) {
                issues.push(IntegrityIssue {
                    category: "GSHOP_ORPHAN_ITEM",
                    source_file: "gshop.data".to_string(),
                    message: format!(
                        "Oferta #{} do GShop vende o item ID {} que NÃO existe no elements.data!",
                        shop_id, item.item_id
                    ),
                    remediation: format!(
                        "Crie o item ID {} no elements.data ou desative a oferta #{} no GShop.",
                        item.item_id, shop_id
                    ),
                });
            }
        }

        // 3. Valida se os monstros apontam para IA Policies válidas no aipolicy.data
        for (&monster_id, monster) in &data.elements.monsters {
            if monster.aipolicy_id > 0 && !data.aipolicy.policies.contains_key(&monster.aipolicy_id) {
                issues.push(IntegrityIssue {
                    category: "ELEMENTS_ORPHAN_AIPOLICY",
                    source_file: "elements.data".to_string(),
                    message: format!(
                        "Monstro '{}' (ID {}) referencia a IA Policy ID {} que NÃO existe no aipolicy.data!",
                        monster.name, monster_id, monster.aipolicy_id
                    ),
                    remediation: format!(
                        "Crie a árvore de IA ID {} no aipolicy.data ou aponte o monstro para a IA padrão (0).",
                        monster.aipolicy_id
                    ),
                });
            }
        }

        if issues.is_empty() {
            info!("Diagnóstico concluído: Todos os arquivos .data estão 100% íntegros e sincronizados!");
        } else {
            warn!(
                "Diagnóstico concluído com {} inconformidades detectadas nos arquivos .data:",
                issues.len()
            );
            for (idx, issue) in issues.iter().enumerate() {
                warn!(
                    "[PROBLEMA #{}] [{}] em {}: {}\n   👉 AÇÃO RECOMENDADA: {}",
                    idx + 1,
                    issue.category,
                    issue.source_file,
                    issue.message,
                    issue.remediation
                );
            }
        }

        issues
    }
}
