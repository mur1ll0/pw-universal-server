//! Enquadramento GNET das mensagens de barramento.
//!
//! O quadro é o mesmo do protocolo com o cliente, porque **é** o mesmo protocolo:
//!
//! ```text
//! [CompactUINT(opcode)] [CompactUINT(tamanho)] [corpo]
//! ```
//!
//! Um `CompactUINT` ocupa de 1 a 5 bytes conforme a magnitude, então o cabeçalho não
//! tem tamanho fixo: só dá para saber onde o corpo começa depois de ler os dois. É por
//! isso que o decodificador devolve `None` em vez de erro quando o buffer ainda está
//! curto — TCP entrega em pedaços, e um quadro parcial é normal, não corrupção.

use crate::message::BusMessage;
use bytes::{Buf, BytesMut};
use pw_wire::gnet::{Reader, Writer};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Error, Debug)]
pub enum BusError {
    #[error("erro de fio: {0}")]
    Wire(#[from] pw_wire::WireError),

    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    /// Um quadro cujo tamanho anunciado é maior do que qualquer mensagem plausível.
    /// Recusar cedo evita que um byte trocado vire uma alocação enorme.
    #[error("quadro de {0} bytes excede o limite de {LIMITE_QUADRO}")]
    QuadroGrandeDemais(usize),
}

/// Teto para o tamanho de um quadro do barramento.
///
/// O maior payload plausível é uma lista de entidades de uma fatia do mundo, que fica
/// na casa das dezenas de KB. 1 MiB dá folga de sobra e ainda assim transforma um
/// cabeçalho corrompido em erro, e não em `Vec::with_capacity(4 GiB)`.
pub const LIMITE_QUADRO: usize = 1024 * 1024;

/// Codec de barramento para usar com `tokio_util::codec::Framed`.
#[derive(Debug, Default, Clone, Copy)]
pub struct BusCodec;

impl Decoder for BusCodec {
    type Item = BusMessage;
    type Error = BusError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        // Lê o cabeçalho sem consumir: se faltar byte, o quadro fica no buffer.
        let mut cabecalho = Reader::new(src);
        let (opcode, tamanho) = match (cabecalho.compact_uint(), cabecalho.compact_uint()) {
            (Ok(o), Ok(t)) => (o, t as usize),
            _ => return Ok(None),
        };
        let consumido = cabecalho.position();

        if tamanho > LIMITE_QUADRO {
            return Err(BusError::QuadroGrandeDemais(tamanho));
        }
        if src.len() < consumido + tamanho {
            return Ok(None);
        }

        src.advance(consumido);
        let corpo = src.split_to(tamanho);

        let mut r = Reader::new(&corpo);
        match BusMessage::read_payload(opcode, &mut r)? {
            Some(m) => Ok(Some(m)),
            None => {
                // Opcode que não é do barramento. O quadro já foi consumido por
                // inteiro, então a conexão segue alinhada; quem chama vê um `None` e
                // continua lendo.
                tracing::debug!("opcode {opcode} fora do barramento, quadro descartado");
                Ok(None)
            }
        }
    }
}

impl Encoder<BusMessage> for BusCodec {
    type Error = BusError;

    fn encode(&mut self, item: BusMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut corpo = Writer::new();
        item.write_payload(&mut corpo);
        let corpo = corpo.into_vec();

        let mut cabecalho = Writer::new();
        cabecalho.compact_uint(item.opcode());
        cabecalho.compact_uint(corpo.len() as u32);

        dst.extend_from_slice(cabecalho.as_slice());
        dst.extend_from_slice(&corpo);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg() -> BusMessage {
        BusMessage::ClientToGame {
            roleid: 1024,
            localsid: 0xDEAD_BEEF,
            data: vec![7, 8, 9],
        }
    }

    #[test]
    fn quadro_completo_faz_ida_e_volta() {
        let mut buf = BytesMut::new();
        BusCodec.encode(msg(), &mut buf).unwrap();
        let lido = BusCodec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(lido, msg());
        assert!(buf.is_empty(), "o quadro inteiro deveria ter sido consumido");
    }

    #[test]
    fn quadro_partido_espera_o_resto_em_vez_de_falhar() {
        // TCP entrega em pedaços; um quadro pela metade é normal.
        let mut completo = BytesMut::new();
        BusCodec.encode(msg(), &mut completo).unwrap();

        for corte in 1..completo.len() {
            let mut parcial = BytesMut::from(&completo[..corte]);
            assert!(
                BusCodec.decode(&mut parcial).unwrap().is_none(),
                "com {corte} de {} bytes deveria pedir mais",
                completo.len()
            );
        }
    }

    #[test]
    fn dois_quadros_no_mesmo_buffer_saem_em_ordem() {
        let mut buf = BytesMut::new();
        BusCodec.encode(msg(), &mut buf).unwrap();
        BusCodec
            .encode(
                BusMessage::PlayerLogout {
                    result: 0,
                    roleid: 1024,
                    provider_link_id: 1,
                    localsid: 2,
                },
                &mut buf,
            )
            .unwrap();

        assert_eq!(BusCodec.decode(&mut buf).unwrap().unwrap(), msg());
        assert!(matches!(
            BusCodec.decode(&mut buf).unwrap().unwrap(),
            BusMessage::PlayerLogout { roleid: 1024, .. }
        ));
        assert!(buf.is_empty());
    }

    #[test]
    fn tamanho_absurdo_e_recusado_antes_de_alocar() {
        let mut cabecalho = Writer::new();
        cabecalho.compact_uint(crate::message::opcode::C2S_GAMEDATA_SEND);
        cabecalho.compact_uint(0x0FFF_FFFF);
        let mut buf = BytesMut::from(cabecalho.as_slice());
        assert!(matches!(
            BusCodec.decode(&mut buf),
            Err(BusError::QuadroGrandeDemais(_))
        ));
    }

    #[test]
    fn payload_vazio_atravessa() {
        let vazia = BusMessage::GameToClient {
            roleid: 1,
            localsid: 2,
            data: Vec::new(),
        };
        let mut buf = BytesMut::new();
        BusCodec.encode(vazia.clone(), &mut buf).unwrap();
        assert_eq!(BusCodec.decode(&mut buf).unwrap().unwrap(), vazia);
    }
}
