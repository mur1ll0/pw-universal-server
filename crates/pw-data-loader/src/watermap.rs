//! `watermap/` — a superfície da água de um mapa.
//!
//! Porta de `gs/pathfinding/GlobalWaterAreaMap.{h,cpp}` e `WaterAreaMap.{h,cpp}`. É o dado
//! que responde `path_finding::GetWaterHeight(plano, x, z)` no original, e dele saem três
//! coisas do jogo: o fôlego (`breath_ctrl`), a recusa de montar debaixo d'água e a **queda**
//! da montaria de quem entra na água montado (`gs/petman.cpp:402-410`).
//!
//! # Os arquivos
//!
//! Cada mapa do realm tem uma pasta `watermap/` ao lado de `map/`:
//!
//! - **`watermap.conf`**, texto, quatro linhas — `Map Width`, `Map Length`, `Submap Width`,
//!   `Submap Length` (`CGlobalWaterAreaMap::Load`, `GlobalWaterAreaMap.cpp:88-99`). No
//!   `realm_155` quase todos são 1×1 submapas de 1024×1024.
//! - **`N.wmap`**, binário, um por submapa: `u32 versão` (`0xCC00_0001`), `f32 largura`,
//!   `f32 comprimento`, `i32 n`, e então `n` áreas de **5 `f32`** — centro `x`, centro `z`,
//!   meia-largura, meio-comprimento e **altura** (`CWaterAreaMap::Load`,
//!   `WaterAreaMap.cpp:38-115`). Sem áreas o arquivo tem 16 bytes, e é o caso da maioria
//!   dos mapas.
//!
//! O nome do arquivo do submapa `(u, v)` é `(comprimento − v − 1) × largura + u + 1`
//! (`GlobalWaterAreaMap.cpp:115-118`) — a numeração cresce de baixo para cima.
//!
//! # A conta
//!
//! O centro do mapa é a origem do mundo (`SetMapCenterAsOrigin`,
//! `GlobalWaterAreaMap.h:58-64`), então um ponto do mundo entra somando meia grade. Achado
//! o submapa, a altura é a da **primeira** área que contém o ponto, e `0.0` quando nenhuma
//! contém (`NO_WATER`, `WaterAreaMap.h:21`) — ou seja, **zero quer dizer "sem água"**, não
//! "água no nível zero".

use std::path::Path;
use tracing::{debug, warn};

/// `WATER_AREA_MAP_VER` (`gs/pathfinding/WaterAreaMap.h:23`).
const VERSAO: u32 = 0xCC00_0001;
/// `NO_WATER` (`WaterAreaMap.h:21`) — a altura que significa "aqui não tem água".
pub const SEM_AGUA: f32 = 0.0;

/// Uma área retangular de água, com a altura da superfície.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaDeAgua {
    pub centro_x: f32,
    pub centro_z: f32,
    pub meia_largura: f32,
    pub meio_comprimento: f32,
    pub altura: f32,
}

impl AreaDeAgua {
    /// `CWaterArea::Inside` (`WaterAreaMap.h:43-48`) — caixa fechada nos dois lados.
    fn contem(&self, x: f32, z: f32) -> bool {
        x <= self.centro_x + self.meia_largura
            && x >= self.centro_x - self.meia_largura
            && z <= self.centro_z + self.meio_comprimento
            && z >= self.centro_z - self.meio_comprimento
    }
}

/// O mapa de água de um mapa do jogo.
#[derive(Debug, Clone, Default)]
pub struct MapaDeAgua {
    largura: i32,
    comprimento: i32,
    submapa_largura: f32,
    submapa_comprimento: f32,
    /// `m_subWaterAreaMaps`, indexado por `v * largura + u` (`SUBMAP`,
    /// `GlobalWaterAreaMap.h:19`). Submapa ausente ou ilegível fica vazio.
    submapas: Vec<Vec<AreaDeAgua>>,
}

impl MapaDeAgua {
    /// Um mapa sem água nenhuma — o que vale para mapa sem a pasta.
    pub fn vazio() -> Self {
        Self::default()
    }

    /// Um mapa montado à mão, para cenário de teste — o mesmo papel que
    /// `velocidades_de_montaria` tem no `GameDataManager`: o mundo de teste não carrega
    /// arquivo nenhum, e a regra de jogo precisa de água para ser testada.
    ///
    /// Com uma grade 1×1, a origem cai no centro do submapa e as coordenadas das áreas
    /// **são** as do mundo.
    pub fn de_areas(
        largura: i32,
        comprimento: i32,
        submapa_largura: f32,
        submapa_comprimento: f32,
        submapas: Vec<Vec<AreaDeAgua>>,
    ) -> Self {
        Self { largura, comprimento, submapa_largura, submapa_comprimento, submapas }
    }

    /// Há pelo menos uma área de água neste mapa?
    ///
    /// Serve para o log e para os testes: quase todo mapa do `realm_155` tem a pasta e
    /// **nenhuma** área, e é bom poder dizer a diferença entre "não li" e "li, não tem".
    pub fn tem_agua(&self) -> bool {
        self.submapas.iter().any(|s| !s.is_empty())
    }

    /// Lê `dir/watermap/`. `dir` é a pasta do mapa (a que contém `map/`), como em
    /// [`crate::Terreno::ler`].
    ///
    /// Pasta ausente não é erro: o mapa fica sem água, que é o mesmo que o original faz
    /// quando o `Load` falha.
    pub fn ler(world_id: i32, dir: &Path) -> Self {
        let pasta = dir.join("watermap");
        let conf = pasta.join("watermap.conf");
        let Ok(texto) = std::fs::read_to_string(&conf) else {
            debug!("watermap: mapa {world_id} sem pasta watermap — segue sem água");
            return Self::vazio();
        };
        let Some((largura, comprimento, sw, sl)) = Self::ler_conf(&texto) else {
            warn!("watermap: {} não tem as quatro medidas — mapa {world_id} sem água", conf.display());
            return Self::vazio();
        };
        if largura <= 0 || comprimento <= 0 || sw <= 0.0 || sl <= 0.0 {
            warn!("watermap: medidas inválidas em {} — mapa {world_id} sem água", conf.display());
            return Self::vazio();
        }

        let mut submapas = vec![Vec::new(); (largura * comprimento) as usize];
        let mut lidos = 0usize;
        let mut areas = 0usize;
        for v in 0..comprimento {
            for u in 0..largura {
                // `iFileID = (m_iLength - i - 1) * m_iWidth + j + 1` com `i = v`, `j = u`.
                let id = (comprimento - v - 1) * largura + u + 1;
                let caminho = pasta.join(format!("{id}.wmap"));
                let Ok(bytes) = std::fs::read(&caminho) else { continue };
                match Self::ler_submapa(&bytes, sw, sl) {
                    Some(lista) => {
                        lidos += 1;
                        areas += lista.len();
                        submapas[(v * largura + u) as usize] = lista;
                    }
                    None => warn!("watermap: {} não bate com o formato; ignorado", caminho.display()),
                }
            }
        }
        debug!(
            "watermap: mapa {world_id} — {lidos}/{} submapas, {areas} áreas de água",
            largura * comprimento
        );

        Self { largura, comprimento, submapa_largura: sw, submapa_comprimento: sl, submapas }
    }

    /// As quatro medidas do `watermap.conf`. O original as lê com `fscanf("Map Width =%d")`,
    /// que ignora o espaço antes do número; aqui basta partir no `=`.
    fn ler_conf(texto: &str) -> Option<(i32, i32, f32, f32)> {
        let mut largura = None;
        let mut comprimento = None;
        let mut sw = None;
        let mut sl = None;
        for linha in texto.lines() {
            let Some((chave, valor)) = linha.split_once('=') else { continue };
            let chave = chave.trim().to_ascii_lowercase();
            let valor = valor.trim();
            match chave.as_str() {
                "map width" => largura = valor.parse::<i32>().ok(),
                "map length" => comprimento = valor.parse::<i32>().ok(),
                "submap width" => sw = valor.parse::<f32>().ok(),
                "submap length" => sl = valor.parse::<f32>().ok(),
                _ => {}
            }
        }
        Some((largura?, comprimento?, sw?, sl?))
    }

    /// Um `N.wmap`. Devolve `None` quando a versão não bate, quando o tamanho não fecha ou
    /// quando o submapa tem medida diferente da que o `.conf` declarou — é o que o original
    /// faz, que **libera** o submapa nesse caso (`GlobalWaterAreaMap.cpp:123-127`).
    fn ler_submapa(bytes: &[u8], sw: f32, sl: f32) -> Option<Vec<AreaDeAgua>> {
        if bytes.len() < 16 {
            return None;
        }
        let u32_em = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().ok().unwrap());
        let f32_em = |i: usize| f32::from_le_bytes(bytes[i..i + 4].try_into().ok().unwrap());
        if u32_em(0) != VERSAO {
            return None;
        }
        let (largura, comprimento) = (f32_em(4), f32_em(8));
        if largura != sw || comprimento != sl {
            return None;
        }
        let n = i32::from_le_bytes(bytes[12..16].try_into().ok()?);
        if n < 0 {
            return None;
        }
        let n = n as usize;
        // **Fecha no último byte**, como todo leitor deste projeto.
        if bytes.len() != 16 + n * 20 {
            return None;
        }
        let mut areas = Vec::with_capacity(n);
        for i in 0..n {
            let b = 16 + i * 20;
            areas.push(AreaDeAgua {
                centro_x: f32_em(b),
                centro_z: f32_em(b + 4),
                meia_largura: f32_em(b + 8),
                meio_comprimento: f32_em(b + 12),
                altura: f32_em(b + 16),
            });
        }
        Some(areas)
    }

    /// `CGlobalWaterAreaMap::GetWaterHeight` (`GlobalWaterAreaMap.h:66-102`): a altura da
    /// superfície da água em `(x, z)`, ou [`SEM_AGUA`] fora de qualquer área.
    pub fn altura_em(&self, x: f32, z: f32) -> f32 {
        if self.submapas.is_empty() {
            return SEM_AGUA;
        }
        // A origem é o centro do mapa.
        let origem_x = self.submapa_largura * self.largura as f32 * 0.5;
        let origem_z = self.submapa_comprimento * self.comprimento as f32 * 0.5;
        let mut fx = x + origem_x;
        let mut fz = z + origem_z;

        let u = (fx / self.submapa_largura) as i32;
        let v = (fz / self.submapa_comprimento) as i32;
        if u < 0 || u >= self.largura || v < 0 || v >= self.comprimento {
            return SEM_AGUA;
        }
        // Dentro do submapa as áreas são relativas ao **centro** dele.
        fx -= (u as f32 + 0.5) * self.submapa_largura;
        fz -= (v as f32 + 0.5) * self.submapa_comprimento;

        let sub = &self.submapas[(v * self.largura + u) as usize];
        sub.iter().find(|a| a.contem(fx, fz)).map_or(SEM_AGUA, |a| a.altura)
    }

    /// O quanto o ponto está **abaixo** da superfície: `altura da água − y`.
    ///
    /// É o `off` do `gplayer_imp::TestUnderWater` (`gs/player.cpp:14322-14348`). Vale zero
    /// onde não há água, e negativo acima da superfície.
    pub fn quanto_abaixo(&self, x: f32, y: f32, z: f32) -> f32 {
        let altura = self.altura_em(x, z);
        if altura == SEM_AGUA {
            return 0.0;
        }
        altura - y
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn wmap(areas: &[(f32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&VERSAO.to_le_bytes());
        v.extend_from_slice(&1024.0f32.to_le_bytes());
        v.extend_from_slice(&1024.0f32.to_le_bytes());
        v.extend_from_slice(&(areas.len() as i32).to_le_bytes());
        for a in areas {
            for f in [a.0, a.1, a.2, a.3, a.4] {
                v.extend_from_slice(&f.to_le_bytes());
            }
        }
        v
    }

    #[test]
    fn o_submapa_fecha_no_ultimo_byte() {
        let bom = wmap(&[(0.0, 0.0, 10.0, 10.0, 5.0)]);
        assert_eq!(bom.len(), 36, "16 de cabeçalho + 20 da área");
        assert!(MapaDeAgua::ler_submapa(&bom, 1024.0, 1024.0).is_some());

        // Um byte a mais e um a menos: recusa nos dois.
        let mut sobra = bom.clone();
        sobra.push(0);
        assert!(MapaDeAgua::ler_submapa(&sobra, 1024.0, 1024.0).is_none(), "sobra não pode passar");
        assert!(MapaDeAgua::ler_submapa(&bom[..35], 1024.0, 1024.0).is_none(), "falta não pode passar");

        // Versão errada e medida diferente da do `.conf`.
        let mut versao = bom.clone();
        versao[0] = 0;
        assert!(MapaDeAgua::ler_submapa(&versao, 1024.0, 1024.0).is_none());
        assert!(MapaDeAgua::ler_submapa(&bom, 512.0, 1024.0).is_none(), "medida diferente do .conf");
    }

    #[test]
    fn a_altura_sai_da_area_que_contem_o_ponto() {
        // Um mapa 1×1 de 1024×1024: o centro do mapa é (0,0) do mundo, então as
        // coordenadas do submapa **são** as do mundo.
        let m = MapaDeAgua {
            largura: 1,
            comprimento: 1,
            submapa_largura: 1024.0,
            submapa_comprimento: 1024.0,
            submapas: vec![vec![
                AreaDeAgua { centro_x: 100.0, centro_z: 0.0, meia_largura: 50.0, meio_comprimento: 20.0, altura: 216.0 },
                AreaDeAgua { centro_x: -200.0, centro_z: 0.0, meia_largura: 10.0, meio_comprimento: 10.0, altura: 30.0 },
            ]],
        };

        assert_eq!(m.altura_em(100.0, 0.0), 216.0, "no centro da primeira área");
        assert_eq!(m.altura_em(150.0, 20.0), 216.0, "a caixa é fechada na borda");
        assert_eq!(m.altura_em(151.0, 0.0), SEM_AGUA, "um passo fora já é terra");
        assert_eq!(m.altura_em(-200.0, 0.0), 30.0, "a segunda área tem altura própria");
        assert_eq!(m.altura_em(0.0, 0.0), SEM_AGUA, "entre as duas não há água");
        assert_eq!(m.altura_em(5000.0, 0.0), SEM_AGUA, "fora da grade");

        // `quanto_abaixo`: positivo submerso, negativo acima, zero em terra.
        assert_eq!(m.quanto_abaixo(100.0, 214.0, 0.0), 2.0);
        assert_eq!(m.quanto_abaixo(100.0, 218.0, 0.0), -2.0);
        assert_eq!(m.quanto_abaixo(0.0, 0.0, 0.0), 0.0, "sem água é zero, não a altura do chão");
    }

    #[test]
    fn o_conf_aceita_o_espaco_que_o_original_ignora() {
        let t = "Map Width = 2\nMap Length = 3\nSubmap Width = 1024.0\nSubmap Length = 512.0";
        assert_eq!(MapaDeAgua::ler_conf(t), Some((2, 3, 1024.0, 512.0)));
        assert_eq!(MapaDeAgua::ler_conf("Map Width = 1"), None, "faltando medida, não lê");
    }
}
