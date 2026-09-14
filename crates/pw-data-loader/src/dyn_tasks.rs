//! O cabeçalho do `dyn_tasks.data` — o pacote de missões dinâmicas.
//!
//! # Para que serve
//!
//! Ao entrar no mundo, o cliente pergunta ao servidor a **marca de tempo** do pacote de
//! missões dinâmicas (`TASK_CLT_NOTIFY_DYN_TIMEMARK`, `TaskProcess.cpp:2185`). Se a marca
//! que o servidor devolve for igual à do `dyn_tasks.data` que o próprio cliente tem, ele
//! carrega as missões do arquivo local e se dá por verificado; se não, pede o pacote
//! inteiro (`ATaskTemplMan::OnDynTasksTimeMark`, `TaskTemplMan.cpp:166-178`).
//!
//! O servidor original responde com a marca **do seu** pacote, e não responde nada quando
//! não tem pacote (`m_ulDynTasksTimeMark == 0`, `TaskTemplMan.cpp:299-309`). Os `.data` do
//! realm vêm do cliente (spec 03 §4), então a marca deste arquivo é a que o cliente espera.
//!
//! # Formato
//!
//! `DYN_TASK_PACK_HEADER` (`EvolvedPWServer/cgame/gs/task/TaskTemplMan.cpp:45-51`), 12 bytes
//! no alvo de 32 bits:
//!
//! | campo | tipo | |
//! | :--- | :--- | :--- |
//! | `pack_size` | `unsigned long` | o tamanho do arquivo inteiro |
//! | `time_mark` | `long` | a marca |
//! | `version` | `unsigned short` | `DYN_TASK_CUR_VERSION` = **10** |
//! | `task_count` | `unsigned short` | |
//!
//! O original recusa o pacote se `version != 10` ou se `pack_size` não for o tamanho lido
//! (`UnmarshalDynTasks`, `TaskTemplMan.cpp:1915-1937`). Este leitor faz as mesmas duas
//! recusas. Só o cabeçalho é lido: as missões em si não são usadas pelo servidor ainda.

use thiserror::Error;

/// `DYN_TASK_CUR_VERSION` (`TaskTemplMan.cpp:39`).
pub const VERSAO_DO_PACOTE: u16 = 10;

/// `sizeof(DYN_TASK_PACK_HEADER)` no alvo original de 32 bits.
pub const BYTES_DO_CABECALHO: usize = 12;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DynTasksError {
    #[error("dyn_tasks.data com {0} bytes não tem nem o cabeçalho ({BYTES_DO_CABECALHO})")]
    Curto(usize),
    #[error("dyn_tasks.data versão {0}, esperada {VERSAO_DO_PACOTE}")]
    Versao(u16),
    #[error("dyn_tasks.data diz ter {declarado} bytes e tem {real}")]
    Tamanho { declarado: u32, real: usize },
}

/// O que o servidor precisa do pacote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CabecalhoDasMissoesDinamicas {
    /// `time_mark`, como os quatro bytes estão no arquivo. O cliente compara como
    /// `unsigned long` (`OnDynTasksTimeMark`), então o sinal não importa.
    pub marca: u32,
    pub versao: u16,
    pub missoes: u16,
}

impl CabecalhoDasMissoesDinamicas {
    pub fn ler(dados: &[u8]) -> Result<Self, DynTasksError> {
        if dados.len() < BYTES_DO_CABECALHO {
            return Err(DynTasksError::Curto(dados.len()));
        }
        let u32_em = |i: usize| u32::from_le_bytes(dados[i..i + 4].try_into().unwrap());
        let u16_em = |i: usize| u16::from_le_bytes(dados[i..i + 2].try_into().unwrap());

        let versao = u16_em(8);
        if versao != VERSAO_DO_PACOTE {
            return Err(DynTasksError::Versao(versao));
        }
        let declarado = u32_em(0);
        if declarado as usize != dados.len() {
            return Err(DynTasksError::Tamanho { declarado, real: dados.len() });
        }
        Ok(Self { marca: u32_em(4), versao, missoes: u16_em(10) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pacote(tamanho: u32, versao: u16, extra: usize) -> Vec<u8> {
        let mut v = tamanho.to_le_bytes().to_vec();
        v.extend_from_slice(&0x5277_6c0du32.to_le_bytes());
        v.extend_from_slice(&versao.to_le_bytes());
        v.extend_from_slice(&3u16.to_le_bytes());
        v.resize(BYTES_DO_CABECALHO + extra, 0);
        v
    }

    #[test]
    fn le_os_quatro_campos_na_ordem_do_original() {
        let c = CabecalhoDasMissoesDinamicas::ler(&pacote(16, 10, 4)).unwrap();
        assert_eq!(c, CabecalhoDasMissoesDinamicas { marca: 0x5277_6c0d, versao: 10, missoes: 3 });
    }

    #[test]
    fn recusa_como_o_unmarshal_do_original() {
        assert_eq!(CabecalhoDasMissoesDinamicas::ler(&[0; 11]), Err(DynTasksError::Curto(11)));
        assert_eq!(
            CabecalhoDasMissoesDinamicas::ler(&pacote(12, 9, 0)),
            Err(DynTasksError::Versao(9))
        );
        assert_eq!(
            CabecalhoDasMissoesDinamicas::ler(&pacote(12, 10, 1)),
            Err(DynTasksError::Tamanho { declarado: 12, real: 13 })
        );
    }
}
