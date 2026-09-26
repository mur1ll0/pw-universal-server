//! Os números de cada habilidade do servidor 1.5.5: recarga, conjuração, custo de mana e o
//! que é preciso para aprender cada nível.
//!
//! # Origem
//!
//! `specs/habilidades_155/habilidades.json`, gerado por `extrair_habilidades.py` a partir dos
//! stubs `cskill/skills/skillNNN.h` do `EvolvedPWServer` — o código que o servidor original
//! compila. Valores por nível (índice 0 = nível 1); `None` quando a expressão do stub depende
//! de mais do que o nível ou não pôde ser reconstruída. O servidor trata `None` como
//! **desconhecido**, nunca como zero.
//!
//! | campo | função do stub | quem usa no original |
//! | :--- | :--- | :--- |
//! | `recarga_ms` | `GetCoolingtime` | `SetCoolDown(id+1024, (int)(0.001*t)*1000)` (`playerwrapper.cpp:170`, `skill.h:577`) |
//! | `estados_ms[0]` | `State1::GetTime` | tempo de conjuração no `OBJECT_CAST_SKILL` (`skill.cpp:797`) |
//! | `nivel_exigido`, `sp_exigido`, `dinheiro_exigido` | `GetRequired*` | `SkillStub::LearnCondition`/`Learn` (`skill.cpp:14-93`) |

use serde::Deserialize;
use std::collections::HashMap;

const HABILIDADES_155_JSON: &str = include_str!("../../../specs/habilidades_155/habilidades.json");
/// Os 823 stubs do `gs` 1.2.6, executados por nível (`specs/habilidades_126/extrair_habilidades_126.py`).
const HABILIDADES_126_JSON: &str = include_str!("../../../specs/habilidades_126/habilidades.json");

#[derive(Debug, Clone, Deserialize)]
pub struct HabilidadeDoServidor {
    pub id: u32,
    /// Classe que aprende (`255` = qualquer).
    pub cls: Option<i32>,
    pub max_level: i32,
    #[serde(rename = "type")]
    pub tipo: Option<i32>,
    pub rank: Option<i32>,
    pub pre_skills: Vec<(u32, i32)>,
    pub mp: Option<Vec<f32>>,
    /// `apcost` do stub (`cskill/skill/skill.h:239`): o chi que a habilidade **consome**.
    /// `SkillStub::Condition` recusa a conjuração com `GetAp() < apcost`
    /// (`cskill/skill/skill.cpp:125`).
    #[serde(default)]
    pub apcost: Option<i32>,
    /// `apgain`: o chi que a habilidade **dá**. Na execução, o original aplica a diferença de
    /// uma vez: `int ap = GetApgain() - GetApcost(); if (ap) ModifyAP(ap)`
    /// (`cskill/skill/playerwrapper.cpp:170-177`).
    #[serde(default)]
    pub apgain: Option<i32>,
    /// `is_movingcast` (`cskill/skill/skill.h:382`): a habilidade é conjurada **andando**.
    /// O original a despacha por `moving_skill` em vez de `session_skill`
    /// (`gs/playercmd.cpp:2066-2088`), e o movimento do jogador não a interrompe. No 1.5.5
    /// são **5**, todas da classe 11 (2909, 2910, 2913, 2914, 2917).
    #[serde(default)]
    pub is_movingcast: Option<i32>,
    pub execucao_ms: Option<Vec<i32>>,
    pub recarga_ms: Option<Vec<i32>>,
    pub nivel_exigido: Option<Vec<i32>>,
    pub sp_exigido: Option<Vec<i32>>,
    pub dinheiro_exigido: Option<Vec<i32>>,
    /// `GetRequiredItem` — o livro que o aprendizado consome. Só o aprendizado de mascote o
    /// usa (`SkillWrapper::PetLearn`, `cskill/skill/skillwrapper.cpp:1558-1564`).
    #[serde(default)]
    pub item_exigido: Option<Vec<i32>>,
    pub estados_ms: Vec<Option<Vec<i32>>>,
    /// `time_type` — 3 é conjuração com carga (`Skill::IsWarmup`, `skill.h:571`).
    #[serde(default)]
    pub time_type: Option<i32>,
    /// `GetPraydistance`.
    #[serde(default)]
    pub alcance: Option<AlcanceDaHabilidade>,
    /// A conta de dano do estado que chama `SetDamage`/`SetXdamage`.
    #[serde(default)]
    pub dano: Option<DanoDaHabilidade>,
    /// `arrowcost` — flechas gastas por conjuração (`object.Attack(..., arrowcost)`,
    /// `playerwrapper.cpp:307`).
    #[serde(default)]
    pub arrowcost: Option<i32>,
    /// `range.type` (`cskill/skill/range.h:18-25`): 0 ponto, 1 linha, 2 bola em si,
    /// 3 bola no alvo, 4 setor, 5 em si.
    #[serde(default)]
    pub tipo_de_area: Option<i32>,
    /// `doenchant`: o golpe leva a habilidade junto (`attached_skill`) e o alvo roda o
    /// `StateAttack` (`SkillWrapper::Attack`, `skillwrapper.cpp:449-484`).
    #[serde(default)]
    pub doenchant: bool,
    /// `dobless`: quem conjura roda o `BlessMe` (`playerwrapper.cpp:244-253`).
    #[serde(default)]
    pub dobless: bool,
    /// `GetRadius`, `GetAttackdistance`, `GetAngle`, `GetHitrate` por nível.
    #[serde(default)]
    pub raio: Option<Vec<f32>>,
    #[serde(default)]
    pub distancia_de_ataque: Option<Vec<f32>>,
    #[serde(default)]
    pub angulo: Option<Vec<f32>>,
    #[serde(default)]
    pub precisao: Option<Vec<f32>>,
    /// `GetEffectdistance`, no formato do alcance.
    #[serde(default)]
    pub distancia_de_efeito: Option<AlcanceDaHabilidade>,
    /// `StateAttack` e `BlessMe` como `[quem, setter, expressão]` — ver
    /// `pw_gs::efeitos::executar_roteiro`. `None` quando o corpo não se deixou ler.
    #[serde(default)]
    pub no_alvo: Option<Vec<(String, String, String)>>,
    #[serde(default)]
    pub em_si: Option<Vec<(String, String, String)>>,
}

/// `GetPraydistance` = `arma × attack_range + fixo[nível]` (`GetRange()`,
/// `playerwrapper.h:95`).
#[derive(Debug, Clone, Deserialize)]
pub struct AlcanceDaHabilidade {
    pub arma: i32,
    pub fixo: Vec<f32>,
}

/// `SetRatio`/`SetPlus` + `SetX(fator × GetAttack|GetMagicattack)` — ver
/// `extrair_habilidades.py::dano`.
#[derive(Debug, Clone, Deserialize)]
pub struct DanoDaHabilidade {
    pub estado: i32,
    /// `fisico` (`GetAttack`) ou `magico` (`GetMagicattack`).
    pub base: String,
    /// `Damage` (físico) ou `Golddamage`, `Wooddamage`, `Waterdamage`, `Firedamage`,
    /// `Earthdamage` — as cinco escolas.
    pub elemento: String,
    pub fator: f32,
    /// Com a carga cheia quando `carga`.
    pub ratio: Vec<f32>,
    pub plus: Vec<f32>,
    pub carga: bool,
}

fn no_nivel<T: Copy>(v: &Option<Vec<T>>, nivel: i32) -> Option<T> {
    v.as_ref()?.get(usize::try_from(nivel - 1).ok()?).copied()
}

impl HabilidadeDoServidor {
    /// O tempo de recarga como o original arma: segundos **truncados**, vezes mil
    /// (`skill.h:577` devolve `(int)(0.001*coolingtime)`).
    /// A habilidade pode ser conjurada andando.
    pub fn conjura_andando(&self) -> bool {
        self.is_movingcast.unwrap_or(0) != 0
    }

    pub fn recarga_armada_ms(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.recarga_ms, nivel).map(|t| ((t as f64 * 0.001) as i32) * 1000)
    }

    /// `State1::GetTime` — o tempo de conjuração enviado ao cliente.
    pub fn conjuracao_ms(&self, nivel: i32) -> Option<i32> {
        no_nivel(self.estados_ms.first()?, nivel)
    }

    /// `State2::GetTime` e seguintes — a **fase de execução**, depois da conjuração.
    ///
    /// A sessão de habilidade do original é um laço de estados: `StartSkill` devolve o tempo
    /// do primeiro e `RunSkill` o do seguinte, a cada volta de `session_skill::RepeatSession`
    /// (`gs/actsession.cpp:466-600`). Só quando não há próximo estado vem o `EndSession`, que
    /// manda o `stop_skill` (`:558-574`). A Flecha Fulgurante (244), por exemplo, tem 3.000 ms
    /// de conjuração e **800 ms de execução** (`cskill/skills/skill244.h:20-80`) — é nessa
    /// segunda fase que o cliente anima o personagem recebendo a bênção.
    ///
    /// O número sai do `GetExecutetime` do stub (`cskill/skill/skill.cpp:617-622`), que para
    /// a 244 é o mesmo 800 do `State2`; se ele faltar, a soma dos estados depois do primeiro.
    pub fn fase_de_execucao_ms(&self, nivel: i32) -> Option<i32> {
        if let Some(t) = no_nivel(&self.execucao_ms, nivel).filter(|t| *t > 0) {
            return Some(t);
        }
        let total: i32 = self
            .estados_ms
            .iter()
            .skip(1)
            .filter_map(|e| e.as_ref().and_then(|v| v.get(usize::try_from(nivel - 1).ok()?).copied()))
            .sum();
        (total > 0).then_some(total)
    }

    /// Conjuração com carga que o jogador solta antes do fim (`time_type == 3`).
    pub fn e_de_carga(&self) -> bool {
        self.time_type == Some(3)
    }

    /// `GetPraydistance` para quem tem `alcance_do_jogador` de ataque.
    pub fn alcance(&self, nivel: i32, alcance_do_jogador: f32) -> Option<f32> {
        let a = self.alcance.as_ref()?;
        let fixo = *a.fixo.get(usize::try_from(nivel - 1).ok()?)?;
        Some(a.arma as f32 * alcance_do_jogador + fixo)
    }

    /// `GetEffectdistance` para quem tem `alcance_do_jogador` de ataque.
    pub fn distancia_de_efeito(&self, nivel: i32, alcance_do_jogador: f32) -> Option<f32> {
        let a = self.distancia_de_efeito.as_ref()?;
        let fixo = *a.fixo.get(usize::try_from(nivel - 1).ok()?)?;
        Some(a.arma as f32 * alcance_do_jogador + fixo)
    }

    pub fn raio(&self, nivel: i32) -> f32 {
        no_nivel(&self.raio, nivel).unwrap_or(0.0)
    }

    pub fn distancia_de_ataque(&self, nivel: i32) -> f32 {
        no_nivel(&self.distancia_de_ataque, nivel).unwrap_or(0.0)
    }

    /// `GetAngle` — o cosseno do meio ângulo do setor (`1 - 0,0111111 × graus`... como o
    /// stub escreve).
    pub fn angulo(&self, nivel: i32) -> f32 {
        no_nivel(&self.angulo, nivel).unwrap_or(1.0)
    }

    /// `GetHitrate` — multiplica a precisão do golpe (`msg.attack_rate`,
    /// `playerwrapper.cpp:262`).
    pub fn precisao(&self, nivel: i32) -> f32 {
        no_nivel(&self.precisao, nivel).unwrap_or(1.0)
    }

    /// `GetMpcost`.
    pub fn mana(&self, nivel: i32) -> Option<f32> {
        no_nivel(&self.mp, nivel)
    }

    pub fn nivel_exigido(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.nivel_exigido, nivel)
    }

    pub fn sp_exigido(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.sp_exigido, nivel)
    }

    pub fn dinheiro_exigido(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.dinheiro_exigido, nivel)
    }

    pub fn item_exigido(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.item_exigido, nivel)
    }
}

#[derive(Deserialize)]
struct Arquivo {
    habilidades: HashMap<String, HabilidadeDoServidor>,
}

/// A tabela inteira, por id.
#[derive(Debug, Clone, Default)]
pub struct TabelaDeHabilidades {
    pub por_id: HashMap<u32, HabilidadeDoServidor>,
}

impl TabelaDeHabilidades {
    /// Uma tabela completa no formato do `habilidades.json` (`{"habilidades": {id: {...}}}`)
    /// — a embutida de uma versão ou a do catálogo do realm
    /// (`data/<realm>/catalogo/habilidades.json`, B102).
    pub fn de_json(texto: &str) -> Result<Self, serde_json::Error> {
        let a: Arquivo = serde_json::from_str(texto)?;
        Ok(Self { por_id: a.habilidades.into_values().map(|h| (h.id, h)).collect() })
    }

    /// A tabela dos stubs do servidor 1.5.5.
    pub fn do_155() -> Self {
        Self::de_json(HABILIDADES_155_JSON).expect("habilidades.json embutido é válido")
    }

    /// A tabela do servidor 1.2.6: só as habilidades que o `gs` 1.2.6 compila (823, as do
    /// `skillstr.txt` do cliente 1.2.6), com **os números do próprio `gs` 1.2.6** — tempos,
    /// mana, aprendizado, alcance, distâncias, raio, ângulo, precisão e dano — extraídos
    /// executando cada função do stub por nível (B100/B101). O arquivo já vem completo
    /// (`extrair_habilidades_126.py`, B102): do stub 1.5.5 de mesmo id ficam só o que não é
    /// função de nível no stub (classe, tipo, pré-requisitos, `time_type`, área, flags, roteiros
    /// `no_alvo`/`em_si`) e, onde a função do 1.2.6 depende de mais que o nível, o valor do
    /// 1.5.5 — hoje só o dano de 317, 529, 666, 667 e 799 (`GetHp`).
    ///
    /// Diferenças medidas contra o 1.5.5: `plus` do dano em 68 habilidades e `ratio` em 10
    /// (a 299: 23,7 no nível 1, contra 124,5 do 1.5.5), custo em dinheiro em 186, nível exigido
    /// em 142, tempos em 18. O texto do `skillstr.txt` 1.2.6 não é fonte (B100).
    pub fn do_126() -> Self {
        Self::de_json(HABILIDADES_126_JSON).expect("habilidades.json do 1.2.6 embutido é válido")
    }

    pub fn get(&self, id: u32) -> Option<&HabilidadeDoServidor> {
        self.por_id.get(&id)
    }

    /// Todas as habilidades da tabela.
    pub fn todas(&self) -> impl Iterator<Item = (&u32, &HabilidadeDoServidor)> {
        self.por_id.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B100 — Enxame de Ferroadas (299) no `gs` 1.2.6: 1.500 ms de conjuração e 1.000 ms de
    /// execução, o mesmo que a captura original mede entre o 85 e o 123 (2.504-2.551 ms).
    #[test]
    fn a_tabela_do_126_tem_os_tempos_do_gs_126() {
        let t = TabelaDeHabilidades::do_126();
        assert_eq!(t.por_id.len(), 823);
        let h = t.get(299).expect("skill 299");
        assert_eq!(h.conjuracao_ms(1), Some(1_500));
        assert_eq!(h.fase_de_execucao_ms(1), Some(1_000));
        assert_eq!(h.recarga_armada_ms(1), Some(1_000));
        // 102 e 250 (nível 2) também medidas na captura: 200+700 e 500+900.
        assert_eq!((t.get(102).unwrap().conjuracao_ms(1), t.get(102).unwrap().fase_de_execucao_ms(1)), (Some(200), Some(700)));
        assert_eq!((t.get(250).unwrap().conjuracao_ms(2), t.get(250).unwrap().fase_de_execucao_ms(2)), (Some(500), Some(900)));
        // Onde o 1.2.6 difere do 1.5.5 vale o 1.2.6: 803 recarrega em 60 s (3 s no 1.5.5).
        assert_eq!(t.get(803).unwrap().recarga_armada_ms(1), Some(60_000));
        assert_eq!(TabelaDeHabilidades::do_155().get(803).unwrap().recarga_armada_ms(1), Some(3_000));
        // B101 — o dano também é do `gs` 1.2.6: a 299 soma 23,7 de madeira no nível 1
        // (`Skill299Stub::State2::Calculate`), não os 124,5 do 1.5.5; 55% do ataque mágico.
        let d = h.dano.as_ref().expect("dano da 299");
        assert_eq!((d.estado, d.base.as_str(), d.elemento.as_str()), (1, "magico", "Wooddamage"));
        assert_eq!((d.ratio[0], d.plus[0], d.plus[1]), (0.55, 23.7, 94.0));
        assert_eq!(h.mana(1), Some(4.0));
        assert_eq!(h.dinheiro_exigido(2), Some(290));
        // A skill 1 do guerreiro: 10,8 no 1.2.6, 102,6 no 1.5.5.
        assert_eq!(t.get(1).unwrap().dano.as_ref().unwrap().plus[0], 10.8);
        // Onde o 1.2.6 depende da vida do jogador fica o dano do 1.5.5.
        assert!(t.get(317).unwrap().dano.is_some());
    }

    #[test]
    fn a_habilidade_1_do_guerreiro_bate_com_o_stub() {
        let t = TabelaDeHabilidades::do_155();
        let h = t.get(1).expect("skill 1");
        assert_eq!(h.max_level, 10);
        assert_eq!(h.recarga_armada_ms(1), Some(3000));
        assert_eq!(h.conjuracao_ms(1), Some(400));
        assert_eq!(h.sp_exigido(2), Some(300));
        assert_eq!(h.dinheiro_exigido(2), Some(30));
        assert_eq!(h.nivel_exigido(11), None, "nível fora da tabela");
    }

    /// B73 — o chi de cada habilidade vem do stub, e o JSON já o trazia.
    ///
    /// `SkillStub::Condition` recusa com `GetAp() < apcost` (`cskill/skill/skill.cpp:125`) e a
    /// execução aplica `GetApgain() - GetApcost()` (`playerwrapper.cpp:170-177`).
    #[test]
    fn o_custo_e_o_ganho_de_chi_vem_do_stub() {
        let t = TabelaDeHabilidades::do_155();
        // Flecha Glacial (245) e Barreira de Asa (249) **gastam**; a 235 e a Flecha
        // Fulgurante (244) **dão**.
        assert_eq!(t.get(245).and_then(|h| h.apcost), Some(25), "Flecha Glacial");
        assert_eq!(t.get(245).and_then(|h| h.apgain), Some(0));
        assert_eq!(t.get(249).and_then(|h| h.apcost), Some(45), "Barreira de Asa");
        assert_eq!(t.get(244).and_then(|h| h.apgain), Some(10), "Flecha Fulgurante");
        assert_eq!(t.get(244).and_then(|h| h.apcost), Some(0));
        assert_eq!(t.get(235).and_then(|h| h.apgain), Some(5));
    }

    /// B78/B82 — conjurar andando é propriedade **da habilidade**, não da classe.
    ///
    /// `is_movingcast` no stub (`cskill/skill/skill.h:382`); o original despacha essas por
    /// `moving_skill` (`gs/playercmd.cpp:2066-2088`). No 1.5.5 são **24**, todas da classe 11.
    ///
    /// Eram "cinco" no B78 porque o extrator só casava dígito e os stubs escrevem o mesmo
    /// campo de dois jeitos: `= 1` em cinco deles e `= true` em dezenove (B82). As duas
    /// primeiras habilidades de ataque da classe — 2571 e 2579, as que um Tormentador tem
    /// no nível 10 — estavam entre as dezenove, e era por isso que andar ainda cortava a
    /// conjuração em jogo.
    #[test]
    fn as_habilidades_que_conjuram_andando_sao_as_24_da_classe_11() {
        let t = TabelaDeHabilidades::do_155();
        let mut andando: Vec<u32> = t.todas().filter(|(_, h)| h.conjura_andando()).map(|(id, _)| *id).collect();
        andando.sort_unstable();
        assert_eq!(andando.len(), 24, "são 24 no 1.5.5 (5 com `= 1`, 19 com `= true`)");
        for id in [2571, 2579, 2909, 2917] {
            assert!(andando.contains(&id), "a {id} conjura andando e sumiu da lista");
        }
        for id in &andando {
            assert_eq!(t.get(*id).and_then(|h| h.cls), Some(11), "a {id} não é da classe 11");
        }
        // As antigas não têm: o Arqueiro não conjura andando.
        assert!(!t.get(244).is_some_and(|h| h.conjura_andando()));
    }

    #[test]
    fn a_recarga_trunca_os_segundos() {
        let h = HabilidadeDoServidor {
            id: 9,
            cls: None,
            max_level: 1,
            tipo: None,
            rank: None,
            pre_skills: vec![],
            mp: None,
            apcost: None,
            apgain: None,
            is_movingcast: None,
            execucao_ms: None,
            recarga_ms: Some(vec![2500]),
            nivel_exigido: None,
            sp_exigido: None,
            dinheiro_exigido: None,
            item_exigido: None,
            estados_ms: vec![],
            time_type: None,
            alcance: None,
            dano: None,
            arrowcost: None,
            tipo_de_area: None,
            doenchant: false,
            dobless: false,
            raio: None,
            distancia_de_ataque: None,
            angulo: None,
            precisao: None,
            distancia_de_efeito: None,
            no_alvo: None,
            em_si: None,
        };
        assert_eq!(h.recarga_armada_ms(1), Some(2000));
    }
}
