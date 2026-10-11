//! E8 (B217): os templates de criação do realm pelo painel — ler e editar o que o servidor usa
//! para montar o personagem novo de cada classe.
//!
//! No original, o personagem novo é a cópia do molde da classe no `clsconfig` do `gamedbd`
//! (`DBCreateRole`, `cnet/gamedbd/dbcreaterole.hrp:46-60` e `:320-360`): `GRoleBase`,
//! `GRoleStatus` (nível, cultivo, exp, sp, pp, atributos, mapa, posição, habilidades), a bolsa
//! com o dinheiro, o corpo e o armazém. Aqui o molde é o template do realm no banco
//! (`class_templates`, `class_template_items`, `class_template_skills`), que o `pw-link` lê a
//! cada criação (`CharacterRepository::create_character`) — a edição vale para o próximo
//! personagem, sem reinício. Personagens existentes não mudam.
//!
//! O GS confere com os dados do realm o que o cliente recusaria: item inexistente, peça fora do
//! slot (`CheckEquipPostion`), requisito de nível/classe/atributos (`VerifyRequirement`),
//! pilha, cultivo da versão, teto de nível e mapa com terreno.
use super::RoteadorDeMapas;
use pw_core::CharacterClass;
use pw_storage::EstatisticasDoTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Política do painel, a mesma da edição de personagem (`administracao.rs`): EXP, SP e
/// dinheiro até 2 000 000 000; pontos livres até 10 000; atributo até 100 000.
const TETO: i64 = 2_000_000_000;
const TETO_DE_PONTOS: i32 = 10_000;
const TETO_DE_ATRIBUTO: i32 = 100_000;
const TETO_DE_COORDENADA: f32 = 100_000.0;
/// Bolsa inicial: 32 slots (`gs/config.h:12-17`, o mesmo tamanho que o painel mostra).
const SLOTS_DA_BOLSA: u16 = 32;
/// Slots do corpo que o painel conhece (`EQUIPIVTR_*`, `EC_IvtrTypes.h:56-96`).
const SLOTS_DO_CORPO: u16 = 39;
/// `EQUIP_MASK64_DYNSKILL_ALL`: o mesmo item não vai aos dois slots (`gs/item.h:223-224`,
/// `player.cpp:8158-8168`).
const SLOTS_DE_HABILIDADE_DINAMICA: [u16; 2] = [27, 28];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstatisticasPedidas {
    pub nivel: i32,
    pub cultivo: i32,
    pub exp: i64,
    pub sp: i64,
    pub dinheiro: i64,
    pub pontos: i32,
    pub forca: i32,
    pub agilidade: i32,
    pub vitalidade: i32,
    pub energia: i32,
    pub mapa: i32,
    pub x: f32,
    /// Ausente = o chão do mapa em `(x, z)`.
    #[serde(default)]
    pub y: Option<f32>,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDoTemplate {
    pub recipiente: String,
    pub slot: u16,
    pub id: u32,
    pub quantidade: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotDoTemplate {
    pub recipiente: String,
    pub slot: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoverNoTemplate {
    pub de: String,
    pub slot_de: u16,
    pub para: String,
    pub slot_para: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HabilidadeDoTemplate {
    pub id: u32,
    /// 0 remove.
    pub nivel: u8,
}

/// Uma edição de template (uma só por operação).
#[derive(Debug, Clone, PartialEq)]
pub enum EdicaoDeTemplate {
    Estatisticas(EstatisticasPedidas),
    Item(ItemDoTemplate),
    RemoverItem(SlotDoTemplate),
    MoverItem(MoverNoTemplate),
    Habilidade(HabilidadeDoTemplate),
}

/// `container_type` do recipiente do template: só bolsa (0) e corpo (1) — o personagem novo
/// não nasce com armazém, bolsa de missão, mascote nem missão (decisão do Murillo, 2026-10-11).
fn recipiente(nome: &str) -> Option<(i16, u16)> {
    match nome {
        "bolsa" => Some((0, SLOTS_DA_BOLSA)),
        "equipamento" => Some((1, SLOTS_DO_CORPO)),
        _ => None,
    }
}

fn falha(codigo: &str) -> Value {
    json!({"estado":"falha","codigo":codigo})
}

impl RoteadorDeMapas {
    fn um_mapa(&self) -> Option<std::sync::Arc<crate::bus_server::BusServer>> {
        self.mapa(self.padrao).or_else(|| self.todos().into_iter().next().map(|(_, m)| m))
    }

    /// Os templates das classes que a versão do realm cria, com nomes e ícones dos itens, a
    /// vida e a mana de nascimento (a conta da classe) e os limites da edição.
    pub async fn consultar_templates(&self, realm: &str) -> Value {
        let Some(bus) = self.um_mapa() else { return json!({"codigo":"sem_mapa"}) };
        let versao = bus.versao();
        let dados = bus.dados_do_realm().await;
        let repo = self.repo.template_repo();
        let mut templates = Vec::new();
        for cls in (0..12u8).filter_map(CharacterClass::from_u8).filter(|c| versao.is_class_supported(*c)) {
            let t = match repo.template_completo(realm, cls as i32).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("painel: templates de {realm} ilegíveis ({e})");
                    return json!({"codigo":"banco_indisponivel"});
                }
            };
            let Some(t) = t else {
                templates.push(json!({"classe": cls as i32, "existe": false}));
                continue;
            };
            let e = t.template.estatisticas();
            let (vida, mana) = vida_e_mana(&dados, cls as i32, e.nivel, e.vitalidade, e.energia);
            let mut recipientes = serde_json::Map::new();
            for (nome, tipo) in [("bolsa", 0i16), ("equipamento", 1)] {
                let itens: Vec<Value> = t.items.iter().filter(|i| i.container_type == tipo).map(|i| {
                    let tid = i.item_id as u32;
                    json!({"slot": i.slot, "id": tid, "quantidade": i.count,
                           "nome": dados.nomes_de_itens.get(&tid).cloned().unwrap_or_default(),
                           "icone": crate::bus_server::icone_em_hex(&dados, tid),
                           "pilha": dados.limite_de_pilha(tid)})
                }).collect();
                recipientes.insert(nome.into(), Value::Array(itens));
            }
            let habilidades: Vec<Value> = t.skills.iter().map(|h| json!({"id": h.skill_id, "nivel": h.level})).collect();
            templates.push(json!({
                "classe": cls as i32, "nome": t.template.name, "existe": true,
                "estatisticas": e, "vida": vida, "mana": mana,
                "recipientes": recipientes, "habilidades": habilidades,
            }));
        }
        let (f, a, v, en) = bus.piso_da_restauracao();
        json!({"estado":"consultado","versao": versao.to_string(), "nivel_maximo": bus.nivel_maximo().await,
               "piso": {"forca": f, "agilidade": a, "vitalidade": v, "energia": en},
               "templates": templates})
    }

    /// Aplica uma edição ao template da classe `cls` do realm. `salvo` = gravado no banco; o
    /// próximo personagem desta classe nasce com o valor novo.
    pub async fn editar_template(&self, realm: &str, cls: i32, edicao: &EdicaoDeTemplate) -> Value {
        let Some(bus) = self.um_mapa() else { return falha("sem_mapa") };
        let versao = bus.versao();
        if !CharacterClass::from_u8(cls as u8).is_some_and(|c| (0..12).contains(&cls) && versao.is_class_supported(c)) {
            return falha("classe_invalida");
        }
        let repo = self.repo.template_repo();
        let atual = match repo.template_completo(realm, cls).await {
            Ok(Some(t)) => t,
            Ok(None) => return falha("template_inexistente"),
            Err(_) => return falha("banco_indisponivel"),
        };
        let dados = bus.dados_do_realm().await;
        let e = atual.template.estatisticas();
        let gravou = match edicao {
            EdicaoDeTemplate::Estatisticas(p) => {
                let maximo = bus.nivel_maximo().await;
                let (pf, pa, pv, pe) = bus.piso_da_restauracao();
                let atributos_ok = (pf..=TETO_DE_ATRIBUTO).contains(&p.forca) && (pa..=TETO_DE_ATRIBUTO).contains(&p.agilidade)
                    && (pv..=TETO_DE_ATRIBUTO).contains(&p.vitalidade) && (pe..=TETO_DE_ATRIBUTO).contains(&p.energia);
                let coordenada = |v: f32| v.is_finite() && v.abs() <= TETO_DE_COORDENADA;
                if !(1..=maximo).contains(&p.nivel) {
                    return json!({"estado":"falha","codigo":"nivel_invalido","nivel_maximo":maximo});
                }
                if !crate::progressao::cultivo_valido(versao, p.cultivo) {
                    return falha("cultivo_invalido");
                }
                if ![p.exp, p.sp, p.dinheiro].iter().all(|v| (0..=TETO).contains(v)) || !(0..=TETO_DE_PONTOS).contains(&p.pontos) {
                    return falha("edicao_invalida");
                }
                if !atributos_ok {
                    return json!({"estado":"falha","codigo":"atributos_invalidos","piso":[pf, pa, pv, pe]});
                }
                if !(coordenada(p.x) && coordenada(p.z) && p.y.map_or(true, coordenada)) {
                    return falha("edicao_invalida");
                }
                // O mapa de nascimento tem de ser servido aqui, e `(x, z)` cair no terreno dele —
                // o mesmo destino que a posição do personagem (`mover_pelo_painel`).
                let Some(destino) = self.mapa(p.mapa) else {
                    return json!({"estado":"falha","codigo":"mapa_indisponivel","mapa":p.mapa});
                };
                let y = match (destino.chao_em(p.x, p.z).await, p.y) {
                    (Err(()), _) => return json!({"estado":"falha","codigo":"fora_do_mapa","mapa":p.mapa}),
                    (Ok(Some(chao)), Some(y)) => y.max(chao),
                    (Ok(Some(chao)), None) => chao,
                    (Ok(None), Some(y)) => y,
                    (Ok(None), None) => return json!({"estado":"falha","codigo":"altura_obrigatoria","mapa":p.mapa}),
                };
                let novo = EstatisticasDoTemplate {
                    nivel: p.nivel, cultivo: p.cultivo, exp: p.exp, sp: p.sp, dinheiro: p.dinheiro, pontos: p.pontos,
                    forca: p.forca, agilidade: p.agilidade, vitalidade: p.vitalidade, energia: p.energia,
                    mapa: p.mapa, x: p.x, y, z: p.z,
                };
                // As peças do corpo continuam vestíveis com o nível e os atributos novos.
                for i in atual.items.iter().filter(|i| i.container_type == 1) {
                    if let Err(codigo) = conferir_requisito(&dados, i.item_id as u32, cls, &novo) {
                        return json!({"estado":"falha","codigo":format!("{codigo}_dos_itens"),"item":i.item_id});
                    }
                }
                repo.definir_estatisticas(realm, cls, &novo).await
            }
            EdicaoDeTemplate::Item(i) => {
                let Some((tipo, tamanho)) = recipiente(&i.recipiente) else { return falha("recipiente_errado") };
                if i.slot >= tamanho || i.id == 0 {
                    return falha("slot_invalido");
                }
                if !dados.nomes_de_itens.contains_key(&i.id) {
                    return json!({"estado":"falha","codigo":"item_inexistente","item":i.id});
                }
                if dados.e_item_de_missao(i.id) {
                    return falha("recipiente_errado");
                }
                if i.quantidade == 0 || i.quantidade > dados.limite_de_pilha(i.id).max(1) {
                    return falha("quantidade_invalida");
                }
                if tipo == 1 {
                    if let Err(codigo) = conferir_vestir(&dados, &atual, i.id, i.slot, cls, &e) {
                        return falha(codigo);
                    }
                }
                // Equipamento nasce com a durabilidade de fábrica, na escala interna (B61).
                let durabilidade = dados.durabilidade_de_fabrica(i.id).map_or(0, |d| d as i32 * pw_core::ESCALA_DA_DURABILIDADE);
                repo.definir_item(realm, cls, tipo, i.slot as i16, i.id as i32, i.quantidade as i32, durabilidade).await
            }
            EdicaoDeTemplate::RemoverItem(s) => {
                let Some((tipo, tamanho)) = recipiente(&s.recipiente) else { return falha("recipiente_errado") };
                if s.slot >= tamanho {
                    return falha("slot_invalido");
                }
                match repo.remover_item(realm, cls, tipo, s.slot as i16).await {
                    Ok(false) => return falha("slot_vazio"),
                    r => r,
                }
            }
            EdicaoDeTemplate::MoverItem(m) => {
                let (Some((de, td)), Some((para, tp))) = (recipiente(&m.de), recipiente(&m.para)) else {
                    return falha("recipiente_errado");
                };
                if m.slot_de >= td || m.slot_para >= tp || (de == para && m.slot_de == m.slot_para) {
                    return falha("slot_invalido");
                }
                let em = |c: i16, s: u16| atual.items.iter().find(|i| i.container_type == c && i.slot == s as i16);
                let Some(origem) = em(de, m.slot_de) else { return falha("slot_vazio") };
                // Cada peça que acaba no corpo tem de caber e ser vestível — nos dois sentidos.
                let mut sem_as_duas = atual.clone();
                sem_as_duas.items.retain(|i| !((i.container_type == de && i.slot == m.slot_de as i16)
                    || (i.container_type == para && i.slot == m.slot_para as i16)));
                if para == 1 {
                    if let Err(codigo) = conferir_vestir(&dados, &sem_as_duas, origem.item_id as u32, m.slot_para, cls, &e) {
                        return falha(codigo);
                    }
                }
                if let (1, Some(alvo)) = (de, em(para, m.slot_para)) {
                    if let Err(codigo) = conferir_vestir(&dados, &sem_as_duas, alvo.item_id as u32, m.slot_de, cls, &e) {
                        return falha(codigo);
                    }
                }
                repo.mover_item(realm, cls, (de, m.slot_de as i16), (para, m.slot_para as i16)).await
            }
            EdicaoDeTemplate::Habilidade(h) => {
                if h.id == 0 || h.id >= 0x10000 {
                    return falha("edicao_invalida");
                }
                match repo.definir_habilidade(realm, cls, h.id as i32, h.nivel as i16).await {
                    Ok(false) => return falha("habilidade_ausente"),
                    r => r,
                }
            }
        };
        match gravou {
            Ok(true) => {
                tracing::info!(realm, classe = cls, ?edicao, "painel: template editado");
                json!({"estado":"salvo","classe":cls})
            }
            Ok(false) => falha("template_inexistente"),
            Err(_) => falha("banco_indisponivel"),
        }
    }
}

/// Vida e mana máximas (cheias) de nascimento: a conta da classe do realm.
fn vida_e_mana(dados: &pw_data_loader::GameDataManager, cls: i32, nivel: i32, vit: i32, eng: i32) -> (i32, i32) {
    dados.base_das_classes.get(cls)
        .map_or((0, 0), |b| b.vida_e_mana_maximas(dados.classes.get(cls), nivel, vit, eng))
}

/// A peça `tid` no slot `slot` do corpo de um template: posição, item de habilidade repetido,
/// reputação e requisitos — o `motivo_para_nao_vestir` do jogo, com o template no lugar do
/// jogador.
fn conferir_vestir(dados: &pw_data_loader::GameDataManager, t: &pw_storage::FullClassTemplate, tid: u32, slot: u16,
                   cls: i32, e: &EstatisticasDoTemplate) -> Result<(), &'static str> {
    if dados.cabe_no_slot(tid, slot as usize) != Some(true) {
        return Err("posicao_invalida");
    }
    if SLOTS_DE_HABILIDADE_DINAMICA.contains(&slot) {
        let repetido = t.items.iter().any(|i| i.container_type == 1 && i.item_id as u32 == tid
            && SLOTS_DE_HABILIDADE_DINAMICA.contains(&(i.slot as u16)) && i.slot as u16 != slot);
        if repetido {
            return Err("habilidade_repetida");
        }
    }
    conferir_requisito(dados, tid, cls, e)
}

/// `VerifyRequirement` (`equip_item.cpp:60-80`): reputação (o personagem novo tem 0), nível,
/// classe e os quatro atributos do bloco de fábrica da peça.
fn conferir_requisito(dados: &pw_data_loader::GameDataManager, tid: u32, cls: i32, e: &EstatisticasDoTemplate)
                      -> Result<(), &'static str> {
    if dados.reputacao_exigida.get(&tid).is_some_and(|r| *r > 0) {
        return Err("reputacao");
    }
    let Some(ficha) = dados.equipamentos.ficha(tid) else { return Ok(()) };
    let d = dados.durabilidade_de_fabrica(tid).map_or(0, |d| d as i32 * pw_core::ESCALA_DA_DURABILIDADE);
    let bloco = pw_core::ConteudoDeEquipamento::novo(ficha, d, d).escrever();
    if pw_core::Requisitos::do_bloco(&bloco).is_some_and(|r| r.atende(e.nivel, cls, (e.vitalidade, e.energia, e.forca, e.agilidade))) {
        Ok(())
    } else {
        Err("requisito")
    }
}
