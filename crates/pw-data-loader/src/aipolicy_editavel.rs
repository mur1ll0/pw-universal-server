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

/// Nó da árvore de condições com o parâmetro cru, para reescrever a árvore (B365).
#[derive(Debug, Clone)]
struct NoCru {
    tipo: i32,
    parametro: Vec<u8>,
    esquerda: Option<Box<NoCru>>,
    direita: Option<Box<NoCru>>,
}

impl NoCru {
    fn de(n: &NoDeCondicao, b: &[u8]) -> Self {
        let t = i32::from_le_bytes(b[n.parametro_em - 4..n.parametro_em].try_into().unwrap()) as usize;
        NoCru {
            tipo: n.tipo_cru,
            parametro: b[n.parametro_em..n.parametro_em + t].to_vec(),
            esquerda: n.esquerda.as_ref().map(|x| Box::new(Self::de(x, b))),
            direita: n.direita.as_ref().map(|x| Box::new(Self::de(x, b))),
        }
    }

    /// `CTriggerData::SaveConditonTree` (`policy.cpp:110-149`): tipo, tamanho, parâmetro; folha → `3`; senão
    /// `1` + esquerda, `2` + direita (os que houver) e `4`.
    fn gravar(&self, b: &mut Vec<u8>) {
        b.extend_from_slice(&self.tipo.to_le_bytes());
        b.extend_from_slice(&(self.parametro.len() as i32).to_le_bytes());
        b.extend_from_slice(&self.parametro);
        if self.esquerda.is_none() && self.direita.is_none() {
            b.extend_from_slice(&3i32.to_le_bytes());
            return;
        }
        if let Some(e) = &self.esquerda {
            b.extend_from_slice(&1i32.to_le_bytes());
            e.gravar(b);
        }
        if let Some(d) = &self.direita {
            b.extend_from_slice(&2i32.to_le_bytes());
            d.gravar(b);
        }
        b.extend_from_slice(&4i32.to_le_bytes());
    }

    fn no_mut(&mut self, caminho: &str) -> Option<&mut NoCru> {
        let mut no = self;
        for c in caminho.chars() {
            no = match c {
                'e' => no.esquerda.as_deref_mut()?,
                'd' => no.direita.as_deref_mut()?,
                _ => return None,
            };
        }
        Some(no)
    }

    /// O nó do caminho está debaixo de aritmética/comparação (papel de valor)?
    fn debaixo_de_valor(&self, caminho: &str) -> Result<bool, String> {
        let mut no = self;
        for c in caminho.chars() {
            if (9..=15).contains(&no.tipo) {
                return Ok(true);
            }
            no = match c {
                'e' => no.esquerda.as_deref(),
                'd' => no.direita.as_deref(),
                _ => None,
            }
            .ok_or_else(|| format!("condição {caminho:?} não existe"))?;
        }
        Ok(false)
    }
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

    /// Renomeia o gatilho (B365): `char szName[128]` logo depois das três flags (`ler_trigger`), GBK como o editor
    /// original grava. É rótulo de editor — o GS não o usa. Recusa vazio, fora do GBK ou com mais de 127 bytes.
    pub fn renomear_gatilho(&mut self, politica: u32, indice: usize, nome: &str) -> Result<(), String> {
        let (g, _, erro) = encoding_rs::GBK.encode(nome);
        if nome.trim().is_empty() || erro || g.len() > 127 || g.contains(&0) {
            return Err("nome do gatilho: de 1 a 127 bytes em GBK".into());
        }
        let o = self.gatilho(politica, indice)?.em + 11;
        self.bytes[o..o + 128].fill(0);
        self.bytes[o..o + g.len()].copy_from_slice(&g);
        self.reler()
    }

    /// A árvore de condições do gatilho como nós crus, e a faixa de bytes que ela ocupa (`em + 139 .. operacoes_em`).
    fn arvore_crua(&self, politica: u32, indice: usize) -> Result<(Option<NoCru>, std::ops::Range<usize>, i32), String> {
        let t = self.gatilho(politica, indice)?;
        let raiz = t.condicao.as_ref().map(|n| NoCru::de(n, &self.bytes));
        let papel_da_raiz = t.condicao.as_ref().map(|n| n.tipo_cru).unwrap_or(-1);
        Ok((raiz, t.em + 139..t.operacoes_em, papel_da_raiz))
    }

    fn gravar_arvore(&mut self, faixa: std::ops::Range<usize>, raiz: &NoCru) -> Result<(), String> {
        let mut b = Vec::new();
        raiz.gravar(&mut b);
        self.bytes.splice(faixa, b);
        self.reler()
    }

    /// Folha nova de **condição** do tipo `tipo` (parâmetro zerado do tamanho que o tipo tem neste arquivo).
    fn folha_nova(&self, tipo: i32) -> Result<NoCru, String> {
        let (folhas, _) = self.catalogo();
        let &(tamanho, valor) = folhas.get(&tipo).ok_or_else(|| format!("o tipo de condição {tipo} não aparece neste arquivo"))?;
        if valor {
            return Err(format!("o tipo {tipo} é valor (vai debaixo de comparação), não condição"));
        }
        Ok(NoCru { tipo, parametro: vec![0; tamanho], esquerda: None, direita: None })
    }

    /// Põe um operador lógico em volta do nó do `caminho` (B365): `5` Não (o nó vira o filho **direito** — medido: os
    /// 113 Não do 1.2.6 e os 1.672 do 1.5.5 só têm o direito; o nosso GS aceita os dois), `6` Ou / `7` E (o nó vira o
    /// esquerdo e uma folha nova do tipo `folha` o direito; E/Ou sempre têm os dois no arquivo: 9 e 770). Só nó de
    /// **condição** (não debaixo de aritmética/comparação). Gravado como `SaveConditonTree` (`policy.cpp:110-149`).
    pub fn envolver_condicao(&mut self, politica: u32, indice: usize, caminho: &str, operador: i32, folha: Option<i32>) -> Result<(), String> {
        let (raiz, faixa, _) = self.arvore_crua(politica, indice)?;
        let mut raiz = raiz.ok_or("o gatilho não tem condição")?;
        if raiz.debaixo_de_valor(caminho)? {
            return Err("este nó é valor de uma comparação: só condição recebe Não/E/Ou".into());
        }
        let novo_direito = match operador {
            5 => None,
            6 | 7 => Some(Box::new(self.folha_nova(folha.ok_or("E/Ou precisam do tipo da condição nova (`folha`)")?)?)),
            _ => return Err("operador: 5 (Não), 6 (Ou) ou 7 (E)".into()),
        };
        let alvo = raiz.no_mut(caminho).ok_or_else(|| format!("condição {caminho:?} não existe"))?;
        let antigo = std::mem::replace(alvo, NoCru { tipo: operador, parametro: vec![], esquerda: None, direita: None });
        if operador == 5 {
            alvo.direita = Some(Box::new(antigo));
        } else {
            alvo.esquerda = Some(Box::new(antigo));
            alvo.direita = novo_direito;
        }
        self.gravar_arvore(faixa, &raiz)
    }

    /// Tira um nó da árvore (B365): um Não sai e deixa o filho no lugar; um nó filho de E/Ou sai e o operador é
    /// trocado pelo irmão. A raiz-folha não sai (o gatilho precisa de condição) nem nó debaixo de comparação.
    pub fn remover_condicao(&mut self, politica: u32, indice: usize, caminho: &str) -> Result<(), String> {
        let (raiz, faixa, _) = self.arvore_crua(politica, indice)?;
        let mut raiz = raiz.ok_or("o gatilho não tem condição")?;
        let no = raiz.no_mut(caminho).ok_or_else(|| format!("condição {caminho:?} não existe"))?;
        if no.tipo == 5 {
            let filho = no.direita.take().or_else(|| no.esquerda.take()).ok_or("Não sem filho")?;
            *no = *filho;
            return self.gravar_arvore(faixa, &raiz);
        }
        if caminho.is_empty() {
            return Err("a raiz só sai trocando de tipo (o gatilho precisa de condição)".into());
        }
        let (pai_c, lado) = caminho.split_at(caminho.len() - 1);
        let pai = raiz.no_mut(pai_c).ok_or("pai")?;
        if !matches!(pai.tipo, 6 | 7) {
            return Err("só sai nó filho de E/Ou, ou um Não".into());
        }
        let irmao = if lado == "e" { pai.direita.take() } else { pai.esquerda.take() }.ok_or("E/Ou sem o outro filho")?;
        *pai = *irmao;
        self.gravar_arvore(faixa, &raiz)
    }

    /// Gatilho novo do zero no fim da política (B365): versão do arquivo, `id` pedido ou o maior + 1, ativo, não rodando,
    /// só em combate (as flags se mudam depois), o `nome`, uma folha de condição `condicao` e uma operação `operacao`
    /// (alvo 0), nos formatos de `bytes_da_operacao` e `SaveConditonTree`. Devolve o id.
    pub fn criar_gatilho(&mut self, politica: u32, nome: &str, condicao: i32, operacao: i32, id_gatilho: Option<u32>, novo_id: Option<u32>) -> Result<u32, String> {
        let p = self.politica_unica(politica)?;
        let id = match novo_id {
            Some(i) => i,
            None => p.triggers.iter().map(|x| x.id).max().map_or(Some(0), |m| m.checked_add(1)).ok_or("sem id livre")?,
        };
        if p.triggers.iter().any(|x| x.id == id) {
            return Err(format!("o id {id} já existe nesta política"));
        }
        let versao = self.dados.policies.values().flat_map(|q| q.triggers.iter().map(|t| t.versao)).max().ok_or("o arquivo não tem gatilho para dar a versão")?;
        let fim = p.triggers.last().map(|t| t.fim).unwrap_or(p.em + 12);
        let em_p = p.em;
        if (4..=6).contains(&operacao) && id_gatilho == Some(id) {
            return Err("o gatilho novo não pode chamar a si mesmo".into());
        }
        self.validar_tipo_de_operacao(politica, operacao, id_gatilho)?;
        let folha = self.folha_nova(condicao)?;
        let (g, _, erro) = encoding_rs::GBK.encode(nome);
        if nome.trim().is_empty() || erro || g.len() > 127 || g.contains(&0) {
            return Err("nome do gatilho: de 1 a 127 bytes em GBK".into());
        }
        let mut b = versao.to_le_bytes().to_vec();
        b.extend_from_slice(&id.to_le_bytes());
        b.extend_from_slice(&[1, 0, 1]);
        let mut n = [0u8; 128];
        n[..g.len()].copy_from_slice(&g);
        b.extend_from_slice(&n);
        folha.gravar(&mut b);
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend(Self::bytes_da_operacao(operacao, versao, id_gatilho, &0i32.to_le_bytes())?);
        self.bytes.splice(fim..fim, b);
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


#[cfg(test)]
mod testes_b365 {
    use super::*;

    /// Nos dois realms (B365): Não e E em volta de uma folha e tirar de volta devolvem o arquivo idêntico; o nome do
    /// gatilho (GBK) muda e relê; o gatilho do zero entra no fim da política, relê e não mexe nas outras.
    #[test]
    fn arvore_nome_e_gatilho_do_zero() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/aipolicy.data"));
            let b = std::fs::read(&p).unwrap();
            let mut a = AiPolicyEditavel::ler(&b).unwrap();
            // Uma política sem id repetido com um gatilho cuja raiz é folha de condição.
            let (pol, gi) = a.dados.policies.iter().filter(|(id, _)| !a.dados.ids_repetidos.contains(id))
                .find_map(|(id, q)| q.triggers.iter().position(|t| t.condicao.as_ref().is_some_and(|c| c.esquerda.is_none() && c.direita.is_none() && !e_operador(c.tipo_cru))).map(|i| (*id, i)))
                .expect("gatilho com raiz folha");
            let folha = a.dados.policies[&pol].triggers[gi].condicao.as_ref().unwrap().tipo_cru;
            a.envolver_condicao(pol, gi, "", 5, None).unwrap();
            let raiz = a.dados.policies[&pol].triggers[gi].condicao.clone().unwrap();
            assert_eq!((raiz.tipo_cru, raiz.esquerda.is_none(), raiz.direita.as_ref().map(|d| d.tipo_cru)), (5, true, Some(folha)), "{realm}: Não com o filho direito");
            a.remover_condicao(pol, gi, "").unwrap();
            assert!(a.gravar() == b, "{realm}: tirar o Não devolve o arquivo");
            a.envolver_condicao(pol, gi, "", 7, Some(folha)).unwrap();
            let raiz = a.dados.policies[&pol].triggers[gi].condicao.clone().unwrap();
            assert_eq!((raiz.tipo_cru, raiz.esquerda.as_ref().map(|x| x.tipo_cru), raiz.direita.as_ref().map(|x| x.tipo_cru)), (7, Some(folha), Some(folha)));
            assert!(a.remover_condicao(pol, gi, "").is_err(), "{realm}: E na raiz não sai");
            a.remover_condicao(pol, gi, "d").unwrap();
            assert!(a.gravar() == b, "{realm}: tirar o filho do E devolve o arquivo");
            assert!(a.envolver_condicao(pol, gi, "", 6, None).is_err(), "Ou sem a condição nova");
            assert!(a.envolver_condicao(pol, gi, "", 9, Some(folha)).is_err(), "operador fora de Não/E/Ou");

            a.renomear_gatilho(pol, gi, "Chefe enfurecido 狂暴").unwrap();
            assert_eq!(a.dados.policies[&pol].triggers[gi].nome, "Chefe enfurecido 狂暴");
            assert!(a.renomear_gatilho(pol, gi, &"x".repeat(128)).is_err());
            assert_eq!(a.gravar().len(), b.len(), "renomear não muda o tamanho");

            let antes = a.dados.policies[&pol].triggers.len();
            let outras: Vec<_> = a.dados.policies.iter().filter(|(id, _)| **id != pol).map(|(id, q)| (*id, q.triggers.len())).collect();
            let op = *a.catalogo().1.iter().find(|t| !(4..=6).contains(*t)).unwrap();
            let id = a.criar_gatilho(pol, "Novo", folha, op, None, None).unwrap();
            let q = &a.dados.policies[&pol];
            let t = q.triggers.last().unwrap();
            assert_eq!((q.triggers.len(), t.id, t.nome.as_str(), t.ativo, t.rodando, t.so_em_combate, t.operacoes.len()), (antes + 1, id, "Novo", true, false, true, 1));
            assert_eq!(t.condicao.as_ref().map(|c| c.tipo_cru), Some(folha));
            assert_eq!(t.operacoes[0].tipo_cru, op);
            assert_eq!(t.versao, a.dados.policies[&pol].triggers[0].versao);
            for (oid, n) in outras {
                assert_eq!(a.dados.policies[&oid].triggers.len(), n, "{realm}: política {oid} intacta");
            }
            assert!(a.criar_gatilho(pol, "Repetido", folha, op, None, Some(id)).is_err(), "id repetido");
            AiPolicyData::load_from_bytes(&a.gravar()).unwrap();
        }
    }
}
