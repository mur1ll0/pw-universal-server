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
    pub execucao_ms: Option<Vec<i32>>,
    pub recarga_ms: Option<Vec<i32>>,
    pub nivel_exigido: Option<Vec<i32>>,
    pub sp_exigido: Option<Vec<i32>>,
    pub dinheiro_exigido: Option<Vec<i32>>,
    pub estados_ms: Vec<Option<Vec<i32>>>,
}

fn no_nivel<T: Copy>(v: &Option<Vec<T>>, nivel: i32) -> Option<T> {
    v.as_ref()?.get(usize::try_from(nivel - 1).ok()?).copied()
}

impl HabilidadeDoServidor {
    /// O tempo de recarga como o original arma: segundos **truncados**, vezes mil
    /// (`skill.h:577` devolve `(int)(0.001*coolingtime)`).
    pub fn recarga_armada_ms(&self, nivel: i32) -> Option<i32> {
        no_nivel(&self.recarga_ms, nivel).map(|t| ((t as f64 * 0.001) as i32) * 1000)
    }

    /// `State1::GetTime` — o tempo de conjuração enviado ao cliente.
    pub fn conjuracao_ms(&self, nivel: i32) -> Option<i32> {
        no_nivel(self.estados_ms.first()?, nivel)
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
    /// A tabela dos stubs do servidor 1.5.5.
    pub fn do_155() -> Self {
        let a: Arquivo = serde_json::from_str(HABILIDADES_155_JSON).expect("habilidades.json embutido é válido");
        Self { por_id: a.habilidades.into_values().map(|h| (h.id, h)).collect() }
    }

    pub fn get(&self, id: u32) -> Option<&HabilidadeDoServidor> {
        self.por_id.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            execucao_ms: None,
            recarga_ms: Some(vec![2500]),
            nivel_exigido: None,
            sp_exigido: None,
            dinheiro_exigido: None,
            estados_ms: vec![],
        };
        assert_eq!(h.recarga_armada_ms(1), Some(2000));
    }
}
