//! Os atributos **base** de cada classe, do `ptemplate.conf` do `gamed`.
//!
//! # Origem
//!
//! `player_template::Data::Load` (`cgame/gs/playertemplate.cpp`), que lê o arquivo por
//! seção e preenche `_template_list[classe]`. As doze seções e a classe de cada uma são
//! as do próprio original:
//!
//! ```text
//! SWORDSMAN 0   MAGE 1    NEC 2     HAG 3
//! ORGE 4        ASN 5     ARCHER 6  ANGEL 7
//! BLADE 8       GENIE 9   SHADOW 10 FAIRY 11
//! ```
//!
//! # Por que é um arquivo `.conf`, e não o `elements.data`
//!
//! São duas fontes diferentes, e confundi-las custa tempo: o `elements.data` traz o
//! `CHARRACTER_CLASS_CONFIG` (ver [`crate::classes`]) com o que **escala** — ganho por
//! nível, por ponto de vitalidade, por ponto de agilidade. O `ptemplate.conf` traz o
//! **ponto de partida** de nível 1: vida, mana, os quatro atributos, velocidades.
//! Nenhum dos dois sozinho dá a vida máxima de um personagem.
//!
//! # Onde o arquivo fica
//!
//! No pacote do servidor original ele está em `gamed/ptemplate.conf`, fora da pasta de
//! `config` onde ficam os `.data`. Aqui ele é procurado na pasta de configuração do realm
//! (`data/<realm>/config/ptemplate.conf`), e **a ausência não é erro** — é o caso de
//! qualquer realm que ainda não teve o arquivo copiado. Quem consulta recebe `None` e
//! decide, em vez de receber número inventado.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

/// As doze seções, na ordem de `USER_CLASS_*` — a lista literal de
/// `player_template::__LoadData`.
pub const SECOES_DE_CLASSE: [&str; 12] = [
    "SWORDSMAN", "MAGE", "NEC", "HAG", "ORGE", "ASN", "ARCHER", "ANGEL", "BLADE", "GENIE",
    "SHADOW", "FAIRY",
];

#[derive(Error, Debug)]
pub enum PTemplateError {
    #[error("ptemplate.conf não tem a seção [{0}]")]
    SecaoAusente(String),
    #[error("ptemplate.conf: a seção [{secao}] não tem '{campo}'")]
    CampoAusente { secao: String, campo: String },
    #[error("ptemplate.conf: '{campo}' da seção [{secao}] não é número: {valor:?}")]
    ValorInvalido {
        secao: String,
        campo: String,
        valor: String,
    },
}

pub type Result<T> = std::result::Result<T, PTemplateError>;

/// O ponto de partida de uma classe, no nível 1 e sem nenhum ponto distribuído.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BaseDaClasse {
    /// `character_class_id` correspondente, 0 a 11.
    pub classe: i32,
    /// `hp` / `mp` → `_template_list[cls].max_hp` / `max_mp`.
    pub vida: i32,
    pub mana: i32,
    /// Os quatro atributos iniciais.
    pub vitalidade: i32,
    pub energia: i32,
    pub forca: i32,
    pub agilidade: i32,
    /// `attack_speed` já vem em *ticks* de 50 ms neste arquivo (30 = 1,5 s), diferente do
    /// `CHARRACTER_CLASS_CONFIG`, onde o mesmo campo está em segundos. O original lê este
    /// com `ReadInt` e o outro com conversão — é fácil trocar um pelo outro.
    pub ataque_em_ticks: i32,
    pub alcance_de_ataque: f32,
    pub regeneracao_de_vida: i32,
    pub regeneracao_de_mana: i32,
    pub velocidade_andando: f32,
    pub velocidade_correndo: f32,
    pub velocidade_nadando: f32,
    pub velocidade_voando: f32,
}

/// O arquivo inteiro.
#[derive(Debug, Clone, Default)]
pub struct TabelaDeBase {
    pub classes: HashMap<i32, BaseDaClasse>,
    /// `[GENERAL] logic_level_limit` — o teto de nível do realm.
    pub nivel_maximo: Option<i32>,
}

impl BaseDaClasse {
    /// Os quatro atributos com que um personagem desta classe nasce, na forma que o
    /// repositório de personagens grava.
    pub fn atributos_iniciais(&self) -> pw_core::AtributosIniciais {
        pw_core::AtributosIniciais {
            forca: self.forca,
            agilidade: self.agilidade,
            vitalidade: self.vitalidade,
            energia: self.energia,
        }
    }
}

impl TabelaDeBase {
    pub fn get(&self, classe: i32) -> Option<&BaseDaClasse> {
        self.classes.get(&classe)
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }
}

/// Um `.conf` no formato do `ONET::Conf`: seções entre colchetes, `chave = valor`, e
/// comentários com `#` ou `;`.
fn separar_em_secoes(texto: &str) -> HashMap<String, HashMap<String, String>> {
    let mut secoes: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut atual = String::new();

    for linha in texto.lines() {
        let linha = linha.trim();
        if linha.is_empty() || linha.starts_with('#') || linha.starts_with(';') {
            continue;
        }
        if let Some(nome) = linha.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            atual = nome.trim().to_string();
            secoes.entry(atual.clone()).or_default();
            continue;
        }
        let Some((chave, valor)) = linha.split_once('=') else {
            continue;
        };
        // O arquivo original é alinhado com tabulações, então os dois lados vêm sujos.
        secoes
            .entry(atual.clone())
            .or_default()
            .insert(chave.trim().to_string(), valor.trim().to_string());
    }

    secoes
}

fn inteiro(secoes: &HashMap<String, HashMap<String, String>>, secao: &str, campo: &str) -> Result<i32> {
    let valor = secoes
        .get(secao)
        .and_then(|s| s.get(campo))
        .ok_or_else(|| PTemplateError::CampoAusente {
            secao: secao.to_string(),
            campo: campo.to_string(),
        })?;
    valor.parse::<i32>().map_err(|_| PTemplateError::ValorInvalido {
        secao: secao.to_string(),
        campo: campo.to_string(),
        valor: valor.clone(),
    })
}

fn decimal(secoes: &HashMap<String, HashMap<String, String>>, secao: &str, campo: &str) -> Result<f32> {
    let valor = secoes
        .get(secao)
        .and_then(|s| s.get(campo))
        .ok_or_else(|| PTemplateError::CampoAusente {
            secao: secao.to_string(),
            campo: campo.to_string(),
        })?;
    valor.parse::<f32>().map_err(|_| PTemplateError::ValorInvalido {
        secao: secao.to_string(),
        campo: campo.to_string(),
        valor: valor.clone(),
    })
}

/// Lê um `ptemplate.conf`.
///
/// Falha quando uma seção de classe existe mas está incompleta — o original também
/// aborta (ele lança exceção em `ReadInt` quando o campo não existe). Seção de classe
/// **ausente** por completo é erro pelo mesmo motivo: um realm com onze classes
/// carregaria em silêncio e a décima segunda ficaria sem atributo nenhum.
pub fn ler(texto: &str) -> Result<TabelaDeBase> {
    let secoes = separar_em_secoes(texto);
    let mut tabela = TabelaDeBase::default();

    for (classe, secao) in SECOES_DE_CLASSE.iter().enumerate() {
        if !secoes.contains_key(*secao) {
            return Err(PTemplateError::SecaoAusente(secao.to_string()));
        }
        let classe = classe as i32;
        tabela.classes.insert(
            classe,
            BaseDaClasse {
                classe,
                vida: inteiro(&secoes, secao, "hp")?,
                mana: inteiro(&secoes, secao, "mp")?,
                vitalidade: inteiro(&secoes, secao, "vitality")?,
                energia: inteiro(&secoes, secao, "energy")?,
                forca: inteiro(&secoes, secao, "strength")?,
                agilidade: inteiro(&secoes, secao, "agility")?,
                ataque_em_ticks: inteiro(&secoes, secao, "attack_speed")?,
                alcance_de_ataque: decimal(&secoes, secao, "attack_range")?,
                regeneracao_de_vida: inteiro(&secoes, secao, "hp_gen")?,
                regeneracao_de_mana: inteiro(&secoes, secao, "mp_gen")?,
                velocidade_andando: decimal(&secoes, secao, "walk_speed")?,
                velocidade_correndo: decimal(&secoes, secao, "run_speed")?,
                velocidade_nadando: decimal(&secoes, secao, "swim_speed")?,
                velocidade_voando: decimal(&secoes, secao, "fly_speed")?,
            },
        );
    }

    tabela.nivel_maximo = inteiro(&secoes, "GENERAL", "logic_level_limit").ok();

    info!(
        "ptemplate.conf: {} classes base carregadas (teto de nível {:?})",
        tabela.classes.len(),
        tabela.nivel_maximo
    );
    Ok(tabela)
}

/// Lê o arquivo da pasta de configuração do realm, se ele estiver lá.
///
/// Devolve `None` quando o arquivo não existe — que é o estado de qualquer realm cujo
/// pacote não o trouxe. O aviso sai no log com a consequência prática, para não virar
/// uma ausência silenciosa que aparece três camadas depois como vida máxima errada.
pub fn ler_da_pasta(dir: &std::path::Path) -> Option<TabelaDeBase> {
    let caminho = dir.join("ptemplate.conf");
    if !caminho.exists() {
        warn!(
            "ptemplate.conf não está em {} — sem ele não há vida/mana máxima de \
             personagem, e quem entrar no mundo vai com o que estiver gravado no banco",
            dir.display()
        );
        return None;
    }
    // **Não** é UTF-8**: o arquivo do pacote original tem comentários em chinês
    // codificados em GBK, e um `read_to_string` falha com "stream did not contain valid
    // UTF-8" — foi o que aconteceu no primeiro teste em jogo. Os bytes altos ficam todos
    // em linhas de comentário (60 deles no arquivo do realm 155), que o leitor descarta,
    // então decodificar de forma tolerante é seguro: nenhuma chave ou valor é afetado.
    match std::fs::read(&caminho) {
        Ok(bytes) => match ler(&String::from_utf8_lossy(&bytes)) {
            Ok(t) => Some(t),
            Err(e) => {
                warn!("ptemplate.conf de {} ilegível: {e}", dir.display());
                None
            }
        },
        Err(e) => {
            warn!("ptemplate.conf de {} não pôde ser lido: {e}", dir.display());
            None
        }
    }
}
