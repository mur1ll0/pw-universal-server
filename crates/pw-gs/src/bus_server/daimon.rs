//! B345 — o Daimon (o `elf_item` do original): distribuir atributos e gênios, injetar experiência, vestir peça e,
//! nos NPCs, aprender e esquecer habilidade. Porte de `gplayer_imp::Elf*` (`gs/player.cpp:21856-22270`) sobre as
//! regras do `elf_item` (`gs/item/item_elf.cpp`). Só o 1.5.5 tem Daimon.
//!
//! B355: conjurar habilidade do Daimon (`CAST_ELF_SKILL`, 116) e o vigor.
//!
//! **Não é igual / `falta`:** o estado de segurança (115), recarregar (117), o bônus de atributo sorteado por nível, refino, decomposição, destruir
//! peça e os serviços 48/49 (devolver pontos).

use super::*;
use super::jogo::Contexto;
use crate::entity::{requisito_de_daimon_cumprido, AtuaisDoDaimon, Daimon};

/// `ERR_FATAL_ERR` e os `ERR_ELF_*` (`common/protocol.h:789-812`, contados a partir do `ERR_SUCCESS`).
mod erro {
    pub const FATAL: i32 = 3;
    pub const SOMAR_ATRIBUTO: i32 = 109;
    pub const SOMAR_GENIO: i32 = 110;
    pub const INJETAR_EXP: i32 = 111;
    pub const VESTIR_PECA: i32 = 113;
    pub const APRENDER: i32 = 117;
    pub const ESQUECER: i32 = 118;
    pub const EM_RECARGA: i32 = 132;
    pub const HABILIDADE_INDISPONIVEL: i32 = 20;
}

/// `ELF_ADD_GENIUS`, `ELF_EQUIP_ITEM`, `ELF_LEARN_SKILL`, `ELF_FORGET_SKILL` do `enum // elf cmd`
/// (`common/protocol.h:946-958`).
mod resultado {
    pub const APRENDER: i32 = 1;
    pub const ESQUECER: i32 = 2;
    pub const SOMAR_GENIO: i32 = 6;
    pub const VESTIR_PECA: i32 = 7;
}

/// `COOLDOWN_INDEX_ELF_CMD` (31º do enum de `gs/cooldowncfg.h`) e o `ELF_CMD_COOLDOWN_TIME` de 1 s.
const RECARGA_DO_COMANDO: i32 = 31;
const RECARGA_DO_COMANDO_MS: u64 = 1000;
/// `DROP_TYPE_TAKEOUT` (`common/protocol.h:931`).
const DROP_TIRADO: u8 = 2;
/// `EQUIP_INDEX_ELF`.
const SLOT_DO_DAIMON: u8 = 23;

use crate::comandos::ids as c2s;

fn i16_em(b: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([b[i], b[i + 1]])
}

impl BusServer {
    /// C2S 111–114 (`playercmd.cpp:3403-3458`): tamanho exato, senão `ERR_FATAL_ERR`.
    pub(super) async fn comando_do_daimon(&self, roleid: i32, id: u16, c: &[u8], envio: &EnvioAoCliente) {
        let tamanho = match id {
            c2s::ELF_ADD_ATTRIBUTE => 8,
            c2s::ELF_ADD_GENIUS => 10,
            c2s::ELF_PLAYER_INSERT_EXP => 5,
            _ => 1,
        };
        if c.len() != tamanho {
            self.responder(roleid, S2CGamedataSend::error_message(erro::FATAL).data, envio).await;
            return;
        }
        let r = self
            .com_contexto(roleid, |ctx| match id {
                c2s::ELF_ADD_ATTRIBUTE => somar_atributos(ctx, [0, 2, 4, 6].map(|i| i16_em(c, i))),
                c2s::ELF_ADD_GENIUS => somar_genios(ctx, [0, 2, 4, 6, 8].map(|i| i16_em(c, i))),
                c2s::ELF_PLAYER_INSERT_EXP => injetar_exp(ctx, u32::from_le_bytes([c[0], c[1], c[2], c[3]]), c[4] != 0),
                _ => vestir_peca(ctx, c[0] as usize),
            })
            .await;
        if r.is_none() {
            debug!("mundo: comando do Daimon {id} de {roleid}, que não está no mapa");
        }
    }

    /// Serviço 50 (`elf_learn_skill_provider`/`executor`, `serviceprovider.cpp:5074-5146`): `{u8 inv_idx_elf; u16
    /// skill_id}`, a habilidade na lista do NPC (senão `ERR_SKILL_NOT_AVAILABLE`).
    pub(super) async fn aprender_habilidade_de_daimon(&self, roleid: i32, c: &[u8]) {
        if c.len() != 3 {
            return;
        }
        let (slot, habilidade) = (c[0] as usize, u16::from_le_bytes([c[1], c[2]]));
        let Some((npc, servicos)) = self.npc_em_conversa(roleid).await else { return };
        if servicos.habilidades_de_daimon.binary_search(&(habilidade as u32)).is_err() {
            debug!("mundo: o NPC {npc} não ensina ao Daimon a habilidade {habilidade}");
            self.enviar_ao_jogador(roleid, S2CGamedataSend::error_message(erro::HABILIDADE_INDISPONIVEL).data).await;
            return;
        }
        self.com_contexto(roleid, |ctx| aprender(ctx, slot, habilidade)).await;
    }

    /// Serviço 51 (`elf_forget_skill_executor`, `serviceprovider.cpp:5150-5180`): `{u8 inv_index; u16 skill_id; i16
    /// forget_level}`, `forget_level > 0`.
    pub(super) async fn esquecer_habilidade_de_daimon(&self, roleid: i32, c: &[u8]) {
        if c.len() != 5 {
            return;
        }
        let (slot, habilidade, niveis) = (c[0] as usize, u16::from_le_bytes([c[1], c[2]]), i16_em(c, 3));
        if niveis <= 0 {
            return;
        }
        let Some((_, servicos)) = self.npc_em_conversa(roleid).await else { return };
        if !servicos.esquecer_habilidade_de_daimon {
            return;
        }
        self.com_contexto(roleid, |ctx| esquecer(ctx, slot, habilidade, niveis)).await;
    }
}

fn erro_ao_cliente(ctx: &mut Contexto, codigo: i32) {
    ctx.para_mim.push(S2CGamedataSend::error_message(codigo).data);
}

/// O `elf_extend_prop` do Daimon vestido.
fn atuais_do_vestido(ctx: &Contexto) -> Option<AtuaisDoDaimon> {
    let d = ctx.p.daimon.as_ref()?;
    let iniciais = ctx.dados.atributos_iniciais_do_daimon(d.item_id).unwrap_or([0; 4]);
    Some(d.estado.atuais(iniciais, |id| ctx.dados.equipamento_de_daimon(id)))
}

/// `PlayerGetItemInfo(IL_EQUIPMENT, EQUIP_INDEX_ELF)`: a ficha do item no corpo.
fn ficha_do_vestido(ctx: &mut Contexto) {
    let Some(d) = ctx.p.daimon.as_mut() else { return };
    d.sujo = true;
    let (item_id, bloco) = (d.item_id, d.estado.bloco());
    ctx.mudou = true;
    ctx.para_mim.push(
        S2CGamedataSend::item_info(
            ContainerType::Equipment.pacote_do_cliente().unwrap_or(1),
            SLOT_DO_DAIMON,
            item_id as i32,
            0,
            0,
            1,
            &bloco,
            None,
        )
        .data,
    );
}

/// `ElfAddAttribute` (`player.cpp:21856-21870`).
fn somar_atributos(ctx: &mut Contexto, mais: [i16; 4]) {
    let Some(d) = ctx.p.daimon.as_mut() else {
        return erro_ao_cliente(ctx, erro::SOMAR_ATRIBUTO);
    };
    if !d.estado.somar_atributos(mais, false) {
        return erro_ao_cliente(ctx, erro::SOMAR_ATRIBUTO);
    }
    ficha_do_vestido(ctx);
}

/// `ElfAddGenius` (`player.cpp:21872-21887`).
fn somar_genios(ctx: &mut Contexto, mais: [i16; 5]) {
    let Some(atuais) = atuais_do_vestido(ctx) else {
        return erro_ao_cliente(ctx, erro::SOMAR_GENIO);
    };
    let Some(d) = ctx.p.daimon.as_mut() else { return };
    if !d.estado.somar_genios(mais, atuais.genios, false) {
        return erro_ao_cliente(ctx, erro::SOMAR_GENIO);
    }
    let item = d.item_id as i32;
    ficha_do_vestido(ctx);
    ctx.para_mim.push(S2CGamedataSend::elf_cmd_result(resultado::SOMAR_GENIO, item, 0, 0).data);
}

/// `ElfPlayerInsertExp` (`player.cpp:21889-21942`): com SP, cada ponto vale 5 de experiência; recarga de 1 s.
fn injetar_exp(ctx: &mut Contexto, exp: u32, com_sp: bool) {
    let agora = std::time::Instant::now();
    if ctx.p.recargas.get(&RECARGA_DO_COMANDO).is_some_and(|ate| *ate > agora) {
        return erro_ao_cliente(ctx, erro::EM_RECARGA);
    }
    ctx.p.recargas.insert(RECARGA_DO_COMANDO, agora + std::time::Duration::from_millis(RECARGA_DO_COMANDO_MS));
    if ctx.p.daimon.is_none() {
        return erro_ao_cliente(ctx, erro::INJETAR_EXP);
    }
    let exp = if com_sp {
        if exp == 0 || ctx.p.sp <= 0 || exp as i64 > ctx.p.sp || exp > 840_000_000 {
            return erro_ao_cliente(ctx, erro::INJETAR_EXP);
        }
        exp * 5
    } else {
        if exp == 0 || ctx.p.exp <= 0 || exp as i64 > ctx.p.exp {
            return erro_ao_cliente(ctx, erro::INJETAR_EXP);
        }
        exp
    };
    // `InsertExp(exp, GetHistoricalMaxLevel(), ...)`: o nível do jogador é o de quem dá.
    let nivel = ctx.p.level.clamp(0, i16::MAX as i32) as i16;
    let tabela = &ctx.dados.progressao;
    let Some(d) = ctx.p.daimon.as_mut() else { return };
    let (gasto, subiu) = d.inserir_exp(exp, nivel, nivel, |n| tabela.exp_para_subir(n as i32).clamp(0, u32::MAX as i64) as u32);
    if gasto == 0 {
        return erro_ao_cliente(ctx, erro::INJETAR_EXP);
    }
    // `skill_point -= (_exp % 5 ? _exp / 5 + 1 : _exp / 5)` ou `exp -= _exp`.
    if com_sp {
        let sp = if gasto % 5 != 0 { gasto / 5 + 1 } else { gasto / 5 };
        ctx.p.sp -= sp as i64;
    } else {
        ctx.p.exp -= gasto as i64;
    }
    ctx.mudou = true;
    if subiu {
        ficha_do_vestido(ctx);
    } else if let Some(d) = ctx.p.daimon.as_mut() {
        d.sujo = true;
        if let Some(p) = ctx.sub.elf_exp(d.estado.exp.min(i32::MAX as u32) as i32) {
            ctx.para_mim.push(p.data);
        }
    }
}

/// `ElfEquipItem` (`player.cpp:21985-22014`): a peça sai da bolsa (`PLAYER_DROP_ITEM` com `DROP_TYPE_TAKEOUT`) e
/// vai para o Daimon; a ficha do item e `ELF_CMD_RESULT(ELF_EQUIP_ITEM, daimon, peça, 0)`.
fn vestir_peca(ctx: &mut Contexto, slot: usize) {
    if slot >= ctx.bolsa.slots.len() {
        return erro_ao_cliente(ctx, erro::FATAL);
    }
    let Some(peca_id) = ctx.bolsa.item_no_slot(slot).map(|i| i.item_id) else {
        return erro_ao_cliente(ctx, erro::VESTIR_PECA);
    };
    let (Some(peca), Some(atuais)) = (ctx.dados.equipamento_de_daimon(peca_id), atuais_do_vestido(ctx)) else {
        return erro_ao_cliente(ctx, erro::VESTIR_PECA);
    };
    let Some(d) = ctx.p.daimon.as_mut() else { return };
    if !d.estado.equipar(peca_id, &peca, atuais.mascara, false) {
        return erro_ao_cliente(ctx, erro::VESTIR_PECA);
    }
    let daimon = d.item_id as i32;
    ctx.para_mim.push(
        ctx.sub
            .player_drop_item(ContainerType::Inventory.pacote_do_cliente().unwrap_or(0), slot as u8, 1, peca_id as i32, DROP_TIRADO)
            .data,
    );
    ctx.bolsa.tirar_do_slot(slot, 1);
    ficha_do_vestido(ctx);
    ctx.para_mim.push(S2CGamedataSend::elf_cmd_result(resultado::VESTIR_PECA, daimon, peca_id as i32, 0).data);
}

/// O Daimon da bolsa no `slot` (`it.body->GetItemType() == ITEM_TYPE_ELF`): `(id do item, estado)`.
fn daimon_da_bolsa(ctx: &Contexto, slot: usize) -> Option<(u32, Daimon)> {
    let i = ctx.bolsa.item_no_slot(slot)?;
    ctx.dados.dados_do_daimon(i.item_id)?;
    Some((i.item_id, Daimon::ler(&i.octets)?))
}

/// `PlayerGetItemInfo(IL_INVENTORY, slot)` com o bloco novo.
fn gravar_na_bolsa(ctx: &mut Contexto, slot: usize, item_id: u32, d: &Daimon) {
    let bloco = d.bloco();
    ctx.bolsa.trocar_octetos(slot, bloco.clone());
    ctx.mudou = true;
    ctx.para_mim.push(
        S2CGamedataSend::item_info(
            ContainerType::Inventory.pacote_do_cliente().unwrap_or(0),
            slot as u8,
            item_id as i32,
            0,
            0,
            1,
            &bloco,
            None,
        )
        .data,
    );
}

/// `ElfLearnSkill` (`player.cpp:22217-22243`) → `elf_item::LearnSkill` → `SkillWrapper::ElfLearnSkill`
/// (`skillwrapper.cpp:1570-1605`): só habilidade de Daimon (`cls` 258) até o `max_level`, a máscara de nível e
/// gênios, o SP e o item exigidos (o item sai da bolsa, o SP é descontado).
fn aprender(ctx: &mut Contexto, slot: usize, habilidade: u16) {
    if slot >= ctx.bolsa.slots.len() {
        return erro_ao_cliente(ctx, erro::FATAL);
    }
    let Some((item_id, mut d)) = daimon_da_bolsa(ctx, slot) else {
        return erro_ao_cliente(ctx, erro::APRENDER);
    };
    let iniciais = ctx.dados.atributos_iniciais_do_daimon(item_id).unwrap_or([0; 4]);
    let atuais = d.atuais(iniciais, |id| ctx.dados.equipamento_de_daimon(id));
    let Some(nivel) = d.proximo_nivel_de_habilidade(habilidade, atuais.vagas) else {
        return erro_ao_cliente(ctx, erro::APRENDER);
    };
    let Some(h) = ctx.dados.habilidades.get(habilidade as u32) else {
        return erro_ao_cliente(ctx, erro::APRENDER);
    };
    let n = nivel as i32;
    let (Some(mascara), Some(sp)) = (h.nivel_exigido(n), h.sp_exigido(n)) else {
        return erro_ao_cliente(ctx, erro::APRENDER);
    };
    let item = h.item_exigido(n).unwrap_or(0);
    if n > h.max_level
        || h.cls != Some(258)
        || !requisito_de_daimon_cumprido(mascara, d.nivel, atuais.genios)
        || sp as i64 > ctx.p.sp
        || (item > 0 && ctx.bolsa.contar(item as u32) == 0)
    {
        return erro_ao_cliente(ctx, erro::APRENDER);
    }
    if item > 0 {
        use crate::missoes::Jogador;
        ctx.tirar_item(item as u32, 1, true);
    }
    ctx.p.sp -= sp as i64;
    d.aprender(habilidade, nivel);
    gravar_na_bolsa(ctx, slot, item_id, &d);
    ctx.para_mim.push(S2CGamedataSend::elf_cmd_result(resultado::APRENDER, item_id as i32, habilidade as i32, nivel as i32).data);
}

/// `ElfForgetSkill` (`player.cpp:22245-22270`) → `elf_item::ForgetSkill`.
fn esquecer(ctx: &mut Contexto, slot: usize, habilidade: u16, niveis: i16) {
    if slot >= ctx.bolsa.slots.len() {
        return erro_ao_cliente(ctx, erro::FATAL);
    }
    let Some((item_id, mut d)) = daimon_da_bolsa(ctx, slot) else {
        return erro_ao_cliente(ctx, erro::ESQUECER);
    };
    if d.esquecer(habilidade, niveis).is_none() {
        return erro_ao_cliente(ctx, erro::ESQUECER);
    }
    gravar_na_bolsa(ctx, slot, item_id, &d);
    ctx.para_mim.push(S2CGamedataSend::elf_cmd_result(resultado::ESQUECER, item_id as i32, habilidade as i32, niveis as i32).data);
}

/// B355 — `ERR_CAST_ELF_SKILL_FAILED`/`_IN_COOLDOWN` (`common/protocol.h`, contados do `ERR_SUCCESS`).
const CONJURAR_FALHOU: i32 = 128;
const CONJURAR_EM_RECARGA: i32 = 131;
/// `COOLDOWN_INDEX_CAST_ELF_SKILL` (30º de `gs/cooldowncfg.h`) e `CAST_ELF_SKILL_COOLDOWN_TIME` (1 s, `:23`).
const RECARGA_DE_CONJURAR: i32 = 30;
const RECARGA_DE_CONJURAR_MS: u64 = 1000;
/// `COOLINGID_BEGIN`.
const INICIO_DAS_RECARGAS: i32 = 1024;
/// `TYPE_PASSIVE`.
const TIPO_PASSIVA: i32 = 5;

/// O que o `Calculate` da habilidade de Daimon decidiu.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct CalculoDoDaimon {
    pub vigor: i32,
    pub resistencia: i32,
    pub fisico: i32,
    pub magico: [i32; 5],
}

/// B355 — avalia o `Calculate` da habilidade de Daimon (`[quem, setter, expressão]` do catálogo) com as variáveis
/// do jogador e do Daimon (`P_Elfstr` = força final × `ELFSTR_ADJ_IN_SKILL_ATTACK` 1.0, `P_Elfagi` idem,
/// `P_Elfvit`, `P_Elfeng`, `P_Elflevel`, `P_Elfmp` = o vigor, `playerwrapper.cpp:2669-2697`). Os setters do jogador
/// são `SetDecelfmp`/`SetDecelfap` (`playerwrapper.h:130-131`); os da habilidade, `SetDamage` e `Set<escola>damage`.
/// `None` se alguma expressão não se deixar avaliar.
pub(crate) fn calcular_habilidade_do_daimon(
    passos: &[(String, String, String)],
    nivel: i32,
    jogador: &HashMap<&'static str, f64>,
) -> Option<CalculoDoDaimon> {
    let vars = |n: &str| -> Option<f64> {
        if n == "L" {
            return Some(nivel as f64);
        }
        if let Some(x) = n.strip_prefix("P_") {
            return jogador.get(x).copied();
        }
        match n {
            "S_T0" | "S_T1" | "S_T2" => Some(0.0),
            _ => None,
        }
    };
    let mut c = CalculoDoDaimon::default();
    for (quem, setter, expr) in passos {
        let v = pw_data_loader::expr::avaliar(expr, &vars)?;
        match (quem.as_str(), setter.as_str()) {
            ("P", "Decelfmp") => c.vigor = v as i32,
            ("P", "Decelfap") => c.resistencia = v as i32,
            ("P", "Perform") => {}
            ("S", "Damage") => c.fisico = v as i32,
            ("S", "Golddamage") => c.magico[0] = v as i32,
            ("S", "Wooddamage") => c.magico[1] = v as i32,
            ("S", "Waterdamage") => c.magico[2] = v as i32,
            ("S", "Firedamage") => c.magico[3] = v as i32,
            ("S", "Earthdamage") => c.magico[4] = v as i32,
            _ => return None,
        }
    }
    Some(c)
}

/// `ElfCondition`: resistência pedida, `apcost / 1000 + apcost % 1000 × (nível − 1)` (`skill.cpp:174-176`).
pub(crate) fn resistencia_pedida(apcost: i32, nivel: i32) -> i32 {
    apcost / 1000 + apcost % 1000 * (nivel - 1)
}

impl BusServer {
    /// C2S `CAST_ELF_SKILL` (116) `{u16 skill_id; u8 force_attack; u8 target_count; int targets[]}` (`playercmd.cpp:
    /// 3478-3494`) → `gplayer_imp::CastElfSkill` (`player.cpp:22045-22100`) → `SkillWrapper::RunElfSkill`
    /// (`cskill/skill/skillwrapper.cpp:1638-1666`): o Daimon vestido tem a habilidade; nem a recarga comum (30, 1 s)
    /// nem a dela; instantânea (`time_type` 1) de `cls` 258; o `ElfCondition` (não passiva, vigor ≥ `GetMpcost`,
    /// resistência ≥ a pedida, não montado, alvo ao alcance + 1 m); o estado 0 roda (`InstantRun`): gasta vigor e
    /// resistência, calcula o dano com os atributos do Daimon e ataca/amaldiçoa/abençoa como a habilidade do jogador
    /// (`SetPerform`). Sucesso: as duas recargas e `CAST_ELF_SKILL` (269) a quem vê. Falha: `ERR_CAST_ELF_SKILL_*`.
    /// **Não é igual:** golpe de habilidade de Daimon em **jogador** (PvP) não sai; terra/ar/água e `itemcost` não são
    /// conferidos; o `_elf_en` (realce de atributo por efeito) é 0.
    pub(super) async fn conjurar_habilidade_do_daimon(&self, roleid: i32, c: &[u8], envio: &EnvioAoCliente) {
        if c.len() < 4 || c.len() != 4 + 4 * c[3] as usize {
            self.responder(roleid, S2CGamedataSend::error_message(erro::FATAL).data, envio).await;
            return;
        }
        let habilidade = u16::from_le_bytes([c[0], c[1]]) as i32;
        let alvos: Vec<i32> =
            (0..c[3] as usize).map(|i| i32::from_le_bytes(c[4 + 4 * i..8 + 4 * i].try_into().unwrap())).collect();
        let preparo = {
            let mundo = self.world.read().await;
            let Some(p) = mundo.players.get(&(roleid as i64)) else { return };
            preparar_habilidade_do_daimon(&mundo, p, habilidade, &alvos)
        };
        let (nivel, calc, golpe, alvo, recarga_ms) = match preparo {
            Ok(x) => x,
            Err(codigo) => {
                debug!("mundo: {roleid} não conjurou a habilidade de Daimon {habilidade} (erro {codigo})");
                self.responder(roleid, S2CGamedataSend::error_message(codigo).data, envio).await;
                return;
            }
        };
        // O estado 0: gasta vigor e resistência (`OI_DrainElfVigor`/`OI_DrainElfStamina`, `player.cpp:22594-22616`).
        let (vigor, maximo, ganho, resistencia) = {
            let mut mundo = self.world.write().await;
            let limites = mundo.players.get(&(roleid as i64)).and_then(|p| mundo.limites_do_vigor(p)).unwrap_or((0.0, 0.0));
            let Some(p) = mundo.players.get_mut(&(roleid as i64)) else { return };
            p.vigor_do_daimon -= (calc.vigor as f32).min(p.vigor_do_daimon.floor()).max(0.0);
            let agora = std::time::Instant::now();
            p.recargas.insert(RECARGA_DE_CONJURAR, agora + std::time::Duration::from_millis(RECARGA_DE_CONJURAR_MS));
            if recarga_ms > 0 {
                p.recargas.insert(habilidade + INICIO_DAS_RECARGAS, agora + std::time::Duration::from_millis(recarga_ms as u64));
            }
            let vigor = p.vigor_do_daimon as i32;
            let Some(d) = p.daimon.as_mut() else { return };
            d.estado.vigor -= calc.resistencia.clamp(0, d.estado.vigor.max(0));
            d.sujo = true;
            (vigor, limites.0 as i32, (limites.1 * 100.0 + 0.5) as i32, d.estado.vigor)
        };
        self.responder(roleid, S2CGamedataSend::elf_vigor(vigor, maximo, ganho).data, envio).await;
        self.responder(roleid, S2CGamedataSend::elf_stamina(resistencia).data, envio).await;
        self.responder(roleid, S2CGamedataSend::set_cooldown(RECARGA_DE_CONJURAR, RECARGA_DE_CONJURAR_MS as i32).data, envio).await;
        if recarga_ms > 0 {
            self.responder(roleid, S2CGamedataSend::set_cooldown(habilidade + INICIO_DAS_RECARGAS, recarga_ms).data, envio).await;
        }
        let no_alvo = if alvo == roleid as i64 { 0 } else { alvo as i32 };
        let aviso = S2CGamedataSend::cast_elf_skill(roleid, no_alvo, habilidade, nivel.clamp(0, 255) as u8).data;
        self.responder(roleid, aviso.clone(), envio).await;
        self.transmitir_a_outros(roleid, aviso).await;
        let aplicou = self.aplicar_habilidade_com(roleid, habilidade, alvo, nivel, 1.0, envio, Some(golpe)).await;
        info!(
            "mundo: {roleid} conjurou a habilidade de Daimon {habilidade} (nível {nivel}) em {alvo}: vigor -{}, resistência -{}, dano {}/{:?}{}",
            calc.vigor,
            calc.resistencia,
            calc.fisico,
            calc.magico,
            if aplicou { "" } else { " — sem efeito portado (alvo jogador?)" }
        );
    }
}

/// O `RunElfSkill` até o `ElfCondition` e o `Calculate`: `(nível, cálculo, golpe, alvo, recarga)` ou o erro.
fn preparar_habilidade_do_daimon(
    mundo: &crate::world::WorldInstance,
    p: &PlayerEntity,
    habilidade: i32,
    alvos: &[i32],
) -> Result<(i32, CalculoDoDaimon, crate::combat::Golpe, i64, i32), i32> {
    let d = p.daimon.as_ref().ok_or(CONJURAR_FALHOU)?;
    let nivel = d
        .estado
        .habilidades
        .iter()
        .find(|(id, _)| *id as i32 == habilidade)
        .map(|(_, n)| *n as i32)
        .ok_or(CONJURAR_FALHOU)?;
    let agora = std::time::Instant::now();
    let em_recarga = |i: i32| p.recargas.get(&i).is_some_and(|ate| *ate > agora);
    if em_recarga(RECARGA_DE_CONJURAR) || em_recarga(habilidade + INICIO_DAS_RECARGAS) {
        return Err(CONJURAR_EM_RECARGA);
    }
    let h = mundo.data_manager.habilidades.get(habilidade as u32).cloned().ok_or(CONJURAR_FALHOU)?;
    if h.cls != Some(258) || h.time_type != Some(1) || h.tipo == Some(TIPO_PASSIVA) {
        return Err(CONJURAR_FALHOU);
    }
    let dh = h.daimon.clone().ok_or(CONJURAR_FALHOU)?;
    let iniciais = mundo.data_manager.atributos_iniciais_do_daimon(d.item_id).unwrap_or([0; 4]);
    let a = d.estado.atuais(iniciais, |id| mundo.data_manager.equipamento_de_daimon(id));
    let mut vars = super::habilidades::vars_do_jogador(p);
    vars.insert("Ap", p.ap as f64);
    vars.insert("Elfstr", a.atributos[0] as f64);
    vars.insert("Elfagi", a.atributos[1] as f64);
    vars.insert("Elfvit", a.atributos[2] as f64);
    vars.insert("Elfeng", a.atributos[3] as f64);
    vars.insert("Elflevel", d.estado.nivel as f64);
    vars.insert("Elfmp", p.vigor_do_daimon.floor() as f64);
    let pede_vigor = dh
        .vigor
        .as_deref()
        .and_then(|e| {
            pw_data_loader::expr::avaliar(e, &|n| {
                if n == "L" {
                    Some(nivel as f64)
                } else {
                    n.strip_prefix("P_").and_then(|x| vars.get(x).copied())
                }
            })
        })
        .ok_or(CONJURAR_FALHOU)?;
    // `GetElfmp() < (int)GetMpcost(skill)` e `GetElfap() < apcost/1000 + ...` (`skill.cpp:172-176`).
    if (p.vigor_do_daimon as i32) < pede_vigor as i32 || d.estado.vigor < resistencia_pedida(h.apcost.unwrap_or(0), nivel) {
        return Err(CONJURAR_FALHOU);
    }
    if p.montaria.is_some() {
        return Err(CONJURAR_FALHOU);
    }
    let calc = calcular_habilidade_do_daimon(dh.calculo.as_deref().ok_or(CONJURAR_FALHOU)?, nivel, &vars).ok_or(CONJURAR_FALHOU)?;
    // `!(range.IsSelf() || (range.IsSelfBall() && !IsAttack() && !IsCurse()))` → `CheckTarget(1 + GetPraydistance)`.
    let area = h.tipo_de_area.unwrap_or(0);
    let alvo = alvos.first().copied().map(|a| a as i64).filter(|a| *a != p.role_id as i64);
    let de_si = area == 5 || (area == 2 && h.tipo != Some(1) && h.tipo != Some(3));
    if !de_si {
        let alcance = h.alcance(nivel, p.attack_range).unwrap_or(0.0) + 1.0;
        let pos = alvo.and_then(|a| {
            mundo.players.get(&a).map(|q| q.position).or_else(|| mundo.monsters.get(&a).map(|(m, _)| m.position))
        });
        match pos {
            Some(pos) if pos.distance(&p.position) <= alcance => {}
            _ => return Err(CONJURAR_FALHOU),
        }
    }
    let mut golpe = CombatEngine::golpe_de_jogador(p);
    golpe.de_habilidade = true;
    golpe.dano_fisico = calc.fisico;
    for (e, v) in calc.magico.iter().enumerate() {
        // `Set<escola>damage`: `(int)(x × (1.0 + GetInc<escola>()))` (`skill.cpp:899-903`).
        golpe.dano_magico[e] = (*v as f64 * (1.0 + p.passivas_comuns.elemento[e] as f64)) as i32;
    }
    golpe.e_fisico = h.attr == Some(1);
    golpe.taxa_de_ataque = (golpe.taxa_de_ataque as f32 * h.precisao(nivel)) as i32;
    let recarga = h.recarga_armada_ms(nivel).unwrap_or(0);
    Ok((nivel, calc, golpe, alvo.unwrap_or(p.role_id as i64), recarga))
}

#[cfg(test)]
mod testes_da_conjuracao {
    use super::*;

    fn calculo(id: &str) -> Vec<(String, String, String)> {
        let j: serde_json::Value = serde_json::from_str(pw_data_loader::habilidades::json_do_servidor("1.5.5").unwrap()).unwrap();
        serde_json::from_value(j["habilidades"][id]["daimon"]["calculo"].clone()).unwrap()
    }

    /// B355 — o `Calculate` da 1000 (`skill1000.h:39-46`) e da 1004 (fogo + físico), à mão.
    #[test]
    fn o_calculo_do_daimon_e_o_do_stub() {
        let vars = HashMap::from([("Elfstr", 50.0), ("Elflevel", 10.0)]);
        let c = calcular_habilidade_do_daimon(&calculo("1000"), 1, &vars).unwrap();
        // vigor 80 − 10 × 0,4; madeira (0,9 × 6) × 1,5 × 9,3 = 75,33; metal (0,1 × 6) × 1,5 × 9,3 = 8,37.
        assert_eq!(c, CalculoDoDaimon { vigor: 76, resistencia: 200, fisico: 0, magico: [8, 75, 0, 0, 0] });
        let c = calcular_habilidade_do_daimon(&calculo("1004"), 3, &HashMap::new()).unwrap();
        // nível 3: 5 + 2 × 13 = 31; fogo 0,6 × 31 × 11 = 204,6; físico 0,4 × 31 × 11 = 136,4; vigor 32; resistência 45.
        assert_eq!(c, CalculoDoDaimon { vigor: 32, resistencia: 45, fisico: 136, magico: [0, 0, 0, 204, 0] });
        assert!(calcular_habilidade_do_daimon(&calculo("1000"), 1, &HashMap::new()).is_none(), "sem a força do Daimon não avalia");
        assert_eq!(resistencia_pedida(200_000, 1), 200);
        assert_eq!(resistencia_pedida(15_015, 3), 45);
    }
}
