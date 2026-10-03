//! Os comandos de GM do mundo — o painel do Ctrl+G do cliente.
//!
//! Porte de `gplayer_controller::GMCommandHandler` (`cgame/gs/playercmd.cpp:4806-5230`). O
//! cliente monta os comandos em `GMCommandInGame.cpp` e os manda pelo `GamedataSend`
//! (`EC_SendC2SCmds.cpp`, `c2s_SendCmdGM*`).
//!
//! | C2S | painel | aqui |
//! | :-- | :-- | :-- |
//! | 201 `GM_MOVETO_PLAYER` | ir até o jogador | o GM vai à posição do alvo, em qualquer mapa do processo |
//! | 202 `GM_CALLIN_PLAYER` | chamar o jogador | o alvo vem à posição do GM |
//! | 204 `GM_INVISIBLE` | invisível | alterna; some da vista dos outros, não ataca, não é ferido |
//! | 205 `GM_INVINCIBLE` | invencível | alterna; não é ferido, estado visível 49 |
//! | 206 `GM_GENERATE` | criar item | `falta`: o `elements.data` ainda não lê o `GM_GENERATOR_ESSENCE` |
//! | 207 `GM_ACTIVE_SPAWNER` | ligar/desligar gerador | `falta`: os controladores do `npcgen` não têm gatilho |
//! | 208 `GM_GENERATE_MOB` | criar monstro | só com `debug_command_mode = active`, como no original |
//!
//! # Privilégio
//!
//! O original só entra no tratador com `_gm_auth` (senão ignora calado, `_load_stats += 3`) e
//! confere um bit por comando (`Has_Hide_BeGod`, `Has_MoveTo_Role`, …). Aqui o privilégio é o
//! nível de GM da conta (`sec_level`, coluna `gm_privileges`): acima de zero vale para todos.
//! Os bits por comando são `falta`.

use super::jogo::erro_s2c;
use super::*;

/// `CreateMinors(param, radius = 6.0f)` (`gs/obj_interface.h:670`): cada um nasce num quadrado
/// de ±6 m em volta do GM (`obj_interface.cpp:1990-1998`).
const RAIO_DO_MONSTRO_DE_GM: f32 = 6.0;

/// `gm_cmd_generate_mob` sem o nome: `int mob_id; int vis_id; short count; short life; size_t
/// name_len;` com `#pragma pack(1)` (`EC_GPDataType.h:563`, `:6579-6587`).
const CABECALHO_DO_MONSTRO_DE_GM: usize = 16;

/// O teto do nome do monstro (`ggm.name_len > 18` é recusado, `playercmd.cpp:4995`).
const TETO_DO_NOME_DO_MONSTRO: usize = 18;

impl BusServer {
    /// Entrada de todos os comandos de GM do mundo.
    pub(super) async fn comando_de_gm(
        &self,
        roleid: i32,
        comando: u16,
        payload: &[u8],
        envio: &EnvioAoCliente,
    ) {
        let nivel = self
            .world
            .read()
            .await
            .players
            .get(&(roleid as i64))
            .map(|p| p.sec_level)
            .unwrap_or(0);
        if nivel == 0 {
            // `if (!_gm_auth) { _load_stats += 3; return 0; }` — nada volta ao cliente.
            warn!("mundo: {roleid} mandou o comando de GM {comando} sem ser GM");
            return;
        }
        match comando {
            ids::GM_INVINCIBLE => self.gm_alternar_invencivel(roleid, envio).await,
            ids::GM_INVISIBLE => self.gm_alternar_invisivel(roleid, envio).await,
            ids::GM_MOVETO_PLAYER => {
                let Some(alvo) = ler_id(payload) else {
                    return self.gm_tamanho_errado(roleid, comando, payload, envio).await;
                };
                self.gm_ir_ate(roleid, alvo).await;
            }
            ids::GM_CALLIN_PLAYER => {
                let Some(alvo) = ler_id(payload) else {
                    return self.gm_tamanho_errado(roleid, comando, payload, envio).await;
                };
                self.gm_chamar(roleid, alvo).await;
            }
            ids::GM_GENERATE_MOB => self.gm_criar_monstro(roleid, payload, envio).await,
            ids::GM_GENERATE => {
                let tid = ler_id(payload).unwrap_or(0);
                warn!(
                    "mundo: GM {roleid} pediu o gerador de item {tid} — falta: o elements.data \
                     ainda não lê o GM_GENERATOR_ESSENCE (playercmd.cpp:4931-4952)"
                );
            }
            ids::GM_ACTIVE_SPAWNER => {
                let (liga, gerador) = match payload {
                    [a, b0, b1, b2, b3] => (*a != 0, i32::from_le_bytes([*b0, *b1, *b2, *b3])),
                    _ => return self.gm_tamanho_errado(roleid, comando, payload, envio).await,
                };
                warn!(
                    "mundo: GM {roleid} pediu para {} o gerador {gerador} — falta: os \
                     controladores do npcgen não têm gatilho (TriggerSpawn/ClearSpawn)",
                    if liga { "ligar" } else { "desligar" }
                );
            }
            _ => {}
        }
    }

    /// `if (size != sizeof(cmd)) error_cmd(S2C::ERR_FATAL_ERR)` (`DEFCMD`, `playercmd.cpp:4808`).
    async fn gm_tamanho_errado(
        &self,
        roleid: i32,
        comando: u16,
        payload: &[u8],
        envio: &EnvioAoCliente,
    ) {
        warn!(
            "mundo: comando de GM {comando} de {roleid} com {} bytes",
            payload.len()
        );
        self.responder(
            roleid,
            S2CGamedataSend::error_message(erro_s2c::ERRO_FATAL).data,
            envio,
        )
        .await;
    }

    /// `GMCMD_TOGGLE_INVINCIBLE` (`playercmd.cpp:4895-4910`): põe ou tira o
    /// `invincible_filter` sem prazo e responde `GM_INVINCIBLE` (175) com o estado novo.
    async fn gm_alternar_invencivel(&self, roleid: i32, envio: &EnvioAoCliente) {
        let ligado = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else {
                return;
            };
            p.efeitos.gm_invencivel = !p.efeitos.gm_invencivel;
            p.efeitos.gm_invencivel
        };
        info!(
            "mundo: GM {roleid} {} o modo invencível",
            if ligado { "ligou" } else { "desligou" }
        );
        if let Some(r) = self.sub.gm_invincible(ligado) {
            self.responder(roleid, r.data, envio).await;
        }
        // O estado visível 49 do `invincible_filter` para quem está em volta.
        self.avisar_efeitos(roleid as i64, false).await;
    }

    /// `GMCMD_TOGGLE_INVISIBLE` (`playercmd.cpp:4879-4893`) → `SetGMInvisible` /
    /// `ClearGMInvisible` (`gs/player.cpp:13357-13377`): sumir é `leave_world` para os outros,
    /// voltar é `appear`; a resposta é `GM_INVISIBLE` (176) com `is_visible`.
    async fn gm_alternar_invisivel(&self, roleid: i32, envio: &EnvioAoCliente) {
        let eu = roleid as i64;
        let invisivel = {
            let mut mundo = self.world.write().await;
            let Some(p) = mundo.players.get_mut(&eu) else {
                return;
            };
            p.efeitos.gm_invisivel = !p.efeitos.gm_invisivel;
            p.efeitos.gm_invisivel
        };
        info!(
            "mundo: GM {roleid} {} invisível",
            if invisivel { "ficou" } else { "deixou de ser" }
        );
        if invisivel {
            self.tirar_da_vista_de_todos(eu).await;
        } else {
            self.aparecer_para_quem_vejo(roleid).await;
        }
        if let Some(r) = self.sub.gm_invisible(!invisivel) {
            self.responder(roleid, r.data, envio).await;
        }
    }

    /// O `appear` do GM que volta a ser visível: cada jogador que ele vê passa a vê-lo.
    async fn aparecer_para_quem_vejo(&self, roleid: i32) {
        let eu = roleid as i64;
        let (outros, minha_vista) = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&eu) else {
                return;
            };
            let outros: Vec<i64> = p
                .visiveis
                .iter()
                .copied()
                .filter(|id| mundo.players.contains_key(id))
                .collect();
            (outros, mundo.vista_de(eu))
        };
        let Some(vista) = minha_vista else {
            return;
        };
        let pacote = self.sub.player_enter_slice(roleid, vista).data;
        for outro in outros {
            if self.passou_a_ver(outro, eu).await {
                self.enviar_ao_jogador(outro as i32, pacote.clone()).await;
            }
        }
    }

    /// Onde está um jogador: neste mapa ou, pelo roteador, noutro mapa do processo.
    async fn gm_localizar(&self, roleid: i32) -> Option<(i32, Vector3)> {
        {
            let mundo = self.world.read().await;
            if let Some(p) = mundo.players.get(&(roleid as i64)) {
                return Some((mundo.world_id as i32, p.position));
            }
        }
        let roteador = self.roteador.get()?.upgrade()?;
        roteador.posicao_de(roleid).await
    }

    /// `GMCMD_MOVE_TO_PLAYER` (`playercmd.cpp:4830-4844`): o original pergunta a posição ao
    /// alvo (`GM_MSG_GM_MQUERY_MOVE_POS`) e o GM salta para lá, troca de mapa inclusive. Alvo
    /// fora do jogo: nada acontece.
    async fn gm_ir_ate(&self, roleid: i32, alvo: i32) {
        let Some((mapa, pos)) = self.gm_localizar(alvo).await else {
            warn!("mundo: GM {roleid} quis ir até {alvo}, que não está neste servidor de mundo");
            return;
        };
        info!("mundo: GM {roleid} vai até {alvo} (mapa {mapa}, {pos:?})");
        self.transportar(roleid, mapa, pos).await;
    }

    /// `GMCMD_RECALL_PLAYER` (`playercmd.cpp:4846-4857`): manda o alvo saltar para a posição
    /// e o mapa do GM (`GM_MSG_GM_RECALL` com o `world_tag`).
    async fn gm_chamar(&self, roleid: i32, alvo: i32) {
        let Some((mapa, pos)) = self.gm_localizar(roleid).await else {
            return;
        };
        if self.gm_localizar(alvo).await.is_none() {
            warn!("mundo: GM {roleid} quis chamar {alvo}, que não está neste servidor de mundo");
            return;
        }
        info!("mundo: GM {roleid} chamou {alvo} para o mapa {mapa} em {pos:?}");
        match self.roteador.get().and_then(|r| r.upgrade()) {
            Some(roteador) => roteador.transportar(alvo, mapa, pos).await,
            None => self.transportar(alvo, mapa, pos).await,
        }
    }

    /// `GMCMD_GENERATE_MOB` (`playercmd.cpp:4989-5030`): `count` monstros `mob_id` com
    /// `remain_time = life` (0 = para sempre), sem líder e sem ódio, cada um a ±6 m do GM.
    async fn gm_criar_monstro(&self, roleid: i32, payload: &[u8], envio: &EnvioAoCliente) {
        let modo_de_depuracao = self
            .world
            .read()
            .await
            .data_manager
            .base_das_classes
            .modo_de_depuracao;
        if !modo_de_depuracao {
            // `if (!player_template::GetDebugMode()) break;` — calado, como no original.
            warn!(
                "mundo: GM {roleid} pediu monstro, mas o ptemplate.conf está sem \
                 `debug_command_mode = active` (playercmd.cpp:4991)"
            );
            return;
        }
        let Some(pedido) = ler_monstro_de_gm(payload) else {
            return self
                .gm_tamanho_errado(roleid, ids::GM_GENERATE_MOB, payload, envio)
                .await;
        };
        let criados = {
            let mut mundo = self.world.write().await;
            let Some(centro) = mundo.players.get(&(roleid as i64)).map(|p| p.position) else {
                return;
            };
            let mut criados = 0;
            for _ in 0..pedido.quantidade {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let mut pos = centro;
                pos.x += rng.gen_range(-RAIO_DO_MONSTRO_DE_GM..=RAIO_DO_MONSTRO_DE_GM);
                pos.z += rng.gen_range(-RAIO_DO_MONSTRO_DE_GM..=RAIO_DO_MONSTRO_DE_GM);
                if mundo
                    .invocar_monstro(pedido.monstro, pos, 0, pedido.vida_s as i32, false, None)
                    .is_some()
                {
                    criados += 1;
                }
            }
            criados
        };
        info!(
            "mundo: GM {roleid} criou {criados}/{} do monstro {} (vida {} s)",
            pedido.quantidade, pedido.monstro, pedido.vida_s
        );
        if pedido.aparencia != 0 || pedido.nome_bytes > 0 {
            // `vis_id` e `mob_name` trocam modelo e nome do monstro; o nosso `NPC_ENTER_SLICE`
            // ainda manda só o `tid` do molde.
            debug!(
                "mundo: aparência {} e nome ({} bytes) do monstro de GM não aplicados (falta)",
                pedido.aparencia, pedido.nome_bytes
            );
        }
    }
}

/// `int pid` / `int tid` — 4 bytes exatos.
fn ler_id(payload: &[u8]) -> Option<i32> {
    let b: [u8; 4] = payload.try_into().ok()?;
    Some(i32::from_le_bytes(b))
}

/// O que o `gm_cmd_generate_mob` pede.
#[derive(Debug, PartialEq)]
struct MonstroDeGm {
    monstro: u32,
    aparencia: i32,
    quantidade: u16,
    vida_s: u16,
    nome_bytes: usize,
}

/// `size < sizeof(ggm) || size != sizeof(ggm) + ggm.name_len || ggm.name_len > 18` →
/// `ERR_FATAL_ERR` (`playercmd.cpp:4994-4999`). O cliente já põe `count` em pelo menos 1
/// (`EC_SendC2SCmds.cpp`, `sCount >= 1 ? sCount : 1`); o servidor faz `for i < count`.
fn ler_monstro_de_gm(payload: &[u8]) -> Option<MonstroDeGm> {
    if payload.len() < CABECALHO_DO_MONSTRO_DE_GM {
        return None;
    }
    let i32_em = |i: usize| i32::from_le_bytes(payload[i..i + 4].try_into().unwrap());
    let i16_em = |i: usize| i16::from_le_bytes(payload[i..i + 2].try_into().unwrap());
    let nome_bytes = u32::from_le_bytes(payload[12..16].try_into().unwrap()) as usize;
    if nome_bytes > TETO_DO_NOME_DO_MONSTRO || payload.len() != CABECALHO_DO_MONSTRO_DE_GM + nome_bytes {
        return None;
    }
    Some(MonstroDeGm {
        monstro: i32_em(0) as u32,
        aparencia: i32_em(4),
        quantidade: i16_em(8).max(0) as u16,
        vida_s: i16_em(10).max(0) as u16,
        nome_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monstro_de_gm_le_o_cabecalho_e_confere_o_nome() {
        let mut p = Vec::new();
        p.extend_from_slice(&8340i32.to_le_bytes());
        p.extend_from_slice(&0i32.to_le_bytes());
        p.extend_from_slice(&3i16.to_le_bytes());
        p.extend_from_slice(&60i16.to_le_bytes());
        p.extend_from_slice(&4u32.to_le_bytes());
        p.extend_from_slice(&[0x41, 0, 0x42, 0]);
        assert_eq!(
            ler_monstro_de_gm(&p),
            Some(MonstroDeGm { monstro: 8340, aparencia: 0, quantidade: 3, vida_s: 60, nome_bytes: 4 })
        );
        // `name_len` que não fecha com o tamanho: recusado.
        assert_eq!(ler_monstro_de_gm(&p[..18]), None);
        // Nome acima de 18 bytes: recusado.
        let mut longo = p[..12].to_vec();
        longo.extend_from_slice(&20u32.to_le_bytes());
        longo.extend_from_slice(&[0; 20]);
        assert_eq!(ler_monstro_de_gm(&longo), None);
    }

    #[test]
    fn id_de_gm_exige_quatro_bytes() {
        assert_eq!(ler_id(&11456i32.to_le_bytes()), Some(11456));
        assert_eq!(ler_id(&[1, 2, 3]), None);
    }
}
