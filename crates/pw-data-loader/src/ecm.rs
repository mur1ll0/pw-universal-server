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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Ecm {
    pub versao: u32,
    pub smd: String,
    pub acoes: Vec<AcaoCombinada>,
}

pub fn ler(b: &[u8]) -> Option<Ecm> {
    let corpo = b.strip_prefix(b"MOXT").unwrap_or(b);
    let texto = crate::ski::gbk(corpo);
    let mut e = Ecm { versao: 0, smd: String::new(), acoes: Vec::new() };
    let mut viu_versao = false;
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
            e.acoes.push(AcaoCombinada { nome: v.to_string(), bases: Vec::new() });
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

#[cfg(test)]
mod testes {
    use super::*;

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
    }
}
