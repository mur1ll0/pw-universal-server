pub mod common;
pub mod v126;
pub mod v148;
pub mod v153;
pub mod v155;
pub mod v172;

use crate::traits::{ProtocolAdapter, WorldProtocol};
use crate::version::GameVersion;
use std::sync::Arc;

pub use v126::{V126Adapter, V126Protocol};
pub use v148::{V148Adapter, V148Protocol};
pub use v153::{V153Adapter, V153Protocol};
pub use v155::{V155Adapter, V155Protocol};
pub use v172::{V172Adapter, V172Protocol};

/// Cria a estratégia de protocolo de mundo para a versão especificada
pub fn create_world_protocol(versao: GameVersion) -> Arc<dyn WorldProtocol> {
    match versao {
        GameVersion::V1_2_6 => Arc::new(V126Protocol),
        GameVersion::V1_4_8 => Arc::new(V148Protocol(V155Protocol)),
        GameVersion::V1_5_3 => Arc::new(V153Protocol(V155Protocol)),
        GameVersion::V1_5_5 => Arc::new(V155Protocol),
        GameVersion::V1_7_2 => Arc::new(V172Protocol(V155Protocol)),
    }
}

/// Cria o adaptador de protocolo para o link/login para a versão especificada
pub fn get_protocol_adapter(versao: GameVersion) -> Arc<dyn ProtocolAdapter> {
    crate::adapter::create_protocol_adapter(versao)
}
