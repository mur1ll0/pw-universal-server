/// Implementação de alta performance da Cifra de Fluxo RC4 (ARC4)
/// Utilizada na criptografia de pacotes TCP do protocolo de rede do Perfect World.
#[derive(Clone)]
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Inicializa o estado interno do RC4 com a chave fornecida (KSA - Key-Scheduling Algorithm)
    pub fn new(key: &[u8]) -> Self {
        assert!(!key.is_empty(), "A chave RC4 não pode ser vazia");
        let mut s = [0u8; 256];
        for (i, val) in s.iter_mut().enumerate() {
            *val = i as u8;
        }

        let mut j: u8 = 0;
        for i in 0..256 {
            let key_byte = key[i % key.len()];
            j = j.wrapping_add(s[i]).wrapping_add(key_byte);
            s.swap(i, j as usize);
        }

        Self { s, i: 0, j: 0 }
    }

    /// Aplica o fluxo de chave (Keystream) diretamente no buffer em memória (in-place)
    /// Como o RC4 é simétrico, a mesma função é usada para encriptar e decriptar.
    pub fn apply_keystream(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);

            let k = self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            *byte ^= k;
        }
    }

    /// Cria uma cópia com criptografia/decriptografia de um slice de bytes
    pub fn process_vec(&mut self, input: &[u8]) -> Vec<u8> {
        let mut output = input.to_vec();
        self.apply_keystream(&mut output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4_symmetry() {
        let key = b"PerfectWorldSecretKey123";
        let original = b"Hello, Perfect World Universal Server!";

        let mut cipher_enc = Rc4::new(key);
        let mut cipher_dec = Rc4::new(key);

        let mut buffer = original.to_vec();
        cipher_enc.apply_keystream(&mut buffer);

        // O buffer encriptado deve ser diferente do original
        assert_ne!(&buffer, original);

        // Decriptando com a mesma chave deve restaurar os dados exatos
        cipher_dec.apply_keystream(&mut buffer);
        assert_eq!(&buffer, original);
    }
}
