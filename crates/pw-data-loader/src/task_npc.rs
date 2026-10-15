//! `data\task_npc.data` do **cliente** — a posição de cada NPC para o rastreador de missões (B246).
//!
//! O cliente o lê do disco (`fopen`, não do `.pck`) em `TaskInterface::Init`
//! (`EC_TaskInterface.cpp:166` → `ATaskTemplMan::LoadNPCInfoFromPack`, `TaskTemplMan.cpp:2149-2185`) e o usa
//! em `GetTaskNPCInfo(id)` para os nomes clicáveis das missões (`DlgNameLink.cpp:243`, `EC_UIHelper.cpp:505`).
//! O `gs` original tem o mesmo leitor mas **nunca o chama**: a cópia no `config` do realm não é usada.
//!
//! # Formato (`TaskTemplMan.cpp:43-60`, `UnmarshalNPCInfo` em `:2067-2103`; `TaskTempl.h:71-77`)
//!
//! ```text
//! TASK_NPC_PACK_HEADER { u32 pack_size; i32 time_mark; u16 version (= 1); u16 npc_count }   // 12 bytes
//! npc_count × NPC_INFO { u32 id; i16 x; i16 y; i16 z; 2 bytes de alinhamento }             // 12 bytes
//! ```
//!
//! `NPC_INFO` fica **antes** do `#pragma pack(1)` do `TaskTempl.h` (linha 221): alinhamento padrão, 12 bytes.
//! Conferido nos dois clientes: 1.5.5 BR 1.572 NPCs em 18.876 bytes, 1.2.6 765 em 9.192. O cliente recusa
//! versão diferente de 1 e `pack_size` diferente do tamanho do arquivo.
//!
//! **Ordem das coordenadas:** o cliente monta o ponto com `Set(pInfo->x, pInfo->z, pInfo->y)` — "NOTICE: different
//! sequence" (`DlgNameLink.cpp:246-247`, igual em `EC_UIHelper.cpp:508`): no arquivo vêm **x, z, altura**. Nos dois
//! arquivos reais a altura é sempre 0, e (0, 0, 0) é NPC sem posição (300 de 765 no 1.2.6, 1.057 de 1.572 no 1.5.5).
//! E o cliente só consulta o rastreador no mapa principal (`MAJOR_MAP` = 1, `DlgWorldMap.h:13`). Os 2 bytes de
//! alinhamento vêm com lixo de memória; são preservados.

use thiserror::Error;

pub const VERSAO: u16 = 1;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroDoRastreador {
    #[error("task_npc.data: {0}")]
    Formato(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcNoRastreador {
    /// Id do modelo do NPC (`NPC_ESSENCE`).
    pub id: u32,
    /// Como está no arquivo: `x`, `z`, altura (ver a ordem no topo).
    pub bruto: [i16; 3],
    /// Os 2 bytes de alinhamento, como vieram (para a ida e volta ser idêntica).
    pub alinhamento: [u8; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rastreador {
    pub time_mark: i32,
    pub npcs: Vec<NpcNoRastreador>,
}

impl Rastreador {
    pub fn ler(b: &[u8]) -> Result<Self, ErroDoRastreador> {
        let erro = |m: String| ErroDoRastreador::Formato(m);
        if b.len() < 12 {
            return Err(erro("menor que o cabeçalho".into()));
        }
        let u32_em = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let u16_em = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap());
        let (tamanho, versao, n) = (u32_em(0) as usize, u16_em(8), u16_em(10) as usize);
        if versao != VERSAO {
            return Err(erro(format!("versão {versao}, o cliente só aceita {VERSAO}")));
        }
        if tamanho != b.len() || b.len() != 12 + n * 12 {
            return Err(erro(format!("{} bytes, cabeçalho diz {tamanho} para {n} NPCs", b.len())));
        }
        let npcs = (0..n)
            .map(|k| {
                let o = 12 + k * 12;
                let i16_em = |d: usize| i16::from_le_bytes(b[o + d..o + d + 2].try_into().unwrap());
                NpcNoRastreador { id: u32_em(o), bruto: [i16_em(4), i16_em(6), i16_em(8)], alinhamento: [b[o + 10], b[o + 11]] }
            })
            .collect();
        Ok(Self { time_mark: u32_em(4) as i32, npcs })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let tamanho = 12 + self.npcs.len() * 12;
        let mut b = Vec::with_capacity(tamanho);
        b.extend_from_slice(&(tamanho as u32).to_le_bytes());
        b.extend_from_slice(&self.time_mark.to_le_bytes());
        b.extend_from_slice(&VERSAO.to_le_bytes());
        b.extend_from_slice(&(self.npcs.len() as u16).to_le_bytes());
        for n in &self.npcs {
            b.extend_from_slice(&n.id.to_le_bytes());
            for c in n.bruto {
                b.extend_from_slice(&c.to_le_bytes());
            }
            b.extend_from_slice(&n.alinhamento);
        }
        b
    }

    /// Posição do NPC no rastreador, na ordem do mundo `[x, altura, z]`; `None` se não está no arquivo ou está
    /// em (0, 0, 0). O cliente guarda num mapa pelo id: com dois, vale o último (`TaskTemplMan.cpp:2097-2100`).
    pub fn posicao(&self, id: u32) -> Option<[i16; 3]> {
        let n = self.npcs.iter().rev().find(|n| n.id == id)?;
        (n.bruto != [0, 0, 0]).then_some([n.bruto[0], n.bruto[2], n.bruto[1]])
    }

    /// Põe o NPC `id` (todas as entradas dele) em `pos` (ordem do mundo `[x, altura, z]`), ou o acrescenta no fim.
    /// Coordenadas fora de `short` são recusadas. Devolve se já existia.
    pub fn definir_posicao(&mut self, id: u32, pos: [f32; 3]) -> Result<bool, ErroDoRastreador> {
        let mut p = [0i16; 3];
        for (k, c) in pos.iter().enumerate() {
            let r = c.round();
            if !(i16::MIN as f32..=i16::MAX as f32).contains(&r) {
                return Err(ErroDoRastreador::Formato(format!("coordenada {c} não cabe em short")));
            }
            p[k] = r as i16;
        }
        let bruto = [p[0], p[2], p[1]];
        let mut achou = false;
        for n in self.npcs.iter_mut().filter(|n| n.id == id) {
            n.bruto = bruto;
            achou = true;
        }
        if !achou {
            if self.npcs.len() >= u16::MAX as usize {
                return Err(ErroDoRastreador::Formato("NPCs demais".into()));
            }
            self.npcs.push(NpcNoRastreador { id, bruto, alinhamento: [0, 0] });
        }
        Ok(achou)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn os_dois_realms_voltam_identicos() {
        for (realm, n) in [("realm_126", 765), ("realm_155", 1572)] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/task_npc.data"));
            let b = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let r = Rastreador::ler(&b).unwrap();
            assert_eq!(r.npcs.len(), n, "{realm}");
            assert!(r.gravar() == b, "{realm} não volta idêntico");
            let mut m = r.clone();
            let id = r.npcs[0].id;
            assert!(m.definir_posicao(id, [10.4, -2.6, 300.0]).unwrap());
            let relido = Rastreador::ler(&m.gravar()).unwrap();
            assert_eq!(relido.posicao(id), Some([10, -3, 300]));
            // No arquivo: x, z, altura (`DlgNameLink.cpp:246-247`).
            assert_eq!(relido.npcs.iter().rev().find(|n| n.id == id).unwrap().bruto, [10, 300, -3]);
            // Entrada real com posição: altura 0 no 3º campo.
            let com = r.npcs.iter().find(|n| n.bruto != [0, 0, 0]).unwrap();
            assert_eq!(com.bruto[2], 0, "{realm}: a altura vem no 3º campo e é 0");
            assert!(!m.definir_posicao(999_999, [1.0, 2.0, 3.0]).unwrap());
            assert_eq!(Rastreador::ler(&m.gravar()).unwrap().npcs.len(), n + 1);
            assert!(m.definir_posicao(id, [40000.0, 0.0, 0.0]).is_err());
        }
        let mut b = std::fs::read(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config/task_npc.data")).unwrap();
        b.push(0);
        assert!(Rastreador::ler(&b).is_err(), "byte a mais");
    }
}
