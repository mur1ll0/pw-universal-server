//! Painel (E6, B194): editar as propriedades de um item — **edição livre nos valores, não no
//! formato** (decisão do Murillo, 2026-10-07).
//!
//! Os octetos continuam a verdade (B190, memória da reforma §6.14). A edição mexe no bloco do
//! jeito que o próprio servidor original mexe:
//!
//! - o **rabo** (furos, máscara das pedras, efeitos) pelo `ConteudoDeEquipamento::alterar_rabo`,
//!   o mesmo do refino e das pedras (`equip_item::OnRefreshItem`, `equip_item.cpp:83-97`, regrava
//!   a essência intacta);
//! - o **cabeçalho** por remendo nos bytes: a `prerequisition` (6 × i16, **vitalidade antes de
//!   agilidade**, `equip_item.h:230-238`), a durabilidade (2 × i32) e o nome do fabricante
//!   (`item_tag_t`, `generate_item_temp.h:312-314`); a essência fica byte a byte como estava.
//!
//! O bloco novo tem de ser lido inteiro por `ConteudoDeEquipamento::ler` (o `SetItemInfo` do
//! cliente, `EC_IvtrEquip.cpp:176-262`), senão nada muda (`formato_invalido`). Limites de
//! **formato**: até 5 furos (`ELEMENTDATAMAN_MAX_NUM_HOLES`) e 32 efeitos
//! (`ELEMENTDATAMAN_MAX_NUM_ADDONS`, `template/itemdataman.h`), id de efeito em 13 bits e até 3
//! parâmetros (`(type & 0x6000) >> 13`), requisitos em i16, nome até 40 bytes
//! (`MAX_USERNAME_LENGTH`). Refino 0–12: o bônus de cada nível é `base × refine_factor[nível]`
//! (`equip_item.cpp:208-223`), que só vai até 12 — acima disso, pela lista de efeitos (crua).
//!
//! Online o cliente aceita o `OWN_ITEM_INFO` (40) sobre o item que já está no slot: troca
//! quantidade e o bloco inteiro e, no corpo, refaz a aparência do próprio jogador
//! (`OnMsgHstOwnItemInfo`, `EC_HostMsg.cpp:1454-1500`). O id do modelo não muda por ali, então
//! não se edita. Vínculo não: o `state` do `OWN_ITEM_INFO` sai sempre 0 (`s2c.rs`, `item_info`)
//! — o servidor ainda não modela o `proc_type`.

use super::pedras_e_refino::{addons_da_pedra, bloco_do_equipamento, furos_de};
use super::*;
use crate::refino::{self, FATOR_DE_REFINO};
use pw_core::{AddonDoItem, ConteudoDeEquipamento, ItemRecord};
use pw_data_loader::GameDataManager;

/// `ELEMENTDATAMAN_MAX_NUM_HOLES` e `_MAX_NUM_ADDONS` (`gs/template/itemdataman.h`).
pub const MAXIMO_DE_FUROS: usize = 5;
pub const MAXIMO_DE_EFEITOS: usize = 32;
/// Os bits de origem do efeito: pedra, conjunto, gravação (`SetItemInfo`, `EC_IvtrEquip.cpp:240-242`).
const EFEITO_COM_ORIGEM: u32 = 0x8000 | 0x10000 | 0x20000;

/// Os requisitos do cabeçalho, na ordem em que o painel os mostra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequisitosEditados {
    pub nivel: i32,
    pub classes: i32,
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EfeitoEditado {
    pub id: u32,
    #[serde(default)]
    pub args: Vec<i32>,
}

/// O que mudar; campo ausente = fica como está.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EdicaoDeItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantidade: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durabilidade: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durabilidade_maxima: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requisitos: Option<RequisitosEditados>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabricante: Option<String>,
    /// Os efeitos **sem origem** (os de pedra, conjunto e gravação ficam como estão).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub efeitos: Option<Vec<EfeitoEditado>>,
    /// Nível de refino 0–12 (aplicado depois de `efeitos`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refino: Option<i32>,
    /// A pedra de cada furo (0 = vazio); o tamanho é o número de furos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pedras: Option<Vec<u32>>,
}

impl EdicaoDeItem {
    fn mexe_no_bloco(&self) -> bool {
        self.durabilidade.is_some() || self.durabilidade_maxima.is_some() || self.requisitos.is_some()
            || self.fabricante.is_some() || self.efeitos.is_some() || self.refino.is_some() || self.pedras.is_some()
    }

    /// Limites de formato, conferidos antes de tocar em qualquer coisa.
    pub fn valida(&self) -> Result<(), &'static str> {
        let i16_ok = |v: i32| (0..=i16::MAX as i32).contains(&v);
        if self.quantidade == Some(0) || self.quantidade.is_some_and(|q| q > i32::MAX as u32) {
            return Err("quantidade_invalida");
        }
        if self.durabilidade.is_some_and(|d| d < 0) || self.durabilidade_maxima.is_some_and(|d| d < 0) {
            return Err("durabilidade_invalida");
        }
        if let Some(r) = self.requisitos {
            if ![r.nivel, r.forca, r.agilidade, r.vitalidade, r.energia].into_iter().all(i16_ok)
                || !(0..=0xFFFF).contains(&r.classes)
            {
                return Err("requisitos_invalidos");
            }
        }
        if self.fabricante.as_ref().is_some_and(|f| f.encode_utf16().count() * 2 > pw_core::TAMANHO_MAXIMO_DO_NOME) {
            return Err("fabricante_invalido");
        }
        if let Some(e) = &self.efeitos {
            if e.len() > MAXIMO_DE_EFEITOS || e.iter().any(|x| x.id == 0 || x.id > 0x1FFF || x.args.len() > 3) {
                return Err("efeitos_invalidos");
            }
        }
        if self.refino.is_some_and(|n| !(0..FATOR_DE_REFINO.len() as i32).contains(&n)) {
            return Err("refino_invalido");
        }
        if self.pedras.as_ref().is_some_and(|p| p.len() > MAXIMO_DE_FUROS) {
            return Err("pedras_invalidas");
        }
        Ok(())
    }
}

/// Remenda os 6 requisitos (offset 0) do cabeçalho.
fn escrever_requisitos(bloco: &mut [u8], r: &RequisitosEditados) {
    // `prerequisition`: nível, classes, força, **vitalidade**, agilidade, energia.
    for (i, v) in [r.nivel, r.classes, r.forca, r.vitalidade, r.agilidade, r.energia].into_iter().enumerate() {
        bloco[2 * i..2 * i + 2].copy_from_slice(&(v as u16).to_le_bytes());
    }
}

/// Troca o nome do fabricante (`item_tag_t { type; size; name[size] }` no offset 22).
fn escrever_fabricante(bloco: &[u8], nome: &[u8]) -> Option<Vec<u8>> {
    let antigo = *bloco.get(23)? as usize;
    let fim = 24usize.checked_add(antigo)?;
    if bloco.len() < fim {
        return None;
    }
    let mut o = bloco[..23].to_vec();
    o.push(nome.len() as u8);
    o.extend_from_slice(nome);
    o.extend_from_slice(&bloco[fim..]);
    Some(o)
}

/// Aplica a edição ao registro. `Err` sem mudar nada. Atualiza também as colunas espelho
/// (durabilidade, refino, furos) no mesmo registro.
pub fn aplicar(item: &mut ItemRecord, e: &EdicaoDeItem, dados: &GameDataManager) -> Result<(), &'static str> {
    e.valida()?;
    let mut novo = item.clone();
    if let Some(q) = e.quantidade {
        novo.count = q;
    }
    if e.mexe_no_bloco() {
        let (bloco, ficha) = bloco_do_equipamento(item, dados).ok_or("nao_e_equipamento")?;
        let alvo = dados.refino.equipamentos.get(&item.item_id).copied();
        if (e.pedras.is_some() || e.refino.is_some()) && alvo.is_none() {
            return Err("sem_refino_nem_furos");
        }
        let mut erro = None;
        let (mut b, ()) = ConteudoDeEquipamento::alterar_rabo(&bloco, &ficha, |c| {
            if let Some(lista) = &e.efeitos {
                c.addons.retain(|a| a.tipo & EFEITO_COM_ORIGEM != 0);
                c.addons.extend(lista.iter().map(|x| AddonDoItem::novo(x.id, x.args.clone())));
            }
            if let (Some(n), Some(alvo)) = (e.refino, alvo) {
                let id = alvo.addon_de_refino;
                c.addons.retain(|a| a.id() != id);
                if n > 0 {
                    // `RefineAddon`: `base × refine_factor[nível] + 0.1` (`refino::refinar`).
                    match crate::geracao::gerar_addon(dados, id) {
                        Some(g) => {
                            let base = g.args.first().copied().unwrap_or(0);
                            let valor = (base as f32 * FATOR_DE_REFINO[n as usize] + 0.1) as i32;
                            c.addons.push(AddonDoItem { tipo: g.tipo, args: vec![valor, n] });
                        }
                        None => erro = Some("refino_sem_addon"),
                    }
                }
            }
            if let (Some(pedras), Some(alvo)) = (&e.pedras, alvo) {
                refino::limpar_pedras(&mut c.furos, &mut c.addons);
                c.furos = vec![0; pedras.len()];
                for (i, &id) in pedras.iter().enumerate().filter(|(_, id)| **id != 0) {
                    let Some(pedra) = dados.refino.pedras.get(&id).copied() else {
                        erro = Some("pedra_inexistente");
                        return;
                    };
                    // A pedra sem octetos: os addons que `generate_stone` daria (`addons_da_pedra`).
                    let modelo = ItemRecord { item_id: id, octets: Vec::new(), ..item.clone() };
                    let addons = addons_da_pedra(&modelo, &pedra, alvo.familia, dados);
                    c.furos[i] = id as i32;
                    c.addons.extend(addons.iter().map(|a| AddonDoItem { tipo: a.tipo | refino::ADDON_EMBUTIDO, args: a.args.clone() }));
                }
                let tabela = &dados.refino.pedras;
                let cor_e_grau = |id: i32| tabela.get(&(id as u32)).map(|p| (p.cor, p.grau)).unwrap_or((0, 0));
                c.mascara_das_pedras = refino::mascara_das_pedras(alvo.familia, &c.furos, c.mascara_das_pedras, cor_e_grau);
            }
            if c.addons.len() > MAXIMO_DE_EFEITOS {
                erro = Some("efeitos_invalidos");
            }
        })
        .ok_or("formato_invalido")?;
        if let Some(erro) = erro {
            return Err(erro);
        }
        if let Some(r) = &e.requisitos {
            escrever_requisitos(&mut b, r);
        }
        if e.durabilidade.is_some() || e.durabilidade_maxima.is_some() {
            let atual = ConteudoDeEquipamento::ler(&b, &ficha).ok_or("formato_invalido")?;
            let (d, m) = (e.durabilidade.unwrap_or(atual.durabilidade), e.durabilidade_maxima.unwrap_or(atual.durabilidade_maxima));
            if !pw_core::escrever_durabilidade(&mut b, d, m) {
                return Err("formato_invalido");
            }
            novo.durability = d.max(0) as u32;
            novo.max_durability = m.max(0) as u32;
        }
        if let Some(nome) = &e.fabricante {
            b = escrever_fabricante(&b, &pw_core::nome_do_fabricante(nome)).ok_or("formato_invalido")?;
        }
        // O juiz: o bloco inteiro lido, fechando no último byte.
        let lido = ConteudoDeEquipamento::ler(&b, &ficha).ok_or("formato_invalido")?;
        if let Some(alvo) = alvo {
            novo.refine_level = refino::nivel_de_refino(&lido.addons, alvo.addon_de_refino).clamp(0, 255) as u8;
        }
        novo.sockets = furos_de(&b, &ficha);
        novo.sockets_count = novo.sockets.len() as u8;
        novo.octets = b;
    }
    *item = novo;
    Ok(())
}

impl BusServer {
    /// Online: aplica, grava o slot e reenvia a ficha do item (`OWN_ITEM_INFO`). No corpo,
    /// refaz o equipamento (atributos e o aviso de equipamento a quem vê). Armazém só offline (o
    /// cliente só tem a cópia dele com a sessão de NPC aberta). `None`: não está neste mapa.
    pub(crate) async fn editar_item_pelo_painel(&self, roleid: i32, recipiente: ContainerType, slot: u16, tid: u32,
        e: &EdicaoDeItem) -> Option<serde_json::Value> {
        let envio = self.envio_de(roleid).await?;
        if !self.world.read().await.players.contains_key(&(roleid as i64)) {
            return None;
        }
        let Some(pacote) = recipiente.pacote_do_cliente() else {
            return Some(serde_json::json!({"erro": "precisa_estar_offline"}));
        };
        let item = match self.editar_item_no_banco(roleid, recipiente, slot, tid, e).await {
            Ok(i) => i,
            Err(codigo) => return Some(serde_json::json!({"erro": codigo})),
        };
        let dados = self.world.read().await.data_manager.clone();
        self.responder(roleid, Self::info_de(pacote, &item, &dados), &envio).await;
        if recipiente == ContainerType::Equipment {
            self.recalcular_equipamento(roleid, true).await;
        }
        tracing::info!(roleid, ?recipiente, slot, tid, "painel: item editado (online)");
        Some(serde_json::json!({"erro": null, "item": tid, "slot": slot}))
    }

    /// Lê o slot, confere o item, aplica e grava (o mesmo para online e offline). Offline, quem
    /// chama segura a guarda de presença e a trava de gravação.
    pub(crate) async fn editar_item_no_banco(&self, roleid: i32, recipiente: ContainerType, slot: u16, tid: u32,
        e: &EdicaoDeItem) -> Result<ItemRecord, &'static str> {
        let repo = self.itens().await;
        let mut item = match repo.get_item_by_slot(roleid, recipiente, slot).await {
            Ok(Some(i)) if i.item_id == tid => i,
            Ok(_) => return Err("slot_mudou"),
            Err(_) => return Err("banco_indisponivel"),
        };
        let dados = self.world.read().await.data_manager.clone();
        aplicar(&mut item, e, &dados)?;
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        Ok(item)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_validacao_recusa_o_que_o_formato_nao_comporta() {
        let ok = EdicaoDeItem { refino: Some(12), pedras: Some(vec![0; 5]), ..Default::default() };
        assert_eq!(ok.valida(), Ok(()));
        for (e, codigo) in [
            (EdicaoDeItem { quantidade: Some(0), ..Default::default() }, "quantidade_invalida"),
            (EdicaoDeItem { refino: Some(13), ..Default::default() }, "refino_invalido"),
            (EdicaoDeItem { pedras: Some(vec![0; 6]), ..Default::default() }, "pedras_invalidas"),
            (EdicaoDeItem { efeitos: Some(vec![EfeitoEditado { id: 0x2000, args: vec![] }]), ..Default::default() }, "efeitos_invalidos"),
            (EdicaoDeItem { efeitos: Some(vec![EfeitoEditado { id: 1, args: vec![1, 2, 3, 4] }]), ..Default::default() }, "efeitos_invalidos"),
            (EdicaoDeItem { fabricante: Some("x".repeat(21)), ..Default::default() }, "fabricante_invalido"),
            (EdicaoDeItem { requisitos: Some(RequisitosEditados { nivel: 40000, classes: 1, forca: 0, agilidade: 0, vitalidade: 0, energia: 0 }), ..Default::default() }, "requisitos_invalidos"),
        ] {
            assert_eq!(e.valida(), Err(codigo));
        }
    }

    #[test]
    fn o_fabricante_troca_o_tamanho_e_mantem_o_resto() {
        // cabeçalho 22 bytes, tipo 4, nome de 2 bytes "AB", e a "essência" 9 9.
        let mut bloco = vec![0u8; 22];
        bloco.extend_from_slice(&[4, 2, b'A', b'B', 9, 9]);
        let novo = escrever_fabricante(&bloco, &[1, 0, 2, 0, 3, 0]).unwrap();
        assert_eq!(&novo[22..], &[4, 6, 1, 0, 2, 0, 3, 0, 9, 9]);
    }

    #[test]
    fn os_requisitos_saem_com_vitalidade_antes_de_agilidade() {
        let mut b = vec![0u8; 12];
        escrever_requisitos(&mut b, &RequisitosEditados { nivel: 30, classes: 0x0FFF, forca: 1, agilidade: 2, vitalidade: 3, energia: 4 });
        let v: Vec<u16> = b.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        assert_eq!(v, [30, 0x0FFF, 1, 3, 2, 4]);
    }
}
