//! Recursos do cliente (P1 do `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`, B233): a
//! versão 0 de cada versão de cliente importada dos `.pck` originais, os pacotes de instalação
//! (`.tar.zst` por pasta, nomes em bytes GBK), o manifesto assinado e o serviço HTTP que o
//! launcher usa. Spec: `specs/07_RECURSOS_DO_CLIENTE.md`.

pub mod admin;
pub mod assinatura;
pub mod fonte;
pub mod governanca;
pub mod importar;
pub mod objetos;
pub mod servir;

// Formatos e nomes são os mesmos do launcher: moram no pw-atualizador (B234).
pub use pw_atualizador::{manifesto, nomes, tar_gnu};

use pw_storage::RegistroDeRecurso;

/// Registros da importação no formato do repositório.
pub fn para_o_banco(registros: &[importar::Registro]) -> Vec<RegistroDeRecurso> {
    registros
        .iter()
        .map(|r| RegistroDeRecurso {
            caminho: r.caminho.clone(),
            origem: r.origem.como_texto(),
            pacote_original: r.pacote_original.clone(),
            pacote: r.pacote.clone(),
            tamanho: r.tamanho as i64,
            sha256: r.sha256.to_vec(),
            md5: r.md5.to_vec(),
        })
        .collect()
}
