//! Núcleo do launcher (P2 do `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md`, B234): fala
//! com o `pw-recursos` (`/versoes`, `/atualizacao/<versao>/…`), confere as assinaturas Ed25519 e os
//! SHA-256, instala a partir dos pacotes `.tar.zst` (nomes em bytes GBK convertidos pela página de
//! código deste Windows), verifica, repara e atualiza arquivo a arquivo, e abre o jogo.
//! Spec: `specs/07_RECURSOS_DO_CLIENTE.md` §6.
//!
//! Também é o que o servidor usa para a prova (`pw-recursos conferir`): um caminho só.

pub mod assinatura;
pub mod cliente;
pub mod manifesto;
pub mod nomes;
pub mod tar_gnu;

pub use cliente::{Origem, Progresso, Servidor};
