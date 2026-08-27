/// Opcodes oficiais dos pacotes C2S e S2C do Perfect World
/// O mapeamento é parametrizado conforme a versão do jogo ativa no Realm.

pub const OP_C2S_CHALLENGE_RES: u32 = 0x02; // Resposta de login do cliente
pub const OP_S2C_CHALLENGE: u32 = 0x01;     // Desafio de conexão inicial com nonce
pub const OP_S2C_LOGIN_SUCCESS: u32 = 0x03; // Confirmação de login
pub const OP_S2C_LOGIN_ERROR: u32 = 0x04;   // Erro de login

pub const OP_C2S_ROLE_LIST: u32 = 0x52;     // Solicitação da lista de personagens
pub const OP_S2C_ROLE_LIST_RES: u32 = 0x53; // Resposta da lista de personagens

pub const OP_C2S_CREATE_ROLE: u32 = 0x54;   // Criação de novo personagem
pub const OP_S2C_CREATE_ROLE_RES: u32 = 0x55;// Resposta da criação de personagem

pub const OP_C2S_DELETE_ROLE: u32 = 0x56;   // Deletar personagem
pub const OP_S2C_DELETE_ROLE_RES: u32 = 0x57;

pub const OP_C2S_SELECT_ROLE: u32 = 0x44;   // Entrar no mundo com o personagem selecionado
pub const OP_S2C_ENTER_WORLD: u32 = 0x45;   // Confirmação de entrada no mundo

pub const OP_C2S_PLAYER_MOVE: u32 = 0x20;   // Movimentação do jogador (coordenadas, velocidade, modo)
pub const OP_S2C_PLAYER_MOVE_BROADCAST: u32 = 0x21; // Transmissão de movimento para jogadores ao redor

pub const OP_C2S_USE_SKILL: u32 = 0x29;     // Conjuração de habilidade
pub const OP_S2C_SKILL_CAST_BROADCAST: u32 = 0x2A;

pub const OP_C2S_CHAT: u32 = 0x70;          // Mensagem de chat (Geral, Clã, Mundo, Sussurro)
pub const OP_S2C_CHAT_BROADCAST: u32 = 0x71;

pub const OP_C2S_HEARTBEAT: u32 = 0x5A;     // Manutenção de conexão viva (Keep-Alive)
pub const OP_S2C_HEARTBEAT_ACK: u32 = 0x5B;

pub const OP_S2C_SPAWN_PLAYER: u32 = 0x0A;  // Jogador aparece no campo de visão (Grid 3D)
pub const OP_S2C_DESPAWN_PLAYER: u32 = 0x0B;// Jogador sai do campo de visão

pub const OP_S2C_SPAWN_NPC: u32 = 0x0C;     // Monstro / NPC aparece no campo de visão
pub const OP_S2C_DESPAWN_NPC: u32 = 0x0D;

pub const OP_S2C_UPDATE_STATUS: u32 = 0x14; // Atualização de HP, MP, EXP, Nível do jogador
