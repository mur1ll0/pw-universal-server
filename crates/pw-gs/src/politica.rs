//! O intérprete do `aipolicy.data` — a política de gatilhos de um monstro (`_at_policy`).
//!
//! Porte de `cgame/gs/aitrigger.h`/`aitrigger.cpp` (`ai_trigger::policy`, `trigger`,
//! `cond_*`, `op_*`, `target_*`) e do montador `cgame/gs/ai/policy_loader.cpp`
//! (`ConvertCondition`/`ConvertOperation`/`LoadAIPolicy`). O arquivo já é lido por
//! [`pw_data_loader::aipolicy`]; aqui ele vira gatilhos que rodam.
//!
//! # Como o original dispara
//!
//! | quando | lista | `ai_policy` |
//! | :--- | :--- | :--- |
//! | batimento de 1 s **em combate** | timers (sem parar) + batimento (para no primeiro `false`) + os de paz | `OnHeartbeat` + `OnPeaceHeartbeatInCombat` (`aipolicy.cpp:282-287`) |
//! | batimento fora de combate | timers + os de batimento **sem** `bAttackValid` | `OnPeaceHeartbeat` |
//! | começa o combate | `StartCombat` — todos, **sem** olhar se estão ligados | `EnableCombat(true)` (`aipolicy.cpp:395-414`) |
//! | acaba o combate | `Reset` dos de combate e dos timers, depois `EndCombat` (sem olhar) | `RollBack` → `EnableCombat(false, true)` |
//! | morre | os de morte, depois `ResetAll` | `ai_policy::OnDeath` |
//! | apanha | os de dano | `OnDamage` |
//! | mata o alvo | os de matar | `KillTarget` |
//!
//! A condição decide a lista (`GetConditionType`; `and`/`or` pela esquerda, `not` pelo
//! filho). `bRun` no arquivo = gatilho que só roda chamado por outro (`o_run_trigger`), e não
//! entra nas listas (`LoadAIPolicy`). `bActive` = ligado de início; os de começo/fim de
//! combate e de morte nascem ligados (`AddTrigger`). `hp_less`, fim de caminho e hora certa
//! se desligam sozinhos depois de disparar (`IsAutoDisable`).
//!
//! # O que sai daqui
//!
//! As operações que mexem só na política (timers, ligar/desligar, variáveis) e na lista de
//! ódio acontecem aqui. As que precisam do mundo viram [`Pedido`]: tarefa nova (atacar,
//! habilidade, fugir), fala e controlador do `npcgen`. O que não tem porte vira
//! [`Pedido::NaoPortado`] e é registrado uma vez.

use pw_core::Vector3;
use pw_data_loader::aipolicy::{
    AiPolicy, NoDeCondicao, ParametroDeCondicao, ParametroDeOperacao, TipoDeAlvo, TipoDeCondicao, TipoDeOperacao, Trigger,
};
use rand::Rng;
use std::collections::HashMap;

/// `condition::TYPE_*` (`aitrigger.h:19-30`): a lista em que o gatilho entra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disparo {
    Batimento,
    Timer,
    MatouAlvo,
    ComecoDeCombate,
    Morte,
    Dano,
    FimDoCaminho,
    FimDeCombate,
}

/// Uma expressão (`expr_*`, `aitrigger.h:1231-1510`).
#[derive(Debug, Clone)]
pub enum Expr {
    Constante(i32),
    /// `expr_common_data` — variável do mundo (`world::GetCommonValue`).
    Global(i32),
    /// `expr_local_value` — as três do monstro (`gnpc_imp::_local_var`).
    Local(i32),
    /// `expr_history_value` / `expr_room_index`: sem porte, valem 0.
    SemPorte,
    Soma(Box<Expr>, Box<Expr>),
    Subtracao(Box<Expr>, Box<Expr>),
    Multiplicacao(Box<Expr>, Box<Expr>),
    /// Divisão por zero é a `expr::Exception` do original: o `TestTrigger` a engole e o
    /// gatilho não dispara.
    Divisao(Box<Expr>, Box<Expr>),
    JogadoresNoRaio(f32),
    JogadoresNaRegiao { x0: f32, z0: f32, x1: f32, z1: f32 },
}

/// Uma condição (`cond_*`).
#[derive(Debug, Clone)]
pub enum Condicao {
    Timer(u32),
    VidaAbaixo(f32),
    Aleatorio(f32),
    E(Box<Condicao>, Box<Condicao>),
    Ou(Box<Condicao>, Box<Condicao>),
    Nao(Box<Condicao>),
    MatouAlvo,
    ComecoDeCombate,
    Morte,
    FimDeCombate,
    Menor(Expr, Expr),
    Maior(Expr, Expr),
    Igual(Expr, Expr),
    Dano { minimo: i32, maximo: i32 },
    /// `cond_path_end(_2)`: o monstro não anda por caminho aqui, nunca chega.
    FimDoCaminho,
    /// `cond_at_history_stage` / `cond_spec_filter`: sem porte, falsas.
    SemPorte(Disparo),
}

impl Condicao {
    fn disparo(&self) -> Disparo {
        match self {
            Condicao::Timer(_) => Disparo::Timer,
            Condicao::E(a, _) | Condicao::Ou(a, _) => a.disparo(),
            Condicao::Nao(a) => a.disparo(),
            Condicao::MatouAlvo => Disparo::MatouAlvo,
            Condicao::ComecoDeCombate => Disparo::ComecoDeCombate,
            Condicao::Morte => Disparo::Morte,
            Condicao::FimDeCombate => Disparo::FimDeCombate,
            Condicao::Dano { .. } => Disparo::Dano,
            Condicao::FimDoCaminho => Disparo::FimDoCaminho,
            Condicao::SemPorte(d) => *d,
            _ => Disparo::Batimento,
        }
    }

    /// `IsAutoDisable`: `hp_less` e fim de caminho sim; `not` pelo filho; `and` se os dois;
    /// `or` se um.
    fn desliga_sozinha(&self) -> bool {
        match self {
            Condicao::VidaAbaixo(_) | Condicao::FimDoCaminho => true,
            Condicao::Nao(a) => a.desliga_sozinha(),
            Condicao::E(a, b) => a.desliga_sozinha() && b.desliga_sozinha(),
            Condicao::Ou(a, b) => a.desliga_sozinha() || b.desliga_sozinha(),
            _ => false,
        }
    }
}

/// `enumPolicyVarType` (`policytype.h:71-78`) com o índice ou o valor.
#[derive(Debug, Clone, Copy)]
pub struct ValorLogico {
    pub tipo: i32,
    pub valor: i32,
}

/// O que uma operação faz (`op_*`).
#[derive(Debug, Clone)]
pub enum Acao {
    Atacar { estrategia: i32 },
    Habilidade { id: i32, nivel: i32 },
    Habilidade2 { id: ValorLogico, nivel: ValorLogico },
    Falar { texto: String, anexos: u32 },
    LimparOdio,
    Rodar(Box<Gatilho>),
    Ligar { id: u32, ligar: bool },
    CriarTimer { id: u32, periodo: i32, vezes: i32 },
    MatarTimer(u32),
    Fugir,
    OdioParaPrimeiro,
    OdioParaUltimo,
    OdioPelaMetade,
    Pular,
    DefinirGlobal { id: i32, valor: i32, direto: bool },
    AjustarGlobal { id: i32, valor: i32 },
    Calcular { destino: ValorLogico, a: ValorLogico, operador: i32, b: ValorLogico },
    Controlador { id: ValorLogico, parar: bool },
    SemPorte(&'static str),
}

#[derive(Debug, Clone)]
pub struct Operacao {
    pub acao: Acao,
    /// `ConvertTarget` (`policy_loader.cpp`); `None` = sem alvo (o original usa o primeiro
    /// da lista de ódio nas operações que precisam de um).
    pub alvo: Option<TipoDeAlvo>,
    pub profissoes: u32,
}

/// Um gatilho (`ai_trigger::trigger`).
#[derive(Debug, Clone)]
pub struct Gatilho {
    pub id: u32,
    pub condicao: Condicao,
    pub operacoes: Vec<Operacao>,
    pub ligado_de_inicio: bool,
    /// `bAttackValid` (`_battle_trigger`).
    pub so_em_combate: bool,
    pub disparo: Disparo,
}

/// A política compilada, uma por id do `aipolicy.data`.
#[derive(Debug, Clone, Default)]
pub struct PoliticaDeIa {
    pub id: u32,
    pub gatilhos: Vec<Gatilho>,
    /// `_peace_trigger_count`: de batimento sem `bAttackValid`.
    de_paz: usize,
}

/// O estado da política num monstro: gatilhos ligados, timers e as três variáveis locais.
#[derive(Debug, Clone, Default)]
pub struct EstadoDaPolitica {
    ligados: Vec<bool>,
    timers: Vec<Timer>,
    /// `_timer_flag`: 1 = algum venceu, 2 = algum acabou.
    bandeira: u8,
    /// `_local_var[3]` (`npc.h:229`), do `MONSTER_ESSENCE` (`npcgenerator.cpp:342`).
    pub locais: [i32; 3],
}

#[derive(Debug, Clone, Copy)]
struct Timer {
    id: u32,
    falta: i32,
    periodo: i32,
    vezes: i32,
}

/// O que a política precisa saber de um alvo (`ai_object::target_info`).
#[derive(Debug, Clone, Copy)]
pub struct InfoDoAlvo {
    pub hp: i32,
    pub mp: i32,
    /// A classe do jogador (o `race & 0x7FFFFFFF` que o `target_class_combo` testa); `None`
    /// para quem não é jogador.
    pub classe: Option<i32>,
    pub posicao: Vector3,
    /// O dono, quando o alvo é mascote (`target_aggro_first_redirected`).
    pub dono: Option<i64>,
}

/// Onde a fala vai (`op_say`, `aitrigger.cpp:552-608`, pelo prefixo do texto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanalDaFala {
    /// Sem prefixo: canal local a quem vê o monstro, com o id dele.
    Local,
    /// `$A`: local, anônima (`srcroleid` 0).
    LocalAnonima,
    /// `$B`: `CHAT_CHANNEL_FARCRY` (1) a todo o mundo.
    Grito,
    /// `$S`: `CHAT_CHANNEL_BROADCAST` (9), mensagem do sistema.
    Sistema,
    /// `$I`/`$X`: `CHAT_CHANNEL_INSTANCE` (11) a todos do mapa.
    Instancia,
}

/// O que a política pede ao mundo.
#[derive(Debug, Clone, PartialEq)]
pub enum Pedido {
    /// `op_attack` → `AddPrimaryTask(target, uType)`.
    Atacar { alvo: i64, estrategia: i32 },
    /// `op_skill(_2)` → `ai_skill_task_2`.
    Habilidade { alvo: i64, id: i32, nivel: i32 },
    /// `op_flee` → `ai_runaway_task`.
    Fugir { alvo: i64 },
    Falar { texto: String, canal: CanalDaFala, dados: Vec<u8> },
    /// `op_active_spawner(_2)` → `TriggerSpawn`/`ClearSpawn` do `iControllerID`.
    Controlador { id: i32, ligar: bool },
    NaoPortado(&'static str),
}

/// O mundo visto pela política num instante.
pub struct Contexto<'a> {
    pub eu: i64,
    pub hp: i64,
    pub max_hp: i64,
    pub posicao: Vector3,
    pub odio: &'a mut HashMap<i64, i64>,
    pub info: &'a dyn Fn(i64) -> Option<InfoDoAlvo>,
    /// Jogadores vivos num raio (`GetSpherePlayerListSize`) ou numa caixa x/z.
    pub jogadores_perto: &'a dyn Fn(Option<f32>, Option<(f32, f32, f32, f32)>) -> i32,
    pub globais: &'a mut HashMap<i32, i32>,
    pub locais: [i32; 3],
    /// `GetLastDamage` — o dano que acabou de chegar (condição de dano).
    pub ultimo_dano: i32,
    /// `GetChiefGainer` — quem leva o crédito do monstro.
    pub matador: Option<i64>,
    pub pedidos: Vec<Pedido>,
}

const VALOR_INVALIDO: i32 = 0x7FFF_FFFF;

impl Contexto<'_> {
    /// A lista de ódio em ordem (`GetAggroEntry(i)`): maior ódio primeiro; empate pelo id.
    fn lista_de_odio(&self) -> Vec<i64> {
        let mut v: Vec<(i64, i64)> = self.odio.iter().map(|(id, t)| (*id, *t)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.into_iter().map(|(id, _)| id).collect()
    }

    fn global(&self, k: i32) -> i32 {
        self.globais.get(&k).copied().unwrap_or(0)
    }

    fn local(&self, i: i32) -> i32 {
        usize::try_from(i).ok().and_then(|i| self.locais.get(i).copied()).unwrap_or(0)
    }

    /// `logic_val::getval` (`aitrigger.cpp:14-68`).
    fn valor(&self, v: ValorLogico) -> i32 {
        match v.tipo {
            0 => self.global(v.valor),
            1 => self.local(v.valor),
            2 => v.valor,
            // `abase::RandNormal(0, index)` (`arandomgen.h:110-116`): a média de dois uniformes.
            3 => {
                let mut r = rand::thread_rng();
                let p = (r.gen::<f64>() + r.gen::<f64>()) * 0.5;
                (p * (v.valor as f64 + 1.0)) as i32
            }
            _ => VALOR_INVALIDO,
        }
    }

    fn definir(&mut self, v: ValorLogico, valor: i32) {
        if valor == VALOR_INVALIDO {
            return;
        }
        match v.tipo {
            0 => {
                self.globais.insert(v.valor, valor);
            }
            1 => {
                if let Some(l) = usize::try_from(v.valor).ok().and_then(|i| self.locais.get_mut(i)) {
                    *l = valor;
                }
            }
            _ => {}
        }
    }

    fn expr(&self, e: &Expr) -> Option<i32> {
        Some(match e {
            Expr::Constante(v) => *v,
            Expr::Global(k) => self.global(*k),
            Expr::Local(i) => self.local(*i),
            Expr::SemPorte => 0,
            Expr::Soma(a, b) => self.expr(a)?.wrapping_add(self.expr(b)?),
            Expr::Subtracao(a, b) => self.expr(a)?.wrapping_sub(self.expr(b)?),
            Expr::Multiplicacao(a, b) => self.expr(a)?.wrapping_mul(self.expr(b)?),
            Expr::Divisao(a, b) => {
                let d = self.expr(b)?;
                if d == 0 {
                    return None;
                }
                self.expr(a)?.wrapping_div(d)
            }
            Expr::JogadoresNoRaio(r) => (self.jogadores_perto)(Some(*r), None),
            Expr::JogadoresNaRegiao { x0, z0, x1, z1 } => (self.jogadores_perto)(None, Some((*x0, *z0, *x1, *z1))),
        })
    }

    /// `target_*::GetTarget` (`aitrigger.cpp:178-436`). `None` = `XID(-1,-1)`.
    fn alvo(&self, tipo: Option<TipoDeAlvo>, profissoes: u32) -> Option<i64> {
        let lista = self.lista_de_odio();
        let mut rng = rand::thread_rng();
        let info = self.info;
        match tipo? {
            TipoDeAlvo::EuMesmo => Some(self.eu),
            TipoDeAlvo::OdioPrimeiro => lista.first().copied(),
            TipoDeAlvo::OdioPrimeiroRedirecionado => {
                let a = *lista.first()?;
                Some(info(a).and_then(|i| i.dono).unwrap_or(a))
            }
            TipoDeAlvo::OdioSegundo => lista.get(1).or(lista.first()).copied(),
            TipoDeAlvo::OdioOutros => {
                if lista.len() > 1 {
                    Some(lista[rng.gen_range(1..lista.len())])
                } else {
                    lista.first().copied()
                }
            }
            TipoDeAlvo::OdioAleatorio => (!lista.is_empty()).then(|| lista[rng.gen_range(0..lista.len())]),
            TipoDeAlvo::OdioMaisProximo | TipoDeAlvo::OdioMaisDistante => {
                let longe = tipo == Some(TipoDeAlvo::OdioMaisDistante);
                let mut melhor: Option<(i64, f32)> = None;
                for id in &lista {
                    let Some(i) = info(*id) else { continue };
                    let (dx, dz) = (i.posicao.x - self.posicao.x, i.posicao.z - self.posicao.z);
                    let d = (dx * dx + dz * dz).sqrt();
                    if melhor.is_none_or(|(_, m)| if longe { d > m } else { d < m }) {
                        melhor = Some((*id, d));
                    }
                }
                melhor.map(|(id, _)| id)
            }
            TipoDeAlvo::MenorHp => lista.iter().filter_map(|id| info(*id).map(|i| (*id, i.hp))).min_by_key(|(_, hp)| *hp).map(|(id, _)| id),
            TipoDeAlvo::MaiorHp => {
                let mut melhor: Option<(i64, i32)> = None;
                for id in &lista {
                    if let Some(i) = info(*id) {
                        if melhor.is_none_or(|(_, m)| i.hp > m) {
                            melhor = Some((*id, i.hp));
                        }
                    }
                }
                melhor.map(|(id, _)| id)
            }
            TipoDeAlvo::MaiorMp => {
                let mut melhor: Option<(i64, i32)> = None;
                for id in &lista {
                    if let Some(i) = info(*id) {
                        if melhor.is_none_or(|(_, m)| i.mp > m) {
                            melhor = Some((*id, i.mp));
                        }
                    }
                }
                melhor.map(|(id, _)| id)
            }
            TipoDeAlvo::ListaDeProfissoes => {
                let escolhidos: Vec<i64> = lista
                    .iter()
                    .filter(|id| info(**id).and_then(|i| i.classe).is_some_and(|c| (0..32).contains(&c) && (1u32 << c) & profissoes != 0))
                    .take(16)
                    .copied()
                    .collect();
                if escolhidos.is_empty() {
                    lista.first().copied()
                } else {
                    Some(escolhidos[rng.gen_range(0..escolhidos.len())])
                }
            }
            TipoDeAlvo::QuemMatouOMonstro => self.matador,
            // `GetMafiaID` (a facção dona do lugar) e os alvos só do 1.7.2: sem porte.
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------
// Montagem (`policy_loader.cpp`)
// ---------------------------------------------------------------------------------

fn expr_do_no(n: &NoDeCondicao) -> Expr {
    let lado = |x: &Option<Box<NoDeCondicao>>| Box::new(x.as_deref().map_or(Expr::Constante(0), expr_do_no));
    match (n.tipo, &n.parametro) {
        (Some(TipoDeCondicao::Constant), ParametroDeCondicao::Valor(v)) => Expr::Constante(*v),
        (Some(TipoDeCondicao::Var), ParametroDeCondicao::Id(k)) => Expr::Global(*k),
        (Some(TipoDeCondicao::LocalVar), ParametroDeCondicao::Id(k)) => Expr::Local(*k),
        (Some(TipoDeCondicao::Plus), _) => Expr::Soma(lado(&n.esquerda), lado(&n.direita)),
        (Some(TipoDeCondicao::Minus), _) => Expr::Subtracao(lado(&n.esquerda), lado(&n.direita)),
        (Some(TipoDeCondicao::Multiply), _) => Expr::Multiplicacao(lado(&n.esquerda), lado(&n.direita)),
        (Some(TipoDeCondicao::Divide), _) => Expr::Divisao(lado(&n.esquerda), lado(&n.direita)),
        (Some(TipoDeCondicao::PlayerCountInRadius), ParametroDeCondicao::Raio(r)) => Expr::JogadoresNoRaio(*r),
        (Some(TipoDeCondicao::PlayerCountInRegion), ParametroDeCondicao::Regiao { minimo, maximo }) => {
            Expr::JogadoresNaRegiao { x0: minimo.x, z0: minimo.z, x1: maximo.x, z1: maximo.z }
        }
        _ => Expr::SemPorte,
    }
}

fn condicao_do_no(n: &NoDeCondicao) -> Condicao {
    let filho = |x: &Option<Box<NoDeCondicao>>| Box::new(x.as_deref().map_or(Condicao::SemPorte(Disparo::Batimento), condicao_do_no));
    let ex = |x: &Option<Box<NoDeCondicao>>| x.as_deref().map_or(Expr::Constante(0), expr_do_no);
    match (n.tipo, &n.parametro) {
        (Some(TipoDeCondicao::TimeCome), ParametroDeCondicao::IdDeTimer(id)) => Condicao::Timer(*id),
        (Some(TipoDeCondicao::HpLess), ParametroDeCondicao::HpAbaixoDe(f)) => Condicao::VidaAbaixo(*f),
        (Some(TipoDeCondicao::Random), ParametroDeCondicao::Probabilidade(p)) => Condicao::Aleatorio(*p),
        (Some(TipoDeCondicao::And), _) => Condicao::E(filho(&n.esquerda), filho(&n.direita)),
        (Some(TipoDeCondicao::Or), _) => Condicao::Ou(filho(&n.esquerda), filho(&n.direita)),
        // `cond_not(ConvertCondition(temp->pRight))`.
        (Some(TipoDeCondicao::Not), _) => Condicao::Nao(filho(if n.direita.is_some() { &n.direita } else { &n.esquerda })),
        (Some(TipoDeCondicao::KillPlayer), _) => Condicao::MatouAlvo,
        (Some(TipoDeCondicao::StartAttack), _) => Condicao::ComecoDeCombate,
        (Some(TipoDeCondicao::Died), _) => Condicao::Morte,
        (Some(TipoDeCondicao::StopFight), _) => Condicao::FimDeCombate,
        (Some(TipoDeCondicao::Less), _) => Condicao::Menor(ex(&n.esquerda), ex(&n.direita)),
        (Some(TipoDeCondicao::Great), _) => Condicao::Maior(ex(&n.esquerda), ex(&n.direita)),
        (Some(TipoDeCondicao::Equ), _) => Condicao::Igual(ex(&n.esquerda), ex(&n.direita)),
        (Some(TipoDeCondicao::BeHurt), ParametroDeCondicao::FaixaDeDano { minimo, maximo }) => {
            Condicao::Dano { minimo: *minimo, maximo: *maximo }
        }
        (Some(TipoDeCondicao::ReachEnd | TipoDeCondicao::ReachEnd2), _) => Condicao::FimDoCaminho,
        _ => Condicao::SemPorte(Disparo::Batimento),
    }
}

fn valor_logico(tipo: u32, valor: u32) -> ValorLogico {
    ValorLogico { tipo: tipo as i32, valor: valor as i32 }
}

fn operacao(o: &pw_data_loader::aipolicy::Operacao, politica: &AiPolicy, profundidade: u32) -> Operacao {
    use ParametroDeOperacao as P;
    use TipoDeOperacao as T;
    let acao = match (o.tipo, &o.parametro) {
        (Some(T::Atacar), P::TipoDeAtaque(t)) => Acao::Atacar { estrategia: *t as i32 },
        (Some(T::UsarSkill), P::Skill { skill, nivel }) => Acao::Habilidade { id: *skill as i32, nivel: *nivel as i32 },
        (Some(T::UsarSkill2), P::Skill2 { skill, tipo_da_skill, nivel, tipo_do_nivel }) => Acao::Habilidade2 {
            id: valor_logico(*tipo_da_skill, *skill),
            nivel: valor_logico(*tipo_do_nivel, *nivel),
        },
        (Some(T::Falar), P::Fala { texto, mascara_de_anexos }) => Acao::Falar { texto: texto.clone(), anexos: *mascara_de_anexos },
        (Some(T::LimparListaDeOdio), _) => Acao::LimparOdio,
        (Some(T::RodarTrigger), P::IdDeTrigger(id)) => {
            match politica.triggers.iter().find(|t| t.id == *id).filter(|_| profundidade < 8) {
                Some(t) => Acao::Rodar(Box::new(gatilho(t, politica, profundidade + 1))),
                None => Acao::SemPorte("RodarTrigger sem o gatilho"),
            }
        }
        (Some(T::PararTrigger), P::IdDeTrigger(id)) => Acao::Ligar { id: *id, ligar: false },
        (Some(T::AtivarTrigger), P::IdDeTrigger(id)) => Acao::Ligar { id: *id, ligar: true },
        (Some(T::CriarTimer), P::Timer { id, periodo, contador }) => {
            Acao::CriarTimer { id: *id, periodo: *periodo as i32, vezes: *contador as i32 }
        }
        (Some(T::MatarTimer), P::IdDeTrigger(id)) => Acao::MatarTimer(*id),
        (Some(T::MatarTimer), P::Timer { id, .. }) => Acao::MatarTimer(*id),
        (Some(T::Fugir), _) => Acao::Fugir,
        (Some(T::OdioParaPrimeiro), _) => Acao::OdioParaPrimeiro,
        (Some(T::OdioParaUltimo), _) => Acao::OdioParaUltimo,
        (Some(T::OdioCinquentaPorCento), _) => Acao::OdioPelaMetade,
        (Some(T::PularOperacao), _) => Acao::Pular,
        (Some(T::DefinirGlobal), P::DefinirGlobal { id, valor, e_valor_direto }) => {
            Acao::DefinirGlobal { id: *id, valor: *valor, direto: *e_valor_direto }
        }
        (Some(T::RevisarGlobal), P::AjustarVariavel { id, valor }) => Acao::AjustarGlobal { id: *id, valor: *valor },
        (Some(T::CalcularVariavel), P::CalcularVariavel { destino, tipo_do_destino, origem1, tipo_da_origem1, operador, origem2, tipo_da_origem2 }) => {
            Acao::Calcular {
                destino: ValorLogico { tipo: *tipo_do_destino, valor: *destino },
                a: ValorLogico { tipo: *tipo_da_origem1, valor: *origem1 },
                operador: *operador,
                b: ValorLogico { tipo: *tipo_da_origem2, valor: *origem2 },
            }
        }
        (Some(T::AtivarControlador), P::Controlador { id, parar }) => {
            Acao::Controlador { id: ValorLogico { tipo: 2, valor: *id as i32 }, parar: *parar }
        }
        (Some(T::AtivarControlador2), P::Controlador2 { id, tipo_do_id, parar }) => {
            Acao::Controlador { id: valor_logico(*tipo_do_id, *id), parar: *parar }
        }
        (Some(T::InvocarMonstro | T::InvocarMonstro2), _) => Acao::SemPorte("InvocarMonstro"),
        (Some(T::InvocarNpc), _) => Acao::SemPorte("InvocarNpc"),
        (Some(T::InvocarMina), _) => Acao::SemPorte("InvocarMina"),
        (Some(T::AndarPorCaminho | T::AndarPorCaminho2), _) => Acao::SemPorte("AndarPorCaminho"),
        (Some(T::TocarAcao), _) => Acao::SemPorte("TocarAcao"),
        (Some(T::RevisarHistorico | T::DefinirHistorico), _) => Acao::SemPorte("Historico"),
        (Some(T::EntregarPontosPvpDeFaccao), _) => Acao::SemPorte("PontosPvpDeFaccao"),
        (Some(T::EntregarMissao | T::EntregarMissaoAleatoriaNaRegiao | T::EntregarMissaoNaListaDeOdio), _) => {
            Acao::SemPorte("EntregarMissao")
        }
        (Some(T::SalvarContagemDeJogadoresNoRaio | T::SalvarContagemDeJogadoresNaRegiao), _) => Acao::SemPorte("ContarJogadores"),
        (Some(T::LimparMissaoDeTorreNaRegiao), _) => Acao::SemPorte("LimparMissaoDeTorre"),
        _ => Acao::SemPorte("operação sem porte"),
    };
    Operacao { acao, alvo: o.alvo.tipo, profissoes: o.alvo.mascara_de_profissoes().unwrap_or(0) }
}

fn gatilho(t: &Trigger, politica: &AiPolicy, profundidade: u32) -> Gatilho {
    let condicao = t.condicao.as_ref().map_or(Condicao::SemPorte(Disparo::Batimento), condicao_do_no);
    let disparo = condicao.disparo();
    let ligado_de_inicio = t.ativo || matches!(disparo, Disparo::ComecoDeCombate | Disparo::Morte | Disparo::FimDeCombate);
    Gatilho {
        id: t.id,
        operacoes: t.operacoes.iter().map(|o| operacao(o, politica, profundidade)).collect(),
        condicao,
        ligado_de_inicio,
        so_em_combate: t.so_em_combate,
        disparo,
    }
}

impl PoliticaDeIa {
    /// `LoadAIPolicy` (`policy_loader.cpp`): cada gatilho sem `bRun` entra na política.
    pub fn compilar(p: &AiPolicy) -> Self {
        let gatilhos: Vec<Gatilho> = p.triggers.iter().filter(|t| !t.rodando).map(|t| gatilho(t, p, 0)).collect();
        let de_paz = gatilhos.iter().filter(|g| g.disparo == Disparo::Batimento && !g.so_em_combate).count();
        Self { id: p.id, gatilhos, de_paz }
    }

    /// As habilidades que as operações citam com id e nível fixos — o mundo as resolve no
    /// catálogo quando o monstro nasce.
    pub fn habilidades_citadas(&self) -> Vec<(i32, i32)> {
        fn visitar(g: &Gatilho, v: &mut Vec<(i32, i32)>) {
            for o in &g.operacoes {
                match &o.acao {
                    Acao::Habilidade { id, nivel } => v.push((*id, *nivel)),
                    Acao::Habilidade2 { id, nivel } if id.tipo == 2 && nivel.tipo == 2 => v.push((id.valor, nivel.valor)),
                    Acao::Rodar(s) => visitar(s, v),
                    _ => {}
                }
            }
        }
        let mut v = Vec::new();
        for g in &self.gatilhos {
            visitar(g, &mut v);
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    pub fn novo_estado(&self, locais: [i32; 3]) -> EstadoDaPolitica {
        EstadoDaPolitica {
            ligados: self.gatilhos.iter().map(|g| g.ligado_de_inicio).collect(),
            timers: Vec::new(),
            bandeira: 0,
            locais,
        }
    }

    /// `policy::OnHeartbeat` + `OnPeaceHeartbeatInCombat` (em combate) ou
    /// `OnPeaceHeartbeat` (fora).
    pub fn batimento(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto, em_combate: bool) {
        e.atualizar_timers();
        if e.bandeira & 0x01 != 0 {
            for i in self.indices(Disparo::Timer) {
                if e.ligados[i] {
                    self.testar(i, e, ctx);
                }
            }
        }
        if e.bandeira & 0x02 != 0 {
            e.bandeira = 0;
            e.timers.retain(|t| t.vezes >= 0);
        }
        if em_combate {
            for i in self.indices(Disparo::Batimento) {
                if e.ligados[i] && !self.testar(i, e, ctx) {
                    break;
                }
            }
        }
        if self.de_paz > 0 {
            for i in self.indices(Disparo::Batimento) {
                if self.gatilhos[i].so_em_combate || !e.ligados[i] {
                    continue;
                }
                if !self.testar(i, e, ctx) {
                    break;
                }
            }
        }
    }

    /// `StartCombat`: `CheckTriggersNoTest` — sem olhar se o gatilho está ligado.
    pub fn comeco_de_combate(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        for i in self.indices(Disparo::ComecoDeCombate) {
            if !self.testar(i, e, ctx) {
                break;
            }
        }
    }

    /// `EnableCombat(false, true)`: `Reset` (os de combate voltam ao início, os timers
    /// somem) e `EndCombat` (`CheckTriggersNoTest`).
    pub fn fim_de_combate(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        for (i, g) in self.gatilhos.iter().enumerate() {
            if g.so_em_combate {
                e.ligados[i] = g.ligado_de_inicio;
            }
        }
        e.bandeira = 0;
        e.timers.clear();
        for i in self.indices(Disparo::FimDeCombate) {
            if !self.testar(i, e, ctx) {
                break;
            }
        }
    }

    /// `OnDeath` + `ResetAll`.
    pub fn morte(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        self.checar(Disparo::Morte, e, ctx);
        for (i, g) in self.gatilhos.iter().enumerate() {
            e.ligados[i] = g.ligado_de_inicio;
        }
        e.bandeira = 0;
        e.timers.clear();
    }

    /// `OnDamage`, com o `GetLastDamage` no contexto.
    pub fn dano(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        self.checar(Disparo::Dano, e, ctx);
    }

    /// `KillTarget`.
    pub fn matou_alvo(&self, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        self.checar(Disparo::MatouAlvo, e, ctx);
    }

    /// `CheckTriggers`: os ligados, até o primeiro que devolve falso.
    fn checar(&self, d: Disparo, e: &mut EstadoDaPolitica, ctx: &mut Contexto) {
        for i in self.indices(d) {
            if e.ligados[i] && !self.testar(i, e, ctx) {
                return;
            }
        }
    }

    fn indices(&self, d: Disparo) -> Vec<usize> {
        self.gatilhos.iter().enumerate().filter(|(_, g)| g.disparo == d).map(|(i, _)| i).collect()
    }

    /// `trigger::TestTrigger` de um gatilho da lista, com o auto-desligar.
    fn testar(&self, i: usize, e: &mut EstadoDaPolitica, ctx: &mut Contexto) -> bool {
        let g = &self.gatilhos[i];
        let (disparou, r) = rodar_gatilho(g, self, e, ctx);
        if disparou && g.condicao.desliga_sozinha() {
            e.ligados[i] = false;
        }
        r
    }
}

impl EstadoDaPolitica {
    /// `policy::RefreshTimer` (`aitrigger.h:447-470`).
    fn atualizar_timers(&mut self) {
        self.bandeira = 0;
        for t in &mut self.timers {
            if t.falta == 0 {
                t.falta = t.periodo;
            }
            t.falta -= 1;
            if t.falta == 0 {
                self.bandeira |= 0x01;
                if t.vezes > 0 {
                    t.vezes -= 1;
                    if t.vezes == 0 {
                        t.vezes = -1;
                        self.bandeira |= 0x02;
                    }
                }
            }
        }
    }

    fn timer_venceu(&self, id: u32) -> bool {
        self.bandeira != 0 && self.timers.iter().find(|t| t.id == id).is_some_and(|t| t.falta == 0)
    }

    /// `policy::CreateTimer`: o mesmo id é refeito do zero.
    fn criar_timer(&mut self, id: u32, periodo: i32, vezes: i32) {
        match self.timers.iter_mut().find(|t| t.id == id) {
            Some(t) => *t = Timer { id, falta: periodo, periodo, vezes },
            None => self.timers.push(Timer { id, falta: periodo, periodo, vezes }),
        }
    }

    /// Os timers que existem agora (para teste e diagnóstico).
    pub fn timers(&self) -> Vec<u32> {
        self.timers.iter().map(|t| t.id).collect()
    }

    pub fn ligado(&self, politica: &PoliticaDeIa, id: u32) -> Option<bool> {
        politica.gatilhos.iter().position(|g| g.id == id).map(|i| self.ligados[i])
    }
}

/// `Check` de uma condição. `None` é a exceção de expressão.
fn condicao_vale(c: &Condicao, e: &EstadoDaPolitica, ctx: &Contexto) -> Option<bool> {
    Some(match c {
        Condicao::Timer(id) => e.timer_venceu(*id),
        Condicao::VidaAbaixo(f) => (ctx.hp as f64) < ctx.max_hp as f64 * *f as f64,
        Condicao::Aleatorio(p) => rand::thread_rng().gen::<f32>() < *p,
        Condicao::E(a, b) => condicao_vale(a, e, ctx)? && condicao_vale(b, e, ctx)?,
        Condicao::Ou(a, b) => condicao_vale(a, e, ctx)? || condicao_vale(b, e, ctx)?,
        Condicao::Nao(a) => !condicao_vale(a, e, ctx)?,
        Condicao::MatouAlvo | Condicao::ComecoDeCombate | Condicao::Morte | Condicao::FimDeCombate => true,
        Condicao::Menor(a, b) => ctx.expr(a)? < ctx.expr(b)?,
        Condicao::Maior(a, b) => ctx.expr(a)? > ctx.expr(b)?,
        Condicao::Igual(a, b) => ctx.expr(a)? == ctx.expr(b)?,
        Condicao::Dano { minimo, maximo } => ctx.ultimo_dano >= *minimo && ctx.ultimo_dano <= *maximo,
        Condicao::FimDoCaminho | Condicao::SemPorte(_) => false,
    })
}

/// `TestTrigger`: condição verdadeira → as operações em ordem, até a primeira que devolve
/// falso. Devolve (disparou, resultado); a exceção de expressão conta como "não disparou,
/// verdadeiro" (o `catch(...)` do original).
fn rodar_gatilho(g: &Gatilho, p: &PoliticaDeIa, e: &mut EstadoDaPolitica, ctx: &mut Contexto) -> (bool, bool) {
    ctx.locais = e.locais;
    let vale = condicao_vale(&g.condicao, e, ctx);
    if vale != Some(true) {
        return (false, true);
    }
    let mut r = true;
    for o in &g.operacoes {
        if !executar(o, p, e, ctx) {
            r = false;
            break;
        }
    }
    e.locais = ctx.locais;
    (true, r)
}

fn executar(o: &Operacao, p: &PoliticaDeIa, e: &mut EstadoDaPolitica, ctx: &mut Contexto) -> bool {
    // `if(!target.IsActive()) DetermineTarget(target)` — o primeiro da lista de ódio.
    let alvo_ou_primeiro = |ctx: &Contexto| ctx.alvo(o.alvo, o.profissoes).or_else(|| ctx.lista_de_odio().first().copied());
    match &o.acao {
        Acao::Atacar { estrategia } => {
            if let Some(alvo) = alvo_ou_primeiro(ctx) {
                ctx.pedidos.push(Pedido::Atacar { alvo, estrategia: *estrategia });
            }
        }
        Acao::Habilidade { id, nivel } => {
            if let Some(alvo) = alvo_ou_primeiro(ctx) {
                ctx.pedidos.push(Pedido::Habilidade { alvo, id: *id, nivel: *nivel });
            }
        }
        Acao::Habilidade2 { id, nivel } => {
            if let Some(alvo) = alvo_ou_primeiro(ctx) {
                let (id, nivel) = (ctx.valor(*id), ctx.valor(*nivel));
                ctx.pedidos.push(Pedido::Habilidade { alvo, id, nivel });
            }
        }
        Acao::Fugir => {
            if let Some(alvo) = alvo_ou_primeiro(ctx) {
                ctx.pedidos.push(Pedido::Fugir { alvo });
            }
        }
        Acao::Falar { texto, anexos } => {
            let alvo = ctx.alvo(o.alvo, o.profissoes);
            ctx.pedidos.push(fala(texto, *anexos, alvo, ctx));
        }
        Acao::LimparOdio => {
            // `aggro_list::RegroupAggro`: todo mundo com 1.
            for t in ctx.odio.values_mut() {
                *t = 1;
            }
        }
        Acao::Rodar(g) => return rodar_gatilho(g, p, e, ctx).1,
        Acao::Ligar { id, ligar } => {
            for (i, g) in p.gatilhos.iter().enumerate() {
                if g.id == *id {
                    e.ligados[i] = *ligar;
                }
            }
        }
        Acao::CriarTimer { id, periodo, vezes } => e.criar_timer(*id, *periodo, *vezes),
        Acao::MatarTimer(id) => {
            if let Some(i) = e.timers.iter().position(|t| t.id == *id) {
                e.timers.remove(i);
            }
        }
        Acao::OdioParaPrimeiro => {
            // `BeTaunted(target, 1)` → `AddToFrist`: um acima do primeiro (`aggrolist.cpp:95-117`).
            if let Some(alvo) = ctx.alvo(o.alvo, o.profissoes) {
                let primeiro = ctx.odio.values().copied().max();
                let novo = match primeiro {
                    None => 1,
                    Some(m) => m + 1,
                };
                ctx.odio.insert(alvo, novo);
            }
        }
        Acao::OdioParaUltimo => {
            // `AddToLast`: ódio 1 (`aggrolist.cpp:164-177`).
            if let Some(alvo) = ctx.alvo(o.alvo, o.profissoes) {
                ctx.odio.insert(alvo, 1);
            }
        }
        Acao::OdioPelaMetade => {
            // `aggro_list::Fade`: metade, no mínimo 1.
            for t in ctx.odio.values_mut() {
                *t = (*t >> 1).max(1);
            }
        }
        Acao::Pular => return false,
        Acao::DefinirGlobal { id, valor, direto } => {
            let v = if *direto { *valor } else { ctx.global(*valor) };
            ctx.globais.insert(*id, v);
        }
        Acao::AjustarGlobal { id, valor } => {
            *ctx.globais.entry(*id).or_insert(0) += *valor;
        }
        Acao::Calcular { destino, a, operador, b } => {
            let (x, y) = (ctx.valor(*a), ctx.valor(*b));
            let r = if x == VALOR_INVALIDO || y == VALOR_INVALIDO {
                VALOR_INVALIDO
            } else {
                match operador {
                    0 => x.wrapping_add(y),
                    1 => x.wrapping_sub(y),
                    2 => x.wrapping_mul(y),
                    3 if y != 0 => x.wrapping_div(y),
                    4 if y != 0 => x.wrapping_rem(y),
                    _ => VALOR_INVALIDO,
                }
            };
            ctx.definir(*destino, r);
        }
        Acao::Controlador { id, parar } => {
            let id = ctx.valor(*id);
            if id > 0 && id != VALOR_INVALIDO {
                ctx.pedidos.push(Pedido::Controlador { id, ligar: !parar });
            }
        }
        Acao::SemPorte(nome) => ctx.pedidos.push(Pedido::NaoPortado(nome)),
    }
    true
}

/// `CHAT_S2C::CHAT_AIPOLICY_VALUE` (`common/chatdata.h:11-16`, 1.5.5).
const CHAT_AIPOLICY_VALUE: i16 = 2;

/// `op_say` / `op_say_2` (`aitrigger.cpp:552-668`): o prefixo `$X` do texto escolhe o canal
/// e sai do texto. Com máscara de anexos, o `data` leva `CHAT_AIPOLICY_VALUE`, a máscara, o id
/// do alvo e as variáveis locais pedidas.
fn fala(texto: &str, anexos: u32, alvo: Option<i64>, ctx: &Contexto) -> Pedido {
    let mut chars = texto.chars();
    let (canal, resto) = match (chars.next(), chars.next()) {
        (Some('$'), Some(c)) if texto.chars().count() > 2 => match c {
            'A' => (Some(CanalDaFala::LocalAnonima), chars.as_str()),
            'B' => (Some(CanalDaFala::Grito), chars.as_str()),
            'S' => (Some(CanalDaFala::Sistema), chars.as_str()),
            'I' | 'X' => (Some(CanalDaFala::Instancia), chars.as_str()),
            'F' | 'T' => return Pedido::NaoPortado("fala de batalha ($F/$T)"),
            _ => (None, texto),
        },
        _ => (None, texto),
    };
    let mut dados = Vec::new();
    if anexos != 0 {
        dados.extend_from_slice(&CHAT_AIPOLICY_VALUE.to_le_bytes());
        dados.extend_from_slice(&(anexos as i32).to_le_bytes());
        if anexos & 0x1 != 0 {
            dados.extend_from_slice(&(alvo.unwrap_or(-1) as i32).to_le_bytes());
        }
        for (bit, i) in [(0x2, 0), (0x4, 1), (0x8, 2)] {
            if anexos & bit != 0 {
                dados.extend_from_slice(&ctx.locais[i].to_le_bytes());
            }
        }
    }
    Pedido::Falar { texto: resto.to_string(), canal: canal.unwrap_or(CanalDaFala::Local), dados }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ctx<'a>(odio: &'a mut HashMap<i64, i64>, globais: &'a mut HashMap<i32, i32>, info: &'a dyn Fn(i64) -> Option<InfoDoAlvo>, perto: &'a dyn Fn(Option<f32>, Option<(f32, f32, f32, f32)>) -> i32) -> Contexto<'a> {
        Contexto {
            eu: -1,
            hp: 1000,
            max_hp: 1000,
            posicao: Vector3::new(0.0, 0.0, 0.0),
            odio,
            info,
            jogadores_perto: perto,
            globais,
            locais: [0; 3],
            ultimo_dano: 0,
            matador: None,
            pedidos: Vec::new(),
        }
    }

    fn g(id: u32, condicao: Condicao, operacoes: Vec<Acao>, ativo: bool) -> Gatilho {
        let disparo = condicao.disparo();
        Gatilho {
            id,
            ligado_de_inicio: ativo || matches!(disparo, Disparo::ComecoDeCombate | Disparo::Morte | Disparo::FimDeCombate),
            so_em_combate: true,
            disparo,
            condicao,
            operacoes: operacoes.into_iter().map(|acao| Operacao { acao, alvo: Some(TipoDeAlvo::OdioPrimeiro), profissoes: 0 }).collect(),
        }
    }

    fn politica(gatilhos: Vec<Gatilho>) -> PoliticaDeIa {
        let de_paz = gatilhos.iter().filter(|g| g.disparo == Disparo::Batimento && !g.so_em_combate).count();
        PoliticaDeIa { id: 1, gatilhos, de_paz }
    }

    /// O padrão mais comum do 1.2.6: ao começar o combate cria um timer; a cada vencimento
    /// o monstro usa a habilidade.
    #[test]
    fn timer_do_comeco_de_combate_dispara_a_habilidade_no_periodo() {
        let p = politica(vec![
            g(1, Condicao::ComecoDeCombate, vec![Acao::CriarTimer { id: 7, periodo: 3, vezes: 0 }], false),
            g(2, Condicao::Timer(7), vec![Acao::Habilidade { id: 99, nivel: 2 }], true),
        ]);
        let mut e = p.novo_estado([0; 3]);
        let (mut odio, mut globais) = (HashMap::from([(5i64, 10i64)]), HashMap::new());
        let info = |_| None;
        let perto = |_: Option<f32>, _: Option<(f32, f32, f32, f32)>| 0;
        let mut c = ctx(&mut odio, &mut globais, &info, &perto);
        p.comeco_de_combate(&mut e, &mut c);
        let mut quando = Vec::new();
        for s in 1..=9 {
            c.pedidos.clear();
            p.batimento(&mut e, &mut c, true);
            if c.pedidos.contains(&Pedido::Habilidade { alvo: 5, id: 99, nivel: 2 }) {
                quando.push(s);
            }
        }
        assert_eq!(quando, vec![3, 6, 9]);
    }

    /// `hp_less` dispara uma vez e se desliga; o fim do combate o religa.
    #[test]
    fn vida_abaixo_dispara_uma_vez_por_combate() {
        let p = politica(vec![g(1, Condicao::VidaAbaixo(0.5), vec![Acao::Habilidade { id: 3, nivel: 1 }], true)]);
        let mut e = p.novo_estado([0; 3]);
        let (mut odio, mut globais) = (HashMap::from([(5i64, 10i64)]), HashMap::new());
        let info = |_| None;
        let perto = |_: Option<f32>, _: Option<(f32, f32, f32, f32)>| 0;
        let mut c = ctx(&mut odio, &mut globais, &info, &perto);
        p.batimento(&mut e, &mut c, true);
        assert!(c.pedidos.is_empty(), "com vida cheia, nada");
        c.hp = 400;
        p.batimento(&mut e, &mut c, true);
        p.batimento(&mut e, &mut c, true);
        assert_eq!(c.pedidos.len(), 1, "uma vez só");
        p.fim_de_combate(&mut e, &mut c);
        c.pedidos.clear();
        p.batimento(&mut e, &mut c, true);
        assert_eq!(c.pedidos.len(), 1, "religado no combate seguinte");
    }

    /// `op_break` interrompe o gatilho e a lista; `Ligar` desliga outro gatilho.
    #[test]
    fn pular_para_a_lista_e_parar_trigger_desliga() {
        let p = politica(vec![
            g(1, Condicao::Aleatorio(2.0), vec![Acao::Ligar { id: 3, ligar: false }, Acao::Pular, Acao::Habilidade { id: 1, nivel: 1 }], true),
            g(2, Condicao::Aleatorio(2.0), vec![Acao::Habilidade { id: 2, nivel: 1 }], true),
            g(3, Condicao::Aleatorio(2.0), vec![], true),
        ]);
        let mut e = p.novo_estado([0; 3]);
        let (mut odio, mut globais) = (HashMap::from([(5i64, 10i64)]), HashMap::new());
        let info = |_| None;
        let perto = |_: Option<f32>, _: Option<(f32, f32, f32, f32)>| 0;
        let mut c = ctx(&mut odio, &mut globais, &info, &perto);
        p.batimento(&mut e, &mut c, true);
        assert!(c.pedidos.is_empty(), "o Pular parou as operações e os gatilhos seguintes");
        assert_eq!(e.ligado(&p, 3), Some(false));
    }

    /// Ódio: para primeiro, para último e metade (`aggrolist.cpp`).
    #[test]
    fn operacoes_de_odio() {
        let p = politica(vec![]);
        let mut e = p.novo_estado([0; 3]);
        let (mut odio, mut globais) = (HashMap::from([(1i64, 100i64), (2, 40)]), HashMap::new());
        let info = |_| None;
        let perto = |_: Option<f32>, _: Option<(f32, f32, f32, f32)>| 0;
        let mut c = ctx(&mut odio, &mut globais, &info, &perto);
        let op = |acao, alvo| Operacao { acao, alvo: Some(alvo), profissoes: 0 };
        executar(&op(Acao::OdioParaPrimeiro, TipoDeAlvo::OdioSegundo), &p, &mut e, &mut c);
        assert_eq!(c.lista_de_odio(), vec![2, 1]);
        executar(&op(Acao::OdioParaUltimo, TipoDeAlvo::OdioPrimeiro), &p, &mut e, &mut c);
        assert_eq!(c.lista_de_odio(), vec![1, 2]);
        executar(&op(Acao::OdioPelaMetade, TipoDeAlvo::EuMesmo), &p, &mut e, &mut c);
        assert_eq!((c.odio[&1], c.odio[&2]), (50, 1));
    }

    /// Fala: o prefixo escolhe o canal e sai do texto.
    #[test]
    fn prefixo_da_fala() {
        let (mut odio, mut globais) = (HashMap::new(), HashMap::new());
        let info = |_| None;
        let perto = |_: Option<f32>, _: Option<(f32, f32, f32, f32)>| 0;
        let c = ctx(&mut odio, &mut globais, &info, &perto);
        assert_eq!(fala("Morra!", 0, None, &c), Pedido::Falar { texto: "Morra!".into(), canal: CanalDaFala::Local, dados: vec![] });
        assert_eq!(fala("$BO chefe acordou", 0, None, &c), Pedido::Falar { texto: "O chefe acordou".into(), canal: CanalDaFala::Grito, dados: vec![] });
    }
}
