//! `tasks.data` editável (B256, Estúdio S7): campos **de tamanho fixo** de uma missão, corrigidos no lugar.
//!
//! O arquivo inteiro fica cru; o leitor do GS (`crate::tasks`) diz onde começam o bloco fixo e o prêmio de cada missão
//! (`TaskTemplate::bloco_fixo_em`/`premio_em`). Trocar um campo de tamanho fixo não muda o tamanho de nada — a tabela de
//! deslocamentos do cabeçalho continua valendo e o resto do arquivo fica byte a byte igual. Campos de tamanho variável
//! (listas de itens, monstros, descrição) ficam para um gravador completo.
//!
//! Deslocamentos: v129 pelas constantes do leitor (`v129::f` e `v129::p`, do `TaskTempl.h` 1.7.2 conferido no arquivo
//! do 1.5.5 BR); v55 pelos do `libtask.so`/`elementclient.exe` 1.2.6 que o leitor usa (spec 03 §3.2). O nome é
//! `wchar[30]` em `+4` nas duas versões, com cada unidade em XOR com `id as u16` (`tasks::texto`).
//!
//! **Textos longos (B262):** descrição, aceitar, recusar e tributo mudam de tamanho. Cada um é `u32` de caracteres
//! (`wcslen`, sem NUL) + as unidades em XOR com `(namechar)m_ID` (`SaveDescriptionBin`/`SaveTributeBin`,
//! `TaskTempl.cpp:4940-5010`; `convert_txt` `:629`). Trocar um reescreve só esse texto e soma a diferença nos
//! deslocamentos da tabela do cabeçalho das missões de topo que começam depois dele; o arquivo é relido.

use crate::tasks::{TaskTemplate, TasksData, TasksError};

/// Em que bloco o campo fica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bloco {
    Fixo,
    Premio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDoCampo {
    U32,
    I32,
    Bool,
    /// O nome: `wchar[30]` em XOR com o id.
    Nome,
}

/// `(nome do campo no TaskTemplate, bloco, tipo, deslocamento)`.
pub type Campo = (&'static str, Bloco, TipoDoCampo, usize);

pub fn campos(versao: u32) -> &'static [Campo] {
    use Bloco::*;
    use TipoDoCampo::*;
    const V129: &[Campo] = &[
        ("name", Fixo, Nome, 4),
        ("limite_de_tempo", Fixo, U32, 73),
        ("frequencia", Fixo, I32, 141),
        ("pode_desistir", Fixo, Bool, 154),
        ("pode_repetir", Fixo, Bool, 155),
        ("entrega_automatica", Fixo, Bool, 226),
        ("npc_que_entrega", Fixo, U32, 238),
        ("npc_que_premia", Fixo, U32, 242),
        ("min_level", Fixo, U32, 333),
        ("max_level", Fixo, U32, 337),
        ("espera", Fixo, U32, 962),
        ("money", Premio, U32, 0),
        ("exp", Premio, U32, 4),
        ("nova_missao", Premio, U32, 13),
        ("sp", Premio, U32, 17),
        ("reputation", Premio, I32, 21),
    ];
    const V55: &[Campo] = &[
        ("name", Fixo, Nome, 4),
        ("limite_de_tempo", Fixo, U32, 0x49),
        ("frequencia", Fixo, I32, 0x66),
        ("pode_desistir", Fixo, Bool, 0x6f),
        ("pode_repetir", Fixo, Bool, 0x70),
        ("entrega_automatica", Fixo, Bool, 0xac),
        ("npc_que_entrega", Fixo, U32, 0xb5),
        ("npc_que_premia", Fixo, U32, 0xb9),
        ("min_level", Fixo, U32, 0xc1),
        ("max_level", Fixo, U32, 0xc5),
        ("espera", Fixo, U32, 0x1e2),
        ("money", Premio, U32, 0),
        ("exp", Premio, U32, 4),
        ("nova_missao", Premio, U32, 8),
        ("sp", Premio, U32, 12),
        ("reputation", Premio, I32, 16),
    ];
    match versao {
        55 => V55,
        129 => V129,
        _ => &[],
    }
}

const BYTES_DO_NOME: usize = 60;

/// Os quatro textos longos, na ordem do arquivo.
pub const TEXTOS: [&str; 4] = ["descricao", "ok", "nao", "tributo"];

/// Posição de cada um dos quatro textos longos: `(onde está o tamanho, quantos caracteres)`.
fn posicoes_dos_textos(bytes: &[u8], t: &TaskTemplate) -> [(usize, usize); 4] {
    let mut o = t.textos_em;
    let mut r = [(0, 0); 4];
    for x in &mut r {
        let n = u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
        *x = (o, n);
        o += 4 + 2 * n;
    }
    r
}

/// Os quatro textos longos de uma missão lida de `bytes` (sem reler o arquivo), decifrados.
pub fn textos_da_missao(bytes: &[u8], t: &TaskTemplate) -> serde_json::Map<String, serde_json::Value> {
    let chave = t.id as u16;
    let mut m = serde_json::Map::new();
    for (nome, (o, n)) in TEXTOS.iter().zip(posicoes_dos_textos(bytes, t)) {
        let u: Vec<u16> = bytes[o + 4..o + 4 + 2 * n].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave).collect();
        m.insert((*nome).into(), String::from_utf16_lossy(&u).into());
    }
    m
}

pub struct TasksEditavel {
    pub bytes: Vec<u8>,
    pub dados: TasksData,
}

impl TasksEditavel {
    pub fn ler(b: &[u8]) -> Result<Self, TasksError> {
        let dados = TasksData::load_from_bytes(b)?;
        Ok(Self { bytes: b.to_vec(), dados })
    }

    pub fn gravar(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn posicao(&self, t: &TaskTemplate, c: &Campo) -> usize {
        match c.1 {
            Bloco::Fixo => t.bloco_fixo_em + c.3,
            Bloco::Premio => t.premio_em + c.3,
        }
    }

    fn posicoes_dos_textos(&self, t: &TaskTemplate) -> [(usize, usize); 4] {
        posicoes_dos_textos(&self.bytes, t)
    }

    /// Os quatro textos longos da missão (`descricao`, `ok`, `nao`, `tributo`), decifrados.
    pub fn textos(&self, id: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        Some(textos_da_missao(&self.bytes, self.dados.tasks.get(&id)?))
    }

    /// Troca um texto longo e corrige a tabela de deslocamentos.
    pub fn definir_texto(&mut self, id: u32, qual: &str, texto: &str) -> Result<(), String> {
        let k = TEXTOS.iter().position(|x| *x == qual).ok_or_else(|| format!("texto inexistente: {qual}"))?;
        if texto.contains('\0') {
            return Err("o texto não pode ter NUL".into());
        }
        let unidades: Vec<u16> = texto.encode_utf16().collect();
        if unidades.len() > 8000 {
            return Err("texto grande demais (guarda do painel: 8000 unidades UTF-16)".into());
        }
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let (o, n) = self.posicoes_dos_textos(t)[k];
        let chave = id as u16;
        let novo: Vec<u8> = unidades.iter().flat_map(|u| (u ^ chave).to_le_bytes()).collect();
        let delta = novo.len() as i64 - 2 * n as i64;
        self.bytes.splice(o + 4..o + 4 + 2 * n, novo);
        self.bytes[o..o + 4].copy_from_slice(&(unidades.len() as u32).to_le_bytes());
        self.corrigir_tabela(o, delta)?;
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Troca um vetor variável (`tasks::vetores`: itens exigidos, entregues, monstros) inteiro: contador no bloco fixo,
    /// registros reescritos e a diferença somada na tabela de deslocamentos (como `definir_texto`). Teto do cliente
    /// recusado; o arquivo é relido.
    pub fn definir_vetor(&mut self, id: u32, nome: &str, valor: &serde_json::Value) -> Result<(), String> {
        let versao = self.dados.version;
        let k = crate::tasks::vetores(versao).iter().position(|v| v.nome == nome).ok_or_else(|| format!("vetor inexistente: {nome}"))?;
        let v = crate::tasks::vetores(versao)[k];
        let lista = valor.as_array().ok_or_else(|| format!("{nome}: lista"))?;
        if lista.len() > v.teto {
            return Err(format!("{nome}: no máximo {} (teto do cliente, TaskInterface.h)", v.teto));
        }
        let mut novo = Vec::new();
        for x in lista {
            let r = if k == 2 {
                let m: crate::tasks::MonstroPedido = serde_json::from_value(x.clone()).map_err(|e| format!("{nome}: {e}"))?;
                if m.monstro == 0 || m.quantidade == 0 && m.item_que_cai == 0 {
                    return Err(format!("{nome}: monstro e quantidade (ou item que cai) obrigatórios"));
                }
                if versao == 55 && (m.dps != 0 || m.dph != 0) {
                    return Err(format!("{nome}: o formato v55 não tem DPS/DPH"));
                }
                crate::tasks::bytes_do_monstro(versao, &m)
            } else {
                let i: crate::tasks::ItemDeMissao = serde_json::from_value(x.clone()).map_err(|e| format!("{nome}: {e}"))?;
                if i.id == 0 || !i.probabilidade.is_finite() {
                    return Err(format!("{nome}: item com id"));
                }
                if versao == 55 && i.validade != 0 {
                    return Err(format!("{nome}: o formato v55 não tem validade"));
                }
                crate::tasks::bytes_do_item(versao, &i)
            };
            debug_assert_eq!(r.len(), v.registro);
            novo.extend(r);
        }
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let em = t.vetores_em[k];
        let contador = t.bloco_fixo_em + v.contador;
        let antigos = u32::from_le_bytes(self.bytes[contador..contador + 4].try_into().unwrap()) as usize;
        let delta = novo.len() as i64 - (antigos * v.registro) as i64;
        self.bytes.splice(em..em + antigos * v.registro, novo);
        self.bytes[contador..contador + 4].copy_from_slice(&(lista.len() as u32).to_le_bytes());
        self.corrigir_tabela(em, delta)?;
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Troca os grupos de itens de prêmio (de sucesso, ou de falha com `falha`): o trecho dos grupos é reescrito, o
    /// contador corrigido no bloco do prêmio e a diferença somada na tabela de deslocamentos. `grupos` é o JSON de
    /// `Vec<GrupoDeItens>`; teto de 12 grupos e 64 itens por grupo. O arquivo é relido.
    pub fn definir_grupos_do_premio(&mut self, id: u32, falha: bool, grupos: &serde_json::Value) -> Result<(), String> {
        use crate::tasks::{bytes_do_item, grupos_do_premio, GrupoDeItens, MAX_AWARD_CANDIDATES, MAX_ITEM_AWARD};
        let versao = self.dados.version;
        let grupos: Vec<GrupoDeItens> = serde_json::from_value(grupos.clone()).map_err(|e| format!("grupos: {e}"))?;
        if grupos.len() > MAX_AWARD_CANDIDATES {
            return Err(format!("no máximo {MAX_AWARD_CANDIDATES} grupos (TaskTempl.h:36)"));
        }
        let mut novo = Vec::new();
        for g in &grupos {
            if g.itens.len() > MAX_ITEM_AWARD {
                return Err(format!("no máximo {MAX_ITEM_AWARD} itens por grupo (TaskInterface.h:142)"));
            }
            novo.push(g.sorteia_um as u8);
            novo.extend_from_slice(&(g.itens.len() as u32).to_le_bytes());
            for i in &g.itens {
                if i.id == 0 || !i.probabilidade.is_finite() {
                    return Err("item de prêmio com id e peso".into());
                }
                if versao == 55 && i.validade != 0 {
                    return Err("o formato v55 não tem validade".into());
                }
                novo.extend(bytes_do_item(versao, i));
            }
        }
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let (cont, bloco) = grupos_do_premio(versao);
        let premio = if falha { t.premio_de_falha_em } else { t.premio_em };
        let contador = premio + cont;
        let antigos = u32::from_le_bytes(self.bytes[contador..contador + 4].try_into().unwrap()) as usize;
        let registro = if versao == 55 { 13 } else { 17 };
        let inicio = premio + bloco;
        let mut fim = inicio;
        for _ in 0..antigos {
            let n = u32::from_le_bytes(self.bytes[fim + 1..fim + 5].try_into().unwrap()) as usize;
            fim += 5 + n * registro;
        }
        let delta = novo.len() as i64 - (fim - inicio) as i64;
        self.bytes.splice(inicio..fim, novo);
        self.bytes[contador..contador + 4].copy_from_slice(&(grupos.len() as u32).to_le_bytes());
        self.corrigir_tabela(inicio, delta)?;
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Clona uma missão de topo sem submissões com o id `novo` (B267): copia os bytes dela para o fim do arquivo, troca
    /// o id no bloco fixo, recifra todo texto que usa o id como chave (nome, assinatura, os quatro textos longos e os
    /// cinco diálogos — os únicos `convert_txt`/`convert_talk_text` de `TaskTempl.cpp`: `:3842`, `:3881`,
    /// `:4739-4751`, `:4827-4895`) com XOR `antigo ^ novo`, zera o parentesco e acrescenta a entrada na tabela do
    /// cabeçalho (todos os deslocamentos andam 4 bytes). O arquivo é relido.
    pub fn clonar_missao(&mut self, origem: u32, novo: u32) -> Result<(), String> {
        let versao = self.dados.version;
        if novo == 0 || novo > MAIOR_ID_DE_MISSAO {
            return Err(format!("id {novo} fora de 1..={MAIOR_ID_DE_MISSAO} (o cliente guarda o id em unsigned short)"));
        }
        if self.dados.tasks.contains_key(&novo) {
            return Err(format!("a missão {novo} já existe"));
        }
        let t = self.dados.tasks.get(&origem).ok_or_else(|| format!("missão {origem} não existe"))?;
        if t.parent.is_some() || !t.sub_tasks.is_empty() {
            return Err("só missão de topo sem submissões pode ser clonada".into());
        }
        let i = self.dados.de_topo.iter().position(|&x| x == origem).ok_or("missão de topo fora da tabela")?;
        let quantas = u32_de(&self.bytes, 8) as usize;
        let inicio = u32_de(&self.bytes, 12 + 4 * i) as usize;
        let fim = if i + 1 < quantas { u32_de(&self.bytes, 12 + 4 * (i + 1)) as usize } else { self.bytes.len() };
        let mut copia = self.bytes[inicio..fim].to_vec();
        let k = (origem as u16) ^ (novo as u16);
        let mut recifrar = |abs: usize, unidades: usize| {
            let o = abs - inicio;
            for c in copia[o..o + 2 * unidades].chunks_exact_mut(2) {
                let u = u16::from_le_bytes([c[0], c[1]]) ^ k;
                c.copy_from_slice(&u.to_le_bytes());
            }
        };
        let (bloco, assinatura) = bloco_e_assinatura(versao);
        let fixo = t.bloco_fixo_em;
        recifrar(fixo + 4, 30);
        if self.bytes[fixo + assinatura] != 0 {
            recifrar(fixo + bloco, 30);
        }
        for (o, n) in posicoes_dos_textos(&self.bytes, t) {
            recifrar(o + 4, n);
        }
        for d in dialogos_da_missao(&self.bytes, t) {
            recifrar(d.texto_em, TEXTO_FIXO);
            for j in &d.janelas {
                recifrar(j.texto_em + 4, u32_de(&self.bytes, j.texto_em) as usize);
                for op in &j.opcoes {
                    recifrar(op.texto_em, TEXTO_FIXO);
                }
            }
        }
        let rel = fixo - inicio;
        copia[rel..rel + 4].copy_from_slice(&novo.to_le_bytes());
        for o in parentesco(versao) {
            copia[rel + o..rel + o + 4].copy_from_slice(&0u32.to_le_bytes());
        }
        // Tabela: mais uma entrada; tudo o que vem depois dela anda 4 bytes.
        for e in 0..quantas {
            let p = 12 + 4 * e;
            let v = u32_de(&self.bytes, p) + 4;
            self.bytes[p..p + 4].copy_from_slice(&v.to_le_bytes());
        }
        let novo_inicio = u32::try_from(self.bytes.len() + 4).map_err(|_| "arquivo grande demais")?;
        self.bytes.splice(12 + 4 * quantas..12 + 4 * quantas, novo_inicio.to_le_bytes());
        self.bytes[8..12].copy_from_slice(&((quantas + 1) as u32).to_le_bytes());
        self.bytes.extend_from_slice(&copia);
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Se `remover_missao(id)` passaria na checagem de citações (sem remover).
    pub fn remover_missao_valida(&self, id: u32) -> bool {
        let Some(t) = self.dados.tasks.get(&id) else { return false };
        if t.parent.is_some() {
            return false;
        }
        let mut arvore = vec![id];
        let mut k = 0;
        while k < arvore.len() {
            arvore.extend(self.dados.tasks[&arvore[k]].sub_tasks.iter().copied());
            k += 1;
        }
        arvore.iter().all(|&x| self.citacoes_da_missao(x).iter().all(|(m, _)| arvore.contains(m)))
    }

    /// As outras missões que citam `id` (B268): pré-missão, exclusiva, missão seguinte do prêmio (sucesso ou falha) e
    /// membro de equipe pedido. Devolve `(missão, campo)`.
    pub fn citacoes_da_missao(&self, id: u32) -> Vec<(u32, &'static str)> {
        let mut r = Vec::new();
        for m in self.dados.tasks.values() {
            if m.id == id {
                continue;
            }
            if m.pre_tasks.contains(&id) { r.push((m.id, "pre_tasks")); }
            if m.missoes_exclusivas.contains(&id) { r.push((m.id, "missoes_exclusivas")); }
            if m.rewards.nova_missao == id { r.push((m.id, "rewards.nova_missao")); }
            if m.premio_de_falha.nova_missao == id { r.push((m.id, "premio_de_falha.nova_missao")); }
            if m.membros_pedidos.iter().any(|x| x.missao == id) { r.push((m.id, "membros_pedidos.missao")); }
        }
        r.sort_unstable();
        r
    }

    /// Remove uma missão de topo e as submissões dela (B268): corta os bytes, tira a entrada da tabela do cabeçalho e
    /// corrige os deslocamentos (−4 de todos, −tamanho dos que vinham depois). Recusa se outra missão a cita (ou cita
    /// uma submissão dela). As citações fora do `tasks.data` (`elements`) são conferidas por quem chama.
    pub fn remover_missao(&mut self, id: u32) -> Result<(), String> {
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        if t.parent.is_some() {
            return Err("só missão de topo pode ser removida (a submissão é parte da árvore da mãe)".into());
        }
        let mut arvore = vec![id];
        let mut k = 0;
        while k < arvore.len() {
            arvore.extend(self.dados.tasks[&arvore[k]].sub_tasks.iter().copied());
            k += 1;
        }
        let de_fora: Vec<String> = arvore
            .iter()
            .flat_map(|&x| self.citacoes_da_missao(x).into_iter().filter(|(m, _)| !arvore.contains(m)).map(move |(m, c)| format!("{m}.{c} → {x}")))
            .collect();
        if !de_fora.is_empty() {
            return Err(format!("citada por outras missões: {}", de_fora.join(", ")));
        }
        let i = self.dados.de_topo.iter().position(|&x| x == id).ok_or("missão de topo fora da tabela")?;
        let quantas = u32_de(&self.bytes, 8) as usize;
        let inicio = u32_de(&self.bytes, 12 + 4 * i) as usize;
        let fim = if i + 1 < quantas { u32_de(&self.bytes, 12 + 4 * (i + 1)) as usize } else { self.bytes.len() };
        let tamanho = fim - inicio;
        self.bytes.drain(inicio..fim);
        for e in 0..quantas {
            let p = 12 + 4 * e;
            let v = u32_de(&self.bytes, p) as usize;
            let novo = if v > inicio { v - tamanho - 4 } else { v - 4 };
            self.bytes[p..p + 4].copy_from_slice(&(novo as u32).to_le_bytes());
        }
        self.bytes.drain(12 + 4 * i..12 + 4 * i + 4);
        self.bytes[8..12].copy_from_slice(&((quantas - 1) as u32).to_le_bytes());
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Os cinco diálogos da missão (B266).
    pub fn dialogos(&self, id: u32) -> Option<Vec<Dialogo>> {
        Some(dialogos_da_missao(&self.bytes, self.dados.tasks.get(&id)?))
    }

    /// Troca um texto de diálogo: o título (`janela = None`, `text[64]`), o texto de uma janela (`opcao = None`,
    /// tamanho variável: `talk_text_len` conta o NUL final, medido em todas as janelas dos dois realms) ou o de uma
    /// opção (`text[64]`). Os fixos levam no máximo 63 caracteres e o resto dos 64 fica zerado (cifrado) — medido: é
    /// como estão ~97% dos textos fixos dos dois arquivos. O arquivo é relido.
    pub fn definir_texto_do_dialogo(&mut self, id: u32, dialogo: usize, janela: Option<usize>, opcao: Option<usize>, texto: &str) -> Result<(), String> {
        if texto.contains('\0') {
            return Err("o texto não pode ter NUL".into());
        }
        let ds = self.dialogos(id).ok_or_else(|| format!("missão {id} não existe"))?;
        let d = ds.get(dialogo).ok_or_else(|| format!("diálogo {dialogo} não existe"))?;
        let chave = id as u16;
        let cifrar = |u: &[u16]| -> Vec<u8> { u.iter().flat_map(|x| (x ^ chave).to_le_bytes()).collect() };
        let mut unidades: Vec<u16> = texto.encode_utf16().collect();
        let fixo = match janela {
            None => Some(d.texto_em),
            Some(j) => {
                let jn = d.janelas.get(j).ok_or_else(|| format!("janela {j} não existe"))?;
                match opcao {
                    Some(o) => Some(jn.opcoes.get(o).ok_or_else(|| format!("opção {o} não existe"))?.texto_em),
                    None => {
                        if unidades.len() > 8000 {
                            return Err("texto grande demais (guarda do painel: 8000 unidades UTF-16)".into());
                        }
                        let o = jn.texto_em;
                        let antigo = u32_de(&self.bytes, o) as usize;
                        unidades.push(0);
                        let novo = cifrar(&unidades);
                        let delta = novo.len() as i64 - 2 * antigo as i64;
                        self.bytes.splice(o + 4..o + 4 + 2 * antigo, novo);
                        self.bytes[o..o + 4].copy_from_slice(&(unidades.len() as u32).to_le_bytes());
                        self.corrigir_tabela(o, delta)?;
                        None
                    }
                }
            }
        };
        if let Some(o) = fixo {
            if unidades.len() >= TEXTO_FIXO {
                return Err(format!("no máximo {} caracteres (text[64] com o NUL)", TEXTO_FIXO - 1));
            }
            unidades.resize(TEXTO_FIXO, 0);
            let b = cifrar(&unidades);
            self.bytes[o..o + 2 * TEXTO_FIXO].copy_from_slice(&b);
            return Ok(());
        }
        self.dados = TasksData::load_from_bytes(&self.bytes).map_err(|e| format!("o tasks.data editado não relê: {e}"))?;
        Ok(())
    }

    /// Soma `delta` nos deslocamentos da tabela do cabeçalho das missões de topo que começam depois de `o`.
    fn corrigir_tabela(&mut self, o: usize, delta: i64) -> Result<(), String> {
        let quantas = u32::from_le_bytes(self.bytes[8..12].try_into().unwrap()) as usize;
        for i in 0..quantas {
            let e = 12 + 4 * i;
            let inicio = u32::from_le_bytes(self.bytes[e..e + 4].try_into().unwrap()) as usize;
            if inicio > o {
                let novo_inicio = u32::try_from(inicio as i64 + delta).map_err(|_| "deslocamento fora de u32")?;
                self.bytes[e..e + 4].copy_from_slice(&novo_inicio.to_le_bytes());
            }
        }
        Ok(())
    }

    /// Troca uma lista de capacidade fixa (`tasks::listas_fixas`: pré-missões, exclusivas, classes): contador e valores
    /// no lugar; as vagas que sobram ficam zeradas (medido: as vagas livres do arquivo original são zero). Pré-missão e
    /// exclusiva têm que ser missões que existem; repetido é recusado.
    pub fn definir_lista(&mut self, id: u32, nome: &str, valores: &[u32]) -> Result<(), String> {
        let l = *crate::tasks::listas_fixas(self.dados.version).iter().find(|l| l.nome == nome).ok_or_else(|| format!("lista inexistente: {nome}"))?;
        if valores.len() > l.capacidade {
            return Err(format!("{nome}: no máximo {} (capacidade do arquivo)", l.capacidade));
        }
        let mut vistos = std::collections::HashSet::new();
        for &v in valores {
            if !vistos.insert(v) {
                return Err(format!("{nome}: {v} repetido"));
            }
            if nome != "req_classes" && !self.dados.tasks.contains_key(&v) {
                return Err(format!("{nome}: a missão {v} não existe"));
            }
            if nome != "req_classes" && v == id {
                return Err(format!("{nome}: a missão não pode citar a si mesma"));
            }
        }
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let base = t.bloco_fixo_em;
        self.bytes[base + l.contador..base + l.contador + 4].copy_from_slice(&(valores.len() as u32).to_le_bytes());
        for i in 0..l.capacidade {
            let v = valores.get(i).copied().unwrap_or(0);
            let o = base + l.inicio + 4 * i;
            self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        let t = self.dados.tasks.get_mut(&id).unwrap();
        match nome {
            "req_classes" => t.req_classes = valores.to_vec(),
            "pre_tasks" => t.pre_tasks = valores.to_vec(),
            _ => t.missoes_exclusivas = valores.to_vec(),
        }
        Ok(())
    }

    /// Os campos editáveis da missão, lidos do arquivo.
    pub fn ler_campos(&self, id: u32) -> Option<serde_json::Map<String, serde_json::Value>> {
        let t = self.dados.tasks.get(&id)?;
        let mut m = serde_json::Map::new();
        for c in campos(self.dados.version) {
            let o = self.posicao(t, c);
            let u = |o: usize| u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap());
            let v = match c.2 {
                TipoDoCampo::U32 => serde_json::json!(u(o)),
                TipoDoCampo::I32 => serde_json::json!(u(o) as i32),
                TipoDoCampo::Bool => serde_json::json!(self.bytes[o] != 0),
                TipoDoCampo::Nome => serde_json::json!(t.name),
            };
            m.insert(c.0.to_string(), v);
        }
        Some(m)
    }

    /// Troca um campo de tamanho fixo no lugar. Recusa campo fora da tabela, valor fora do tipo e nome que não cabe em
    /// `wchar[30]` (com o NUL). Não relê o arquivo: quem chama relê (`TasksEditavel::ler`) para conferir.
    pub fn definir_campo(&mut self, id: u32, nome: &str, valor: &serde_json::Value) -> Result<(), String> {
        let t = self.dados.tasks.get(&id).ok_or_else(|| format!("missão {id} não existe"))?;
        let c = campos(self.dados.version).iter().find(|c| c.0 == nome).ok_or_else(|| format!("campo não editável: {nome}"))?;
        let o = self.posicao(t, c);
        match c.2 {
            TipoDoCampo::U32 => {
                let v = valor.as_i64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| format!("{nome}: inteiro sem sinal"))?;
                self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            TipoDoCampo::I32 => {
                let v = valor.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or_else(|| format!("{nome}: inteiro"))?;
                self.bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            TipoDoCampo::Bool => self.bytes[o] = valor.as_bool().ok_or_else(|| format!("{nome}: verdadeiro ou falso"))? as u8,
            TipoDoCampo::Nome => {
                let s = valor.as_str().ok_or_else(|| format!("{nome}: texto"))?;
                if s == t.name {
                    return Ok(());
                }
                let u: Vec<u16> = s.encode_utf16().collect();
                if u.len() + 1 > BYTES_DO_NOME / 2 || u.contains(&0) {
                    return Err(format!("{nome}: no máximo {} caracteres", BYTES_DO_NOME / 2 - 1));
                }
                let chave = id as u16;
                for k in 0..BYTES_DO_NOME / 2 {
                    let x = u.get(k).copied().unwrap_or(0) ^ chave;
                    self.bytes[o + 2 * k..o + 2 * k + 2].copy_from_slice(&x.to_le_bytes());
                }
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ clonar missão (B267)

/// Deslocamentos, no bloco fixo, de `m_ulParent`, `m_ulPrevSibling`, `m_ulNextSibling`, `m_ulFirstChild`
/// (`TaskTempl.h:2461-2464`). Medidos nos dados (`medir_parentesco`: o único deslocamento que bate em todas as
/// missões): v129 1131/1135/1139/1143 (a mãe confere com `v129::f::PAI`); v55 518/522/526/530 (fim do bloco de 534).
pub fn parentesco(version: u32) -> [usize; 4] {
    if version == 55 { [518, 522, 526, 530] } else { [1131, 1135, 1139, 1143] }
}

/// O tamanho do bloco fixo e onde fica a marca de assinatura (`m_bHasSign`; a assinatura, `wchar[30]`, vem logo
/// depois do bloco — `TaskTempl.cpp:3876-3881`; v55 em `0x40`, como o leitor).
fn bloco_e_assinatura(version: u32) -> (usize, usize) {
    if version == 55 { (534, 0x40) } else { (1157, 64) }
}

/// Maior id de missão que o cliente guarda: `unsigned short` na lista de concluídas e na ativa
/// (`TaskProcess.h:115`, `:273-275`).
pub const MAIOR_ID_DE_MISSAO: u32 = u16::MAX as u32;

// ------------------------------------------------------------------ diálogos (B266)

/// Os cinco diálogos da missão, na ordem de `LoadBinary` (`TaskTempl.cpp:4736-4746`).
pub const DIALOGOS: [&str; 5] = ["entrega", "nao_qualificado", "entrega_de_item", "execucao", "premio"];

/// `talk_proc` (`CCommon/ExpTypes.h`): `id_talk u32`, `text[64]`, `num_window i32` e as janelas — cada uma `id u32`,
/// `id_parent u32`, `talk_text_len i32` + o texto, `num_option i32` e as opções (`id u32`, `text[64]`, `param u32`:
/// 136 bytes). Todo texto em XOR com `(namechar)m_ID`, os fixos nos 64 caracteres inteiros (`convert_talk_text`,
/// `TaskTempl.cpp:637-653`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Dialogo {
    pub id_talk: u32,
    pub texto: String,
    pub janelas: Vec<Janela>,
    #[serde(skip)]
    pub texto_em: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Janela {
    pub id: u32,
    pub pai: u32,
    pub texto: String,
    pub opcoes: Vec<Opcao>,
    /// Onde está `talk_text_len`.
    #[serde(skip)]
    pub texto_em: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Opcao {
    pub id: u32,
    pub texto: String,
    pub param: u32,
    #[serde(skip)]
    pub texto_em: usize,
}

const OPCAO: usize = 136;
const TEXTO_FIXO: usize = 64;

fn decifrar(bytes: &[u8], chave: u16) -> String {
    let u: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave).take_while(|&u| u != 0).collect();
    String::from_utf16_lossy(&u)
}

fn u32_de(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// Os cinco diálogos da missão `t`, lidos de `bytes` (começam depois do tributo).
pub fn dialogos_da_missao(bytes: &[u8], t: &TaskTemplate) -> Vec<Dialogo> {
    let chave = t.id as u16;
    let [.., (o, n)] = posicoes_dos_textos(bytes, t);
    let mut o = o + 4 + 2 * n;
    let mut r = Vec::with_capacity(5);
    for _ in 0..5 {
        let id_talk = u32_de(bytes, o);
        let texto_em = o + 4;
        let texto = decifrar(&bytes[texto_em..texto_em + 2 * TEXTO_FIXO], chave);
        o = texto_em + 2 * TEXTO_FIXO;
        let nj = u32_de(bytes, o) as usize;
        o += 4;
        let mut janelas = Vec::with_capacity(nj);
        for _ in 0..nj {
            let (id, pai) = (u32_de(bytes, o), u32_de(bytes, o + 4));
            let texto_em = o + 8;
            let n = u32_de(bytes, texto_em) as usize;
            let texto = decifrar(&bytes[texto_em + 4..texto_em + 4 + 2 * n], chave);
            o = texto_em + 4 + 2 * n;
            let no = u32_de(bytes, o) as usize;
            o += 4;
            let opcoes = (0..no)
                .map(|k| {
                    let p = o + k * OPCAO;
                    Opcao { id: u32_de(bytes, p), texto: decifrar(&bytes[p + 4..p + 4 + 2 * TEXTO_FIXO], chave), param: u32_de(bytes, p + 4 + 2 * TEXTO_FIXO), texto_em: p + 4 }
                })
                .collect();
            o += no * OPCAO;
            janelas.push(Janela { id, pai, texto, opcoes, texto_em });
        }
        r.push(Dialogo { id_talk, texto, janelas, texto_em });
    }
    r
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Nos dois realms: ler, trocar campos de uma missão (fixo e prêmio, inteiro, bool e nome), reler pelo leitor do
    /// GS (vê os valores novos e nada mais muda), e voltar os valores devolve o arquivo idêntico.
    #[test]
    fn editar_no_lugar_e_voltar_devolve_o_arquivo() {
        for (realm, id) in [("realm_126", 1173u32), ("realm_155", 1173)] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let mut t = TasksEditavel::ler(&b).unwrap();
            assert!(t.gravar() == b);
            let antes = t.ler_campos(id).unwrap_or_else(|| panic!("{realm}: sem a missão {id}"));
            let outra = *t.dados.de_topo.iter().find(|&&x| x != id).unwrap();
            let outra_antes = serde_json::to_value(&t.dados.tasks[&outra]).unwrap();
            for (k, v) in [("min_level", serde_json::json!(7)), ("exp", serde_json::json!(4321)), ("pode_repetir", serde_json::json!(true)), ("name", serde_json::json!("Missão do Estúdio"))] {
                t.definir_campo(id, k, &v).unwrap();
            }
            assert!(t.definir_campo(id, "name", &serde_json::json!("x".repeat(30))).is_err(), "não cabe");
            assert!(t.definir_campo(id, "descricao", &serde_json::json!("x")).is_err(), "tamanho variável não");
            assert_eq!(t.gravar().len(), b.len(), "o tamanho não muda");
            let relido = TasksData::load_from_bytes(&t.gravar()).unwrap();
            let m = &relido.tasks[&id];
            assert_eq!((m.min_level, m.rewards.exp, m.pode_repetir, m.name.as_str()), (7, 4321, true, "Missão do Estúdio"), "{realm}");
            assert_eq!(serde_json::to_value(&relido.tasks[&outra]).unwrap(), outra_antes, "{realm}: a outra missão não muda");
            let mut volta = TasksEditavel::ler(&t.gravar()).unwrap();
            for (k, v) in &antes {
                volta.definir_campo(id, k, v).unwrap();
            }
            // O nome volta pelo texto; os bytes depois do NUL do nome original podem não ser os zeros em XOR que
            // gravamos, então compara-se o arquivo fora do campo do nome.
            let o = t.dados.tasks[&id].bloco_fixo_em + 4;
            let (x, y) = (volta.gravar(), b.clone());
            assert!(x[..o] == y[..o] && x[o + BYTES_DO_NOME..] == y[o + BYTES_DO_NOME..], "{realm}: voltar devolve o resto idêntico");
            assert_eq!(TasksData::load_from_bytes(&x).unwrap().tasks[&id].name, TasksData::load_from_bytes(&y).unwrap().tasks[&id].name);
        }
    }

    /// Medida (B266): o texto das janelas termina em NUL? Os textos fixos de 64 estão preenchidos com zeros cifrados
    /// depois do NUL? E o leitor estruturado fecha onde o leitor do GS diz que a missão termina?
    #[test]
    fn medir_dialogos() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let t = TasksEditavel::ler(&b).unwrap();
            let (mut janelas, mut com_nul, mut vazias, mut fixos, mut fixos_sujos) = (0, 0, 0, 0, 0);
            for m in t.dados.tasks.values() {
                let chave = m.id as u16;
                for d in dialogos_da_missao(&b, m) {
                    let mut fixos_aqui = vec![d.texto_em];
                    for j in &d.janelas {
                        janelas += 1;
                        let n = u32_de(&b, j.texto_em) as usize;
                        if n == 0 { vazias += 1; } else {
                            let ultimo = u16::from_le_bytes([b[j.texto_em + 4 + 2 * n - 2], b[j.texto_em + 4 + 2 * n - 1]]) ^ chave;
                            if ultimo == 0 { com_nul += 1; }
                        }
                        fixos_aqui.extend(j.opcoes.iter().map(|o| o.texto_em));
                    }
                    for f in fixos_aqui {
                        fixos += 1;
                        let u: Vec<u16> = b[f..f + 128].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave).collect();
                        let fim = u.iter().position(|&x| x == 0).unwrap_or(64);
                        if u[fim..].iter().any(|&x| x != 0) { fixos_sujos += 1; }
                    }
                }
            }
            eprintln!("{realm}: janelas {janelas}, com NUL final {com_nul}, vazias {vazias}; textos fixos {fixos}, com lixo depois do NUL {fixos_sujos}");
            // O leitor estruturado fecha: nas missões de topo sem filhas, depois dos diálogos vem só o `i32` de filhas
            // (zero) e a próxima missão de topo.
            let inicios: Vec<usize> = (0..t.dados.de_topo.len()).map(|i| u32_de(&b, 12 + 4 * i) as usize).collect();
            let mut conferidas = 0;
            for (i, id) in t.dados.de_topo.iter().enumerate() {
                let m = &t.dados.tasks[id];
                if !m.sub_tasks.is_empty() { continue; }
                let d = dialogos_da_missao(&b, m);
                let ultimo = d.last().unwrap();
                let fim = ultimo.janelas.last().map(|j| {
                    let n = u32_de(&b, j.texto_em) as usize;
                    let o = j.texto_em + 4 + 2 * n;
                    o + 4 + OPCAO * u32_de(&b, o) as usize
                }).unwrap_or(ultimo.texto_em + 2 * TEXTO_FIXO + 4);
                let proxima = inicios.get(i + 1).copied().unwrap_or(b.len());
                assert_eq!(fim + 4, proxima, "{realm}: missão {id}");
                conferidas += 1;
            }
            assert!(conferidas > 100, "{realm}: {conferidas}");
        }
    }

    /// Nos dois realms: trocar título, texto de janela (maior) e opção de uma missão de topo do meio cujos textos fixos
    /// estão limpos; relido; a última de topo igual; 64 caracteres no fixo recusados; voltar = arquivo idêntico.
    #[test]
    fn textos_dos_dialogos_mudam_e_voltam() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let limpo = |o: usize, chave: u16| {
                let u: Vec<u16> = b[o..o + 128].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) ^ chave).collect();
                let f = u.iter().position(|&x| x == 0).unwrap_or(64);
                u[f..].iter().all(|&x| x == 0)
            };
            let meio = t.dados.de_topo.len() / 2;
            let (id, di) = t.dados.de_topo[meio..].iter().find_map(|&id| {
                let ds = t.dialogos(id).unwrap();
                ds.iter().position(|d| limpo(d.texto_em, id as u16) && d.janelas.first().is_some_and(|j| j.opcoes.first().is_some_and(|o| limpo(o.texto_em, id as u16))))
                    .map(|di| (id, di))
            }).expect("missão com diálogo de janela e opção limpos");
            let antes = t.dialogos(id).unwrap()[di].clone();
            let ultima = *t.dados.de_topo.last().unwrap();
            let ultima_antes = serde_json::to_value(&t.dados.tasks[&ultima]).unwrap();
            t.definir_texto_do_dialogo(id, di, None, None, "Título do Estúdio").unwrap();
            t.definir_texto_do_dialogo(id, di, Some(0), None, &"Fala longa da janela, escrita pelo Estúdio. ".repeat(4)).unwrap();
            t.definir_texto_do_dialogo(id, di, Some(0), Some(0), "Opção nova").unwrap();
            assert!(t.definir_texto_do_dialogo(id, di, Some(0), Some(0), &"x".repeat(64)).is_err());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!(serde_json::to_value(&r.tasks[&ultima]).unwrap(), ultima_antes, "{realm}: a última de topo não muda");
            let d = &t.dialogos(id).unwrap()[di];
            assert_eq!((d.texto.as_str(), d.janelas[0].opcoes[0].texto.as_str()), ("Título do Estúdio", "Opção nova"), "{realm}");
            assert!(d.janelas[0].texto.starts_with("Fala longa"));
            t.definir_texto_do_dialogo(id, di, None, None, &antes.texto).unwrap();
            t.definir_texto_do_dialogo(id, di, Some(0), None, &antes.janelas[0].texto).unwrap();
            t.definir_texto_do_dialogo(id, di, Some(0), Some(0), &antes.janelas[0].opcoes[0].texto).unwrap();
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }

    /// Nos dois realms: clonar uma missão de topo sem submissões (com diálogos) — relida, mesma ficha com outro id e o
    /// mesmo nome, textos e diálogos; as outras missões iguais; id existente, id > 65535 e missão com submissões
    /// recusados.
    #[test]
    fn clonar_missao_de_topo() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let origem = *t.dados.de_topo.iter().find(|&&id| {
                let m = &t.dados.tasks[&id];
                m.sub_tasks.is_empty() && !m.descricao.is_empty() && t.dialogos(id).unwrap().iter().any(|d| !d.janelas.is_empty())
            }).expect("missão de topo sem filhas, com descrição e diálogo");
            let novo = (1..=MAIOR_ID_DE_MISSAO).rev().find(|x| !t.dados.tasks.contains_key(x)).unwrap();
            let com_filhas = *t.dados.de_topo.iter().find(|&&id| !t.dados.tasks[&id].sub_tasks.is_empty()).unwrap();
            assert!(t.clonar_missao(origem, origem).is_err());
            assert!(t.clonar_missao(origem, 70000).is_err());
            assert!(t.clonar_missao(com_filhas, novo).is_err());
            let antes = serde_json::to_value(&t.dados.tasks[&origem]).unwrap();
            let (textos, dialogos) = (t.textos(origem).unwrap(), serde_json::to_value(t.dialogos(origem).unwrap()).unwrap());
            t.clonar_missao(origem, novo).unwrap();
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!((r.tasks.len(), r.de_topo.len(), *r.de_topo.last().unwrap()), (t.dados.tasks.len(), t.dados.de_topo.len(), novo));
            let mut clone = serde_json::to_value(&r.tasks[&novo]).unwrap();
            clone["id"] = serde_json::json!(origem);
            assert_eq!(clone, antes, "{realm}: a ficha do clone é a da origem");
            assert_eq!(t.textos(novo).unwrap(), textos, "{realm}: textos recifrados");
            assert_eq!(serde_json::to_value(t.dialogos(novo).unwrap()).unwrap(), dialogos, "{realm}: diálogos recifrados");
            assert_eq!(serde_json::to_value(&r.tasks[&origem]).unwrap(), antes, "{realm}: a origem não muda");
            let original = TasksData::load_from_bytes(&b).unwrap();
            let outra = original.de_topo[original.de_topo.len() / 3];
            assert_eq!(serde_json::to_value(&r.tasks[&outra]).unwrap(), serde_json::to_value(&original.tasks[&outra]).unwrap());
        }
    }

    /// Nos dois realms: clonar e remover o clone devolve o arquivo idêntico; uma missão citada por outra não se remove;
    /// remover uma de topo do meio (com submissões, não citada) relê com a tabela certa e as outras iguais.
    #[test]
    fn remover_missao_de_topo() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let origem = *t.dados.de_topo.iter().find(|&&id| t.dados.tasks[&id].sub_tasks.is_empty()).unwrap();
            let novo = (1..=MAIOR_ID_DE_MISSAO).rev().find(|x| !t.dados.tasks.contains_key(x)).unwrap();
            t.clonar_missao(origem, novo).unwrap();
            t.remover_missao(novo).unwrap();
            assert!(t.gravar() == b, "{realm}: clonar e remover = arquivo idêntico");
            let citada = *t.dados.de_topo.iter().find(|&&id| !t.citacoes_da_missao(id).is_empty()).expect("missão citada");
            assert!(t.remover_missao(citada).is_err(), "{realm}: citada");
            let sub = *t.dados.tasks.values().find(|m| m.parent.is_some()).map(|m| &m.id).unwrap();
            assert!(t.remover_missao(sub).is_err(), "{realm}: submissão");
            let meio = t.dados.de_topo.len() / 2;
            let alvo = *t.dados.de_topo[meio..].iter().find(|&&id| {
                let m = &t.dados.tasks[&id];
                !m.sub_tasks.is_empty() && t.remover_missao_valida(id)
            }).expect("missão de topo com filhas, não citada");
            let filhas = t.dados.tasks[&alvo].sub_tasks.clone();
            let (n, topo) = (t.dados.tasks.len(), t.dados.de_topo.len());
            let original = TasksData::load_from_bytes(&b).unwrap();
            t.remover_missao(alvo).unwrap();
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!(r.de_topo.len(), topo - 1, "{realm}");
            assert!(r.tasks.len() <= n - 1 - filhas.len(), "{realm}: a mãe e ao menos as filhas diretas saíram");
            assert!(!r.tasks.contains_key(&alvo) && filhas.iter().all(|f| !r.tasks.contains_key(f)));
            for id in [original.de_topo[0], *original.de_topo.last().unwrap()] {
                if id != alvo {
                    assert_eq!(serde_json::to_value(&r.tasks[&id]).unwrap(), serde_json::to_value(&original.tasks[&id]).unwrap(), "{realm}: {id}");
                }
            }
        }
    }

    /// Medida (B267): onde ficam, no bloco fixo, os ids de mãe, irmã anterior, irmã seguinte e primeira filha —
    /// o deslocamento em que o `u32` bate com o esperado em todas as missões. E nas de topo sem filhas, se os quatro
    /// são zero.
    #[test]
    fn medir_parentesco() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let t = TasksEditavel::ler(&b).unwrap();
            let fixo = if t.dados.version == 55 { 534 } else { 1157 };
            let ts = &t.dados.tasks;
            let mut esperados: Vec<(&str, Box<dyn Fn(&TaskTemplate) -> Option<u32>>)> = vec![
                ("mae", Box::new(|m: &TaskTemplate| m.parent)),
                ("primeira_filha", Box::new(|m: &TaskTemplate| m.sub_tasks.first().copied())),
            ];
            esperados.push(("irma_seguinte", Box::new(move |m: &TaskTemplate| {
                let mae = m.parent?;
                let irmas = &ts[&mae].sub_tasks;
                let i = irmas.iter().position(|&x| x == m.id)?;
                Some(irmas.get(i + 1).copied().unwrap_or(0))
            })));
            esperados.push(("irma_anterior", Box::new(move |m: &TaskTemplate| {
                let mae = m.parent?;
                let irmas = &ts[&mae].sub_tasks;
                let i = irmas.iter().position(|&x| x == m.id)?;
                Some(if i == 0 { 0 } else { irmas[i - 1] })
            })));
            for (nome, f) in &esperados {
                let casos: Vec<(&TaskTemplate, u32)> = ts.values().filter_map(|m| f(m).filter(|&x| x != 0).map(|x| (m, x))).collect();
                let achados: Vec<usize> = (0..fixo - 3).step_by(1).filter(|&o| casos.iter().all(|(m, x)| u32_de(&b, m.bloco_fixo_em + o) == *x)).collect();
                eprintln!("{realm}: {nome}: {} casos, deslocamentos {:?}", casos.len(), achados);
            }
        }
    }

    /// Nos dois realms: nenhuma missão passa dos tetos de grupos/itens de prêmio (medido); trocar os grupos do prêmio de
    /// sucesso e do de falha de uma missão com prêmio em itens, relido, a última de topo igual, teto recusado; voltar =
    /// arquivo idêntico.
    #[test]
    fn grupos_do_premio_mudam_e_voltam() {
        use crate::tasks::{MAX_AWARD_CANDIDATES, MAX_ITEM_AWARD};
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let mut acima = (0, 0);
            for m in t.dados.tasks.values() {
                for r in [&m.rewards, &m.premio_de_falha] {
                    if r.grupos_de_itens.len() > MAX_AWARD_CANDIDATES { acima.0 += 1; }
                    if r.grupos_de_itens.iter().any(|g| g.itens.len() > MAX_ITEM_AWARD) { acima.1 += 1; }
                }
            }
            assert_eq!(acima, (0, 0), "{realm}: acima do teto (grupos, itens)");
            let id = *t.dados.tasks.values().filter(|m| m.rewards.grupos_de_itens.iter().any(|g| !g.itens.is_empty()) && m.parent.is_none())
                .map(|m| &m.id).min().expect("missão de topo com prêmio em itens");
            let ultima = *t.dados.de_topo.last().unwrap();
            let ultima_antes = serde_json::to_value(&t.dados.tasks[&ultima]).unwrap();
            let sucesso = serde_json::to_value(&t.dados.tasks[&id].rewards.grupos_de_itens).unwrap();
            let falha = serde_json::to_value(&t.dados.tasks[&id].premio_de_falha.grupos_de_itens).unwrap();
            let item = serde_json::json!({"id": 777, "comum": true, "quantidade": 5, "probabilidade": 0.5, "validade": 0});
            let novos = serde_json::json!([{"sorteia_um": true, "itens": [item, item]}, {"sorteia_um": false, "itens": [item]}]);
            t.definir_grupos_do_premio(id, false, &novos).unwrap();
            t.definir_grupos_do_premio(id, true, &serde_json::json!([{"sorteia_um": false, "itens": [item]}])).unwrap();
            assert!(t.definir_grupos_do_premio(id, false, &serde_json::Value::Array(vec![serde_json::json!({"sorteia_um": false, "itens": []}); 13])).is_err());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            let m = &r.tasks[&id];
            assert_eq!((m.rewards.grupos_de_itens.len(), m.rewards.grupos_de_itens[0].itens.len(), m.premio_de_falha.grupos_de_itens[0].itens[0].id), (2, 2, 777), "{realm}");
            assert_eq!(serde_json::to_value(&r.tasks[&ultima]).unwrap(), ultima_antes, "{realm}: a última de topo não muda");
            t.definir_grupos_do_premio(id, false, &sucesso).unwrap();
            t.definir_grupos_do_premio(id, true, &falha).unwrap();
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }

    /// Nos dois realms: nenhuma missão passa do teto do cliente (medido); os gravadores de item e monstro refazem os
    /// bytes de todos os registros do arquivo; trocar os três vetores de uma missão do meio (mais itens, menos
    /// monstros), relido, a última de topo igual; teto e item sem id recusados; voltar = arquivo idêntico.
    #[test]
    fn vetores_mudam_de_tamanho_e_voltam() {
        use crate::tasks::{bytes_do_item, bytes_do_monstro, vetores};
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let v = t.dados.version;
            let mut acima = [0usize; 3];
            for m in t.dados.tasks.values() {
                let listas = [m.itens_exigidos.len(), m.itens_entregues.len(), m.monster_kills.len()];
                for k in 0..3 {
                    if listas[k] > vetores(v)[k].teto { acima[k] += 1; }
                }
                for (k, itens) in [(0, &m.itens_exigidos), (1, &m.itens_entregues)] {
                    let mut o = m.vetores_em[k];
                    for i in itens.iter() {
                        let r = bytes_do_item(v, i);
                        assert!(b[o..o + r.len()] == r[..], "{realm}: item {} da missão {}", i.id, m.id);
                        o += r.len();
                    }
                }
                let mut o = m.vetores_em[2];
                for x in &m.monster_kills {
                    let r = bytes_do_monstro(v, x);
                    assert!(b[o..o + r.len()] == r[..], "{realm}: monstro {} da missão {}", x.monstro, m.id);
                    o += r.len();
                }
            }
            eprintln!("{realm}: missões acima do teto (itens exigidos, entregues, monstros) = {acima:?}");
            let id = *t.dados.tasks.values().filter(|m| !m.monster_kills.is_empty() && m.parent.is_none())
                .map(|m| &m.id).min().expect("alguma missão de topo com monstros");
            let ultima = *t.dados.de_topo.last().unwrap();
            let ultima_antes = serde_json::to_value(&t.dados.tasks[&ultima]).unwrap();
            let m = t.dados.tasks[&id].clone();
            let antes: Vec<serde_json::Value> = [&m.itens_exigidos, &m.itens_entregues].iter().map(|x| serde_json::to_value(x).unwrap())
                .chain([serde_json::to_value(&m.monster_kills).unwrap()]).collect();
            let item = serde_json::json!({"id": 12345, "comum": true, "quantidade": 2, "probabilidade": 1.0, "validade": 0});
            t.definir_vetor(id, "itens_exigidos", &serde_json::json!([item, item])).unwrap();
            t.definir_vetor(id, "itens_entregues", &serde_json::json!([item])).unwrap();
            t.definir_vetor(id, "monster_kills", &serde_json::json!([])).unwrap();
            assert!(t.definir_vetor(id, "itens_exigidos", &serde_json::Value::Array(vec![item.clone(); 11])).is_err(), "teto");
            assert!(t.definir_vetor(id, "itens_exigidos", &serde_json::json!([{"id": 0, "comum": true, "quantidade": 1, "probabilidade": 1.0, "validade": 0}])).is_err());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!((r.tasks[&id].itens_exigidos.len(), r.tasks[&id].itens_entregues[0].id, r.tasks[&id].monster_kills.len()), (2, 12345, 0), "{realm}");
            assert_eq!(serde_json::to_value(&r.tasks[&ultima]).unwrap(), ultima_antes, "{realm}: a última de topo não muda");
            for (k, nome) in ["itens_exigidos", "itens_entregues", "monster_kills"].iter().enumerate() {
                t.definir_vetor(id, nome, &antes[k]).unwrap();
            }
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }

    /// Nos dois realms: as vagas livres das listas fixas são zero em todas as missões (a convenção que `definir_lista`
    /// segue); trocar pré-missões, exclusivas e classes, reler pelo leitor do GS, voltar = arquivo idêntico.
    #[test]
    fn listas_fixas_no_lugar_e_voltam() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let listas = crate::tasks::listas_fixas(t.dados.version);
            let mut sujas = 0;
            for m in t.dados.tasks.values() {
                for l in listas {
                    let o = m.bloco_fixo_em;
                    let n = u32::from_le_bytes(b[o + l.contador..o + l.contador + 4].try_into().unwrap()) as usize;
                    for i in n.min(l.capacidade)..l.capacidade {
                        if b[o + l.inicio + 4 * i..o + l.inicio + 4 * i + 4] != [0; 4] { sujas += 1; }
                    }
                }
            }
            assert_eq!(sujas, 0, "{realm}: vagas livres não zeradas");
            let id = 1173;
            let outras: Vec<u32> = t.dados.de_topo.iter().copied().filter(|&x| x != id).take(3).collect();
            let antes = (t.dados.tasks[&id].pre_tasks.clone(), t.dados.tasks[&id].missoes_exclusivas.clone(), t.dados.tasks[&id].req_classes.clone());
            t.definir_lista(id, "pre_tasks", &outras[..2]).unwrap();
            t.definir_lista(id, "missoes_exclusivas", &outras[2..]).unwrap();
            t.definir_lista(id, "req_classes", &[0, 3]).unwrap();
            assert!(t.definir_lista(id, "pre_tasks", &[999_999_999]).is_err(), "missão que não existe");
            assert!(t.definir_lista(id, "pre_tasks", &[outras[0], outras[0]]).is_err(), "repetida");
            assert!(t.definir_lista(id, "missoes_exclusivas", &vec![outras[0]; 6]).is_err(), "capacidade");
            assert_eq!(t.gravar().len(), b.len());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!((r.tasks[&id].pre_tasks.as_slice(), r.tasks[&id].missoes_exclusivas.as_slice(), r.tasks[&id].req_classes.as_slice()),
                (&outras[..2], &outras[2..], &[0u32, 3][..]), "{realm}");
            t.definir_lista(id, "pre_tasks", &antes.0).unwrap();
            t.definir_lista(id, "missoes_exclusivas", &antes.1).unwrap();
            t.definir_lista(id, "req_classes", &antes.2).unwrap();
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }

    /// Nos dois realms: trocar a descrição (maior) e o tributo (vazio) de uma missão de topo do meio e de uma
    /// submissão; o leitor do GS relê tudo, as missões de topo seguintes continuam iguais; voltar = arquivo idêntico.
    #[test]
    fn textos_longos_mudam_de_tamanho_e_voltam() {
        for realm in ["realm_126", "realm_155"] {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/{realm}/config/tasks.data"));
            let b = std::fs::read(&p).unwrap();
            let mut t = TasksEditavel::ler(&b).unwrap();
            let topo = t.dados.de_topo[t.dados.de_topo.len() / 2];
            let sub = *t.dados.tasks.values().find(|x| x.parent.is_some() && x.textos_em > 0).map(|x| &x.id).unwrap();
            let ultima = *t.dados.de_topo.last().unwrap();
            let ultima_antes = serde_json::to_value(&t.dados.tasks[&ultima]).unwrap();
            let antes_topo = t.textos(topo).unwrap();
            let antes_sub = t.textos(sub).unwrap();
            assert_eq!(antes_topo["descricao"].as_str().unwrap(), t.dados.tasks[&topo].descricao, "{realm}: decifra igual ao leitor");
            let longa = "Descrição nova do Estúdio — bem mais comprida do que costuma ser, para mudar o tamanho.".repeat(3);
            t.definir_texto(topo, "descricao", &longa).unwrap();
            t.definir_texto(topo, "tributo", "").unwrap();
            t.definir_texto(sub, "ok", "Aceito!").unwrap();
            assert!(t.definir_texto(topo, "titulo", "x").is_err());
            let r = TasksData::load_from_bytes(&t.gravar()).unwrap();
            assert_eq!(r.tasks[&topo].descricao, longa, "{realm}");
            assert_eq!(t.textos(sub).unwrap()["ok"], "Aceito!");
            assert_eq!(serde_json::to_value(&r.tasks[&ultima]).unwrap(), ultima_antes, "{realm}: a última de topo não muda");
            for (k, v) in &antes_topo { t.definir_texto(topo, k, v.as_str().unwrap()).unwrap(); }
            for (k, v) in &antes_sub { t.definir_texto(sub, k, v.as_str().unwrap()).unwrap(); }
            assert!(t.gravar() == b, "{realm}: voltar devolve o arquivo idêntico");
        }
    }
}
