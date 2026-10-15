//! `npcgen.data` editável, com ida e volta byte a byte (Estúdio S4, B243).
//!
//! A mesma leitura do `CNPCGenMan::Load` (`EvolvedPWServer/cgame/gs/template/npcgendata.cpp:62-315`): cabeçalho
//! (`u32 versão` + 2, 3 ou 4 contagens), cada área de IA seguida dos seus geradores, cada área de recurso seguida
//! dos seus recursos, os objetos dinâmicos e os controladores. Cada registro é guardado em bytes com o tamanho da
//! struct `#pragma pack(1)` da versão (`npcgendata.h:32-286`; `size_t` = 4 no `gs` de 32 bits):
//!
//! | registro | versões | tamanho |
//! | :--- | :--- | ---: |
//! | área de IA (`NPCGENFILEAREA` / `7`) | < 7 / ≥ 7 | 59 / 71 |
//! | gerador (`NPCGENFILEAIGEN10` / `AIGEN`) | < 11 / ≥ 11 | 60 / 64 |
//! | área de recurso (`RESAREA` / `6` / `7`) | < 6 / 6 / ≥ 7 | 31 / 34 / 42 |
//! | recurso (`NPCGENFILERES`) | todas | 20 |
//! | objeto dinâmico (`DYNOBJ` / `9` / `10`) | < 9 / 9 / ≥ 10 | 19 / 23 / 24 |
//! | controlador (`NPCGENFILECTRL` / `8`) | < 8 / ≥ 8 | 195 / 199 |
//!
//! Gravar sem editar devolve o arquivo idêntico; as contagens (`iNumGen` +4 da área, `iNumRes` +20 da área de
//! recurso e as do cabeçalho) saem dos vetores, então acrescentar/remover gerador ou área mantém o arquivo coerente.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ErroDoNpcgen {
    #[error("npcgen: versão {0} fora de 1..=11")]
    Versao(u32),
    #[error("npcgen: acabou antes do fim ({0} de {1} bytes)")]
    Curto(usize, usize),
    #[error("npcgen: sobram {0} bytes depois dos controladores")]
    Sobra(usize),
    #[error("npcgen: contagem negativa")]
    Negativa,
    #[error("npcgen: campo {0} inexistente em {1}")]
    Campo(String, &'static str),
    #[error("npcgen: índice {0} inexistente")]
    Indice(usize),
}

type R<T> = Result<T, ErroDoNpcgen>;

#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub bruto: Vec<u8>,
    pub geradores: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaDeRecurso {
    pub bruto: Vec<u8>,
    pub recursos: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NpcgenEditavel {
    pub versao: u32,
    pub areas: Vec<Area>,
    pub areas_de_recurso: Vec<AreaDeRecurso>,
    pub objetos: Vec<Vec<u8>>,
    pub controladores: Vec<Vec<u8>>,
}

pub fn tamanho_da_area(v: u32) -> usize {
    if v < 7 { 59 } else { 71 }
}
pub fn tamanho_do_gerador(v: u32) -> usize {
    if v < 11 { 60 } else { 64 }
}
pub fn tamanho_da_area_de_recurso(v: u32) -> usize {
    if v < 6 { 31 } else if v < 7 { 34 } else { 42 }
}
pub const TAMANHO_DO_RECURSO: usize = 20;
pub fn tamanho_do_objeto(v: u32) -> usize {
    if v < 9 { 19 } else if v < 10 { 23 } else { 24 }
}
pub fn tamanho_do_controlador(v: u32) -> usize {
    if v < 8 { 195 } else { 199 }
}

struct Leitor<'a> {
    b: &'a [u8],
    p: usize,
}

impl Leitor<'_> {
    fn pedaco(&mut self, n: usize) -> R<Vec<u8>> {
        let fim = self.p + n;
        let v = self.b.get(self.p..fim).ok_or(ErroDoNpcgen::Curto(self.b.len(), fim))?.to_vec();
        self.p = fim;
        Ok(v)
    }
    fn contagem(&mut self) -> R<usize> {
        let v = i32::from_le_bytes(self.pedaco(4)?.try_into().unwrap());
        usize::try_from(v).map_err(|_| ErroDoNpcgen::Negativa)
    }
}

fn i32_em(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

impl NpcgenEditavel {
    pub fn ler(buf: &[u8]) -> R<Self> {
        let mut l = Leitor { b: buf, p: 0 };
        let versao = u32::from_le_bytes(l.pedaco(4)?.try_into().unwrap());
        if !(1..=11).contains(&versao) {
            return Err(ErroDoNpcgen::Versao(versao));
        }
        let n_areas = l.contagem()?;
        let n_res = l.contagem()?;
        let n_obj = if versao >= 6 { l.contagem()? } else { 0 };
        let n_ctrl = if versao >= 7 { l.contagem()? } else { 0 };
        let mut areas = Vec::with_capacity(n_areas.min(100_000));
        for _ in 0..n_areas {
            let bruto = l.pedaco(tamanho_da_area(versao))?;
            let n = usize::try_from(i32_em(&bruto, 4)).map_err(|_| ErroDoNpcgen::Negativa)?;
            let geradores = (0..n).map(|_| l.pedaco(tamanho_do_gerador(versao))).collect::<R<_>>()?;
            areas.push(Area { bruto, geradores });
        }
        let mut areas_de_recurso = Vec::with_capacity(n_res.min(100_000));
        for _ in 0..n_res {
            let bruto = l.pedaco(tamanho_da_area_de_recurso(versao))?;
            let n = usize::try_from(i32_em(&bruto, 20)).map_err(|_| ErroDoNpcgen::Negativa)?;
            let recursos = (0..n).map(|_| l.pedaco(TAMANHO_DO_RECURSO)).collect::<R<_>>()?;
            areas_de_recurso.push(AreaDeRecurso { bruto, recursos });
        }
        let objetos = (0..n_obj).map(|_| l.pedaco(tamanho_do_objeto(versao))).collect::<R<_>>()?;
        let controladores = (0..n_ctrl).map(|_| l.pedaco(tamanho_do_controlador(versao))).collect::<R<_>>()?;
        if l.p != buf.len() {
            return Err(ErroDoNpcgen::Sobra(buf.len() - l.p));
        }
        Ok(Self { versao, areas, areas_de_recurso, objetos, controladores })
    }

    pub fn gravar(&self) -> Vec<u8> {
        let v = self.versao;
        let mut s = v.to_le_bytes().to_vec();
        s.extend((self.areas.len() as i32).to_le_bytes());
        s.extend((self.areas_de_recurso.len() as i32).to_le_bytes());
        if v >= 6 {
            s.extend((self.objetos.len() as i32).to_le_bytes());
        }
        if v >= 7 {
            s.extend((self.controladores.len() as i32).to_le_bytes());
        }
        for a in &self.areas {
            let mut b = a.bruto.clone();
            b[4..8].copy_from_slice(&(a.geradores.len() as i32).to_le_bytes());
            s.extend(b);
            a.geradores.iter().for_each(|g| s.extend(g));
        }
        for a in &self.areas_de_recurso {
            let mut b = a.bruto.clone();
            b[20..24].copy_from_slice(&(a.recursos.len() as i32).to_le_bytes());
            s.extend(b);
            a.recursos.iter().for_each(|r| s.extend(r));
        }
        self.objetos.iter().for_each(|o| s.extend(o));
        self.controladores.iter().for_each(|c| s.extend(c));
        s
    }
}

// ------------------------------------------------------------------ campos (deslocamentos das structs)

/// Tipo do campo no registro.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    I32,
    U32,
    F32,
    Bool,
    /// `unsigned char` (`dir[2]`, `rad`, `scale` dos objetos dinâmicos, B247).
    U8,
}

/// `(nome, tipo, deslocamento, versão mínima)` — os nomes seguem as structs de `npcgendata.h`.
pub type Campo = (&'static str, Tipo, usize, u32);

pub const CAMPOS_DA_AREA: &[Campo] = &[
    ("iType", Tipo::I32, 0, 1), ("vPos_x", Tipo::F32, 8, 1), ("vPos_y", Tipo::F32, 12, 1), ("vPos_z", Tipo::F32, 16, 1),
    ("vDir_x", Tipo::F32, 20, 1), ("vDir_y", Tipo::F32, 24, 1), ("vDir_z", Tipo::F32, 28, 1),
    ("vExts_x", Tipo::F32, 32, 1), ("vExts_y", Tipo::F32, 36, 1), ("vExts_z", Tipo::F32, 40, 1),
    ("iNPCType", Tipo::I32, 44, 1), ("iGroupType", Tipo::I32, 48, 1), ("bInitGen", Tipo::Bool, 52, 1),
    ("bAutoRevive", Tipo::Bool, 53, 1), ("bValidOnce", Tipo::Bool, 54, 1), ("dwGenID", Tipo::U32, 55, 1),
    ("idCtrl", Tipo::I32, 59, 7), ("iLifeTime", Tipo::I32, 63, 7), ("iMaxNum", Tipo::I32, 67, 7),
];

pub const CAMPOS_DO_GERADOR: &[Campo] = &[
    ("dwID", Tipo::U32, 0, 1), ("dwNum", Tipo::U32, 4, 1), ("iRefresh", Tipo::I32, 8, 1), ("dwDiedTimes", Tipo::U32, 12, 1),
    ("dwAggressive", Tipo::U32, 16, 1), ("fOffsetWater", Tipo::F32, 20, 1), ("fOffsetTrn", Tipo::F32, 24, 1),
    ("dwFaction", Tipo::U32, 28, 1), ("dwFacHelper", Tipo::U32, 32, 1), ("dwFacAccept", Tipo::U32, 36, 1),
    ("bNeedHelp", Tipo::Bool, 40, 1), ("bDefFaction", Tipo::Bool, 41, 1), ("bDefFacHelper", Tipo::Bool, 42, 1),
    ("bDefFacAccept", Tipo::Bool, 43, 1), ("iPathID", Tipo::I32, 44, 1), ("iLoopType", Tipo::I32, 48, 1),
    ("iSpeedFlag", Tipo::I32, 52, 1), ("iDeadTime", Tipo::I32, 56, 1), ("iRefreshLower", Tipo::I32, 60, 11),
];

pub const CAMPOS_DA_AREA_DE_RECURSO: &[Campo] = &[
    ("vPos_x", Tipo::F32, 0, 1), ("vPos_y", Tipo::F32, 4, 1), ("vPos_z", Tipo::F32, 8, 1), ("fExtX", Tipo::F32, 12, 1),
    ("fExtZ", Tipo::F32, 16, 1), ("bInitGen", Tipo::Bool, 24, 1), ("bAutoRevive", Tipo::Bool, 25, 1),
    ("bValidOnce", Tipo::Bool, 26, 1), ("dwGenID", Tipo::U32, 27, 1), ("idCtrl", Tipo::I32, 34, 7), ("iMaxNum", Tipo::I32, 38, 7),
];

pub const CAMPOS_DO_RECURSO: &[Campo] = &[
    ("iResType", Tipo::I32, 0, 1), ("idTemplate", Tipo::I32, 4, 1), ("dwRefreshTime", Tipo::U32, 8, 1),
    ("dwNumber", Tipo::U32, 12, 1), ("fHeiOff", Tipo::F32, 16, 1),
];

/// Objeto dinâmico (`NPCGENFILEDYNOBJ`/`9`/`10`, `npcgendata.h:205-232`, `pack(1)`).
pub const CAMPOS_DO_OBJETO: &[Campo] = &[
    ("dwDynObjID", Tipo::U32, 0, 1), ("vPos_x", Tipo::F32, 4, 1), ("vPos_y", Tipo::F32, 8, 1), ("vPos_z", Tipo::F32, 12, 1),
    ("dir_0", Tipo::U8, 16, 1), ("dir_1", Tipo::U8, 17, 1), ("rad", Tipo::U8, 18, 1),
    ("idController", Tipo::U32, 19, 9), ("scale", Tipo::U8, 23, 10),
];

/// Controlador (`NPCGENFILECTRL`/`8`, `npcgendata.h:243-282`, `pack(1)`). O nome (`szName[128]`, bytes 8..136, GBK)
/// não está aqui: lê-se à parte (`nome_do_controlador`). Horários: ano [1900-2100], mês [0,11], dia da semana [0,6]
/// (0 = domingo), dia [1,31], hora [0,23], minuto [0,59]; -1 = qualquer (`NPCCTRLTIME`, `:233-241`).
pub const CAMPOS_DO_CONTROLADOR: &[Campo] = &[
    ("id", Tipo::U32, 0, 1), ("iControllerID", Tipo::I32, 4, 1), ("bActived", Tipo::Bool, 136, 1),
    ("iWaitTime", Tipo::I32, 137, 1), ("iStopTime", Tipo::I32, 141, 1), ("bActiveTimeInvalid", Tipo::Bool, 145, 1),
    ("bStopTimeInvalid", Tipo::Bool, 146, 1),
    ("ActiveTime_iYear", Tipo::I32, 147, 1), ("ActiveTime_iMouth", Tipo::I32, 151, 1), ("ActiveTime_iWeek", Tipo::I32, 155, 1),
    ("ActiveTime_iDay", Tipo::I32, 159, 1), ("ActiveTime_iHours", Tipo::I32, 163, 1), ("ActiveTime_iMinutes", Tipo::I32, 167, 1),
    ("StopTime_iYear", Tipo::I32, 171, 1), ("StopTime_iMouth", Tipo::I32, 175, 1), ("StopTime_iWeek", Tipo::I32, 179, 1),
    ("StopTime_iDay", Tipo::I32, 183, 1), ("StopTime_iHours", Tipo::I32, 187, 1), ("StopTime_iMinutes", Tipo::I32, 191, 1),
    ("iActiveTimeRange", Tipo::I32, 195, 8),
];

/// Os bytes do nome do controlador até o primeiro NUL (GBK, como o editor gravou).
pub fn nome_do_controlador(b: &[u8]) -> &[u8] {
    let n = &b[8..136.min(b.len())];
    &n[..n.iter().position(|&c| c == 0).unwrap_or(n.len())]
}

/// Os campos de um registro como JSON `{nome: valor}`, só os que a versão tem.
pub fn ler_campos(b: &[u8], campos: &[Campo], versao: u32) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    for &(nome, tipo, o, min) in campos {
        let largura = if matches!(tipo, Tipo::Bool | Tipo::U8) { 1 } else { 4 };
        if versao < min || o + largura > b.len() {
            continue;
        }
        let v = match tipo {
            Tipo::I32 => serde_json::json!(i32::from_le_bytes(b[o..o + 4].try_into().unwrap())),
            Tipo::U32 => serde_json::json!(u32::from_le_bytes(b[o..o + 4].try_into().unwrap())),
            Tipo::F32 => serde_json::json!(f32::from_le_bytes(b[o..o + 4].try_into().unwrap())),
            Tipo::Bool => serde_json::json!(b[o] != 0),
            Tipo::U8 => serde_json::json!(b[o]),
        };
        m.insert(nome.to_string(), v);
    }
    m
}

/// Troca um campo pelo nome. Recusa campo que a versão não tem e valor fora do tipo.
pub fn definir_campo(b: &mut [u8], campos: &[Campo], versao: u32, nome: &str, valor: &serde_json::Value, onde: &'static str) -> R<()> {
    let &(_, tipo, o, min) = campos.iter().find(|c| c.0 == nome).ok_or_else(|| ErroDoNpcgen::Campo(nome.into(), onde))?;
    if versao < min {
        return Err(ErroDoNpcgen::Campo(nome.into(), onde));
    }
    let erro = || ErroDoNpcgen::Campo(format!("{nome} (valor inválido)"), onde);
    match tipo {
        Tipo::I32 => {
            let v = i32::try_from(valor.as_i64().ok_or_else(erro)?).map_err(|_| erro())?;
            b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        Tipo::U32 => {
            let v = u32::try_from(valor.as_i64().ok_or_else(erro)?).map_err(|_| erro())?;
            b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        Tipo::F32 => {
            let v = valor.as_f64().filter(|x| x.is_finite()).ok_or_else(erro)? as f32;
            b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        Tipo::Bool => b[o] = valor.as_bool().ok_or_else(erro)? as u8,
        Tipo::U8 => b[o] = u8::try_from(valor.as_i64().ok_or_else(erro)?).map_err(|_| erro())?,
    }
    Ok(())
}
