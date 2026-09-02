//! Erros comuns aos dois formatos de fio.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// Faltaram bytes no buffer. Guarda o que era preciso e o que havia, porque num
    /// protocolo binário a diferença entre "faltou 1" e "faltou 400" separa um pacote
    /// truncado de um campo lido no lugar errado.
    #[error("buffer insuficiente: precisava de {needed} byte(s) em {offset}, restavam {available}")]
    Underflow {
        offset: usize,
        needed: usize,
        available: usize,
    },

    /// Prefixo de `CompactUINT` que não corresponde a nenhuma das quatro formas.
    #[error("prefixo de CompactUINT inválido: 0x{0:02X}")]
    InvalidCompactUint(u8),

    /// Um `CompactUINT` de tamanho que não cabe em `usize` no alvo atual, ou que
    /// anuncia mais bytes do que o buffer inteiro tem. Recusar aqui evita transformar
    /// um pacote corrompido num pedido de alocação enorme.
    #[error("tamanho anunciado ({announced}) maior que o buffer restante ({available})")]
    LengthTooLarge { announced: u64, available: usize },

    /// Posicionamento fora do buffer, no modelo endereçado por deslocamento.
    #[error("deslocamento {offset} fora do buffer de {len} byte(s)")]
    OutOfBounds { offset: usize, len: usize },
}

pub type WireResult<T> = Result<T, WireError>;
