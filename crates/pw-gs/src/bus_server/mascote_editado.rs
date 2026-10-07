//! Painel (E6, B197): a jaula de mascotes — ver, editar os campos do registro e libertar.
//!
//! Cada mascote é um item `PetCorral` cujos octetos são o `pet_data`/`info_pet` de 192 bytes
//! ([`pw_core::InfoPet`]); é o que o GS grava (`gravar_mascote`) e manda ao cliente. Online o cliente
//! recebe o slot novo por `PET_ROOM` (`UpdatePets`, `EC_HostMsg.cpp:5390-5400`, o mesmo aviso de
//! `avisar_slot_da_jaula`) e a libertação por `FREE_PET` (`FreePet` + atalho limpo,
//! `EC_HostMsg.cpp:5253-5268`; `pet_manager::FreePet`, `petman.cpp:1505-1520`). O mascote
//! **invocado** não se edita por fora: o mundo tem a cópia viva dele (`world.mascote_de`).
//!
//! Limites de **formato** (o cliente indexa tabelas com eles): fome 0–11 (`HUNGER_LEVEL_COUNT`,
//! `EC_PetCorral.h:61-75`), lealdade 0–999 (`HONOR_POINT_MAX`, `petman.h:102`), nível 1 até o
//! `level_max` do modelo (`PetLevelUpExp.exp[nível-1]`, `DlgHostPet.cpp:140-142`), nome até 16
//! bytes (UTF-16, `pet_data::name`), 8 habilidades (o nível de cada uma é conferido pela API contra
//! o `max_level` do stub do cliente).

use super::*;
use pw_core::{InfoPet, ItemRecord};

/// `HUNGER_LEVEL_COUNT` − 1 e `HONOR_POINT_MAX`.
pub const FOME_MAXIMA: i32 = 11;
pub const LEALDADE_MAXIMA: i32 = 999;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EdicaoDeMascote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nivel: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lealdade: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fome: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pontos: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nome: Option<String>,
    /// Até 8 `[id, nível]`; a lista substitui as habilidades do mascote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub habilidades: Option<Vec<[i32; 2]>>,
    /// Tira o mascote da jaula (os outros campos são ignorados).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub libertar: bool,
}

impl EdicaoDeMascote {
    pub fn valida(&self) -> Result<(), &'static str> {
        if self.lealdade.is_some_and(|v| !(0..=LEALDADE_MAXIMA).contains(&v))
            || self.fome.is_some_and(|v| !(0..=FOME_MAXIMA).contains(&v))
            || self.exp.is_some_and(|v| v < 0)
            || self.pontos.is_some_and(|v| v < 0)
            || self.nivel.is_some_and(|v| v < 1)
        {
            return Err("valores_invalidos");
        }
        if self.nome.as_ref().is_some_and(|n| n.is_empty() || n.encode_utf16().count() * 2 > 16) {
            return Err("nome_invalido");
        }
        if self.habilidades.as_ref().is_some_and(|h| h.len() > 8 || h.iter().any(|[id, n]| *id <= 0 || *n < 1 || *n > 255)) {
            return Err("habilidades_invalidas");
        }
        Ok(())
    }
}

/// O nome do registro (UTF-16LE em `name[..name_len]`).
pub fn nome_do_mascote(info: &InfoPet) -> String {
    let n = (info.name_len as usize).min(16);
    let u: Vec<u16> = info.name[..n].chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
    String::from_utf16_lossy(&u)
}

/// Aplica a edição ao registro. `nivel_maximo` é o `level_max` do modelo.
pub fn aplicar(info: &mut InfoPet, e: &EdicaoDeMascote, nivel_maximo: i32) -> Result<(), &'static str> {
    e.valida()?;
    let mut novo = info.clone();
    if let Some(n) = e.nivel {
        if nivel_maximo > 0 && n > nivel_maximo {
            return Err("nivel_do_mascote_invalido");
        }
        novo.level = n as i16;
    }
    if let Some(v) = e.exp { novo.exp = v; }
    if let Some(v) = e.lealdade { novo.honor_point = v; }
    if let Some(v) = e.fome { novo.hunger = v; }
    if let Some(v) = e.pontos { novo.skill_point = v; }
    if let Some(nome) = &e.nome {
        let bytes: Vec<u8> = nome.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        novo.name = [0; 16];
        novo.name[..bytes.len()].copy_from_slice(&bytes);
        novo.name_len = bytes.len() as u16;
    }
    if let Some(h) = &e.habilidades {
        novo.skills = [(0, 0); 8];
        for (k, [id, n]) in h.iter().enumerate() {
            novo.skills[k] = (*id, *n);
        }
    }
    *info = novo;
    Ok(())
}

impl BusServer {
    /// A jaula do personagem, do banco (a fonte que o GS grava), com o nome e o ícone do modelo.
    pub(crate) async fn mascotes_do_painel(&self, roleid: i32) -> serde_json::Value {
        let itens = match self.itens().await.list_by_container(roleid, ContainerType::PetCorral).await {
            Ok(v) => v,
            Err(_) => return serde_json::json!({"codigo":"banco_indisponivel"}),
        };
        let (dados, invocado) = {
            let mundo = self.world.read().await;
            (mundo.data_manager.clone(), mundo.mascote_de(roleid as i64).map(|m| m.slot))
        };
        let lista: Vec<serde_json::Value> = itens.iter().filter_map(|i| {
            let info = InfoPet::do_bloco(&i.octets)?;
            let (modelo, icone) = dados.mascotes_do_elements.get(&(info.pet_tid as u32)).cloned().unwrap_or_default();
            let nivel_maximo = dados.modelos_de_mascote.get(&(info.pet_tid as u32)).map(|m| m.nivel_maximo);
            Some(serde_json::json!({
                "slot": i.slot, "tid": info.pet_tid, "ovo": info.pet_egg_tid, "nome": nome_do_mascote(&info),
                "modelo": modelo, "icone": icone.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                "nivel": info.level, "nivel_maximo": nivel_maximo, "exp": info.exp, "lealdade": info.honor_point,
                "fome": info.hunger, "pontos": info.skill_point, "vinculo": info.is_bind, "classe": info.pet_class,
                "habilidades": info.skills.iter().filter(|(id, _)| *id > 0).map(|(id, n)| [*id, *n]).collect::<Vec<_>>(),
                "invocado": invocado == Some(i.slot),
            }))
        }).collect();
        serde_json::json!({"mascotes": lista})
    }

    /// Online: aplica, grava e avisa o cliente (`PET_ROOM` do slot, ou `FREE_PET`). `None`: não
    /// está neste mapa.
    pub(crate) async fn mascote_pelo_painel(&self, roleid: i32, slot: u16, tid: i32, e: &EdicaoDeMascote) -> Option<serde_json::Value> {
        if !self.world.read().await.players.contains_key(&(roleid as i64)) {
            return None;
        }
        if self.world.read().await.mascote_de(roleid as i64).is_some_and(|m| m.slot == slot) {
            return Some(serde_json::json!({"erro": "mascote_invocado"}));
        }
        match self.mascote_no_banco(roleid, slot, tid, e).await {
            Ok(Some(info)) => {
                let mut corpo = (slot as i32).to_le_bytes().to_vec();
                corpo.extend_from_slice(&info.para_bytes());
                self.enviar_ao_jogador(roleid, S2CGamedataSend::pet_room(1, &corpo).data).await;
            }
            Ok(None) => {
                self.enviar_ao_jogador(roleid, self.sub.free_pet(slot as i32, tid).data).await;
            }
            Err(codigo) => return Some(serde_json::json!({"erro": codigo})),
        }
        info!(roleid, slot, tid, "painel: mascote editado (online)");
        Some(serde_json::json!({"erro": null, "slot": slot, "tid": tid}))
    }

    /// Lê o slot, confere o modelo, aplica e grava; `Ok(None)` = libertado. Offline, quem chama
    /// segura a guarda de presença e a trava de gravação.
    pub(crate) async fn mascote_no_banco(&self, roleid: i32, slot: u16, tid: i32, e: &EdicaoDeMascote) -> Result<Option<InfoPet>, &'static str> {
        let repo = self.itens().await;
        let mut item: ItemRecord = match repo.get_item_by_slot(roleid, ContainerType::PetCorral, slot).await {
            Ok(Some(i)) => i,
            Ok(None) => return Err("slot_mudou"),
            Err(_) => return Err("banco_indisponivel"),
        };
        let mut info = InfoPet::do_bloco(&item.octets).ok_or("formato_invalido")?;
        if info.pet_tid != tid {
            return Err("slot_mudou");
        }
        if e.libertar {
            repo.delete_item_by_slot(roleid, ContainerType::PetCorral, slot).await.map_err(|_| "banco_indisponivel")?;
            return Ok(None);
        }
        let maximo = self.world.read().await.data_manager.modelos_de_mascote.get(&(tid as u32)).map(|m| m.nivel_maximo).unwrap_or(0);
        aplicar(&mut info, e, maximo)?;
        item.octets = info.para_bytes();
        repo.upsert_item(&item).await.map_err(|_| "banco_indisponivel")?;
        Ok(Some(info))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn aplicar_respeita_o_formato_e_o_nivel_do_modelo() {
        let mut info = InfoPet::default();
        info.pet_tid = 8000;
        let e = EdicaoDeMascote { nivel: Some(50), lealdade: Some(999), fome: Some(0), nome: Some("Garça".into()),
            habilidades: Some(vec![[100, 3], [101, 1]]), ..Default::default() };
        aplicar(&mut info, &e, 100).unwrap();
        assert_eq!((info.level, info.honor_point, info.hunger, nome_do_mascote(&info)), (50, 999, 0, "Garça".to_string()));
        assert_eq!((info.skills[0], info.skills[1], info.skills[2]), ((100, 3), (101, 1), (0, 0)));
        let copia = info.clone();
        assert_eq!(aplicar(&mut info, &EdicaoDeMascote { nivel: Some(101), ..Default::default() }, 100), Err("nivel_do_mascote_invalido"));
        assert_eq!(aplicar(&mut info, &EdicaoDeMascote { fome: Some(12), ..Default::default() }, 100), Err("valores_invalidos"));
        assert_eq!(aplicar(&mut info, &EdicaoDeMascote { lealdade: Some(1000), ..Default::default() }, 100), Err("valores_invalidos"));
        assert_eq!(aplicar(&mut info, &EdicaoDeMascote { nome: Some("123456789".into()), ..Default::default() }, 100), Err("nome_invalido"));
        assert_eq!(info, copia, "recusa não muda nada");
        assert_eq!(InfoPet::do_bloco(&copia.para_bytes()).unwrap(), copia, "o registro de 192 bytes se relê");
    }
}
