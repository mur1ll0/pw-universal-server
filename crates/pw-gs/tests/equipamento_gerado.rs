//! B53 — equipamento sorteado no drop contra o `elements.data` real do `realm_155`:
//! o conteúdo se relê, fecha no último byte, e as propriedades adicionais entram nos
//! atributos. Sem a pasta do realm o teste avisa e não verifica nada.

use pw_core::{ContainerType, ConteudoDeEquipamento, ItemRecord};
use pw_data_loader::GameDataManager;
use pw_gs::entity::{BonusDeAddons, Equipamento};
use std::path::PathBuf;

fn realm() -> Option<GameDataManager> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/realm_155/config");
    if !dir.join("elements.data").exists() {
        eprintln!("AVISO: sem {} — este teste NÃO verificou nada.", dir.display());
        return None;
    }
    let mut d = GameDataManager::new();
    d.load_from_directory(&dir);
    Some(d)
}

#[test]
fn o_drop_sorteia_addons_que_se_releem_e_somam_nos_atributos() {
    let Some(d) = realm() else { return };
    assert!(d.addons.por_id.len() > 1000, "addons carregados: {}", d.addons.por_id.len());
    assert!(!d.geracao.is_empty());

    let mut com_addons = 0;
    let mut somados = 0;
    let mut sem_porte = 0;
    let mut ids: Vec<u32> = d.geracao.keys().copied().collect();
    ids.sort();
    for tid in ids.iter().take(3000) {
        for _ in 0..3 {
            let Some(c) = pw_gs::geracao::gerar_equipamento(&d, *tid) else { continue };
            let b = c.escrever();
            let modelo = d.equipamentos.ficha(*tid).unwrap();
            let lido = ConteudoDeEquipamento::ler(&b, &modelo).unwrap_or_else(|| panic!("o conteúdo de {tid} não se relê"));
            assert_eq!(lido, c);
            if c.addons.is_empty() {
                continue;
            }
            com_addons += 1;
            let mut item = ItemRecord::new(1, ContainerType::Equipment, 1, *tid, 1);
            item.octets = b;
            let e = Equipamento::dos_itens_com_addons(&[item], &d.equipamentos, Some(&d.addons));
            sem_porte += e.addons_sem_porte.len();
            if e.addons != BonusDeAddons::default() {
                somados += 1;
            }
        }
    }
    eprintln!("{com_addons} peças com addons, {somados} com bônus somado, {sem_porte} addons sem porte");
    assert!(com_addons > 50, "quase nada saiu com addon: {com_addons}");
    assert!(somados > com_addons / 3, "os addons não entraram nos atributos: {somados} de {com_addons}");
}

#[test]
fn o_refino_soma_o_valor_gravado_no_addon() {
    let mut b = BonusDeAddons::default();
    assert!(b.somar("refine_damage", &[24, 3]));
    assert!(b.somar("refine_magic_damage", &[10, 1]));
    assert!(b.somar("enhance_str_addon", &[4]));
    assert!(b.somar("IA_EA_ESS<offsetof(armor_essence,defense)>", &[9]));
    assert!(!b.somar("item_skill_addon", &[1, 2]));
    assert_eq!((b.dano, b.dano_magico, b.forca, b.defesa), (34, 10, 4, 0));
}

/// B60 — o item que a missão dá entra na bolsa **gerado**, com bloco de dados e propriedades.
///
/// No original, todo item de prêmio de missão passa por `generate_item_for_drop`
/// (`DeliverCommonItem`, `task/taskman.cpp:281-303`) — a mesma geração do drop de monstro.
/// Nós criávamos o item seco: o cliente mostrava a faixa do modelo no tooltip
/// ("Destreza +1~2") e o servidor não somava nada (relato do set Halo, 2026-09-18).
#[test]
fn o_premio_de_missao_entra_na_bolsa_com_o_equipamento_gerado() {
    let Some(d) = realm() else { return };
    // Peitoral do set Halo, o do relato.
    const PEITORAL_HALO: u32 = 28790;
    assert!(d.geracao.contains_key(&PEITORAL_HALO), "o 28790 devia ter modelo de geração");

    let mut bolsa = pw_gs::economia::Bolsa::nova(1, ContainerType::Inventory, 32, Vec::new());
    let e = bolsa.empilhar_gerado(PEITORAL_HALO, 1, &d).expect("cabe na bolsa");
    let item = bolsa.slots[e.slot].clone().expect("o slot ficou com o item");
    assert_eq!(item.item_id, PEITORAL_HALO);
    assert!(!item.octets.is_empty(), "o item entrou sem bloco de dados");
    assert!(item.durability > 0, "sem durabilidade de fábrica");

    // O bloco se relê, e o que ele traz soma nos atributos do jogador.
    let ficha = d.equipamentos.ficha(PEITORAL_HALO).expect("ficha do 28790");
    let c = ConteudoDeEquipamento::ler(&item.octets, &ficha).expect("o bloco tem de se reler");
    assert!(!c.addons.is_empty(), "o peitoral saiu sem propriedade adicional");
    let eq = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert_ne!(eq.addons, BonusDeAddons::default(), "as propriedades não somaram nos atributos");
}

/// B61 — a durabilidade sai na escala interna, que é a que o cliente divide por 100.
///
/// O gerador do original multiplica os dois números pelo `DURABILITY_UNIT_COUNT` no fim
/// (`update_require_data`, `gs/item/item_addon.h:454-458`, chamado em
/// `generate_item_temp.h:367`), e o cliente divide arredondando para cima
/// (`EC_IvtrEquip.cpp:281`). Sem a multiplicação, o ★Arco Real (50 no `elements.data`)
/// aparecia como **1/1** em jogo (relato de 2026-09-18).
#[test]
fn a_durabilidade_gerada_vai_na_escala_interna() {
    let Some(d) = realm() else { return };
    const ARCO_REAL: u32 = 36121;
    let de_fabrica = d.durabilidade_de_fabrica(ARCO_REAL).expect("o arco tem durabilidade") as i32;
    assert_eq!(de_fabrica, 50, "o elements do realm_155 mudou");

    let c = pw_gs::geracao::gerar_equipamento(&d, ARCO_REAL).expect("o arco se gera");
    assert_eq!(c.durabilidade_maxima, de_fabrica * pw_core::ESCALA_DA_DURABILIDADE);
    assert_eq!(c.durabilidade, c.durabilidade_maxima, "durability_drop == durability no 36121");
    // O que o cliente mostra: `(v + 99) / 100`.
    let na_tela = (c.durabilidade + pw_core::ESCALA_DA_DURABILIDADE - 1) / pw_core::ESCALA_DA_DURABILIDADE;
    assert_eq!(na_tela, 50, "o cliente mostraria {na_tela}/50");

    // E o bloco gravado carrega o mesmo número, no lugar onde o cliente o lê.
    let b = c.escrever();
    let ficha = d.equipamentos.ficha(ARCO_REAL).expect("ficha do arco");
    assert_eq!(ConteudoDeEquipamento::ler(&b, &ficha).expect("relê").durabilidade, c.durabilidade);
}

/// B61 — peça acabada não conta: `equip_item::VerifyRequirement` exige `durability > 0`
/// (`gs/item/equip_item.cpp:60-80`).
#[test]
fn a_peca_com_durabilidade_zerada_nao_entra_nos_atributos() {
    let Some(d) = realm() else { return };
    const PEITORAL_HALO: u32 = 28790;
    let mut bolsa = pw_gs::economia::Bolsa::nova(1, ContainerType::Equipment, 32, Vec::new());
    let e = bolsa.empilhar_gerado(PEITORAL_HALO, 1, &d).expect("cabe");
    let mut item = bolsa.slots[e.slot].clone().expect("o slot ficou com o item");
    item.slot = 4;

    let inteiro = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert!(inteiro.defesa > 0, "o peitoral inteiro devia dar defesa");

    item.durability = 0;
    let quebrado = Equipamento::dos_itens_com_addons(std::slice::from_ref(&item), &d.equipamentos, Some(&d.addons));
    assert_eq!(quebrado.defesa, 0, "peça acabada ainda somava defesa");
    assert_eq!(quebrado.addons, BonusDeAddons::default(), "peça acabada ainda somava propriedade");
}

/// B67 — o amuleto de vida e o hierograma de mana têm conteúdo próprio: 8 bytes.
///
/// `amulet_essence { int point; float trigger_percent; }` (`gs/item/item_amulet.h:16-19`),
/// escrito sem cabeçalho de requisito (`generate_item_temp.h:2296-2310`). Sem ele o cliente
/// mostrava o item zerado (relato de 2026-09-19).
#[test]
fn o_amuleto_e_o_hierograma_saem_com_os_oito_bytes_da_essencia() {
    let Some(d) = realm() else { return };
    const AMULETO: u32 = 35370; // Amuleto do Guardião - 1 (AUTOHP_ESSENCE)
    const HIEROGRAMA: u32 = 35376; // Hierograma do Guardião - 1 (AUTOMP_ESSENCE)

    let a = d.conteudo_do_amuleto(AMULETO).expect("o 35370 é um AUTOHP_ESSENCE");
    assert_eq!(a.len(), 8, "o conteúdo do amuleto tem 8 bytes");
    assert_eq!(i32::from_le_bytes([a[0], a[1], a[2], a[3]]), 5400, "total_hp do elements");
    assert!((f32::from_le_bytes([a[4], a[5], a[6], a[7]]) - 0.5).abs() < 1e-6, "trigger_amount");

    let m = d.conteudo_do_amuleto(HIEROGRAMA).expect("o 35376 é um AUTOMP_ESSENCE");
    assert_eq!(i32::from_le_bytes([m[0], m[1], m[2], m[3]]), 18000, "total_mp do elements");
    assert!((f32::from_le_bytes([m[4], m[5], m[6], m[7]]) - 0.75).abs() < 1e-6);

    // E um item qualquer não é amuleto.
    assert!(d.conteudo_do_amuleto(1796).is_none(), "poção não tem essência de amuleto");
}

/// B67 — a poção restaura ao longo do tempo, e o `elements.data` diz em quanto.
#[test]
fn a_pocao_de_vida_traz_o_total_e_o_tempo() {
    let Some(d) = realm() else { return };
    // Poção Pequena de Cura: 25 de vida em 10 s, recarga de 15 s.
    let (hp, hp_s, mp, mp_s, recarga) = d.quanto_o_remedio_restaura_no_tempo(1796).expect("1796 é remédio");
    assert_eq!((hp, hp_s), (25, 10), "hp_add_total/hp_add_time do elements");
    assert_eq!((mp, mp_s), (0, 0));
    assert_eq!(recarga, 15000, "cool_time");

    // Foram as outras duas usadas pelo eaa no relato de 2026-09-20. O número continua
    // vindo do arquivo real: vida e mana têm famílias de recarga separadas, ambas 15 s.
    assert_eq!(
        d.quanto_o_remedio_restaura_no_tempo(36584),
        Some((450, 3, 0, 0, 15000)),
        "Orvalho do Florescer Vermelho"
    );
    assert_eq!(
        d.quanto_o_remedio_restaura_no_tempo(36585),
        Some((0, 0, 450, 3, 15000)),
        "Orvalho do Fluxo Gélido"
    );
}

/// B76 — a Flor de Safira não produz item: ela **acorda um monstro**.
///
/// O `MINE_ESSENCE` tem `npcgen_1..4` (`gs/npcgenerator.cpp:1280-1365` monta a matéria com
/// eles), e é por aí que a missão 31779 funciona: colher a flor (44566) solta o Guardião de
/// Almas (44608), que é quem deixa cair o Estame (44371) com 80 % ao morrer.
#[test]
fn a_mina_da_flor_de_safira_acorda_o_guardiao() {
    let Some(d) = realm() else { return };
    let mina = d.minas.get(&44566).expect("a Flor de Safira é uma mina do realm");
    assert!(mina.materiais.iter().all(|m| m.item == 0), "a flor não produz material nenhum");
    assert_eq!(mina.missao_de_saida, 31779, "a mina é da missão das Flores do Guardião de Almas");
    assert_eq!(
        mina.monstros_ao_colher,
        vec![(44608, 1, 0.0, 30)],
        "colher devia acordar um Guardião de Almas por 30 s"
    );

    // E o Guardião é agressivo: ele parte para cima de quem o acordou.
    let guardiao = d.monstros.get(44608).expect("o 44608 está no elements");
    assert_ne!(guardiao.agressivo, 0, "o Guardião de Almas tem de ser agressivo");
}

/// B74 — item de missão na bolsa comum não é defeito: é o que o `tasks.data` manda.
///
/// Quem escolhe a bolsa é o `m_bDropCmnItem` de cada `MONSTER_WANTED`
/// (`gs/task/TaskTempl.inl:2028-2037`), e ele acompanha o tipo do item no `elements.data`:
/// **`TASKMATTER_ESSENCE` vai para a bolsa de missão, `TASKNORMALMATTER_ESSENCE` para a
/// comum** — "matéria de missão normal" é justamente a que fica no inventário normal. As
/// Almas que o Murillo pegou (44357 da Ninfa, 44363 da Pantera) são desse segundo tipo.
#[test]
fn o_tipo_do_item_de_missao_decide_a_bolsa() {
    let Some(d) = realm() else { return };
    let e = d.elements_generic.as_ref().expect("o realm tem elements.data");
    let ids_da = |tabela: &str| -> std::collections::HashSet<u32> {
        e.get(tabela).iter().filter_map(|r| r.get("ID").and_then(|v| v.as_i32())).map(|i| i as u32).collect()
    };
    let de_missao = ids_da("TASKMATTER_ESSENCE");
    let normais = ids_da("TASKNORMALMATTER_ESSENCE");
    assert!(!de_missao.is_empty() && !normais.is_empty(), "as duas tabelas existem no realm");

    let (mut matter_na_comum, mut normal_na_de_missao, mut vistos_matter, mut vistos_normal) = (0, 0, 0, 0);
    for m in d.tasks.tasks.values() {
        for k in &m.monster_kills {
            if k.item_que_cai == 0 {
                continue;
            }
            if de_missao.contains(&k.item_que_cai) {
                vistos_matter += 1;
                if k.item_comum {
                    matter_na_comum += 1;
                }
            } else if normais.contains(&k.item_que_cai) {
                vistos_normal += 1;
                if !k.item_comum {
                    normal_na_de_missao += 1;
                }
            }
        }
    }
    assert!(vistos_matter > 100 && vistos_normal > 100, "amostra pequena demais: {vistos_matter}/{vistos_normal}");
    assert_eq!(matter_na_comum, 0, "algum TASKMATTER foi marcado para a bolsa comum");
    // O tipo do item **descreve** a regra; quem manda é o bit. A única exceção do realm é a
    // "Presa de Filhote de Lobo" (2654), um TASKNORMALMATTER que uma missão quer na bolsa de
    // missão — e o servidor tem de obedecer ao bit, não ao tipo.
    assert_eq!(normal_na_de_missao, 1, "mudou o número de exceções do realm");

    // E os dois itens do relato, nomeados.
    for id in [44357, 44363] {
        assert!(normais.contains(&id), "o {id} devia ser TASKNORMALMATTER_ESSENCE");
    }
}

/// B73 — o amuleto vestido traz o total, o gatilho e a recarga do `elements.data`.
///
/// `OnActivate` entrega `point` e `trigger_percent` ao jogador (`gs/item/item_amulet.cpp:22-46`)
/// e `OnAutoTrigger` arma o `cool_time` do próprio item depois de cada disparo (`:9-20`).
#[test]
fn o_amuleto_traz_o_total_o_gatilho_e_a_recarga() {
    let Some(d) = realm() else { return };
    assert_eq!(
        d.dados_do_amuleto(35370),
        Some((5400, 0.5, 10000, true)),
        "Amuleto do Guardião - 1: 5400 de vida, dispara a 50 %, recarrega em 10 s"
    );
    let (ponto, gatilho, recarga, de_vida) = d.dados_do_amuleto(35376).expect("o 35376 é um AUTOMP_ESSENCE");
    assert_eq!(ponto, 18000, "Hierograma do Guardião - 1");
    assert!((gatilho - 0.75).abs() < 1e-6, "dispara a 75 % de mana");
    assert!(recarga > 0, "sem recarga o hierograma dispararia todo segundo");
    assert!(!de_vida, "o hierograma é de mana");
    assert_eq!(d.dados_do_amuleto(1796), None, "poção não é amuleto");
}

/// B71 — quem decide a família de recarga é o `id_major_type`, não o que a poção restaura.
///
/// `set_to_classid` (`gs/template/setclassid.cpp:81-101`) traduz 1794 em
/// `CLS_ITEM_HEALING_POTION`, 1802 em `CLS_ITEM_MANA_POTION`, 1810 em
/// `CLS_ITEM_REJUVENATION_POTION` e 1815/2038 nos antídotos; cada `OnUse` arma o
/// `COOLDOWN_INDEX_*` da sua classe (`gs/item/item_potion.cpp:18-110`).
#[test]
fn o_tipo_maior_do_remedio_separa_as_familias_de_recarga() {
    let Some(d) = realm() else { return };
    assert_eq!(d.tipo_maior_do_remedio(1796), Some(1794), "Poção Pequena de Cura");
    assert_eq!(d.tipo_maior_do_remedio(1804), Some(1802), "Poção Pequena do Espírito");
    assert_eq!(d.tipo_maior_do_remedio(1812), Some(1810), "Nove Sóis Pequeno (vida e mana)");
    assert_eq!(d.tipo_maior_do_remedio(1817), Some(1815), "Pílula Desintoxicante");
    assert_eq!(d.tipo_maior_do_remedio(2040), Some(2038), "Pílula das Nove Desintoxicações");
    assert_eq!(d.tipo_maior_do_remedio(35370), None, "amuleto não é remédio");

    // O antídoto restaura zero de vida e zero de mana: só o `id_major_type` o separa da
    // poção de mana, e era nela que a divisão por hp/mp o punha.
    assert_eq!(d.quanto_o_remedio_restaura_no_tempo(1817), Some((0, 0, 0, 0, 15000)));
}
