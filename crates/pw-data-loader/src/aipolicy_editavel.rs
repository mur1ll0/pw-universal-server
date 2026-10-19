//! `aipolicy.data` editável (B259, Estúdio S7): flags dos gatilhos e parâmetros de tamanho fixo, corrigidos no lugar.
//!
//! O arquivo fica cru; o leitor do GS (`crate::aipolicy`) registra onde começam o gatilho (`Trigger::em`) e o parâmetro
//! de cada condição e operação (`parametro_em`). Os parâmetros editáveis são os de tamanho fixo, com a mesma codificação
//! do leitor (`ler_parametro_de_tamanho_fixo` e `interpretar_parametro_de_condicao`, porte de `policytype.h` 1.7.2):
//! trocar um valor não muda o tamanho de nada. Fala, invocação e os brutos ficam de fora.
//!
//! Gatilho: `versão u32, id u32, ativo u8, rodando u8, só em combate u8` (`ler_trigger`).
//!
//! **Edição estrutural (B261):** remover e duplicar gatilho e trocar o texto de uma fala mudam o tamanho do arquivo,
//! mas sem reserializar nada: o leitor registra a faixa de cada gatilho (`Trigger::em..fim`) e o começo de cada
//! política (`AiPolicy::em`: `versão u32, id u32, quantidade i32`). Remover corta a faixa e decrementa a quantidade;
//! duplicar copia a faixa logo depois da original com um id novo; a fala é `uSize u32` + `uSize` bytes UTF-16
//! (`policy.cpp:550-552`, `:610-611`; 1.5.5 sem máscara) e o GS a envia como está (`op_say`, `aitrigger.h:1844-1849`),
//! então o texto novo mantém o NUL final só se a fala antiga o tinha. Depois de cada uma, o arquivo é relido.
//! Remover é recusado se outro gatilho da política o cita (`RodarTrigger`/`PararTrigger`/`AtivarTrigger`): o
//! `GetIndex` daria -1 e o `GetTriggerPtr(-1)` do GS não confere (`policy_loader.cpp:237-238`).

use crate::aipolicy::{AiPolicyData, NoDeCondicao, ParametroDeCondicao, ParametroDeOperacao, TipoDeOperacao, Trigger};
use std::collections::{BTreeMap, BTreeSet};

/// Papel de uma folha da árvore de condições (B271): debaixo de aritmética/comparação (tipos 9..=15) é **valor**;
/// senão, **condição**. Medido (`medir_condicoes`): nenhum tipo de folha aparece nos dois papéis; condição = 0-4, 8,
/// 18-20, 22, 24, 25; valor = 16, 17, 21, 23, 26, 27; com filhas = 5-7, 9-15.
fn e_operador(tipo: i32) -> bool {
    matches!(tipo, 5..=7 | 9..=15)
}

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

    /// O que o próprio arquivo mostra (B271): por tipo de folha de condição, `(tamanho do parâmetro, é valor?)` —
    /// medido, um tamanho só por tipo; e os tipos de operação que aparecem.
    pub fn catalogo(&self) -> (BTreeMap<i32, (usize, bool)>, BTreeSet<i32>) {
        let mut folhas = BTreeMap::new();
        fn andar(n: &NoDeCondicao, b: &[u8], pai: Option<i32>, f: &mut BTreeMap<i32, (usize, bool)>) {
            if n.esquerda.is_none() && n.direita.is_none() && !e_operador(n.tipo_cru) {
                let t = i32::from_le_bytes(b[n.parametro_em - 4..n.parametro_em].try_into().unwrap()) as usize;
                f.entry(n.tipo_cru).or_insert((t, pai.is_some_and(|x| (9..=15).contains(&x))));
            }
            for x in [&n.esquerda, &n.direita].into_iter().flatten() {
                andar(x, b, Some(n.tipo_cru), f);
            }
        }
        let mut ops = BTreeSet::new();
        for p in self.dados.policies.values() {
            for t in &p.triggers {
                if let Some(c) = &t.condicao {
                    andar(c, &self.bytes, None, &mut folhas);
                }
                ops.extend(t.operacoes.iter().map(|o| o.tipo_cru));
            }
        }
        (folhas, ops)
    }

    /// Troca o tipo de uma folha da árvore de condições (B271): só para um tipo do mesmo papel (condição ou valor) que
    /// já apareça no arquivo; o parâmetro vira zeros do tamanho que esse tipo tem no arquivo. Operadores (com filhas)
    /// não mudam. O arquivo é relido.
    pub fn trocar_tipo_da_condicao(&mut self, politica: u32, indice: usize, caminho: &str, novo: i32) -> Result<(), String> {
        let (folhas, _) = self.catalogo();
        let t = self.gatilho(politica, indice)?;
        let no = no_pelo_caminho(t.condicao.as_ref(), caminho).ok_or_else(|| format!("condição {caminho:?} não existe"))?;
        if no.esquerda.is_some() || no.direita.is_some() || e_operador(no.tipo_cru) {
            return Err("só condição sem filhas (folha) troca de tipo".into());
        }
        let pai = if caminho.is_empty() { None } else { no_pelo_caminho(t.condicao.as_ref(), &caminho[..caminho.len() - 1]).map(|x| x.tipo_cru) };
        let e_valor = pai.is_some_and(|x| (9..=15).contains(&x));
        let &(tamanho, valor) = folhas.get(&novo).ok_or_else(|| format!("o tipo de condição {novo} não aparece neste arquivo"))?;
        if valor != e_valor {
            return Err(format!("o tipo {novo} é {} e este nó é {}", if valor { "valor" } else { "condição" }, if e_valor { "valor" } else { "condição" }));
        }
        let o = no.parametro_em;
        let antigo = i32::from_le_bytes(self.bytes[o - 4..o].try_into().unwrap()) as usize;
        self.bytes.splice(o..o + antigo, vec![0u8; tamanho]);
        self.bytes[o - 8..o - 4].copy_from_slice(&novo.to_le_bytes());
        self.bytes[o - 4..o].copy_from_slice(&(tamanho as i32).to_le_bytes());
        self.reler()
    }

    /// Os bytes de uma operação nova do tipo `tipo` no gatilho de versão `versao`: tipo, parâmetro zerado (as falas,
    /// vazias com o NUL — medido no B261) e, para rodar/parar/ativar gatilho, o `id` (que tem que existir na política).
    fn bytes_da_operacao(tipo: i32, versao: u32, id_gatilho: Option<u32>, alvo: &[u8]) -> Result<Vec<u8>, String> {
        let mut b = tipo.to_le_bytes().to_vec();
        if AiPolicyData::le_parametro_nesta_versao(tipo, versao) {
            let fala = |b: &mut Vec<u8>| {
                b.extend_from_slice(&2u32.to_le_bytes());
                b.extend_from_slice(&[0, 0]);
                if versao >= 17 {
                    b.extend_from_slice(&0u32.to_le_bytes());
                }
            };
            match tipo {
                2 => fala(&mut b),
                // SkillComFala: quatro u32 e a fala; Falar2: a fala e quatro u32 (`ler_parametro_de_operacao`).
                t if TipoDeOperacao::de_i32(t) == Some(TipoDeOperacao::SkillComFala) => { b.extend_from_slice(&[0; 16]); fala(&mut b) }
                t if TipoDeOperacao::de_i32(t) == Some(TipoDeOperacao::Falar2) => { fala(&mut b); b.extend_from_slice(&[0; 16]) }
                4..=6 => b.extend_from_slice(&id_gatilho.ok_or("rodar/parar/ativar gatilho precisa do id de um gatilho da política")?.to_le_bytes()),
                _ => b.extend(vec![0u8; crate::aipolicy::tamanho_do_parametro(tipo, versao).ok_or_else(|| format!("operação {tipo} sem tamanho conhecido"))?]),
            }
        }
        b.extend_from_slice(alvo);
        Ok(b)
    }

    fn validar_tipo_de_operacao(&self, politica: u32, tipo: i32, id_gatilho: Option<u32>) -> Result<(), String> {
        let (_, ops) = self.catalogo();
        if !ops.contains(&tipo) {
            return Err(format!("o tipo de operação {tipo} não aparece neste arquivo"));
        }
        if (4..=6).contains(&tipo) {
            let id = id_gatilho.ok_or("rodar/parar/ativar gatilho precisa de `id_gatilho`")?;
            if !self.dados.policies[&politica].triggers.iter().any(|t| t.id == id) {
                return Err(format!("a política não tem o gatilho {id}"));
            }
        }
        Ok(())
    }

    /// Onde a operação `op` começa e termina, e o trecho do alvo (os bytes depois do parâmetro).
    fn faixa_da_operacao(&self, t: &Trigger, op: usize) -> Result<(usize, usize, Vec<u8>), String> {
        let o = t.operacoes.get(op).ok_or_else(|| format!("operação {op} não existe"))?;
        let inicio = o.parametro_em - 4;
        let fim = t.operacoes.get(op + 1).map(|x| x.parametro_em - 4).unwrap_or(t.fim);
        let n = 4 + crate::aipolicy::tamanho_do_parametro_de_alvo(o.alvo.tipo_cru);
        Ok((inicio, fim, self.bytes[fim - n..fim].to_vec()))
    }

    /// Troca o tipo da operação (B271): parâmetro novo zerado (ou o gatilho, nas de gatilho); o alvo fica.
    pub fn trocar_tipo_da_operacao(&mut self, politica: u32, indice: usize, op: usize, tipo: i32, id_gatilho: Option<u32>) -> Result<(), String> {
        self.validar_tipo_de_operacao(politica, tipo, id_gatilho)?;
        let t = self.gatilho(politica, indice)?.clone();
        let (inicio, fim, alvo) = self.faixa_da_operacao(&t, op)?;
        let b = Self::bytes_da_operacao(tipo, t.versao, id_gatilho, &alvo)?;
        self.bytes.splice(inicio..fim, b);
        self.reler()
    }

    /// Acrescenta uma operação no fim do gatilho (B271), com o alvo da última (ou alvo 0 sem parâmetro).
    pub fn acrescentar_operacao(&mut self, politica: u32, indice: usize, tipo: i32, id_gatilho: Option<u32>) -> Result<(), String> {
        self.validar_tipo_de_operacao(politica, tipo, id_gatilho)?;
        let t = self.gatilho(politica, indice)?.clone();
        let alvo = match t.operacoes.len() {
            0 => 0i32.to_le_bytes().to_vec(),
            n => self.faixa_da_operacao(&t, n - 1)?.2,
        };
        let b = Self::bytes_da_operacao(tipo, t.versao, id_gatilho, &alvo)?;
        self.bytes.splice(t.fim..t.fim, b);
        let o = t.operacoes_em;
        let n = i32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap()) + 1;
        self.bytes[o..o + 4].copy_from_slice(&n.to_le_bytes());
        self.reler()
    }

    /// Remove uma operação (B271); a última do gatilho fica (um gatilho sem nada a fazer não é editado aqui).
    pub fn remover_operacao(&mut self, politica: u32, indice: usize, op: usize) -> Result<(), String> {
        let t = self.gatilho(politica, indice)?.clone();
        if t.operacoes.len() <= 1 {
            return Err("o gatilho precisa de ao menos uma operação (remova o gatilho)".into());
        }
        let (inicio, fim, _) = self.faixa_da_operacao(&t, op)?;
        self.bytes.drain(inicio..fim);
        let o = t.operacoes_em;
        let n = i32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap()) - 1;
        self.bytes[o..o + 4].copy_from_slice(&n.to_le_bytes());
        self.reler()
    }

    fn reler(&mut self) -> Result<(), String> {
        self.dados = AiPolicyData::load_from_bytes(&self.bytes).map_err(|e| format!("o arquivo editado não relê: {e}"))?;
        Ok(())
    }

    fn politica_unica(&self, politica: u32) -> Result<&crate::aipolicy::AiPolicy, String> {
        if self.dados.ids_repetidos.contains(&politica) {
            return Err(format!("a política {politica} aparece mais de uma vez no arquivo; edição estrutural recusada"));
        }
        self.dados.policies.get(&politica).ok_or_else(|| format!("política {politica} não existe"))
    }

    fn somar_na_quantidade(&mut self, em_da_politica: usize, delta: i32) {
        let o = em_da_politica + 8;
        let n = i32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap()) + delta;
        self.bytes[o..o + 4].copy_from_slice(&n.to_le_bytes());
    }

    /// Os gatilhos da política que citam o id (rodar, parar, ativar gatilho), como `(índice, operação)`.
    pub fn citacoes(&self, politica: u32, id: u32) -> Vec<(usize, usize)> {
        let Some(p) = self.dados.policies.get(&politica) else { return vec![] };
        let mut r = Vec::new();
        for (ti, t) in p.triggers.iter().enumerate() {
            for (oi, o) in t.operacoes.iter().enumerate() {
                let cita = matches!(o.tipo, Some(TipoDeOperacao::RodarTrigger | TipoDeOperacao::PararTrigger | TipoDeOperacao::AtivarTrigger))
                    && o.parametro == ParametroDeOperacao::IdDeTrigger(id);
                if cita {
                    r.push((ti, oi));
                }
            }
        }
        r
    }

    /// Remove o gatilho (recusa se outro o cita e não há outro com o mesmo id).
    pub fn remover_gatilho(&mut self, politica: u32, indice: usize) -> Result<(), String> {
        let p = self.politica_unica(politica)?;
        let t = p.triggers.get(indice).ok_or_else(|| format!("gatilho {indice} não existe"))?;
        let (em_p, em, fim, id) = (p.em, t.em, t.fim, t.id);
        let mesmo_id = p.triggers.iter().filter(|x| x.id == id).count();
        let citado: Vec<_> = self.citacoes(politica, id).into_iter().filter(|(ti, _)| *ti != indice).collect();
        if mesmo_id == 1 && !citado.is_empty() {
            return Err(format!("o gatilho {id} é chamado por {} operação(ões) (gatilho {}); tire essas chamadas antes", citado.len(), citado[0].0));
        }
        self.bytes.drain(em..fim);
        self.somar_na_quantidade(em_p, -1);
        self.reler()
    }

    /// Copia o gatilho logo depois do original, com `novo_id` (ou o maior id da política + 1). Devolve o id.
    pub fn duplicar_gatilho(&mut self, politica: u32, indice: usize, novo_id: Option<u32>) -> Result<u32, String> {
        let p = self.politica_unica(politica)?;
        let t = p.triggers.get(indice).ok_or_else(|| format!("gatilho {indice} não existe"))?;
        let id = match novo_id {
            Some(i) => i,
            None => p.triggers.iter().map(|x| x.id).max().unwrap_or(0).checked_add(1).ok_or("sem id livre")?,
        };
        if p.triggers.iter().any(|x| x.id == id) {
            return Err(format!("o id {id} já existe nesta política"));
        }
        let (em_p, em, fim) = (p.em, t.em, t.fim);
        let mut copia = self.bytes[em..fim].to_vec();
        copia[4..8].copy_from_slice(&id.to_le_bytes());
        self.bytes.splice(fim..fim, copia);
        self.somar_na_quantidade(em_p, 1);
        self.reler()?;
        Ok(id)
    }

    /// Troca o texto de uma fala (`Falar`, `Falar2`, `SkillComFala`).
    pub fn definir_fala(&mut self, politica: u32, indice: usize, op: usize, texto: &str) -> Result<(), String> {
        if texto.contains('\0') {
            return Err("a fala não pode ter NUL".into());
        }
        if texto.encode_utf16().count() > 2048 {
            return Err("fala grande demais (guarda do painel: 2048 unidades UTF-16)".into());
        }
        let t = self.gatilho(politica, indice)?;
        let o = t.operacoes.get(op).ok_or_else(|| format!("operação {op} não existe"))?;
        let em = match o.parametro {
            ParametroDeOperacao::Fala { .. } | ParametroDeOperacao::Fala2 { .. } => o.parametro_em,
            // `O_USE_SKILL_2` (quatro u32) antes de `O_TALK_TEXT`.
            ParametroDeOperacao::SkillComFala { .. } => o.parametro_em + 16,
            _ => return Err("esta operação não é uma fala".into()),
        };
        let tamanho = u32::from_le_bytes(self.bytes[em..em + 4].try_into().unwrap()) as usize;
        let antigo = &self.bytes[em + 4..em + 4 + tamanho];
        let com_nul = tamanho >= 2 && antigo[tamanho - 2..] == [0, 0];
        let mut novo: Vec<u8> = texto.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        if com_nul {
            novo.extend_from_slice(&[0, 0]);
        }
        let n = u32::try_from(novo.len()).unwrap();
        self.bytes.splice(em + 4..em + 4 + tamanho, novo);
        self.bytes[em..em + 4].copy_from_slice(&n.to_le_bytes());
        self.reler()
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

    /// Nos dois realms (B271): trocar o tipo de uma folha de condição (mesmo papel) e de uma operação, acrescentar uma
    /// de rodar gatilho e removê-la; relido pelo leitor do GS; a outra política igual; papel errado, tipo que não aparece
    /// no arquivo, gatilho inexistente e remover a última operação recusados; voltar = arquivo idêntico.
    #[test]
    fn trocar_tipos_e_operacoes() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/aipolicy.data"));
            let b = std::fs::read(&p).unwrap();
            let mut a = AiPolicyEditavel::ler(&b).unwrap();
            let mut ids: Vec<u32> = a.dados.policies.keys().copied().filter(|x| !a.dados.ids_repetidos.contains(x)).collect();
            ids.sort_unstable();
            // Uma folha de condição na raiz: HpLess (1) → Random (3), os dois de 4 bytes; e volta.
            let (cp, ct) = ids.iter().find_map(|&pid| a.dados.policies[&pid].triggers.iter().position(|t| t.condicao.as_ref().is_some_and(|c| c.tipo_cru == 1 && c.esquerda.is_none())).map(|ti| (pid, ti))).unwrap();
            let antes_c = a.dados.policies[&cp].triggers[ct].condicao.as_ref().unwrap().parametro.clone();
            a.trocar_tipo_da_condicao(cp, ct, "", 2).unwrap(); // StartAttack: 0 bytes
            assert_eq!(a.dados.policies[&cp].triggers[ct].condicao.as_ref().unwrap().tipo_cru, 2, "{realm}");
            assert!(a.trocar_tipo_da_condicao(cp, ct, "", 999).is_err());
            // Uma operação de habilidade (1) com 2+ operações no gatilho → atacar (0) e volta.
            let (op_p, op_t, op_i) = ids.iter().find_map(|&pid| a.dados.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| {
                (t.operacoes.len() >= 2).then(|| t.operacoes.iter().position(|o| o.tipo_cru == 1)).flatten().map(|oi| (pid, ti, oi))
            })).unwrap();
            let outra = *ids.iter().find(|&&x| x != op_p && x != cp).unwrap();
            let outra_antes = serde_json::to_value(&a.dados.policies[&outra]).unwrap();
            let op_antes = a.dados.policies[&op_p].triggers[op_t].operacoes[op_i].parametro.clone();
            a.trocar_tipo_da_operacao(op_p, op_t, op_i, 0, None).unwrap();
            assert_eq!(a.dados.policies[&op_p].triggers[op_t].operacoes[op_i].tipo_cru, 0);
            let gid = a.dados.policies[&op_p].triggers[op_t].id;
            assert!(a.acrescentar_operacao(op_p, op_t, 4, None).is_err(), "gatilho obrigatório");
            assert!(a.acrescentar_operacao(op_p, op_t, 4, Some(987_654)).is_err(), "gatilho inexistente");
            a.acrescentar_operacao(op_p, op_t, 4, Some(gid)).unwrap();
            let n = a.dados.policies[&op_p].triggers[op_t].operacoes.len();
            assert_eq!(a.dados.policies[&op_p].triggers[op_t].operacoes[n - 1].parametro, ParametroDeOperacao::IdDeTrigger(gid));
            let r = AiPolicyData::load_from_bytes(&a.gravar()).unwrap();
            assert_eq!(serde_json::to_value(&r.policies[&outra]).unwrap(), outra_antes, "{realm}: a outra não muda");
            // Volta.
            a.remover_operacao(op_p, op_t, n - 1).unwrap();
            a.trocar_tipo_da_operacao(op_p, op_t, op_i, 1, None).unwrap();
            if let ParametroDeOperacao::Skill { skill, nivel } = op_antes {
                let m = serde_json::json!({ "skill": skill, "nivel": nivel });
                a.definir_operacao(op_p, op_t, op_i, m.as_object().unwrap()).unwrap();
            }
            a.trocar_tipo_da_condicao(cp, ct, "", 1).unwrap();
            if let ParametroDeCondicao::HpAbaixoDe(f) = antes_c {
                a.definir_condicao(cp, ct, "", serde_json::json!({ "fracao": f }).as_object().unwrap()).unwrap();
            }
            assert!(a.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
            // A única operação de um gatilho não sai.
            if let Some((pp, tt)) = ids.iter().find_map(|&pid| a.dados.policies[&pid].triggers.iter().position(|t| t.operacoes.len() == 1).map(|ti| (pid, ti))) {
                assert!(a.remover_operacao(pp, tt, 0).is_err());
            }
        }
    }

    /// Medida (B271): por tipo de condição, os tamanhos de parâmetro vistos e se o nó tem filhas.
    #[test]
    fn medir_condicoes() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/aipolicy.data"));
            let b = std::fs::read(&p).unwrap();
            let a = AiPolicyEditavel::ler(&b).unwrap();
            let mut m: std::collections::BTreeMap<i32, (std::collections::BTreeSet<usize>, usize, usize)> = Default::default();
            fn andar(n: &NoDeCondicao, b: &[u8], m: &mut std::collections::BTreeMap<i32, (std::collections::BTreeSet<usize>, usize, usize)>) {
                let t = i32::from_le_bytes(b[n.parametro_em - 4..n.parametro_em].try_into().unwrap()) as usize;
                let e = m.entry(n.tipo_cru).or_default();
                e.0.insert(t);
                if n.esquerda.is_some() || n.direita.is_some() { e.1 += 1 } else { e.2 += 1 }
                if let Some(x) = &n.esquerda { andar(x, b, m) }
                if let Some(x) = &n.direita { andar(x, b, m) }
            }
            for pol in a.dados.policies.values() {
                for t in &pol.triggers {
                    if let Some(c) = &t.condicao { andar(c, &b, &mut m) }
                }
            }
            eprintln!("{realm}: tipo -> (tamanhos, com filhas, folhas): {m:?}");
            // Papel das folhas: debaixo de aritmética/comparação (9..=15) = valor; senão = condição.
            let mut papel: std::collections::BTreeMap<i32, (usize, usize)> = Default::default();
            fn papel_de(n: &NoDeCondicao, pai: Option<i32>, p: &mut std::collections::BTreeMap<i32, (usize, usize)>) {
                if n.esquerda.is_none() && n.direita.is_none() {
                    let e = p.entry(n.tipo_cru).or_default();
                    if pai.is_some_and(|x| (9..=15).contains(&x)) { e.0 += 1 } else { e.1 += 1 }
                }
                if let Some(x) = &n.esquerda { papel_de(x, Some(n.tipo_cru), p) }
                if let Some(x) = &n.direita { papel_de(x, Some(n.tipo_cru), p) }
            }
            for pol in a.dados.policies.values() {
                for t in &pol.triggers {
                    if let Some(c) = &t.condicao { papel_de(c, None, &mut papel) }
                }
            }
            eprintln!("{realm}: folha -> (como valor, como condição): {papel:?}");
        }
    }

    /// Nos dois realms: duplicar, trocar a fala e remover voltam ao arquivo original; a outra política não muda;
    /// gatilho citado não se remove. Mede quantas falas têm NUL final (a convenção que `definir_fala` mantém).
    #[test]
    fn duplicar_falar_e_remover_voltam() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/aipolicy.data"));
            let b = std::fs::read(&p).unwrap();
            let mut a = AiPolicyEditavel::ler(&b).unwrap();
            let mut ids: Vec<u32> = a.dados.policies.keys().copied().filter(|x| !a.dados.ids_repetidos.contains(x)).collect();
            ids.sort_unstable();
            // Medida: falas com e sem NUL final.
            let (mut com, mut sem) = (0, 0);
            for pid in &ids {
                for t in &a.dados.policies[pid].triggers {
                    for o in &t.operacoes {
                        if let ParametroDeOperacao::Fala { .. } = o.parametro {
                            let n = u32::from_le_bytes(b[o.parametro_em..o.parametro_em + 4].try_into().unwrap()) as usize;
                            let fim = o.parametro_em + 4 + n;
                            if n >= 2 && b[fim - 2..fim] == [0, 0] { com += 1 } else { sem += 1 }
                        }
                    }
                }
            }
            eprintln!("{realm}: falas com NUL final {com}, sem {sem}");
            // Uma fala qualquer.
            let (pid, ti, oi, texto) = ids.iter().find_map(|&pid| {
                a.dados.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| {
                    t.operacoes.iter().enumerate().find_map(|(oi, o)| match &o.parametro {
                        ParametroDeOperacao::Fala { texto, .. } => Some((pid, ti, oi, texto.clone())),
                        _ => None,
                    })
                })
            }).expect("alguma fala");
            let outra = *ids.iter().find(|&&x| x != pid).unwrap();
            let outra_antes = serde_json::to_value(&a.dados.policies[&outra]).unwrap();
            let n = a.dados.policies[&pid].triggers.len();
            a.definir_fala(pid, ti, oi, "Fala trocada pelo Estúdio, bem mais comprida que a original").unwrap();
            let novo = a.duplicar_gatilho(pid, ti, None).unwrap();
            assert!(a.duplicar_gatilho(pid, ti, Some(novo)).is_err(), "id repetido");
            let r = AiPolicyData::load_from_bytes(&a.gravar()).unwrap();
            let pr = &r.policies[&pid];
            assert_eq!((pr.triggers.len(), pr.triggers[ti + 1].id), (n + 1, novo), "{realm}");
            for k in [ti, ti + 1] {
                assert!(matches!(&pr.triggers[k].operacoes[oi].parametro, ParametroDeOperacao::Fala { texto, .. } if texto.starts_with("Fala trocada")));
            }
            assert_eq!(serde_json::to_value(&r.policies[&outra]).unwrap(), outra_antes, "{realm}: a outra não muda");
            a.remover_gatilho(pid, ti + 1).unwrap();
            a.definir_fala(pid, ti, oi, &texto).unwrap();
            assert!(a.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
            // Gatilho citado por outro não se remove.
            let citado = ids.iter().find_map(|&pid| {
                a.dados.policies[&pid].triggers.iter().enumerate().find_map(|(ti, t)| {
                    let fora = a.citacoes(pid, t.id).iter().any(|(x, _)| *x != ti);
                    let unico = a.dados.policies[&pid].triggers.iter().filter(|x| x.id == t.id).count() == 1;
                    (fora && unico).then_some((pid, ti))
                })
            });
            if let Some((cp, ct)) = citado {
                assert!(a.remover_gatilho(cp, ct).is_err(), "{realm}: citado");
            } else {
                eprintln!("{realm}: nenhum gatilho citado por outro — a recusa NÃO foi verificada aqui");
            }
        }
    }
}
