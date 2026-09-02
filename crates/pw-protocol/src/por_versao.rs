//! Os subcomandos cujo layout **depende da versão do jogo**.
//!
//! # Por que este módulo existe
//!
//! Durante muito tempo o projeto teve um layout só, escrito a partir do IR do 1.5.3, valendo
//! para os três realms. Não era descuido: **não havia um segundo layout para o qual
//! ramificar**, e inventar os bytes do outro ramo seria a prática que o item 46 acabou de
//! desfazer em treze comandos.
//!
//! Isso mudou. Uma captura de um servidor 1.2.6 em funcionamento (itens 54 a 56) mediu 175
//! comandos: **106 são idênticos ao 1.5.3 e 32 diferem**. Os que diferem estão aqui, com o
//! layout de cada versão vindo de **bytes observados**, não de dedução.
//!
//! # O que está aqui e o que não está
//!
//! Só entram comandos com diferença **medida**. Um comando que a captura mostrou idêntico
//! continua onde estava, como função de [`S2CGamedataSend`] — duplicá-lo aqui só criaria
//! dois lugares para a mesma verdade.
//!
//! # As três famílias de diferença
//!
//! **1. `attack_flag` era `char` e virou `int`.** Aparece em cinco comandos de resultado de
//! ataque, sempre com a mesma assinatura de −3 bytes (ou −4, quando o `section` também
//! falta). Cinco medições independentes concordando é o que separa padrão de coincidência.
//!
//! **2. Um campo no fim que o 1.5.3 acrescentou.** `NPC_INFO_00` e `PLAYER_INFO_00` ganharam
//! `iTargetID`; `INST_DATA_CHECKOUT` ganhou um quinto timestamp; `ENTER_SANCTUARY` e
//! `LEAVE_SANCTUARY` ganharam o `id` — no 1.2.6 eles não têm payload nenhum.
//!
//! **3. Campos que eram de 16 bits e viraram 32.** `RECEIVE_EXP` e as contagens do
//! `EQUIP_ITEM`.
//!
//! # A ironia que vale registrar
//!
//! Três destes — `npc_info_00`, `enter_sanctuary` e `equip_item` — tinham no código, antes
//! do item 46, **exatamente o layout do 1.2.6**. Eu os "corrigi" para o 1.5.3 avisando que
//! era uma aposta. A aposta foi cobrada, e o que volta agora não é o código antigo: é o
//! mesmo layout, desta vez **medido** e com a versão escolhendo qual usar.

use crate::octets::OctetsStream;
use crate::packets::s2c::S2CGamedataSend;
use crate::version::GameVersion;

/// Escreve subcomandos no layout da versão de um realm.
///
/// Guardar a versão numa struct, em vez de passá-la como argumento em cada chamada, é
/// deliberado: um argumento a mais é um argumento que se esquece, e esquecer aqui produz um
/// pacote que o cliente **descarta em silêncio** (item 46) — o pior tipo de erro que este
/// protocolo tem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PorVersao {
    versao: GameVersion,
}

/// O 1.2.6 é o único que temos medido com layout próprio.
///
/// O 1.4.8 fica com o layout do 1.5.3 **por falta de medição**, e não porque saibamos que
/// são iguais: não temos cliente nem servidor daquela versão. É a mesma ressalva do
/// `server_version_code`, e some no dia em que houver uma captura de 1.4.8.
fn e_126(v: GameVersion) -> bool {
    matches!(v, GameVersion::V1_2_6)
}

impl PorVersao {
    pub fn new(versao: GameVersion) -> Self {
        Self { versao }
    }

    pub fn versao(&self) -> GameVersion {
        self.versao
    }

    /// `HOST_ATTACKRESULT` (24) — o resultado do golpe do próprio jogador.
    ///
    /// | | 1.2.6 | 1.5.3 |
    /// | :--- | ---: | ---: |
    /// | `idTarget` | 4 | 4 |
    /// | `iDamage` | 4 | 4 |
    /// | `attack_flag` | **1** | **4** |
    /// | `attack_speed` | 1 | 1 |
    /// | | **10** | **13** |
    ///
    /// Medido: 52 ocorrências de 10 bytes, com o dano variando e os dois últimos bytes
    /// estáveis (`00 10`).
    pub fn host_attack_result(
        &self,
        target_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
    ) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::host_attack_result(target_id, damage, attack_flag, speed);
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(24);
        s.write_i32_le(target_id);
        s.write_i32_le(damage);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `HOST_ATTACKED` (26) — o jogador levou um golpe.
    ///
    /// 11 bytes no 1.2.6 contra 14: `idAttacker(4) iDamage(4) cEquipment(1)
    /// attack_flag(**1**) speed(1)`. Medido em 25 ocorrências, todas com
    /// `cEquipment = 0x7f` e `speed = 0x1b`.
    pub fn host_attacked(
        &self,
        atacante: i32,
        dano: i32,
        equipamento: u8,
        attack_flag: i32,
        speed: u8,
    ) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::host_attacked(atacante, dano, attack_flag);
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(26);
        s.write_i32_le(atacante);
        s.write_i32_le(dano);
        s.write_u8(equipamento);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `HOST_SKILL_ATTACK_RESULT` (142) — o dano de uma habilidade do jogador.
    ///
    /// 14 bytes no 1.2.6 contra 18: o `attack_flag` cabe em 1 byte **e não existe
    /// `section`**. `4+4+4+1+1 = 14`, que é exatamente o observado em 18 ocorrências.
    pub fn self_skill_attack_result(
        &self,
        target_id: i32,
        skill_id: i32,
        damage: i32,
        attack_flag: i32,
        speed: u8,
        section: u8,
    ) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::self_skill_attack_result(
                target_id,
                skill_id,
                damage,
                attack_flag,
                speed,
                section,
            );
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(142);
        s.write_i32_le(target_id);
        s.write_i32_le(skill_id);
        s.write_i32_le(damage);
        s.write_i8(estreitar(attack_flag));
        s.write_u8(speed);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `NPC_INFO_00` (33) — a barra de vida de um monstro.
    ///
    /// **12 bytes no 1.2.6: sem o `iTargetID`.** Medido em 80 ocorrências, com o `iHP`
    /// caindo (29 → 22 → 17 → 11 → 2) enquanto o `iMaxHP` fica em 29 — o que também
    /// confirma a ordem dos dois.
    pub fn npc_info_00(&self, nid: i32, hp: i32, max_hp: i32, alvo: i32) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::npc_info_00(nid, hp, max_hp, alvo);
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(33);
        s.write_i32_le(nid);
        s.write_i32_le(hp);
        s.write_i32_le(max_hp);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `PLAYER_INFO_00` (32) — a barra de vida de outro jogador.
    ///
    /// **24 bytes no 1.2.6: sem o `iTargetID`**, igual ao 33. Medido em 73 ocorrências.
    #[allow(clippy::too_many_arguments)]
    pub fn player_info_00(
        &self,
        player_id: i32,
        level: i16,
        level2: u8,
        hp: i32,
        max_hp: i32,
        mp: i32,
        max_mp: i32,
        alvo: i32,
    ) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::player_info_00(
                player_id, level, level2, hp, max_hp, mp, max_mp, alvo,
            );
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(32);
        s.write_i32_le(player_id);
        s.write_i16_le(level);
        s.write_u8(0); // State
        s.write_u8(level2);
        s.write_i32_le(hp);
        s.write_i32_le(max_hp);
        s.write_i32_le(mp);
        s.write_i32_le(max_mp);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `RECEIVE_EXP` (36) — a experiência e o SP de um abate.
    ///
    /// **4 bytes no 1.2.6: os dois campos são de 16 bits.** As 36 ocorrências trazem sete
    /// valores distintos, e um deles fecha a leitura sozinho: `(15, 36)` e `(30, 72)` —
    /// exatamente o dobro. Lido como um `int` só, seriam 2.359.311 e 4.718.622, que não é
    /// experiência de um abate no nível 3.
    pub fn receive_exp(&self, exp: i32, sp: i32) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::receive_exp(exp, sp);
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(36);
        s.write_u16_le(estreitar_u16(exp));
        s.write_u16_le(estreitar_u16(sp));
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `EQUIP_ITEM` (48) — o item saiu da bolsa e foi para o corpo.
    ///
    /// **6 bytes no 1.2.6: as duas contagens são `unsigned short`.** Medido em 9
    /// ocorrências, com os índices variando (`07 00`, `06 04`, `04 01`) e as contagens
    /// alternando entre `01 00` e `00 00`.
    pub fn equip_item(
        &self,
        idx_ivtr: u8,
        idx_equip: u8,
        count_ivtr: u32,
        count_equip: u32,
    ) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::equip_item(idx_ivtr, idx_equip, count_ivtr, count_equip);
        }
        let mut s = OctetsStream::new();
        s.write_u16_le(48);
        s.write_u8(idx_ivtr);
        s.write_u8(idx_equip);
        s.write_u16_le(count_ivtr.min(u16::MAX as u32) as u16);
        s.write_u16_le(count_equip.min(u16::MAX as u32) as u16);
        S2CGamedataSend { data: s.into_bytes().to_vec() }
    }

    /// `ENTER_SANCTUARY` (164) — entrou na zona segura.
    ///
    /// **Sem payload no 1.2.6.** Medido em 11 ocorrências, todas com zero bytes. O `id`
    /// que o 1.5.3 carrega não existe naquela versão.
    pub fn enter_sanctuary(&self, id: i32) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::enter_sanctuary(id);
        }
        so_o_cabecalho(164)
    }

    /// `LEAVE_SANCTUARY` (165) — saiu da zona segura. Mesma história do 164, 9 ocorrências.
    pub fn leave_sanctuary(&self, id: i32) -> S2CGamedataSend {
        if !e_126(self.versao) {
            return S2CGamedataSend::leave_sanctuary(id);
        }
        so_o_cabecalho(165)
    }

    /// `INST_DATA_CHECKOUT` (206) — os carimbos de tempo dos dados do servidor.
    ///
    /// **16 bytes no 1.2.6: quatro campos, sem o `gshop_time_stamp2`.** A amostra mostra
    /// `idInst = 1` e três carimbos, dois deles iguais — o padrão de
    /// `region`/`precinct`/`gshop`.
    ///
    /// **26 bytes no 1.5.5: um sexto campo, `gshop_time_stamp3`**, achado comparando o
    /// IR do 1.5.5 com o do 1.5.3 (2026-09-02) — o 1.5.3 já tinha cinco campos (o quarto e
    /// quinto são o `gshop`/`gshop2` que o `S2CGamedataSend::inst_data_checkout` de baixo
    /// escreve, hoje com o mesmo valor nos dois — questão em aberto, não desta mudança).
    /// `gshop3` só entra quando a versão é 1.5.5 **e** o chamador passa `Some`; do
    /// contrário sai o layout de cinco campos de sempre.
    pub fn inst_data_checkout(
        &self,
        id_inst: i32,
        region: u32,
        precinct: u32,
        gshop: u32,
        gshop3: Option<u32>,
    ) -> S2CGamedataSend {
        if e_126(self.versao) {
            let mut s = OctetsStream::new();
            s.write_u16_le(206);
            s.write_i32_le(id_inst);
            s.write_u32_le(region);
            s.write_u32_le(precinct);
            s.write_u32_le(gshop);
            return S2CGamedataSend { data: s.into_bytes().to_vec() };
        }

        let base = S2CGamedataSend::inst_data_checkout(id_inst, region, precinct, gshop);
        match gshop3 {
            Some(g3) if self.versao == GameVersion::V1_5_5 => {
                let mut bytes = base.data;
                bytes.extend_from_slice(&g3.to_le_bytes());
                S2CGamedataSend { data: bytes }
            }
            _ => base,
        }
    }
}

/// Um comando sem payload — só o cabeçalho de 2 bytes.
fn so_o_cabecalho(id: u16) -> S2CGamedataSend {
    let mut s = OctetsStream::new();
    s.write_u16_le(id);
    S2CGamedataSend { data: s.into_bytes().to_vec() }
}

/// Encolhe um `attack_flag` de 32 bits para os 8 do 1.2.6, **saturando**.
///
/// Truncar seria pior que perder o valor: um `flag` com um bit alto ligado viraria zero, e
/// zero significa "golpe comum". Saturar preserva ao menos "houve alguma marcação".
///
/// Hoje o `attack_flag` sai sempre zero — os bits dele não estão em nenhuma fonte que
/// temos (ver `SEM_MARCACAO` no `pw-gs`) —, então este cuidado é para o dia em que
/// estiverem.
fn estreitar(v: i32) -> i8 {
    v.clamp(i8::MIN as i32, i8::MAX as i32) as i8
}

/// Encolhe exp/sp de 32 para 16 bits, com teto em vez de estouro.
///
/// Um abate que desse mais de 65.535 de experiência truncaria para um número pequeno e
/// aleatório — o jogador veria "ganhou 3 de exp" ao matar um chefe. Com teto ele vê o
/// máximo que a versão consegue representar, que é errado por menos.
fn estreitar_u16(v: i32) -> u16 {
    v.clamp(0, u16::MAX as i32) as u16
}
