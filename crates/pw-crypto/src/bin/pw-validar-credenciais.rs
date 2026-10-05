//! Ponte local para o verificador de pw-auth. Segredos só por stdin; sem sessão de jogo.
use serde::Deserialize;
use std::io::{self, Read};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credenciais {
    usuario: String,
    senha: String,
    hash: String,
}

fn main() {
    let mut entrada = String::new();
    if io::stdin().take(8193).read_to_string(&mut entrada).is_err() || entrada.len() > 8192 {
        std::process::exit(2);
    }
    let Ok(credenciais) = serde_json::from_str::<Credenciais>(&entrada) else {
        std::process::exit(2);
    };
    // Compatibilidade: pw-crypto/src/password.rs:57-112 e pw-auth/src/service.rs:98.
    let resultado =
        pw_crypto::verify_password(&credenciais.usuario, &credenciais.senha, &credenciais.hash);
    println!("{}", resultado.is_valid);
}
