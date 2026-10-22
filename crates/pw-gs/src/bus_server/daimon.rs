//! B345 — o Daimon (o `elf_item` do original): distribuir atributos e gênios, injetar experiência, vestir peça e,
//! nos NPCs, aprender e esquecer habilidade. Porte de `gplayer_imp::Elf*` (`gs/player.cpp:21856-22270`) sobre as
//! regras do `elf_item` (`gs/item/item_elf.cpp`). Só o 1.5.5 tem Daimon.
//!
//! **Não é igual / `falta`:** conjurar habilidade do Daimon (`CAST_ELF_SKILL`, 116: o catálogo não tem o dano —
//! os stubs usam `GetElfstr`/`GetElflevel`, que o extrator não avaliou), o estado de segurança (115), recarregar
//! (117), vigor e resistência do Daimon, o bônus de atributo sorteado por nível, refino, decomposição, destruir
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
