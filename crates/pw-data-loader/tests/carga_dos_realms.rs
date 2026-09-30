//! B102 — cada realm carrega a sua pasta `data/<realm>/config` inteira sem falha, e os
//! arquivos que nenhum leitor abre ficam listados. Sem a pasta o teste avisa e sai.

use pw_data_loader::GameDataManager;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn arquivos(raiz: &Path, dir: &Path, saida: &mut Vec<String>) {
    let Ok(entradas) = std::fs::read_dir(dir) else { return };
    for e in entradas.flatten() {
        let p = e.path();
        if p.is_dir() {
            arquivos(raiz, &p, saida);
        } else {
            saida.push(p.strip_prefix(raiz).unwrap().to_string_lossy().replace("\\", "/"));
        }
    }
}

/// Carrega a pasta do realm e devolve (falhas, não lidos). Arquivo de mapa (`.hmap`, `.rmap`,
/// `.wmap`, `.dhmap`, `.octr`, `.bht`) fica fora: o mundo os abre por mapa, não pelo relatório.
fn conferir(realm: &str) -> Option<(Vec<String>, Vec<String>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    let rel = d.load_from_directory(&dir);
    let mut todos = Vec::new();
    arquivos(&dir, &dir, &mut todos);
    // O relatório escreve "arquivo (detalhe)": vale o nome antes do parêntese.
    let lidos: Vec<String> = rel
        .lidos
        .iter()
        .map(|l| l.split(" (").next().unwrap_or(l).replace("\\", "/"))
        .collect();
    let mapa = ["hmap", "rmap", "wmap", "dhmap", "octr", "bht"];
    let mut nao_lidos: BTreeMap<String, usize> = BTreeMap::new();
    for a in &todos {
        let nome = a.rsplit('/').next().unwrap_or(a);
        let ext = nome.rsplit_once('.').map(|x| x.1).unwrap_or("").to_lowercase();
        if mapa.contains(&ext.as_str()) || ext == "conf" && a.contains('/') {
            continue;
        }
        if !lidos.iter().any(|l| l == a || l == nome || a.ends_with(&format!("/{l}"))) {
            *nao_lidos.entry(nome.to_string()).or_default() += 1;
        }
    }
    let falhas: Vec<String> = rel.falhas.iter().map(|f| f.to_string()).collect();
    eprintln!("{realm}: {} lidos, {} falhas; não lidos: {:?}", lidos.len(), falhas.len(), nao_lidos);
    Some((falhas, nao_lidos.into_keys().collect()))
}

#[test]
fn o_realm_126_carrega_sem_falha() {
    let Some((falhas, _)) = conferir("realm_126") else { return };
    assert!(falhas.is_empty(), "{falhas:?}");
}

/// As três falhas do 155 são dos próprios arquivos: `a46/npcgen.data` e `a50/precinct.sev`
/// terminam antes do que declaram.
#[test]
fn o_realm_155_carrega_so_com_as_falhas_dos_arquivos_truncados() {
    let Some((falhas, _)) = conferir("realm_155") else { return };
    assert!(falhas.iter().all(|f| f.contains("a46/npcgen.data") || f.contains("a50/precinct.sev")), "{falhas:?}");
}

/// B102 — o catálogo do realm (`data/<realm>/catalogo/`) vale no lugar do embutido: layout do
/// `elements.data` e tabela de habilidades vêm de arquivo, sem código por versão. Um layout de
/// outra versão é falha registrada, não dado lido errado.
#[test]
fn o_catalogo_do_realm_substitui_o_embutido() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let elements = raiz.join("data/realm_126/config/elements.data");
    if !elements.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", elements.display());
        return;
    }
    let realm = std::env::temp_dir().join(format!("pw_catalogo_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&realm);
    std::fs::create_dir_all(realm.join("config")).unwrap();
    std::fs::create_dir_all(realm.join("catalogo")).unwrap();
    std::fs::copy(&elements, realm.join("config/elements.data")).unwrap();
    std::fs::copy(raiz.join("specs/elements_layouts/v7.json"), realm.join("catalogo/elements_layout.json")).unwrap();
    std::fs::copy(raiz.join("specs/habilidades_126/habilidades.json"), realm.join("catalogo/habilidades.json")).unwrap();

    let mut d = GameDataManager::new();
    let rel = d.load_from_directory(&realm.join("config"));
    assert!(rel.sem_falhas(), "{rel}");
    assert!(rel.lidos.iter().any(|l| l.starts_with("catalogo/elements_layout.json (v7)")), "{:?}", rel.lidos);
    assert!(rel.lidos.iter().any(|l| l.starts_with("catalogo/habilidades.json (823")), "{:?}", rel.lidos);
    assert_eq!(d.habilidades.get(299).and_then(|h| h.dano.as_ref()).map(|x| x.plus[0]), Some(23.7));
    assert!(d.servicos_de_npc.get(&3518).is_some_and(|s| s.missoes_entregues.contains(&1177)));

    // Layout de outra versão: falha registrada, sem elements carregado.
    std::fs::copy(raiz.join("specs/elements_layouts/v156.json"), realm.join("catalogo/elements_layout.json")).unwrap();
    let mut d = GameDataManager::new();
    let rel = d.load_from_directory(&realm.join("config"));
    assert!(rel.falhas.iter().any(|f| f.to_string().contains("versão 156")), "{rel}");
    assert!(d.elements_generic.is_none());
    let _ = std::fs::remove_dir_all(&realm);
}

/// B105 — corpo e renascimento vêm do gerador (`iDeadTime`, `iRefresh`, `iRefreshLower`):
/// os Filhotes de Mandrágora (3303) e as Plantas Devoradoras (3302) da área inicial não têm
/// corpo e renascem 15 s + 0 depois, como a captura original (~15,5 s, sem `disappear`).
#[test]
fn o_gerador_do_126_da_o_corpo_e_o_renascimento() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let mut c = BTreeMap::<(u32, u32, u32, u32), usize>::new();
    for s in d.map_spawns[&1].instances.iter().filter(|s| s.template_id == 3302 || s.template_id == 3303) {
        *c.entry((s.template_id, s.corpo_s, s.renascer_min_s, s.renascer_max_s)).or_default() += 1;
    }
    eprintln!("(modelo, corpo s, renascer min, max) → spawns: {c:?}");
    let todos: Vec<_> = d.map_spawns[&1].instances.iter().filter(|s| s.spawn_type == pw_data_loader::SpawnType::Monster).collect();
    let sem_corpo = todos.iter().filter(|s| s.corpo_s == 0).count();
    eprintln!("mapa 1: {} monstros de gerador, {} sem corpo", todos.len(), sem_corpo);
    // `iRefresh` = 0 nessas áreas: 15 s + 0 (a captura dá 15,1–15,9 s). 27.610 de 27.618 sem corpo.
    assert!(c.iter().any(|((t, corpo, min, max), _)| *t == 3303 && *corpo == 0 && *min == 15 && *max == 15));
    assert!(sem_corpo * 100 > todos.len() * 99);
}

/// B107 — o `tasks.data` v55 do 1.2.6 traz `m_bAutoDeliver`, nível, pré-missões, gênero e zona
/// de entrega (deslocamentos medidos no `libtask.so` 1.2.6, ver `missao_v55`). Antes nenhuma
/// missão saía como automática e o cliente novo não recebia nada.
#[test]
fn as_missoes_automaticas_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let t = &d.tasks;
    let auto: Vec<u32> = t.de_topo.iter().copied().filter(|id| t.get_task(*id).is_some_and(|m| m.entrega_automatica)).collect();
    let todas: Vec<_> = t.de_topo.iter().filter_map(|id| t.get_task(*id)).collect();
    // Dados do próprio arquivo: a 949 "Pai e Filho" pede a 947, que não existe; dez "Teste de
    // Liu Weijun" têm mínimo acima do máximo (o `CheckLevel` nunca as libera). Leitura
    // deslocada daria nível acima do teto de 150 ou gênero fora de 0..2 em massa.
    let pre_invalidas = todas.iter().flat_map(|m| m.pre_tasks.iter()).filter(|p| t.get_task(**p).is_none()).count();
    let nivel_absurdo = todas.iter().filter(|m| m.min_level > 150 || m.max_level > 150).count();
    let genero_absurdo = todas.iter().filter(|m| m.genero > 2).count();
    for id in [9376u32, 1177] {
        if let Some(m) = t.get_task(id) {
            eprintln!("{id} {:?}: auto {} nível {}..{} classes {:?} pré {:?} gênero {} zona {} mundo {} {:?}", m.name, m.entrega_automatica, m.min_level, m.max_level, m.req_classes, m.pre_tasks, m.genero, m.entrega_em_zona, m.mundo_de_entrega, m.regioes_de_entrega);
        }
    }
    eprintln!("{} automáticas; {} pré-missões inexistentes; {} níveis acima de 150; {} gêneros fora de 0..2 (de {} raízes)", auto.len(), pre_invalidas, nivel_absurdo, genero_absurdo, todas.len());
    assert_eq!(auto.len(), 81);
    assert_eq!((pre_invalidas, nivel_absurdo, genero_absurdo), (1, 0, 0));
    assert!(t.get_task(9376).is_some_and(|m| m.entrega_automatica));
}

/// B110 — o lugar das missões "chegar a um lugar" (método 4) do 1.2.6, medido no
/// `OnTaskReachSite` do `libtask.so` 1.2.6: mundo +0x1de, caixa +0x1c6..+0x1d2. Leitura
/// deslocada daria caixa invertida ou mundo absurdo.
#[test]
fn os_lugares_a_alcancar_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let t = &d.tasks;
    let mut ids: Vec<u32> = Vec::new();
    let mut fila: Vec<u32> = t.de_topo.clone();
    while let Some(id) = fila.pop() {
        if let Some(m) = t.get_task(id) {
            fila.extend(m.sub_tasks.iter().copied());
            if m.metodo == 4 {
                ids.push(id);
            }
        }
    }
    let ruins: Vec<u32> = ids
        .iter()
        .copied()
        .filter(|id| {
            let m = t.get_task(*id).unwrap();
            let r = &m.lugares_a_alcancar[0];
            m.mundo_a_alcancar == 0 || m.mundo_a_alcancar > 400 || (0..3).any(|i| !r.min[i].is_finite() || !r.max[i].is_finite() || r.min[i].abs() > 10_000.0 || r.max[i].abs() > 10_000.0)
        })
        .collect();
    let m = t.get_task(5911).expect("5911");
    eprintln!("5911 {:?}: mundo {} {:?}", m.name, m.mundo_a_alcancar, m.lugares_a_alcancar);
    for id in &ruins { let m = t.get_task(*id).unwrap(); eprintln!("INCOERENTE {id} {:?} mundo {} {:?}", m.name, m.mundo_a_alcancar, m.lugares_a_alcancar); }
    eprintln!("{} missões de chegar a um lugar; incoerentes: {:?}", ids.len(), &ruins[..ruins.len().min(10)]);
    assert!(ids.len() > 50);
    assert!(ruins.is_empty());
    // Dado do arquivo: nove "Estágio Fácil/Normal/Difícil 3-x" (4735-4770) têm mínimo e máximo
    // trocados em algum eixo — coordenadas plausíveis do mapa 1, que o `is_in_zone` do
    // original também nunca cumpre.
    let invertidas = ids.iter().filter(|id| { let r = &t.get_task(**id).unwrap().lugares_a_alcancar[0]; (0..3).any(|i| r.min[i] > r.max[i]) }).count();
    assert_eq!(invertidas, 9);
    assert_eq!(m.mundo_a_alcancar, 1);
}

/// Mascote de combate: o `PET_ESSENCE` dos dois realms vira `ModeloDeMascote` com as recusas
/// do `pet_dataman::LoadTemplate`, e o layout v7 corrigido (sem o `pet_snd_type` inventado, com
/// `damage_d`, visão, comida e habitat nas posições que o `gs` 1.2.6 lê, VA 0x8143580) dá ao
/// Filhote de Lobo Feroz (10386) os mesmos coeficientes do 1.5.5.
#[test]
fn os_modelos_de_mascote_dos_dois_realms() {
    let mut vistos = Vec::new();
    for realm in ["realm_126", "realm_155"] {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
        if !dir.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
            continue;
        }
        let mut d = GameDataManager::new();
        d.load_from_directory(&dir);
        let combate = d.modelos_de_mascote.values().filter(|m| m.classe == 1).count();
        let m = d.modelos_de_mascote.get(&10386).expect("10386").clone();
        let a = m.atributos(2);
        eprintln!("{realm}: {} modelos, {combate} de combate; 10386 nível 2: {a:?} (atraso {} tiques, golpe {} tiques, visão {})",
            d.modelos_de_mascote.len(), m.atraso_do_dano, m.intervalo_do_golpe, m.visao);
        assert_eq!(m.classe, 1);
        assert!(combate > 10);
        assert_eq!((m.atraso_do_dano, m.intervalo_do_golpe, m.alcance, m.corpo), (23, 25, 3.0, 0.9));
        assert!(a.vida > 0 && a.dano > 0 && a.correr > 1.0);
        vistos.push((m.hp, m.dano, m.velocidade));
    }
    if vistos.len() == 2 {
        assert_eq!(vistos[0], vistos[1], "coeficientes do 10386 diferentes entre 1.2.6 e 1.5.5");
    }
}

/// A curva de experiência do mascote (`PLAYER_LEVELEXP_CONFIG` id 592, `playertemplate.cpp:385-400`).
#[test]
fn a_curva_do_mascote_dos_dois_realms() {
    for realm in ["realm_126", "realm_155"] {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
        if !dir.exists() {
            continue;
        }
        let mut d = GameDataManager::new();
        d.load_from_directory(&dir);
        let tem_592 = d.elements_generic.as_ref().is_some_and(|g| g.get("PLAYER_LEVELEXP_CONFIG").iter().any(|r| r.get("ID").and_then(|v| v.as_i32()) == Some(592)));
        let curva: Vec<i64> = (1..=10).map(|n| d.progressao.exp_do_mascote_para_subir(n)).collect();
        let jogador: Vec<i64> = (1..=5).map(|n| d.progressao.exp_para_subir(n)).collect();
        eprintln!("{realm}: id 592 no arquivo {tem_592}; mascote 1..10 {curva:?}; jogador 1..5 {jogador:?}");
    }
}

/// B117 — `m_bClearAcquired` do 1.2.6 em +0xae (o `libtask.so` 1.2.6 o testa antes de
/// `RemoveAcquiredItem`, em `RecursiveAward` 0xabee e `RecursiveClearTask` 0xd723). Nas missões
/// da cadeia da Fera Psíquica ele bate com o do 1.5.5 (`true` nas de coleta, `false` na 5920).
#[test]
fn o_126_le_o_limpa_adquiridos_das_missoes() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let (d126, d155) = (raiz.join("realm_126/config"), raiz.join("realm_155/config"));
    if !d126.exists() || !d155.exists() {
        eprintln!("AVISO: sem os realms — este teste NÃO verificou nada.");
        return;
    }
    let carregar = |d: &Path| {
        let mut g = GameDataManager::new();
        g.load_from_directory(d);
        g
    };
    let (a, b) = (carregar(&d126), carregar(&d155));
    for id in [5933u32, 5925, 5922, 5920, 1177, 9374] {
        let (x, y) = (a.tasks.get_task(id).unwrap().limpa_adquiridos, b.tasks.get_task(id).unwrap().limpa_adquiridos);
        assert_eq!(x, y, "missão {id}: 1.2.6 {x}, 1.5.5 {y}");
    }
    assert!(a.tasks.get_task(5933).unwrap().limpa_adquiridos);
}

/// B119 — `m_ulFuryULimit` do prêmio v55 em +40 (`libtask.so` 1.2.6, `DeliverByAwardData`
/// 0xb4b3-0xb4d2). São 8 missões no `tasks.data` 1.2.6, como as 8 do 1.5.5.
#[test]
fn as_missoes_de_teto_de_chi_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let mut tetos: Vec<(u32, u32)> = Vec::new();
    for id in 1..40_000u32 {
        if let Some(t) = d.tasks.get_task(id) {
            if t.rewards.teto_de_chi != 0 {
                tetos.push((id, t.rewards.teto_de_chi));
            }
        }
    }
    assert_eq!(tetos, vec![(915, 99), (922, 199), (925, 299), (966, 99), (973, 99), (1888, 399), (2804, 399), (2818, 399)]);
}

/// B143 — o preço do vendedor com os dados do 1.2.6: `shop_price` 9.600 da armadura 139 ×
/// 1,05 + 0,5 → 10.080 → `AdjustVendorFee` → 10.100; e as taxas do `NPC_ESSENCE` v7
/// (1.441 NPCs com 0, 28 com 0,05, 2 com 17).
#[test]
fn o_preco_do_vendedor_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    assert_eq!(d.preco_de_compra(139), Some(9600));
    assert_eq!(d.preco_de_loja(139, 0.0), Some(10_100));
    assert_eq!(d.taxas_de_npc.values().filter(|t| **t == 17.0).count(), 2);
    assert_eq!(d.taxas_de_npc.values().filter(|t| (**t - 0.05).abs() < 1e-6).count(), 28);
}

/// B145 — receitas e serviço de produção do 1.2.6 com o layout v7 corrigido (sem `bind_type`
/// no `RECIPE_ESSENCE`, sem `produce_type` no `NPC_MAKE_SERVICE`): a 54 "Espada Unificadora" é
/// da habilidade 158 no nível 2, 11 s (220 tiques), 4 × 800, 8 × 825 e 2 × 771; um NPC de forja a oferece.
#[test]
fn as_receitas_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let r = &d.receitas[&54];
    assert_eq!((r.habilidade, r.nivel_exigido, r.tempo_em_tiques, r.exp), (158, 2, 220, 50));
    assert_eq!(r.materiais, vec![(800, 4), (825, 8), (771, 2)]);
    assert_eq!(r.vinculo, 0, "o v7 não tem bind_type");
    assert!(d.producao_do_npc.values().any(|s| s.habilidade == 158 && s.receitas.contains(&54)));
}

/// B151 — arma, armadura e acessório do v7 pelo `gs` 1.2.6 (`generate_weapon/armor/decoration
/// <NORMAL>`, VA 0x81f4eae, 0x81f57a4, 0x81f6026): sem `require_reputation`, com `fixed_props`, 4
/// probabilidades de número de addons e 16 únicos. A foice ★★★Nighthawk 15964 (fabricada pelo
/// Murillo) tem arma de nível 13, propriedades fixas 473/1008/1321 e durabilidade 300 (245 no
/// drop); e em todo modelo do realm as probabilidades de número de addons somam 1 (ou 0) — no
/// layout antigo a 5ª era o `probability_unique` e a 6ª o id do 1º addon.
#[test]
fn a_geracao_de_equipamento_do_126() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_126/config");
    if !dir.exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    let foice = &d.geracao[&15964];
    assert!(foice.fixed_props);
    assert_eq!(foice.quantos_addons, vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(foice.addons.iter().take(3).map(|a| a.0).collect::<Vec<_>>(), vec![473, 1008, 1321]);
    assert_eq!((foice.durabilidade, foice.durabilidade_no_drop), ((300, 300), (245, 245)));
    assert_eq!(foice.furos_na_producao, vec![0.0, 0.9, 0.1]);
    assert_eq!(d.equipamentos.armas[&15964].nivel, 13);
    assert_eq!(d.equipamentos.armas[&15964].reputacao_exigida, 0);
    let arco = &d.geracao[&212];
    assert!((arco.chance_de_unico - 0.044118).abs() < 1e-6);
    assert_eq!(arco.unicos[15].0, 421, "o 16º único existe no v7");
    assert_eq!(arco.addons_da_producao[0].0, 784);
    let fora: Vec<u32> = d
        .geracao
        .iter()
        .filter(|(_, m)| {
            let s: f32 = m.quantos_addons.iter().sum();
            s.abs() > 1e-3 && (s - 1.0).abs() > 1e-3
        })
        .map(|(id, _)| *id)
        .collect();
    assert!(fora.is_empty(), "{} de {} modelos com addon_num que não soma 1: {:?}", fora.len(), d.geracao.len(), &fora[..fora.len().min(10)]);
}

/// B152 — o serviço de restauração de atributos (`NPC_RESETPROP_SERVICE`, 368 B nas duas
/// versões; no `gs` 1.2.6 `id_resetprop_service` em +0x32c do `NPC_ESSENCE` e 15 entradas de
/// 20 B em +0x44). Os serviços 10226/10227 têm as 15 opções; a última é o 12764 com 100 em cada
/// atributo, e a primeira o 9834 com 10 de força.
#[test]
fn a_restauracao_de_atributos_dos_realms() {
    for realm in ["realm_126", "realm_155"] {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
        if !dir.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
            continue;
        }
        let mut d = GameDataManager::new();
        d.load_from_directory(&dir);
        let com: Vec<_> = d.servicos_de_npc.values().filter(|s| !s.restauracao_de_atributos.is_empty()).collect();
        assert!(!com.is_empty(), "{realm}: nenhum NPC com o serviço");
        for s in com {
            let l = &s.restauracao_de_atributos;
            assert_eq!(l.len(), 15, "{realm}");
            assert_eq!((l[0].item, l[0].forca, l[0].agilidade), (9834, 10, 0), "{realm}");
            let u = l[14];
            assert_eq!((u.item, u.forca, u.agilidade, u.vitalidade, u.energia), (12764, 100, 100, 100, 100), "{realm}");
        }
    }
}

/// B153 — `m_ulPetInventorySize` do prêmio: +53 no v129 (`TaskTempl.h:1149`) e +36 no v55
/// (`libtask.so` 1.2.6, `DeliverByAwardData` 0xb48c → `SetPetInventorySize`). A série "Jaula de
/// Mascote"/"Expandir Jaula" da Gerente de Mascotes (3327–3330) dá 2, 3, 4 e 5 vagas nos dois
/// realms, e a 8990 dá 10.
#[test]
fn as_missoes_que_ampliam_a_jaula() {
    for realm in ["realm_126", "realm_155"] {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data").join(realm).join("config");
        if !dir.exists() {
            eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
            continue;
        }
        let mut d = GameDataManager::new();
        d.load_from_directory(&dir);
        let vagas = |id: u32| d.tasks.get_task(id).map(|t| t.rewards.vagas_na_jaula);
        assert_eq!([3327, 3328, 3329, 3330, 8990].map(vagas), [Some(2), Some(3), Some(4), Some(5), Some(10)], "{realm}");
        let fora = (1..40_000u32).filter_map(|id| d.tasks.get_task(id)).filter(|t| t.rewards.vagas_na_jaula > 20).count();
        assert_eq!(fora, 0, "{realm}: vagas acima de MAX_PET_CAPACITY");
    }
}
