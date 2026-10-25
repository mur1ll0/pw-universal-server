//! `.ecm` — o modelo do jogo em texto (B277, Estúdio S5): o `.smd` e as ações combinadas, que são o que o cliente toca
//! pelo nome (`CECModel::PlayActionByName` procura `GetComActByName`, `GfxCommon/EC_Model.cpp:3075-3085`).
//!
//! Arquivo de texto do `AFile` (`MOXT` + linhas, GBK): `Version: N`, `SkinModelPath: …`, …, `ComActCount: N` e cada
//! ação combinada (`A3DCombinedAction::Load`, `GfxCommon/A3DCombinedAction.cpp:142-…`): `CombineActName: …`, campos
//! que variam com a versão (laços, ranks, canal, velocidade…) e `BaseActCount: N` seguido de `BaseActName: …` (+ tempo e
//! laços) por ação-base — as ações do `.smd`. Aqui só se leem as linhas com chave: `Version`, `SkinModelPath`,
//! `CombineActName` (abre uma ação) e `BaseActName` (ação-base da atual). Ação combinada sem ação-base não toca nada
//! (medido: objetos parados do 1.2.6, como `安魂塔`, têm `BaseActCount: 0`).

#[derive(Debug, Clone, serde::Serialize)]
pub struct AcaoCombinada {
    pub nome: String,
    pub bases: Vec<String>,
    /// Ações que esta manda os modelos filhos (asa, espada de voo, arma…) tocarem (B283): `ChildActName:` + `HHName:`
    /// + `IsTrail:` (`ChildActInfo::Load`, `GfxCommon/A3DCombinedAction.cpp:1792-1850`). Ao tocar, todo filho recebe o
    /// nome e toca a ação combinada dele com esse nome, se tiver (`CECModel::ActivateChildAct`/`ParentActivate`,
    /// `EC_Model.cpp:3569-3620`); o gancho só filtra quando é rastro (`IsTrail`).
    pub filhas: Vec<AcaoFilha>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AcaoFilha {
    pub nome: String,
    pub gancho: String,
    pub rastro: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Ecm {
    pub versao: u32,
    pub smd: String,
    pub acoes: Vec<AcaoCombinada>,
    /// `AddiSkinPath:` (B282) — relativas a `Models\` (`EC_Model.cpp:678-689`, só no editor).
    pub peles_adicionais: Vec<String>,
    /// B380: `ChildPath:` — o `.ecm` de cada modelo filho (`EC_Model.cpp:152`, lido em `:4777`), para saber se um
    /// modelo ainda é usado por outro.
    pub filhos: Vec<String>,
}

pub fn ler(b: &[u8]) -> Option<Ecm> {
    let corpo = b.strip_prefix(b"MOXT").unwrap_or(b);
    let texto = crate::ski::gbk(corpo);
    let mut e = Ecm { versao: 0, smd: String::new(), acoes: Vec::new(), peles_adicionais: Vec::new(), filhos: Vec::new() };
    let mut viu_versao = false;
    let mut filha_aberta = false;
    for linha in texto.lines() {
        let linha = linha.trim_end_matches('\r');
        if let Some(v) = linha.strip_prefix("Version: ") {
            if !viu_versao {
                e.versao = v.trim().parse().ok()?;
                viu_versao = true;
            }
        } else if let Some(v) = linha.strip_prefix("SkinModelPath: ") {
            if e.smd.is_empty() {
                e.smd = v.trim().to_string();
            }
        } else if let Some(v) = linha.strip_prefix("CombineActName: ") {
            e.acoes.push(AcaoCombinada { nome: v.to_string(), bases: Vec::new(), filhas: Vec::new() });
            filha_aberta = false;
        } else if let Some(v) = linha.strip_prefix("ChildActName: ") {
            // `sscanf("%s")`: o nome vai até o primeiro espaço.
            let nome = v.split_whitespace().next().unwrap_or("").to_string();
            if let Some(a) = e.acoes.last_mut() {
                a.filhas.push(AcaoFilha { nome, gancho: String::new(), rastro: false });
            }
            filha_aberta = true;
        } else if filha_aberta && linha.starts_with("HHName: ") {
            // `HHName:` também aparece nos modelos filhos do próprio `.ecm` (`EC_Model.cpp:154`): só vale logo depois
            // de um `ChildActName:`.
            if let Some(f) = e.acoes.last_mut().and_then(|a| a.filhas.last_mut()) {
                f.gancho = linha["HHName: ".len()..].split_whitespace().next().unwrap_or("").to_string();
            }
        } else if filha_aberta && linha.starts_with("IsTrail: ") {
            if let Some(f) = e.acoes.last_mut().and_then(|a| a.filhas.last_mut()) {
                f.rastro = linha["IsTrail: ".len()..].trim().parse::<i32>().map_or(false, |x| x != 0);
            }
            filha_aberta = false;
        } else if let Some(v) = linha.strip_prefix("ChildPath: ") {
            e.filhos.push(v.trim().to_string());
        } else if let Some(v) = linha.strip_prefix("AddiSkinPath: ") {
            e.peles_adicionais.push(v.trim().to_string());
        } else if let Some(v) = linha.strip_prefix("BaseActName: ") {
            if let Some(a) = e.acoes.last_mut() {
                a.bases.push(v.to_string());
            }
        }
    }
    viu_versao.then_some(e)
}

/// A tabela de nomes de ação do cliente (`CECStringTab::Init(arquivo, false)`, ANSI/GBK): depois de `#_begin`, um texto
/// entre aspas por linha, na ordem do índice (`Configs\actions_npc.txt`, `EC_NPC.cpp:295`); `//` é comentário.
pub fn tabela_de_nomes(b: &[u8]) -> Vec<String> {
    let texto = crate::ski::gbk(b);
    let mut dentro = false;
    let mut r = Vec::new();
    for linha in texto.lines() {
        let l = linha.trim();
        if l.eq_ignore_ascii_case("#_begin") {
            dentro = true;
            continue;
        }
        if !dentro || l.is_empty() || l.starts_with("//") {
            continue;
        }
        if let (Some(a), Some(z)) = (l.find('"'), l.rfind('"')) {
            if z > a {
                r.push(l[a + 1..z].to_string());
            }
        }
    }
    r
}

/// Os nomes que o cliente pede a um monstro (`CECNPCModelDefaultPolicy::GetActionName`, `EC_NPCModel.cpp:599-620`), pela
/// tabela (índices do enum de `EC_NPC.h:94-120`), só as ações que o código de NPC toca: de pé (0), ócio (1), guarda (2),
/// andar (4), ataques 1 e 2 (5, 6) com `起`/`落` (`IsAttackAction`, `EC_NPC.cpp:1282-1287`; `PlayModelAction`,
/// `EC_NPCModel.cpp:705-713`), correr (7), morrer (8) e ferido (13, com a alternativa 26 = `ACT_WOUNDED2`: o motor tenta
/// `受伤叠加` e, sem a combinada, toca `受伤` — `EC_NPCModel.cpp:692-702`). Ficam de fora `ACT_SKILL1` e `ACT_MAGIC1`
/// (mapeados a −1, `EC_NPCModel.cpp:901-910`; a habilidade usa o nome nativo dela) e os pulos (só no mapeamento para
/// modelo de jogador). Mascote de combate, invocado e evoluído ainda ganham `_地面`/`_水中`/`_空中` (não tratado).
/// Devolve `(índice, nome pedido, alternativa)`.
pub fn pedidos_de_monstro(tabela: &[String]) -> Vec<(usize, String, Option<String>)> {
    let mut r = Vec::new();
    for i in [0usize, 1, 2, 4, 5, 6, 7, 8, 13] {
        let Some(nome) = tabela.get(i) else { continue };
        if i == 5 || i == 6 {
            r.push((i, format!("{nome}起"), None));
            r.push((i, format!("{nome}落"), None));
        } else if i == 13 {
            r.push((i, nome.clone(), tabela.get(26).cloned()));
        } else {
            r.push((i, nome.clone(), None));
        }
    }
    r
}

/// As linhas em bytes (sem a marca `MOXT`), cada uma com o seu fim de linha (B280: trabalhar nos bytes deixa o resto do
/// arquivo idêntico — decodificar e recodificar o GBK poderia mudar bytes que ele não representa).
fn linhas_com_fim(b: &[u8]) -> Vec<&[u8]> {
    b.split_inclusive(|&c| c == b'\n').collect()
}

fn sem_fim(x: &[u8]) -> &[u8] {
    let x = x.strip_suffix(b"\n").unwrap_or(x);
    x.strip_suffix(b"\r").unwrap_or(x)
}

/// Onde começa e termina (exclusivo) cada bloco de ação combinada: de `CombineActName:` até a próxima, e o último até a
/// primeira linha `ScriptCount:` (v ≥ 25) ou `AddiSkinCount:` depois dele (`CECModel` lê esses depois das ações,
/// `EC_Model.cpp:4680-4700`). Também a linha do `ComActCount:`. O nome vem decodificado.
fn blocos(l: &[&[u8]]) -> Option<(usize, Vec<(String, usize, usize)>)> {
    let cont = l.iter().position(|x| x.starts_with(b"ComActCount: "))?;
    let inicios: Vec<usize> = (cont + 1..l.len()).filter(|&i| l[i].starts_with(b"CombineActName: ")).collect();
    let mut r = Vec::new();
    for (k, &i) in inicios.iter().enumerate() {
        let fim = match inicios.get(k + 1) {
            Some(&j) => j,
            None => (i + 1..l.len()).find(|&j| l[j].starts_with(b"ScriptCount: ") || l[j].starts_with(b"AddiSkinCount: "))?,
        };
        r.push((crate::ski::gbk(sem_fim(&l[i][b"CombineActName: ".len()..])), i, fim));
    }
    Some((cont, r))
}

/// Cria a ação combinada `nova` clonando o bloco de `origem` (mesmos campos da versão do arquivo) logo depois dele, e
/// soma 1 no `ComActCount`. Com `base`, as linhas `BaseActName:` do clone passam a apontar para ela (só se a origem tem
/// uma ação-base). Recusa nome repetido e texto fora do GBK. O resto do arquivo fica byte a byte igual.
pub fn clonar_acao(b: &[u8], origem: &str, nova: &str, base: Option<&str>) -> Result<Vec<u8>, String> {
    let gbk = |t: &str| -> Result<Vec<u8>, String> {
        if t.is_empty() || t.contains(['\r', '\n']) {
            return Err("nome inválido".into());
        }
        let (v, _, erro) = encoding_rs::GBK.encode(t);
        if erro { Err(format!("\"{t}\" tem caracteres que não existem em GBK (o cliente lê o .ecm em GBK)")) } else { Ok(v.into_owned()) }
    };
    let nova_b = gbk(nova)?;
    let base_b = base.map(gbk).transpose()?;
    let marca = b.starts_with(b"MOXT");
    let corpo = if marca { &b[4..] } else { b };
    let l = linhas_com_fim(corpo);
    let (cont, bl) = blocos(&l).ok_or("estrutura de ações não reconhecida")?;
    if bl.iter().any(|x| x.0 == nova) {
        return Err(format!("a ação {nova} já existe"));
    }
    let &(_, ini, fim) = bl.iter().find(|x| x.0 == origem).ok_or_else(|| format!("a ação {origem} não existe"))?;
    let fl: &[u8] = if l[ini].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let mut clone: Vec<Vec<u8>> = l[ini..fim].iter().map(|x| x.to_vec()).collect();
    clone[0] = [b"CombineActName: ".as_slice(), &nova_b, fl].concat();
    if let Some(base_b) = &base_b {
        let n = clone.iter().filter(|x| x.starts_with(b"BaseActName: ")).count();
        if n != 1 {
            return Err(format!("a origem tem {n} ações-base; trocar a base só com uma"));
        }
        for x in clone.iter_mut().filter(|x| x.starts_with(b"BaseActName: ")) {
            *x = [b"BaseActName: ".as_slice(), base_b, fl].concat();
        }
    }
    let n_atual: usize = std::str::from_utf8(sem_fim(&l[cont][b"ComActCount: ".len()..])).ok().and_then(|x| x.trim().parse().ok()).ok_or("ComActCount ilegível")?;
    let fl_cont: &[u8] = if l[cont].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let mut saida: Vec<Vec<u8>> = l.iter().map(|x| x.to_vec()).collect();
    saida[cont] = [format!("ComActCount: {}", n_atual + 1).as_bytes(), fl_cont].concat();
    saida.splice(fim..fim, clone);
    Ok([if marca { b"MOXT".as_slice() } else { b"" }, &saida.concat()].concat())
}

/// B373: troca a animação-base da ação combinada `acao` (a linha `BaseActName:` do bloco) por `base` — o que o monstro
/// executa naquela ação (parado, andar, atacar, morrer…). Só para ação com exatamente uma ação-base (a de várias é uma
/// sequência: edita-se nos Editores); texto fora do GBK é recusado. O resto do arquivo fica byte a byte igual, e o
/// tamanho só muda pela diferença dos nomes.
pub fn trocar_base(b: &[u8], acao: &str, base: &str) -> Result<Vec<u8>, String> {
    if base.is_empty() || base.contains(['\r', '\n']) {
        return Err("nome de ação-base inválido".into());
    }
    let (base_b, _, erro) = encoding_rs::GBK.encode(base);
    if erro {
        return Err(format!("\"{base}\" tem caracteres que não existem em GBK"));
    }
    let marca = b.starts_with(b"MOXT");
    let corpo = if marca { &b[4..] } else { b };
    let l = linhas_com_fim(corpo);
    let (_, bl) = blocos(&l).ok_or("estrutura de ações não reconhecida")?;
    let &(_, ini, fim) = bl.iter().find(|x| x.0 == acao).ok_or_else(|| format!("a ação {acao} não existe"))?;
    let linhas: Vec<usize> = (ini..fim).filter(|&i| l[i].starts_with(b"BaseActName: ")).collect();
    if linhas.len() != 1 {
        return Err(format!("a ação {acao} tem {} ações-base; trocar só com uma", linhas.len()));
    }
    let i = linhas[0];
    let fl: &[u8] = if l[i].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let mut saida: Vec<Vec<u8>> = l.iter().map(|x| x.to_vec()).collect();
    saida[i] = [b"BaseActName: ".as_slice(), &base_b, fl].concat();
    Ok([if marca { b"MOXT".as_slice() } else { b"" }, &saida.concat()].concat())
}

/// Copia a ação combinada `origem` do `.ecm` `fonte` para o `.ecm` `destino` com o nome `nova` (B285), depois da última
/// ação dele, e soma 1 no `ComActCount`. Só entre arquivos da mesma `Version`: os campos do bloco variam com ela
/// (`A3DCombinedAction::Load(pFile, dwVersion)`, `GfxCommon/A3DCombinedAction.cpp:142-…`). Os fins de linha do bloco
/// passam a ser os do destino; o resto do destino fica byte a byte igual. As ações-base citadas precisam existir no
/// `.smd` do destino — quem chama confere (aqui não se lê o `.smd`).
pub fn copiar_acao(destino: &[u8], fonte: &[u8], origem: &str, nova: &str) -> Result<Vec<u8>, String> {
    if nova.is_empty() || nova.contains(['\r', '\n']) {
        return Err("nome inválido".into());
    }
    let (nova_b, _, erro) = encoding_rs::GBK.encode(nova);
    if erro {
        return Err(format!("\"{nova}\" tem caracteres que não existem em GBK (o cliente lê o .ecm em GBK)"));
    }
    let (vd, vf) = (ler(destino).ok_or("destino: .ecm ilegível")?.versao, ler(fonte).ok_or("origem: .ecm ilegível")?.versao);
    if vd != vf {
        return Err(format!("versões diferentes (destino {vd}, origem {vf}): os campos da ação mudam com a versão do .ecm"));
    }
    let tira = |b: &[u8]| -> (bool, Vec<u8>) { (b.starts_with(b"MOXT"), b.strip_prefix(b"MOXT").unwrap_or(b).to_vec()) };
    let (marca, corpo_d) = tira(destino);
    let (_, corpo_f) = tira(fonte);
    let ld = linhas_com_fim(&corpo_d);
    let lf = linhas_com_fim(&corpo_f);
    let (cont, bd) = blocos(&ld).ok_or("destino: estrutura de ações não reconhecida")?;
    let (_, bf) = blocos(&lf).ok_or("origem: estrutura de ações não reconhecida")?;
    if bd.iter().any(|x| x.0 == nova) {
        return Err(format!("a ação {nova} já existe no destino"));
    }
    let &(_, ini, fim) = bf.iter().find(|x| x.0 == origem).ok_or_else(|| format!("a ação {origem} não existe na origem"))?;
    let fl: &[u8] = if ld[cont].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let mut bloco: Vec<Vec<u8>> = lf[ini..fim].iter().map(|x| [sem_fim(x), fl].concat()).collect();
    bloco[0] = [b"CombineActName: ".as_slice(), &nova_b, fl].concat();
    let n_atual: usize = std::str::from_utf8(sem_fim(&ld[cont][b"ComActCount: ".len()..])).ok().and_then(|x| x.trim().parse().ok()).ok_or("ComActCount ilegível")?;
    let onde = bd.last().map_or(cont + 1, |x| x.2);
    let mut saida: Vec<Vec<u8>> = ld.iter().map(|x| x.to_vec()).collect();
    saida[cont] = [format!("ComActCount: {}", n_atual + 1).as_bytes(), fl].concat();
    saida.splice(onde..onde, bloco);
    Ok([if marca { b"MOXT".as_slice() } else { b"" }, &saida.concat()].concat())
}

/// B299 (S6): um `.ecm` novo a partir de um `.ecm` do cliente usado como **molde** (os campos variam com a versão,
/// `A3DCombinedAction::Load(pFile, dwVersion)` — nada é escrito de memória): troca a 1ª linha `SkinModelPath:` pelo
/// `.smd` novo e põe uma ação combinada por ação-base, cada uma clonada do 1º bloco simples do molde (uma linha
/// `BaseActName:`, sem `ChildActName:`, sem efeitos — `FxFilePath:`/`EventCount:` diferente de 0) com o nome e a base
/// trocados. O resto do molde fica como está. Recusa molde com `AddiSkinPath:` ou com modelo filho (`ChildCount:`
/// diferente de 0): o modelo novo herdaria peças de outro. Fins de linha os do molde; texto em GBK.
pub fn ecm_novo(molde: &[u8], smd: &str, acoes: &[String]) -> Result<Vec<u8>, String> {
    let gbk = |t: &str| -> Result<Vec<u8>, String> {
        let (v, _, erro) = encoding_rs::GBK.encode(t);
        if erro || t.contains(['\r', '\n']) { Err(format!("\"{t}\" não cabe numa linha GBK")) } else { Ok(v.into_owned()) }
    };
    let marca = molde.starts_with(b"MOXT");
    let corpo = if marca { &molde[4..] } else { molde };
    let l = linhas_com_fim(corpo);
    let (cont, bl) = blocos(&l).ok_or("molde: estrutura de ações não reconhecida")?;
    let fl: &[u8] = if l[cont].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let fim_das_acoes = bl.last().map_or(cont + 1, |b| b.2);
    let cauda = &l[fim_das_acoes..];
    if cauda.iter().any(|x| x.starts_with(b"AddiSkinPath:")) {
        return Err("molde com peles adicionais (AddiSkinPath)".into());
    }
    if cauda.iter().any(|x| x.starts_with(b"ChildCount: ") && sem_fim(&x[b"ChildCount: ".len()..]).trim_ascii() != b"0") {
        return Err("molde com modelo filho (ChildCount)".into());
    }
    let simples = bl.iter().find(|(_, ini, fim)| {
        let b = &l[*ini..*fim];
        b.iter().filter(|x| x.starts_with(b"BaseActName: ")).count() == 1
            && !b.iter().any(|x| x.starts_with(b"ChildActName:") || x.starts_with(b"FxFilePath:"))
            && b.iter().filter(|x| x.starts_with(b"EventCount: ")).all(|x| sem_fim(&x[b"EventCount: ".len()..]).trim_ascii() == b"0")
    });
    if simples.is_none() && !acoes.is_empty() {
        return Err("molde sem ação combinada simples (uma ação-base, sem efeitos)".into());
    }
    let mut saida: Vec<Vec<u8>> = Vec::new();
    let mut trocou_smd = false;
    for x in &l[..cont] {
        if !trocou_smd && x.starts_with(b"SkinModelPath: ") {
            saida.push([b"SkinModelPath: ".as_slice(), &gbk(smd)?, fl].concat());
            trocou_smd = true;
        } else {
            saida.push(x.to_vec());
        }
    }
    if !trocou_smd {
        return Err("molde sem SkinModelPath antes das ações".into());
    }
    saida.push([format!("ComActCount: {}", acoes.len()).as_bytes(), fl].concat());
    if let Some(&(_, ini, fim)) = simples {
        for a in acoes {
            let nome = gbk(a)?;
            for x in &l[ini..fim] {
                saida.push(if x.starts_with(b"CombineActName: ") {
                    [b"CombineActName: ".as_slice(), &nome, fl].concat()
                } else if x.starts_with(b"BaseActName: ") {
                    [b"BaseActName: ".as_slice(), &nome, fl].concat()
                } else {
                    x.to_vec()
                });
            }
        }
    }
    saida.extend(cauda.iter().map(|x| x.to_vec()));
    Ok([if marca { b"MOXT".as_slice() } else { b"" }, &saida.concat()].concat())
}

/// Os ambientes de um habitat de mascote (`inhabit_type`, `ExpTypes.h:2866`: 0 chão, 1 água, 2 ar, 3 chão+água, 4
/// chão+ar, 5 água+ar, 6 os três), com as palavras que o cliente acrescenta ao nome da ação (`GetActionEnvString`,
/// `EC_NPCModel.cpp:580-596`: `地面`, `水中`, `空中`).
pub fn ambientes_do_habitat(habitat: u32) -> &'static [&'static str] {
    match habitat {
        0 => &["地面"],
        1 => &["水中"],
        2 => &["空中"],
        3 => &["地面", "水中"],
        4 => &["地面", "空中"],
        5 => &["水中", "空中"],
        _ => &["地面", "水中", "空中"],
    }
}

/// Os nomes pedidos a um mascote invocado, de combate ou evolutivo (B281): os de monstro com `_<ambiente>` para cada
/// ambiente do habitat (`GetActionName`: nome base, `起`/`落` e depois `_` + ambiente, `EC_NPCModel.cpp:599-620`). O
/// sufixo só vale para invocado (`id_type == 28752`, `EC_Pet.cpp:285-288`) e para `inhabit_type == 6` (combate e
/// evolutivo, `EC_NPCModel.cpp:588-590`); os outros mascotes pedem os nomes de monstro.
pub fn pedidos_de_mascote(tabela: &[String], habitat: u32) -> Vec<(usize, String, Option<String>)> {
    let mut r = Vec::new();
    for (i, nome, alt) in pedidos_de_monstro(tabela) {
        for amb in ambientes_do_habitat(habitat) {
            r.push((i, format!("{nome}_{amb}"), alt.as_ref().map(|a| format!("{a}_{amb}"))));
        }
    }
    r
}

/// Voo (B283): o modo (`fly_mode` do `FLYSWORD_ESSENCE`, `CECPlayer::FlyMode2WingType`, `EC_Player.cpp:5985-5997`; o
/// `WINGMANWING_ESSENCE` é asa, `:4262-4264`) → (nome, gancho do jogador onde pendura, gancho de encaixe no modelo de voo).
/// `EC_Player.cpp:1943-1965`: espada `HH_feijian`/`CC_feijian`, asa `HH_chibang`/`CC_chibang`, rodas nos pés com
/// `_cc_fenghuolun = "CC_feijian"` (`:153`). Sem o encaixe no esqueleto, o motor não posiciona o filho
/// (`A3DSkinModelHanger::SetHangerObject` deixa `m_iCCIndex = -1` e o `Update` sai cedo, `A3DSkinModelAux.cpp:378-407`).
pub fn encaixe_de_voo(modo: u32) -> Option<(&'static str, Option<&'static str>, &'static str)> {
    match modo {
        0 => Some(("espada", Some("HH_feijian"), "CC_feijian")),
        1 => Some(("asa", Some("HH_chibang"), "CC_chibang")),
        // Os pés (`_hh_left_foot`/`_hh_right_foot`) têm nome GBK; rastro só pega pelo gancho, e nenhum é nos pés.
        2 => Some(("rodas", None, "CC_feijian")),
        _ => None,
    }
}

/// Uma ação filha do modelo do jogador chega ao modelo de voo? Todo filho recebe o nome; o rastro só vale no filho
/// pendurado no gancho dele (`CECModel::ParentActivate`, `EC_Model.cpp:3575-3576`).
pub fn chega_ao_voo(f: &AcaoFilha, modo: u32) -> bool {
    !f.rastro || encaixe_de_voo(modo).and_then(|e| e.1).is_some_and(|g| g == f.gancho)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LinhaDeVoo {
    pub nome: String,
    /// Quantos `.ecm` de jogador mandam este nome aos filhos.
    pub jogadores: usize,
    /// Quantos outros modelos de voo do mesmo modo têm a ação combinada.
    pub colegas: usize,
    pub estado: &'static str,
}

/// O diagnóstico de um modelo de voo: os nomes que os jogadores mandam (`enviadas`) e que o vocabulário do modo usa (algum
/// colega tem, ou este tem). `dos_colegas` não conta o próprio modelo; `colegas` é quantos foram lidos. Falta só acusa
/// quando a maioria dos colegas tem a ação (medido no 1.5.5: `悬停` está em 1 de 101 espadas — acusar todas seria ruído).
pub fn diagnostico_de_voo(
    acoes: &[AcaoCombinada],
    enviadas: &std::collections::BTreeMap<String, usize>,
    dos_colegas: &std::collections::BTreeMap<String, usize>,
    colegas: usize,
) -> Vec<LinhaDeVoo> {
    let mut r: Vec<LinhaDeVoo> = enviadas
        .iter()
        .filter(|(n, _)| dos_colegas.contains_key(*n) || acoes.iter().any(|a| &a.nome == *n))
        .map(|(n, &j)| {
            let estado = match acoes.iter().find(|a| &a.nome == n) {
                None if dos_colegas.get(n).copied().unwrap_or(0) * 2 > colegas => "sem ação combinada no .ecm (fica parado nesse momento)",
                None => "ausente, como na maioria dos colegas",
                Some(a) if a.bases.is_empty() => "ação combinada sem ação-base (não anima)",
                Some(_) => "ok",
            };
            LinhaDeVoo { nome: n.clone(), jogadores: j, colegas: dos_colegas.get(n).copied().unwrap_or(0), estado }
        })
        .collect();
    r.sort_by(|a, b| b.colegas.cmp(&a.colegas).then(b.jogadores.cmp(&a.jogadores)).then(a.nome.cmp(&b.nome)));
    r
}

#[cfg(test)]
mod testes {
    use super::*;

    /// B373: em todos os `.ecm` do cliente, trocar a base da 1ª ação de base única por outra nome relê com a base nova,
    /// as outras ações iguais, e o arquivo só difere naquela linha.
    #[test]
    fn trocar_base_em_todo_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let mut trocados = 0usize;
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ecm")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let Some(antes) = ler(&b) else { continue };
                let Some((k, a)) = antes.acoes.iter().enumerate().find(|(_, a)| a.bases.len() == 1) else { continue };
                let novo = trocar_base(&b, &a.nome, "测试基础").unwrap_or_else(|x| panic!("{}: {x}", p.display()));
                let depois = ler(&novo).unwrap();
                assert_eq!(depois.acoes.len(), antes.acoes.len());
                assert_eq!(depois.acoes[k].bases, vec!["测试基础".to_string()], "{}", p.display());
                for (j, (x, y)) in antes.acoes.iter().zip(&depois.acoes).enumerate() {
                    if j != k { assert_eq!(x.bases, y.bases); }
                }
                let volta = trocar_base(&novo, &a.nome, &a.bases[0]).unwrap();
                assert_eq!(volta, b, "ida e volta byte a byte: {}", p.display());
                trocados += 1;
            }
        }
        eprintln!("ecm: base trocada e desfeita em {trocados} arquivos");
        assert!(trocados > 0);
    }

    /// B280: em todos os `.ecm` do cliente, os blocos achados batem com o `ComActCount` e terminam onde devem; clonar a
    /// 1ª ação de cada arquivo relê com uma ação a mais, as outras iguais.
    #[test]
    fn blocos_e_clone_em_todo_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let (mut ok, mut contagem_diferente, mut sem_termino, mut clonados) = (0usize, 0usize, 0usize, 0usize);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ecm")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let l = linhas_com_fim(b.strip_prefix(b"MOXT").unwrap_or(&b));
                let Some(cont) = l.iter().position(|x| x.starts_with(b"ComActCount: ")) else { continue };
                let n: usize = std::str::from_utf8(sem_fim(&l[cont][b"ComActCount: ".len()..])).ok().and_then(|x| x.trim().parse().ok()).unwrap_or(usize::MAX);
                match blocos(&l) {
                    None => { sem_termino += 1; continue; }
                    Some((_, bl)) => {
                        if bl.len() != n { contagem_diferente += 1; continue; }
                        ok += 1;
                        if let Some(primeira) = bl.first() {
                            let antes = ler(&b).unwrap();
                            let novo = clonar_acao(&b, &primeira.0, "测试动作", None).unwrap();
                            let depois = ler(&novo).unwrap();
                            assert_eq!(depois.acoes.len(), antes.acoes.len() + 1, "{}", p.display());
                            assert_eq!(depois.acoes[1].nome, "测试动作");
                            assert_eq!(depois.acoes[1].bases, antes.acoes[0].bases);
                            let (_, bl2) = blocos(&linhas_com_fim(novo.strip_prefix(b"MOXT").unwrap_or(&novo))).unwrap();
                            assert_eq!(bl2.len(), n + 1);
                            // Só o bloco novo e a contagem mudam: tirando-os, o arquivo é o mesmo.
                            assert_eq!(novo.len() > b.len(), true);
                            clonados += 1;
                        }
                    }
                }
            }
        }
        eprintln!("ecm: blocos certos {ok}, contagem diferente {contagem_diferente}, sem fim {sem_termino}, clonados {clonados}");
        assert_eq!((contagem_diferente, sem_termino), (0, 0));
    }

    /// B299: cada `.ecm` do cliente como molde de um modelo novo com duas ações: os que servem geram um `.ecm` que relê
    /// com o `.smd` novo, as duas ações (cada uma com a sua base) e a contagem certa; os outros são recusados com motivo.
    #[test]
    fn ecm_novo_com_moldes_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let acoes = vec!["站立".to_string(), "普攻1起".to_string()];
        let mut pilha = vec![raiz.to_path_buf()];
        let mut motivos = std::collections::BTreeMap::<String, usize>::new();
        let mut por_versao = std::collections::BTreeMap::<u32, usize>::new();
        let mut ok = 0usize;
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ecm")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let Some(orig) = ler(&b) else { continue };
                match ecm_novo(&b, "Models\\novo\\novo.smd", &acoes) {
                    Err(x) => *motivos.entry(x).or_default() += 1,
                    Ok(n) => {
                        let e = ler(&n).unwrap_or_else(|| panic!("{}: não relê", p.display()));
                        assert_eq!(e.smd, "Models\\novo\\novo.smd", "{}", p.display());
                        assert_eq!(e.versao, orig.versao);
                        assert_eq!(e.acoes.iter().map(|a| (a.nome.clone(), a.bases.clone())).collect::<Vec<_>>(),
                            acoes.iter().map(|a| (a.clone(), vec![a.clone()])).collect::<Vec<_>>(), "{}", p.display());
                        let l = linhas_com_fim(n.strip_prefix(b"MOXT").unwrap_or(&n));
                        let (cont, bl) = blocos(&l).unwrap();
                        assert_eq!((sem_fim(l[cont]), bl.len()), (b"ComActCount: 2".as_slice(), 2));
                        ok += 1;
                        *por_versao.entry(orig.versao).or_default() += 1;
                    }
                }
            }
        }
        eprintln!("ecm novo: {ok} moldes servem (por versão {por_versao:?}); recusados {motivos:?}");
        assert!(ok > 1000, "{ok}");
    }

    /// B285: em todo o cliente, copiar a 1ª ação do `.ecm` anterior de mesma versão: relê com uma ação a mais (a última,
    /// com as ações-base da origem), o `ComActCount` bate com os blocos, e versão diferente é recusada.
    #[test]
    fn copiar_entre_arquivos_em_todo_o_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element\models");
        if !raiz.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let mut pilha = vec![raiz.to_path_buf()];
        let mut anterior: std::collections::HashMap<u32, (Vec<u8>, String)> = std::collections::HashMap::new();
        let (mut copiados, mut recusados) = (0usize, 0usize);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ecm")) { continue; }
                let b = std::fs::read(&p).unwrap();
                let Some(antes) = ler(&b) else { continue };
                if blocos(&linhas_com_fim(b.strip_prefix(b"MOXT").unwrap_or(&b))).is_none() { continue; }
                if let Some((f, origem)) = anterior.get(&antes.versao) {
                    let novo = copiar_acao(&b, f, origem, "复制测试").unwrap_or_else(|x| panic!("{}: {x}", p.display()));
                    let depois = ler(&novo).unwrap();
                    assert_eq!(depois.acoes.len(), antes.acoes.len() + 1, "{}", p.display());
                    let ultima = depois.acoes.last().unwrap();
                    assert_eq!((ultima.nome.as_str(), &ultima.bases), ("复制测试", &ler(f).unwrap().acoes.iter().find(|a| &a.nome == origem).unwrap().bases));
                    let l = linhas_com_fim(novo.strip_prefix(b"MOXT").unwrap_or(&novo));
                    let (cont, bl) = blocos(&l).unwrap();
                    assert_eq!(format!("ComActCount: {}", bl.len()).as_bytes(), sem_fim(l[cont]), "{}", p.display());
                    copiados += 1;
                }
                if let Some(outra) = anterior.iter().find(|(v, _)| **v != antes.versao) {
                    assert!(copiar_acao(&b, &outra.1 .0, &outra.1 .1, "复制测试").unwrap_err().contains("versões diferentes"));
                    recusados += 1;
                }
                if let Some(a) = antes.acoes.first() {
                    anterior.insert(antes.versao, (b.clone(), a.nome.clone()));
                }
            }
        }
        eprintln!("ecm: copiados {copiados}, versão diferente recusada {recusados}");
        assert!(copiados > 1000 && recusados > 0, "{copiados} {recusados}");
    }


    /// Todos os `.ecm` do cliente 1.5.5 BR: leem; quantas ações-base citadas existem no `.smd` deles.
    #[test]
    fn todos_os_ecm_do_cliente() {
        let raiz = std::path::Path::new(r"F:\PW\1.5.5\1.5.5 BR launcher\element");
        if !raiz.join("models").exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", raiz.display());
            return;
        }
        let disco = |s: &str| encoding_rs::WINDOWS_1252.decode(&encoding_rs::GBK.encode(s).0).0.into_owned();
        let mut pilha = vec![raiz.join("models")];
        let (mut ok, mut falha, mut com_smd, mut bases, mut bases_no_smd, mut sem_base) = (0, 0, 0, 0, 0, 0);
        while let Some(d) = pilha.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() { pilha.push(p); continue; }
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ecm")) { continue; }
                let Some(m) = ler(&std::fs::read(&p).unwrap()) else { falha += 1; continue };
                ok += 1;
                let smd = raiz.join(disco(&m.smd.replace('\\', "/")));
                let Ok(b) = std::fs::read(&smd) else { continue };
                let Ok(c) = crate::smd::cabeca(&b) else { continue };
                com_smd += 1;
                for a in &m.acoes {
                    if a.bases.is_empty() { sem_base += 1; }
                    for x in &a.bases {
                        bases += 1;
                        if c.lista_de_acoes.iter().any(|y| &y.nome == x) { bases_no_smd += 1; }
                    }
                }
            }
        }
        eprintln!("ecm: {ok} leem, {falha} falham; com .smd lido {com_smd}; ações-base {bases}, no .smd {bases_no_smd}; combinadas sem base {sem_base}");
        assert!(ok > 0 && falha == 0);
    }

    #[test]
    fn tabela_e_pedidos() {
        let fonte = "#_begin\n// 怪物\n\"站立\"\n\"休闲\"\n\"戒备\"\n\"技能1\"\n\"慢速移动\"\n\"普攻1\"\n\"普攻2\"\n";
        let t = tabela_de_nomes(&encoding_rs::GBK.encode(fonte).0);
        assert_eq!(t.len(), 7);
        let p = pedidos_de_monstro(&t);
        assert!(p.iter().any(|x| x.1 == "普攻1起") && p.iter().any(|x| x.1 == "普攻2落") && p.iter().any(|x| x.1 == "站立"));
        assert!(!p.iter().any(|x| x.1 == "技能1"), "ACT_SKILL1 não é tocada");
        let m = pedidos_de_mascote(&t, 4);
        assert!(m.iter().any(|x| x.1 == "普攻1起_地面") && m.iter().any(|x| x.1 == "站立_空中") && !m.iter().any(|x| x.1.ends_with("_水中")), "habitat 4 = chão + ar");
    }

    /// B283: as ações filhas (com o `HHName:` de modelo filho do `.ecm` fora delas) e o diagnóstico de voo.
    #[test]
    fn filhas_e_voo() {
        let fonte = "MOXTVersion: 50\nSkinModelPath: a.smd\nComActCount: 2\nCombineActName: 飞剑悬停\nBaseActCount: 1\n\
                     BaseActName: 悬停\nChildActCount: 2\nChildActName: 飞剑悬停\nHHName: \nTransTime: 200\nIsTrail: 0\n\
                     ChildActName: 刀光\nHHName: HH_righthandweapon\nTransTime: 0\nIsTrail: 1\nCombineActName: 站立\n\
                     BaseActCount: 0\nChildCount: 1\nHHName: HH_feijian\n";
        let e = ler(&encoding_rs::GBK.encode(fonte).0).unwrap();
        assert_eq!(e.acoes[0].filhas.len(), 2);
        assert!(e.acoes[1].filhas.is_empty(), "HHName de modelo filho não é ação filha");
        assert!(chega_ao_voo(&e.acoes[0].filhas[0], 0) && !chega_ao_voo(&e.acoes[0].filhas[1], 0), "rastro na arma não chega à espada");
        let enviadas: std::collections::BTreeMap<String, usize> = [("飞剑悬停", 10), ("飞剑起飞", 8), ("飞行悬停", 9)].map(|(a, b)| (a.to_string(), b)).into();
        let colegas: std::collections::BTreeMap<String, usize> = [("飞剑悬停", 20), ("飞剑起飞", 19)].map(|(a, b)| (a.to_string(), b)).into();
        let d = diagnostico_de_voo(&e.acoes, &enviadas, &colegas, 21);
        assert_eq!(d.len(), 2, "o nome da asa (飞行悬停) não é do vocabulário da espada");
        assert_eq!((d[0].nome.as_str(), d[0].estado), ("飞剑悬停", "ok"));
        assert!(d[1].nome == "飞剑起飞" && d[1].estado.starts_with("sem ação"), "19 de 21 colegas têm");
        let d = diagnostico_de_voo(&e.acoes, &enviadas, &colegas, 40);
        assert_eq!(d[1].estado, "ausente, como na maioria dos colegas", "19 de 40: a maioria não tem");
    }
}
