//! Tabelas de texto do cliente (B260, Estúdio S7): `Configs\fixed_msg.txt`, `item_desc.txt`, `item_ext_desc.txt`.
//!
//! O cliente as carrega em `CECGame::Init` (`EC_Game.cpp:517-525`) por `CECStringTab::LoadWideStrings`
//! (`EC_StringTab.cpp`): `AWScriptFile` (UTF-16LE com BOM); tudo até `#_begin` é cabeçalho (`#_index` antes dele = a
//! tabela é indexada); depois, uma entrada por ficha — indexada: `id "texto"` (`GetNextTokenAsInt` + `GetNextToken`; id
//! repetido não entra, `m_WStrTab.put` recusa e vale o primeiro); sequencial: `"texto"`, o índice é a ordem.
//!
//! A edição é **por linha**: o arquivo fica como está e só a linha da entrada mudada é reescrita (o prefixo até a
//! primeira aspa é mantido). Medido nos três arquivos do 1.5.5 BR: LF, uma entrada por linha. Texto com aspas ou
//! quebra de linha é recusado (o leitor do cliente não tem escape).

use anyhow::{bail, Context, Result};

pub const ARQUIVOS: [&str; 3] = ["fixed_msg.txt", "item_desc.txt", "item_ext_desc.txt"];

#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    /// Linha do arquivo.
    pub linha: usize,
    /// O id (tabela indexada) ou a posição (sequencial).
    pub chave: i64,
    pub texto: String,
}

#[derive(Debug, Clone)]
pub struct TabelaDeTextos {
    linhas: Vec<String>,
    pub indexada: bool,
    pub entradas: Vec<Entrada>,
}

fn texto_entre_aspas(l: &str) -> Option<(usize, String)> {
    let i = l.find('"')?;
    let fim = l[i + 1..].find('"')? + i + 1;
    Some((i, l[i + 1..fim].to_string()))
}

impl TabelaDeTextos {
    pub fn ler(b: &[u8]) -> Result<Self> {
        let corpo = b.strip_prefix(&[0xFF, 0xFE]).context("tabela de texto sem BOM UTF-16LE")?;
        if corpo.len() % 2 != 0 {
            bail!("UTF-16 com byte sobrando");
        }
        let u: Vec<u16> = corpo.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let t = String::from_utf16(&u).context("UTF-16 inválido")?;
        let linhas: Vec<String> = t.split('\n').map(str::to_string).collect();
        let (mut indexada, mut comeco) = (false, None);
        for (i, l) in linhas.iter().enumerate() {
            let l = l.trim();
            if l.eq_ignore_ascii_case("#_index") {
                indexada = true;
            } else if l.eq_ignore_ascii_case("#_begin") {
                comeco = Some(i + 1);
                break;
            }
        }
        let comeco = comeco.context("sem #_begin")?;
        let mut entradas = Vec::new();
        for (i, l) in linhas.iter().enumerate().skip(comeco) {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") {
                continue;
            }
            let (_, texto) = texto_entre_aspas(s).with_context(|| format!("linha {}: sem texto entre aspas", i + 1))?;
            let chave = if indexada {
                s.split_whitespace().next().and_then(|x| x.parse().ok()).with_context(|| format!("linha {}: sem id", i + 1))?
            } else {
                entradas.len() as i64
            };
            entradas.push(Entrada { linha: i, chave, texto });
        }
        Ok(Self { linhas, indexada, entradas })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let t = self.linhas.join("\n");
        let mut b = vec![0xFF, 0xFE];
        for c in t.encode_utf16() {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b
    }

    /// A entrada que o cliente usa para a chave (no indexado, a primeira com o id).
    pub fn entrada(&self, chave: i64) -> Option<&Entrada> {
        self.entradas.iter().find(|e| e.chave == chave)
    }

    fn validar(texto: &str) -> Result<()> {
        if texto.contains(['"', '\n', '\r']) {
            bail!("o texto não pode ter aspas nem quebra de linha (use \\r, como o próprio arquivo)");
        }
        if texto.chars().count() > 4000 {
            bail!("texto grande demais");
        }
        Ok(())
    }

    /// Troca o texto da entrada; o resto da linha (id, espaços, o que vem depois das aspas) fica.
    pub fn definir(&mut self, chave: i64, texto: &str) -> Result<()> {
        Self::validar(texto)?;
        let i = self.entradas.iter().position(|e| e.chave == chave).with_context(|| format!("entrada {chave} não existe"))?;
        let n = self.entradas[i].linha;
        let l = &self.linhas[n];
        let a = l.find('"').context("linha sem aspas")?;
        let b = l[a + 1..].find('"').context("linha sem aspas")? + a + 1;
        self.linhas[n] = format!("{}\"{}\"{}", &l[..a], texto, &l[b + 1..]);
        self.entradas[i].texto = texto.to_string();
        Ok(())
    }

    /// Entrada nova no fim (só na tabela indexada, com um id que ainda não existe), no formato `id  "texto"`.
    pub fn acrescentar(&mut self, id: i64, texto: &str) -> Result<()> {
        if !self.indexada {
            bail!("só a tabela indexada recebe entrada nova (na sequencial o índice é a ordem)");
        }
        Self::validar(texto)?;
        if self.entrada(id).is_some() {
            bail!("o id {id} já existe");
        }
        // Antes das linhas vazias do fim, para o arquivo continuar terminando como terminava.
        let mut pos = self.linhas.len();
        while pos > 0 && self.linhas[pos - 1].trim().is_empty() {
            pos -= 1;
        }
        self.linhas.insert(pos, format!("{id}  \"{texto}\""));
        self.entradas.push(Entrada { linha: pos, chave: id, texto: texto.to_string() });
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn bytes(t: &str) -> Vec<u8> {
        let mut b = vec![0xFF, 0xFE];
        t.encode_utf16().for_each(|c| b.extend_from_slice(&c.to_le_bytes()));
        b
    }

    #[test]
    fn indexada_e_sequencial() {
        let i = bytes("//  Element item extend descriptions.\n\n#_index\n#_begin\n\n10037  \"^ffcb4aUma \\rinvenção\"\n10042  \"Outra\"\n");
        let mut t = TabelaDeTextos::ler(&i).unwrap();
        assert!(t.gravar() == i);
        assert!(t.indexada);
        assert_eq!((t.entradas.len(), t.entrada(10037).unwrap().texto.as_str()), (2, "^ffcb4aUma \\rinvenção"));
        t.definir(10042, "Trocada").unwrap();
        t.acrescentar(20000, "Nova").unwrap();
        assert!(t.acrescentar(20000, "x").is_err());
        assert!(t.definir(10042, "com \"aspas\"").is_err());
        let r = TabelaDeTextos::ler(&t.gravar()).unwrap();
        assert_eq!((r.entrada(10042).unwrap().texto.as_str(), r.entrada(20000).unwrap().texto.as_str()), ("Trocada", "Nova"));
        let s = bytes("//  Element fixed notify messages.\n\n#_begin\n\n\"Ganhou %d moedas\"\n\"Ganhou %d de experiência\"\n");
        let mut t = TabelaDeTextos::ler(&s).unwrap();
        assert!(!t.indexada && t.entrada(1).unwrap().texto == "Ganhou %d de experiência");
        assert!(t.acrescentar(5, "x").is_err());
        t.definir(0, "Recebeu %d moedas").unwrap();
        assert_eq!(TabelaDeTextos::ler(&t.gravar()).unwrap().entrada(0).unwrap().texto, "Recebeu %d moedas");
    }

    /// Os três arquivos reais do cliente 1.5.5 BR (fora do repositório; sem eles, avisa): ida e volta idêntica.
    #[test]
    fn os_tres_arquivos_do_cliente_voltam_identicos() {
        for a in ARQUIVOS {
            let p = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\configs").join(a);
            let Ok(b) = std::fs::read(&p) else {
                eprintln!("AVISO: sem {} — este teste NÃO verificou esse arquivo.", p.display());
                continue;
            };
            let t = TabelaDeTextos::ler(&b).unwrap();
            assert!(t.gravar() == b, "{a}");
            assert!(!t.entradas.is_empty(), "{a}");
        }
    }
}
