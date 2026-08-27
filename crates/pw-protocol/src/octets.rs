use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::{Cursor, Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OctetsError {
    #[error("Buffer insuficiente para leitura (faltam bytes)")]
    BufferUnderflow,

    #[error("Erro de I/O na serialização: {0}")]
    Io(#[from] std::io::Error),

    #[error("String UTF-16 / UTF-8 inválida")]
    InvalidString,

    #[error("Tamanho compacto de inteiro inválido (prefixo desconhecido)")]
    InvalidCompactUint,
}

pub type Result<T> = std::result::Result<T, OctetsError>;

/// Leitor e Escritor de Streams Binários do Perfect World (OctetsStream)
/// Suporta o formato oficial de compactação de inteiros do CNet (Wanmei Engine)
#[derive(Debug, Clone, Default)]
pub struct OctetsStream {
    buffer: BytesMut,
}

impl OctetsStream {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            buffer: BytesMut::from(bytes),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    pub fn into_bytes(self) -> Bytes {
        self.buffer.freeze()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    // =========================================================================
    // ESCRITA DE DADOS (Serialization)
    // =========================================================================

    pub fn write_u8(&mut self, val: u8) {
        self.buffer.put_u8(val);
    }

    pub fn write_i8(&mut self, val: i8) {
        self.buffer.put_i8(val);
    }

    pub fn write_u16_le(&mut self, val: u16) {
        self.buffer.put_u16_le(val);
    }

    pub fn write_i16_le(&mut self, val: i16) {
        self.buffer.put_i16_le(val);
    }

    pub fn write_u32_le(&mut self, val: u32) {
        self.buffer.put_u32_le(val);
    }

    pub fn write_i32_le(&mut self, val: i32) {
        self.buffer.put_i32_le(val);
    }

    pub fn write_u64_le(&mut self, val: u64) {
        self.buffer.put_u64_le(val);
    }

    pub fn write_i64_le(&mut self, val: i64) {
        self.buffer.put_i64_le(val);
    }

    pub fn write_f32_le(&mut self, val: f32) {
        self.buffer.put_f32_le(val);
    }

    pub fn write_f64_le(&mut self, val: f64) {
        self.buffer.put_f64_le(val);
    }

    /// Escreve um inteiro compacto no formato oficial CUint32 do PW
    pub fn write_compact_uint(&mut self, val: u32) {
        if val < 0x80 {
            self.buffer.put_u8(val as u8);
        } else if val < 0x4000 {
            let encoded = ((val | 0x8000) as u16).to_be_bytes();
            self.buffer.put_slice(&encoded);
        } else if val < 0x20000000 {
            let encoded = (val | 0xC0000000).to_be_bytes();
            self.buffer.put_slice(&encoded);
        } else {
            self.buffer.put_u8(0xE0);
            self.buffer.put_slice(&val.to_be_bytes());
        }
    }

    /// Escreve um bloco de bytes prefixado com seu tamanho compacto (Octets)
    pub fn write_octets(&mut self, data: &[u8]) {
        self.write_compact_uint(data.len() as u32);
        self.buffer.put_slice(data);
    }

    /// Escreve uma String UTF-16LE com prefixo de tamanho em bytes (formato do cliente do PW)
    pub fn write_string_utf16le(&mut self, text: &str) {
        let utf16_bytes: Vec<u8> = text
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        self.write_octets(&utf16_bytes);
    }

    /// Escreve uma String UTF-8 padrão
    pub fn write_string_utf8(&mut self, text: &str) {
        self.write_octets(text.as_bytes());
    }

    // =========================================================================
    // LEITURA DE DADOS (Deserialization)
    // =========================================================================

    pub fn read_u8(&mut self) -> Result<u8> {
        if self.buffer.remaining() < 1 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_u8())
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        if self.buffer.remaining() < 1 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_i8())
    }

    pub fn read_u16_le(&mut self) -> Result<u16> {
        if self.buffer.remaining() < 2 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_u16_le())
    }

    pub fn read_i16_le(&mut self) -> Result<i16> {
        if self.buffer.remaining() < 2 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_i16_le())
    }

    pub fn read_u32_le(&mut self) -> Result<u32> {
        if self.buffer.remaining() < 4 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_u32_le())
    }

    pub fn read_i32_le(&mut self) -> Result<i32> {
        if self.buffer.remaining() < 4 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_i32_le())
    }

    pub fn read_u64_le(&mut self) -> Result<u64> {
        if self.buffer.remaining() < 8 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_u64_le())
    }

    pub fn read_i64_le(&mut self) -> Result<i64> {
        if self.buffer.remaining() < 8 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_i64_le())
    }

    pub fn read_f32_le(&mut self) -> Result<f32> {
        if self.buffer.remaining() < 4 {
            return Err(OctetsError::BufferUnderflow);
        }
        Ok(self.buffer.get_f32_le())
    }

    /// Lê um inteiro compacto CUint32
    pub fn read_compact_uint(&mut self) -> Result<u32> {
        if self.buffer.remaining() < 1 {
            return Err(OctetsError::BufferUnderflow);
        }

        let first = self.buffer.get_u8();
        match first & 0xE0 {
            0xE0 => {
                if self.buffer.remaining() < 4 {
                    return Err(OctetsError::BufferUnderflow);
                }
                Ok(self.buffer.get_u32()) // Big-Endian
            }
            0xC0 => {
                if self.buffer.remaining() < 3 {
                    return Err(OctetsError::BufferUnderflow);
                }
                let b1 = first & 0x3F;
                let b2 = self.buffer.get_u8();
                let b3 = self.buffer.get_u8();
                let b4 = self.buffer.get_u8();
                Ok(((b1 as u32) << 24) | ((b2 as u32) << 16) | ((b3 as u32) << 8) | (b4 as u32))
            }
            0x80 | 0xA0 => {
                if self.buffer.remaining() < 1 {
                    return Err(OctetsError::BufferUnderflow);
                }
                let b1 = first & 0x7F;
                let b2 = self.buffer.get_u8();
                Ok(((b1 as u32) << 8) | (b2 as u32))
            }
            _ => Ok(first as u32),
        }
    }

    /// Lê um bloco de bytes prefixado com tamanho compacto (Octets)
    pub fn read_octets(&mut self) -> Result<Vec<u8>> {
        let len = self.read_compact_uint()? as usize;
        if self.buffer.remaining() < len {
            return Err(OctetsError::BufferUnderflow);
        }
        let mut data = vec![0u8; len];
        self.buffer.copy_to_slice(&mut data);
        Ok(data)
    }

    /// Lê uma String UTF-16LE com prefixo de tamanho em bytes
    pub fn read_string_utf16le(&mut self) -> Result<String> {
        let bytes = self.read_octets()?;
        if bytes.len() % 2 != 0 {
            return Err(OctetsError::InvalidString);
        }
        let u16_vec: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16(&u16_vec).map_err(|_| OctetsError::InvalidString)
    }

    /// Lê uma String UTF-8 com prefixo de tamanho
    pub fn read_string_utf8(&mut self) -> Result<String> {
        let bytes = self.read_octets()?;
        String::from_utf8(bytes).map_err(|_| OctetsError::InvalidString)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_uint_roundtrip() {
        let test_values = [0u32, 1, 127, 128, 500, 16383, 16384, 100000, 500000000];

        for &val in &test_values {
            let mut stream = OctetsStream::new();
            stream.write_compact_uint(val);

            let mut reader = OctetsStream::from_bytes(stream.as_slice());
            let read_val = reader.read_compact_uint().expect("Falha ao ler compact uint");
            assert_eq!(val, read_val, "Valor compacto não bate para {}", val);
        }
    }

    #[test]
    fn test_string_utf16_roundtrip() {
        let text = "MeuPersonagem123";
        let mut stream = OctetsStream::new();
        stream.write_string_utf16le(text);

        let mut reader = OctetsStream::from_bytes(stream.as_slice());
        let read_text = reader.read_string_utf16le().expect("Falha ao ler string UTF-16");
        assert_eq!(text, read_text);
    }
}
