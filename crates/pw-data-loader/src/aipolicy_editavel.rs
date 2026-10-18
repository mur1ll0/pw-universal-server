//! `aipolicy.data` editável (B259, Estúdio S7): flags dos gatilhos e parâmetros de tamanho fixo, corrigidos no lugar.
//!
//! O arquivo fica cru; o leitor do GS (`crate::aipolicy`) registra onde começam o gatilho (`Trigger::em`) e o parâmetro
//! de cada condição e operação (`parametro_em`). Os parâmetros editáveis são os de tamanho fixo, com a mesma codificação
//! do leitor (`ler_parametro_de_tamanho_fixo` e `interpretar_parametro_de_condicao`, porte de `policytype.h` 1.7.2):
//! trocar um valor não muda o tamanho de nada. Fala, invocação e os brutos ficam de fora.
//!
//! Gatilho: `versão u32, id u32, ativo u8, rodando u8, só em combate u8` (`ler_trigger`).

use crate::aipolicy::{AiPolicyData, NoDeCondicao, ParametroDeCondicao, ParametroDeOperacao, Trigger};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tipo {
    U32,
    I32,
    F32,
}

/// Os campos editáveis de um parâmetro de operação, na ordem do arquivo; vazio = não editável.
pub fn campos_da_operacao(p: &ParametroDeOperacao) -> Vec<(&'static str, Tipo)> {
    use Tipo::*;
    match p {
        ParametroDeOperacao::TipoDeAtaque(_) => vec![("tipo_de_ataque", U32)],
        ParametroDeOperacao::Skill { .. } => vec![("skill", U32), ("nivel", U32)],
        ParametroDeOperacao::IdDeTrigger(_) => vec![("id", U32)],
        ParametroDeOperacao::Timer { .. } => vec![("id", U32), ("periodo", U32), ("contador", U32)],
        ParametroDeOperacao::AjustarVariavel { .. } => vec![("id", I32), ("valor", I32)],
        _ => vec![],
    }
}

/// O mesmo para o parâmetro de uma condição.
pub fn campos_da_condicao(p: &ParametroDeCondicao) -> Vec<(&'static str, Tipo)> {
    use Tipo::*;
    match p {
        ParametroDeCondicao::IdDeTimer(_) => vec![("id", U32)],
        ParametroDeCondicao::HpAbaixoDe(_) => vec![("fracao", F32)],
        ParametroDeCondicao::Probabilidade(_) => vec![("probabilidade", F32)],
        ParametroDeCondicao::Raio(_) => vec![("raio", F32)],
        ParametroDeCondicao::Id(_) => vec![("id", I32)],
        ParametroDeCondicao::Valor(_) => vec![("valor", I32)],
        ParametroDeCondicao::FaixaDeDano { .. } => vec![("minimo", I32), ("maximo", I32)],
        _ => vec![],
    }
}

/// O nó da árvore pelo caminho: `""` a raiz, `"e"`/`"d"` esquerda/direita, e assim por diante (`"ed"`, …).
pub fn no_pelo_caminho<'a>(raiz: Option<&'a NoDeCondicao>, caminho: &str) -> Option<&'a NoDeCondicao> {
    let mut no = raiz?;
    for c in caminho.chars() {
        no = match c {
            'e' => no.esquerda.as_deref()?,
            'd' => no.direita.as_deref()?,
            _ => return None,
        };
    }
    Some(no)
}

pub struct AiPolicyEditavel {
    pub bytes: Vec<u8>,
    pub dados: AiPolicyData,
}

impl AiPolicyEditavel {
    pub fn ler(b: &[u8]) -> Result<Self, String> {
        Ok(Self { bytes: b.to_vec(), dados: AiPolicyData::load_from_bytes(b).map_err(|e| e.to_string())? })
    }

    pub fn gravar(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn gatilho(&self, politica: u32, indice: usize) -> Result<&Trigger, String> {
        self.dados
            .policies
            .get(&politica)
            .ok_or_else(|| format!("política {politica} não existe"))?
            .triggers
            .get(indice)
            .ok_or_else(|| format!("gatilho {indice} não existe"))
    }

    /// `ativo`, `rodando` ou `so_em_combate` do gatilho.
    pub fn definir_flag(&mut self, politica: u32, indice: usize, nome: &str, valor: bool) -> Result<(), String> {
        let t = self.gatilho(politica, indice)?;
        let d = match nome {
            "ativo" => 8,
            "rodando" => 9,
            "so_em_combate" => 10,
            _ => return Err(format!("flag inexistente: {nome}")),
        };
        let o = t.em + d;
        self.bytes[o] = valor as u8;
        Ok(())
    }

    fn escrever(&mut self, o: usize, campos: &[(&'static str, Tipo)], valores: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
        for (k, _) in valores {
            if !campos.iter().any(|c| c.0 == k) {
                return Err(format!("campo inexistente: {k}"));
            }
        }
        for (i, (nome, tipo)) in campos.iter().enumerate() {
            let Some(v) = valores.get(*nome) else { continue };
            let bytes = match tipo {
                Tipo::U32 => v.as_i64().and_then(|x| u32::try_from(x).ok()).ok_or_else(|| format!("{nome}: inteiro sem sinal"))?.to_le_bytes(),
                Tipo::I32 => v.as_i64().and_then(|x| i32::try_from(x).ok()).ok_or_else(|| format!("{nome}: inteiro"))?.to_le_bytes(),
                Tipo::F32 => (v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| format!("{nome}: número"))? as f32).to_le_bytes(),
            };
            self.bytes[o + 4 * i..o + 4 * i + 4].copy_from_slice(&bytes);
        }
        Ok(())
    }

    /// Troca campos do parâmetro da operação `op` do gatilho (os nomes de `campos_da_operacao`).
    pub fn definir_operacao(&mut self, politica: u32, indice: usize, op: usize, valores: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
        let t = self.gatilho(politica, indice)?;
        let o = t.operacoes.get(op).ok_or_else(|| format!("operação {op} não existe"))?;
        let campos = campos_da_operacao(&o.parametro);
        if campos.is_empty() {
            return Err("o parâmetro desta operação não é editável".into());
        }
        let em = o.parametro_em;
        self.escrever(em, &campos, valores)
    }

    /// Troca campos do parâmetro da condição no `caminho` da árvore do gatilho.
    pub fn definir_condicao(&mut self, politica: u32, indice: usize, caminho: &str, valores: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
        let t = self.gatilho(politica, indice)?;
        let no = no_pelo_caminho(t.condicao.as_ref(), caminho).ok_or_else(|| format!("condição {caminho:?} não existe"))?;
        let campos = campos_da_condicao(&no.parametro);
        if campos.is_empty() {
            return Err("o parâmetro desta condição não é editável".into());
        }
        let em = no.parametro_em;
        self.escrever(em, &campos, valores)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    /// Nos dois realms: uma operação de habilidade e uma condição de vida, trocadas e relidas pelo leitor do GS; as
    /// outras políticas não mudam; voltar os valores devolve o arquivo idêntico.
    #[test]
    fn editar_no_lugar_e_voltar() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/aipolicy.data"));
            let b = std::fs::read(&p).unwrap();
            let mut a = AiPolicyEditavel::ler(&b).unwrap();
            assert!(a.gravar() == b);
            let mut ids: Vec<u32> = a.dados.policies.keys().copied().collect();
            ids.sort_unstable();
            // Uma operação de habilidade.
            let (pid, ti, oi, skill, nivel) = ids.iter().find_map(|&pid| {
                a.dados.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| {
                    t.operacoes.iter().enumerate().find_map(|(oi, o)| match o.parametro {
                        ParametroDeOperacao::Skill { skill, nivel } => Some((pid, ti, oi, skill, nivel)),
                        _ => None,
                    })
                })
            }).expect("alguma operação de habilidade");
            // Uma condição de vida na raiz.
            let (cp, ct, fracao) = ids.iter().find_map(|&pid| {
                a.dados.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| match t.condicao.as_ref().map(|c| &c.parametro) {
                    Some(ParametroDeCondicao::HpAbaixoDe(f)) => Some((pid, ti, *f)),
                    _ => None,
                })
            }).expect("alguma condição de vida");
            let ativo = a.dados.policies[&pid].triggers[ti].ativo;
            let outra = *ids.iter().find(|&&x| x != pid && x != cp).unwrap();
            let outra_antes = serde_json::to_value(&a.dados.policies[&outra]).unwrap();
            let m = |v: serde_json::Value| v.as_object().unwrap().clone();
            a.definir_operacao(pid, ti, oi, &m(json!({ "skill": 12345, "nivel": 7 }))).unwrap();
            a.definir_condicao(cp, ct, "", &m(json!({ "fracao": 0.25 }))).unwrap();
            a.definir_flag(pid, ti, "ativo", !ativo).unwrap();
            assert!(a.definir_operacao(pid, ti, oi, &m(json!({ "nada": 1 }))).is_err());
            assert!(a.definir_flag(pid, ti, "voar", true).is_err());
            let r = AiPolicyData::load_from_bytes(&a.gravar()).unwrap();
            assert_eq!(r.policies[&pid].triggers[ti].operacoes[oi].parametro, ParametroDeOperacao::Skill { skill: 12345, nivel: 7 }, "{realm}");
            assert_eq!(r.policies[&cp].triggers[ct].condicao.as_ref().unwrap().parametro, ParametroDeCondicao::HpAbaixoDe(0.25));
            assert_eq!(r.policies[&pid].triggers[ti].ativo, !ativo);
            assert_eq!(serde_json::to_value(&r.policies[&outra]).unwrap(), outra_antes, "{realm}: a outra não muda");
            let mut v = AiPolicyEditavel::ler(&a.gravar()).unwrap();
            v.definir_operacao(pid, ti, oi, &m(json!({ "skill": skill, "nivel": nivel }))).unwrap();
            v.definir_condicao(cp, ct, "", &m(json!({ "fracao": fracao }))).unwrap();
            v.definir_flag(pid, ti, "ativo", ativo).unwrap();
            assert!(v.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }
}
