//! O barramento entre daemons: o que `glinkd` e `gamed` dizem um ao outro.
//!
//! # Por que este crate existe
//!
//! Hoje o `gateway.rs` do `pw-link` tem 1.379 linhas, e ~650 delas são um único braço
//! de `match` tratando o `GamedataSend` — ou seja, **a simulação do mundo 3D roda dentro
//! do daemon de link**. O `pw-gs` tem um tick loop e nenhuma rede: não está no caminho
//! do jogo. É essa a causa de "o cliente entra mas nada funciona".
//!
//! Este crate é a costura que permite separar os dois, e ele não inventa nada: as
//! mensagens são protocolos GNET reais, catalogados em `specs/protocol/gnet_153.json`,
//! e o campo `daemons` de cada um no IR diz quem fala com quem.
//!
//! # Os dois formatos, de novo
//!
//! O envelope do barramento é **GNET** — big-endian, `CompactUINT`. O `data` que ele
//! carrega são os subcomandos do mundo 3D, em **outro** formato — little-endian,
//! `pack(1)`. O barramento nunca olha dentro do `data`; quem interpreta é o `pw-gs`,
//! com o `pw_wire::gamedata`. Manter essa fronteira nítida é metade do motivo de o
//! crate existir.

pub mod codec;
pub mod message;
pub mod transport;

pub use codec::{BusCodec, BusError, LIMITE_QUADRO};
pub use message::{opcode, BusMessage};
pub use transport::{BusClient, BusListener};
