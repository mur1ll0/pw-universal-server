use byteorder::{LittleEndian, ReadBytesExt};
use pw_core::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum NpcGenError {
    #[error("Erro de I/O na leitura do npcgen.data: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formato ou versão do npcgen.data inválido: versão={0}")]
    InvalidVersion(u32),
}

pub type Result<T> = std::result::Result<T, NpcGenError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnType {
    Monster,
    Npc,
    ResourceMine,
    DynamicObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnInstance {
    pub instance_id: i32,      // ID único de instância no mundo em execução (ex: 20001, 20002...)
    pub template_id: u32,      // Template ID do elements.data
    pub spawn_type: SpawnType,
    pub pos: Vector3,
    pub dir: Vector3,
    pub respawn_sec: u32,
    pub aggressive: u32,
    /// O centro da área geradora, como o arquivo o traz.
    pub centro_da_area: Vector3,
    /// `vExts` da área — o **tamanho** da caixa, não o raio.
    pub extensao_da_area: Vector3,
    /// `iType` da área: no chão ou numa caixa. É o que decide como a altura é resolvida —
    /// ver [`TipoDeArea`].
    pub tipo_de_area: TipoDeArea,
    /// `fOffsetTrn` do gerador (monstro/NPC) ou `fHeiOff` do recurso: quanto acima do chão
    /// a entidade nasce. **Zero em 18.902 dos 18.903 geradores do mundo** do 155BR.
    pub acima_do_chao: f32,
    /// `fOffsetWater` do gerador: o mesmo, medido da superfície da água. O servidor ainda
    /// não tem mapa de água, então isto só é guardado.
    pub acima_da_agua: f32,
    /// `iPathID` do gerador: o caminho que o monstro patrulha, ou 0.
    pub caminho: i32,
    /// `iSpeedFlag` do gerador: `true` corre no caminho, `false` anda.
    pub corre_no_caminho: bool,
}

impl SpawnInstance {
    /// A altura com que esta entidade nasce, dada a altura do chão sob ela.
    ///
    /// É a regra dos dois geradores de posição do original, sem nenhuma dedução nossa
    /// (`gs/npcgenerator.cpp:4296-4346`):
    ///
    /// | área | altura final |
    /// | :--- | :--- |
    /// | [`TipoDeArea::NoChao`] | `chão + acima_do_chao` |
    /// | [`TipoDeArea::NaCaixa`] | `max(y sorteado na caixa, chão) + acima_do_chao` |
    ///
    /// `chao = None` (fora do mapa de alturas, ou mapa sem `.hmap`) não tem o que assentar:
    /// vale o `y` que o arquivo e a dispersão deram.
    ///
    /// # O que isto substituiu
    ///
    /// O item 42d deduzia o deslocamento como `centro.y − chão(centro)`, supondo que o `y`
    /// do centro da área fosse "chão + `fOffsetTrn`" e que esse campo não estivesse
    /// disponível. As duas suposições caíram quando o campo foi lido: o `fOffsetTrn` é
    /// **zero** em 18.902 dos 18.903 geradores do mundo, e o que separa uma caverna de um
    /// monstro no chão é o **tipo da área**, não o deslocamento. A dedução preservava a
    /// altura de qualquer área de chão cujo centro o editor tivesse posto um pouco acima
    /// do terreno — um NPC flutuando, por exemplo.
    pub fn altura_resolvida(&self, chao: Option<f32>) -> f32 {
        let Some(chao) = chao else {
            return self.pos.y;
        };
        match self.tipo_de_area {
            TipoDeArea::NoChao => chao + self.acima_do_chao,
            TipoDeArea::NaCaixa => self.pos.y.max(chao) + self.acima_do_chao,
        }
    }
}

/// `iType` da área do `npcgen.data` — decide o gerador de posição do original
/// (`base_spawner::SetRegion`, `gs/npcgenerator.cpp:4348-4362`).
///
/// Quem resolve a altura final é `WorldInstance::init_spawns`, onde está o mapa de alturas;
/// este leitor não depende do terreno (são 88 MB por mapa, e há 68 pastas de mapa no realm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoDeArea {
    /// `iType = 0`, `terrain_gen_pos`: `x`/`z` sorteados na caixa e **`y` no chão**, mais o
    /// deslocamento do gerador:
    ///
    /// ```cpp
    /// pos.y = offset;  ...  pos.y += plane->GetHeightAt(pos.x, pos.z);
    /// ```
    ///
    /// Recursos (minério, erva) são **sempre** deste tipo: o original monta a área deles
    /// com `SetRegion(0, ...)` (`npcgenerator.cpp:3900-3902`).
    NoChao,
    /// `iType = 1`, `box_gen_pos`: os **três** eixos sorteados na caixa, e o chão vale como
    /// **piso** — nunca como teto:
    ///
    /// ```cpp
    /// float height = plane->GetHeightAt(pos.x,pos.z);
    /// if (pos.y < height) pos.y = height;
    /// pos.y += offset;
    /// ```
    ///
    /// É assim que caverna e gerador aéreo existem com o mesmo mapa de alturas.
    NaCaixa,
}

impl TipoDeArea {
    fn do_arquivo(i_type: i32) -> Self {
        // O original faz `ASSERT(false)` em qualquer outro valor. Aqui cai no chão, que é
        // o gerador comum — e é o que 11.306 das 14.823 áreas do mundo usam.
        if i_type == 1 {
            TipoDeArea::NaCaixa
        } else {
            TipoDeArea::NoChao
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpatialGrid {
    pub cell_size: f32, // padrão 64.0m
    pub cells: HashMap<(i32, i32), Vec<SpawnInstance>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, spawn: SpawnInstance) {
        let gx = (spawn.pos.x / self.cell_size).floor() as i32;
        let gz = (spawn.pos.z / self.cell_size).floor() as i32;
        self.cells.entry((gx, gz)).or_default().push(spawn);
    }

    /// Retorna todas as instâncias de NPCs/monstros no raio de visão especificado
    pub fn query_radius(&self, pos: Vector3, radius: f32) -> Vec<&SpawnInstance> {
        let mut results = Vec::new();
        let min_gx = ((pos.x - radius) / self.cell_size).floor() as i32;
        let max_gx = ((pos.x + radius) / self.cell_size).floor() as i32;
        let min_gz = ((pos.z - radius) / self.cell_size).floor() as i32;
        let max_gz = ((pos.z + radius) / self.cell_size).floor() as i32;

        let r_sq = radius * radius;
        for gx in min_gx..=max_gx {
            for gz in min_gz..=max_gz {
                if let Some(cell_spawns) = self.cells.get(&(gx, gz)) {
                    for spawn in cell_spawns {
                        let dx = spawn.pos.x - pos.x;
                        let dz = spawn.pos.z - pos.z;
                        if (dx * dx + dz * dz) <= r_sq {
                            results.push(spawn);
                        }
                    }
                }
            }
        }
        results
    }
}

#[derive(Debug, Clone, Default)]
pub struct NpcGenData {
    pub version: u32,
    pub instances: Vec<SpawnInstance>,
    pub grid: SpatialGrid,
}

/// Onde um monstro nasce dentro da área que o gera.
///
/// # O que o original faz
///
/// `base_spawner::SetRegion` monta a caixa da área com `pos ∓ exts/2`, e
/// `terrain_gen_pos::Generate` sorteia `x` e `z` uniformemente dentro dela
/// (`abase::Rand(pos_min.x, pos_max.x)`), tentando até cinco vezes achar um ponto válido
/// pelo *pathfinding*, e por fim assenta `y` na altura do terreno.
///
/// # Onde este porte difere, e por quê
///
/// - **Determinístico**, não aleatório. O original sorteia a cada nascimento; aqui a
///   posição é uma função de `(id da instância, índice)`, então o mesmo `npcgen.data`
///   sempre produz o mesmo mundo. Reiniciar o servidor deixa de teleportar todo monstro,
///   e o teste passa a poder afirmar posição. A distribuição continua espalhada — é só a
///   semente que é fixa.
/// - **Sem altura de terreno.** O leitor não tem o mapa; `y` fica no centro da área. Em
///   área plana não muda nada; em encosta o monstro pode ficar um pouco acima ou abaixo
///   do chão, e é isso que falta para fechar com o original.
///
/// # Por que isto importa
///
/// Antes, `exts` era lido e descartado, e a posição vinha de um deslocamento fixo de até
/// três metros em diagonal — todo monstro de uma área nascia empilhado no mesmo ponto.
/// Foi o que o Murillo viu em jogo em 2026-09-07: "estão spawnando todos agrupados".
fn posicao_na_area(centro: Vector3, exts: Vector3, id: i32, indice: u32) -> Vector3 {
    // Área sem tamanho é um ponto: um monstro só, no centro. O original trata este caso
    // à parte (`_pos_min.squared_distance(_pos_max) < 1e-3`).
    if exts.x.abs() < 1e-3 && exts.y.abs() < 1e-3 && exts.z.abs() < 1e-3 {
        return centro;
    }

    // Dois valores em 0..1 a partir de `(id, índice)`. É um hash de inteiros barato
    // (splitmix64), não um gerador de qualidade — o que se quer é espalhar, não simular.
    let mistura = |mut x: u64| -> f32 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // 24 bits bastam para posição em metros e evitam o degrau de arredondamento de f32.
        (z >> 40) as f32 / (1u32 << 24) as f32
    };

    let semente = ((id as u32 as u64) << 20) | indice as u64;
    let fx = mistura(semente);
    let fz = mistura(semente ^ 0xA5A5_A5A5_A5A5_A5A5);

    // `exts` é o **tamanho** da caixa, não o raio: metade para cada lado do centro.
    //
    // O `y` também é sorteado na caixa: é o que `box_gen_pos` faz. Para a área no chão ele
    // não importa — `init_spawns` o substitui pela altura do terreno —, mas para a área em
    // caixa ele é a altura de verdade.
    let fy = mistura(semente ^ 0x5A5A_5A5A_5A5A_5A5A);
    Vector3::new(
        centro.x + (fx - 0.5) * exts.x,
        centro.y + (fy - 0.5) * exts.y,
        centro.z + (fz - 0.5) * exts.z,
    )
}

impl NpcGenData {
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        info!("Carregando npcgen.data (Spawns oficiais de Monstros e NPCs do Perfect World)...");

        let version = cursor.read_u32::<LittleEndian>()?;
        // O cabeçalho tem 2, 3 ou 4 inteiros conforme a versão
        // (`NPCGENFILEHEADER`/`HEADER6`/`HEADER7`, `cgame/gs/template/npcgendata.h`,
        // EvolvedPW) — `iNumDynObj` só existe a partir de `version >= 6`, `iNumNPCCtrl` só a
        // partir de `version >= 7`. Confirmado contra dois arquivos reais e vazios do 1.5.5
        // (`a03`/`a04`, `version = 5`, 12 bytes exatos: `[version][num_ai_gen][num_res_area]`,
        // nada mais).
        let num_ai_gen = cursor.read_i32::<LittleEndian>()? as usize;
        let num_res_area = cursor.read_i32::<LittleEndian>()? as usize;
        let num_dyn_obj = if version >= 6 {
            cursor.read_i32::<LittleEndian>()? as usize
        } else {
            0
        };
        let num_npc_ctrl = if version >= 7 {
            cursor.read_i32::<LittleEndian>()? as usize
        } else {
            0
        };
        // A struct de área (`NPCGENFILEAREA7`, 71 bytes) só é a certa pra `version >= 7` —
        // versões mais antigas usam `NPCGENFILEAREA`, sem `idCtrl`/`iLifeTime`/`iMaxNum` (59
        // bytes), formato que este parser ainda não implementa por falta de um arquivo real
        // pra confirmar contra. Só é seguro pular essa checagem quando não há nenhuma área
        // pra ler (caso real conhecido: `a03`/`a04`) — recusar em vez de arriscar ler errado.
        if version < 7 && (num_ai_gen > 0 || num_res_area > 0) {
            return Err(NpcGenError::InvalidVersion(version));
        }

        info!(
            "npcgen.data v{}: {} áreas de IA, {} áreas de recursos, {} objetos dinâmicos, {} controladores",
            version, num_ai_gen, num_res_area, num_dyn_obj, num_npc_ctrl
        );

        let mut instances = Vec::with_capacity(36000);
        let mut grid = SpatialGrid::new(64.0);
        let mut instance_counter: u32 = 1000;

        // A decisão "esta área liga sozinha no boot?" não pode ser tomada aqui: ela
        // depende do estado (`ativado`) do controlador referido por `id_ctrl`, e os
        // controladores só vêm na SEÇÃO 4, no fim do arquivo. Em vez de reler o arquivo
        // duas vezes, as seções 1-3 só **armazenam** o que vão precisar (posição,
        // `id_ctrl`, geradores) e a decisão real — e a criação das `SpawnInstance` — só
        // acontece depois da seção 4, quando o mapa de controladores já existe. Ver o
        // comentário da seção 4 para o porquê disto ser necessário (achado em
        // 2026-09-04: `id_ctrl != 0` não quer dizer "inativo", quer dizer "controlado" —
        // e a maioria dos controladores referenciados já nasce `ativado`).
        struct AreaPendente {
            id_ctrl: i32,
            b_init_gen: bool,
            tipo: TipoDeArea,
            pos: Vector3,
            /// `vExts` do `NPCGENFILEAREA`: o **tamanho** da caixa da área, não o raio. O
            /// original monta a caixa com `pos ∓ exts/2` (`base_spawner::SetRegion`) e
            /// sorteia cada monstro dentro dela. Ler e descartar isto foi o que fez todos
            /// os monstros de uma área nascerem empilhados.
            exts: Vector3,
            dir: Vector3,
            geradores: Vec<GeradorPendente>,
        }
        struct GeradorPendente {
            tid: u32,
            quantidade: u32,
            refresh: i32,
            agressivo: u32,
            acima_do_chao: f32,
            acima_da_agua: f32,
            caminho: i32,
            corre: bool,
        }
        struct ResAreaPendente {
            id_ctrl: i32,
            b_init_gen: bool,
            pos: Vector3,
            /// `fExtX`/`fExtZ`. Eram lidos e descartados, e todo recurso de uma área nascia
            /// no mesmo ponto — o "Eufórbio todo junto" do teste de 2026-09-12.
            extensao: Vector3,
            geradores: Vec<(u32, u32, u32, f32)>, // template_id, refresh, count, fHeiOff
        }
        struct DynObjPendente {
            id: u32,
            id_ctrl: i32,
            pos: Vector3,
        }
        let mut areas_pendentes = Vec::with_capacity(num_ai_gen);
        let mut res_areas_pendentes = Vec::with_capacity(num_res_area);
        let mut dynobjs_pendentes = Vec::with_capacity(num_dyn_obj);

        // 1. Áreas de IA (Monstros e NPCs de Cidade) - Estrutura NPCGENFILEAREA7 (71 bytes)
        for _ in 0..num_ai_gen {
            let area_type = cursor.read_i32::<LittleEndian>()?;
            let num_gen = cursor.read_i32::<LittleEndian>()? as usize;
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;
            let dir_x = cursor.read_f32::<LittleEndian>()?;
            let dir_y = cursor.read_f32::<LittleEndian>()?;
            let dir_z = cursor.read_f32::<LittleEndian>()?;
            let ext_x = cursor.read_f32::<LittleEndian>()?;
            let ext_y = cursor.read_f32::<LittleEndian>()?;
            let ext_z = cursor.read_f32::<LittleEndian>()?;
            let _npc_type = cursor.read_i32::<LittleEndian>()?;
            let _grp_type = cursor.read_i32::<LittleEndian>()?;
            let b_init_gen = cursor.read_u8()? != 0;
            let _b_auto_revive = cursor.read_u8()? != 0;
            let _b_valid_once = cursor.read_u8()? != 0;
            let _dw_gen_id = cursor.read_u32::<LittleEndian>()?;
            let id_ctrl = cursor.read_i32::<LittleEndian>()?;
            let _life_time = cursor.read_i32::<LittleEndian>()?;
            let _max_num = cursor.read_i32::<LittleEndian>()?;

            let mut geradores = Vec::with_capacity(num_gen);
            for _ in 0..num_gen {
                // `NPCGENFILEAIGEN10`/`NPCGENFILEAIGEN` (`gs/template/npcgendata.h:96-145`).
                let tid = cursor.read_u32::<LittleEndian>()?;
                let count = cursor.read_u32::<LittleEndian>()?;
                let refresh = cursor.read_i32::<LittleEndian>()?;
                let _died_times = cursor.read_u32::<LittleEndian>()?;
                let aggressive = cursor.read_u32::<LittleEndian>()?;
                let acima_da_agua = cursor.read_f32::<LittleEndian>()?;
                let acima_do_chao = cursor.read_f32::<LittleEndian>()?;
                // `dwFaction`, `dwFacHelper`, `dwFacAccept` (12) e os quatro `bool` de
                // facção (4): o servidor ainda não tem sistema de facção de monstro.
                cursor.seek(SeekFrom::Current(16))?;
                let caminho = cursor.read_i32::<LittleEndian>()?;
                let _loop_type = cursor.read_i32::<LittleEndian>()?;
                let speed_flag = cursor.read_i32::<LittleEndian>()?;
                let _dead_time = cursor.read_i32::<LittleEndian>()?;

                // O registro de gerador tem tamanho **diferente** conforme a versão: 60
                // bytes (`NPCGENFILEAIGEN10`) pra `version < 11`, 64 (`NPCGENFILEAIGEN`,
                // ganhou o campo `iRefreshLower` no fim) pra `version >= 11` — 20 já lidos
                // acima, faltam 40 ou 44. **Esse é o bug que travava o `npcgen.data` do
                // 1.5.5** (`version = 11`): o parser lia sempre 60 bytes, e cada gerador
                // deslocava 4 bytes a mais a partir daí — cascata que arrebentava o arquivo
                // inteiro (`failed to fill whole buffer`). Confirmado batendo os 4.273.236
                // bytes exatos de `data/realm_155/config/world/npcgen.data` com esta conta
                // (zero sobra) e por conteúdo real dos controladores no fim do arquivo
                // (nomes de evento legíveis em chinês, ex. "年兽刷新-改圣诞老人触发怪") — ver
                // `docs/ESTADO_E_RETOMADA.md`.
                //
                // Os 40 bytes que antes eram pulados de uma vez agora são lidos campo a
                // campo acima; o que sobra é o `iRefreshLower`, só do `version >= 11`.
                if version >= 11 {
                    let _refresh_lower = cursor.read_i32::<LittleEndian>()?;
                }

                if tid > 0 {
                    geradores.push(GeradorPendente {
                        tid,
                        quantidade: count,
                        refresh,
                        agressivo: aggressive,
                        acima_do_chao,
                        acima_da_agua,
                        caminho,
                        corre: speed_flag == 1,
                    });
                }
            }
            areas_pendentes.push(AreaPendente {
                id_ctrl,
                b_init_gen,
                tipo: TipoDeArea::do_arquivo(area_type),
                pos: Vector3::new(pos_x, pos_y, pos_z),
                exts: Vector3::new(ext_x, ext_y, ext_z),
                dir: Vector3::new(dir_x, dir_y, dir_z),
                geradores,
            });
        }

        // 2. Áreas de Recursos / Minérios - Estrutura NPCGENFILERESAREA7 (42 bytes)
        for _ in 0..num_res_area {
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;
            let ext_x = cursor.read_f32::<LittleEndian>()?;
            let ext_z = cursor.read_f32::<LittleEndian>()?;
            let num_res = cursor.read_i32::<LittleEndian>()? as usize;
            let b_init_gen = cursor.read_u8()? != 0;
            let _b_auto_revive = cursor.read_u8()? != 0;
            let _b_valid_once = cursor.read_u8()? != 0;
            let _dw_gen_id = cursor.read_u32::<LittleEndian>()?;
            let _dir = cursor.read_u16::<LittleEndian>()?;
            let _rad = cursor.read_u8()?;
            let id_ctrl = cursor.read_i32::<LittleEndian>()?;
            let _max_num = cursor.read_i32::<LittleEndian>()?;

            let mut geradores = Vec::with_capacity(num_res);
            for _ in 0..num_res {
                let _res_type = cursor.read_i32::<LittleEndian>()?;
                let template_id = cursor.read_u32::<LittleEndian>()?;
                let refresh = cursor.read_u32::<LittleEndian>()?;
                let count = cursor.read_u32::<LittleEndian>()?;
                let hei_off = cursor.read_f32::<LittleEndian>()?;

                if template_id > 0 {
                    geradores.push((template_id, refresh, count, hei_off));
                }
            }
            res_areas_pendentes.push(ResAreaPendente {
                id_ctrl,
                b_init_gen,
                pos: Vector3::new(pos_x, pos_y, pos_z),
                // O original monta a caixa do recurso com `{fExtX, 0, fExtZ}`
                // (`npcgenerator.cpp:3900`): sem altura, porque recurso é sempre no chão.
                extensao: Vector3::new(ext_x, 0.0, ext_z),
                geradores,
            });
        }

        // 3. Objetos dinâmicos (decorações/interativos do mapa) — `NPCGENFILEDYNOBJ10` (24
        // bytes: id u32, pos 3×f32, dir[2] u8, rad u8, id_controller u32, scale u8).
        // `version >= 10` sempre usa este formato (nossos dois arquivos reais, 10 e 11,
        // qualificam); `dir`/`rad` são uma direção comprimida que ainda não decodifiquei —
        // fica `Vector3::new(0.0, 0.0, 1.0)` de propósito, mesmo placeholder que os spawns
        // de recurso já usam, em vez de inventar um valor. Confirmado por amostra real:
        // posições plausíveis (ex. id=92, pos=(-2688.65, 220.13, 4645.19), coerente com as
        // coordenadas de mapa já vistas em outros testes deste projeto).
        for _ in 0..num_dyn_obj {
            let dyn_obj_id = cursor.read_u32::<LittleEndian>()?;
            let pos_x = cursor.read_f32::<LittleEndian>()?;
            let pos_y = cursor.read_f32::<LittleEndian>()?;
            let pos_z = cursor.read_f32::<LittleEndian>()?;
            cursor.seek(SeekFrom::Current(2))?; // dir[2], comprimido -- não decodificado ainda
            let _rad = cursor.read_u8()?;
            let id_ctrl = cursor.read_i32::<LittleEndian>()?;
            let _scale = cursor.read_u8()?;

            if dyn_obj_id > 0 {
                dynobjs_pendentes.push(DynObjPendente {
                    id: dyn_obj_id,
                    id_ctrl,
                    pos: Vector3::new(pos_x, pos_y, pos_z),
                });
            }
        }

        // 4. Controladores de gatilho (`NPCGENFILECTRL8`, 199 bytes: id u32, controller_id
        // i32, nome char[128], ativado bool, tempos de espera/parada i32×2, flags de
        // validade de horário bool×2, ActiveTime/StopTime `NPCCTRLTIME` (6 i32 cada, 24
        // bytes), faixa de horário i32).
        //
        // **Achado em 2026-09-04, com evidência de hex, não suposição**: `id_ctrl != 0`
        // NÃO significa "esta área começa desligada". Inspecionando o controlador que
        // guarda a área do NPC "Ancião" que faltava no mundo (`id_ctrl=2077` na área,
        // `data/realm_155BR/config/world/npcgen.data`): o registro do controlador 2077
        // tem `ativado=1` e o nome (GBK) decodifica como "大地图默认长老" — "Ancião
        // Padrão do Mapa Aberto". Ou seja, a maioria das áreas com `id_ctrl != 0` está
        // ligada a um controlador **já ativado por padrão** (o mecanismo normal de
        // respawn/registro de NPCs permanentes), e só uma minoria de fato representa
        // evento sazonal desligado (`ativado=0` — dois exemplos reais no mesmo arquivo,
        // nomes que remetem a eventos e fluxo de missão). Tratar `id_ctrl != 0` como
        // sinônimo de "inativo" (o comportamento até esta sessão) deixava de fora
        // qualquer NPC/monstro/recurso permanente cuja área usa um controlador — inclusive
        // NPCs-âncora de cidade inicial, o que bastava pra sumir com uma cidade inteira
        // de NPCs "normais". Corrigido: cada `id_ctrl` só desativa a área se apontar pra
        // um controlador que existe **e** está com `ativado=0`; um `id_ctrl` que não bate
        // com nenhum controlador do arquivo (não deveria acontecer, mas por segurança)
        // continua contando como ativo — a suposição seguraa é "existe", não "sumiu".
        const NPCGENFILECTRL8_SIZE: i64 = 4 + 4 + 128 + 1 + 4 + 4 + 1 + 1 + 24 + 24 + 4;
        let mut controladores_ativados: HashMap<u32, bool> = HashMap::with_capacity(num_npc_ctrl);
        for _ in 0..num_npc_ctrl {
            let id = cursor.read_u32::<LittleEndian>()?;
            let _controller_id = cursor.read_i32::<LittleEndian>()?;
            cursor.seek(SeekFrom::Current(128))?; // nome, char[128]
            let ativado = cursor.read_u8()? != 0;
            // resto do registro: espera/parar (2×i32), 2 bools de horário, ActiveTime/
            // StopTime (24B cada), faixa de horário (i32) — nada mais precisa ser lido.
            let resto = NPCGENFILECTRL8_SIZE - 4 - 4 - 128 - 1;
            cursor.seek(SeekFrom::Current(resto))?;
            controladores_ativados.insert(id, ativado);
        }
        let esta_ativa = |id_ctrl: i32| -> bool {
            id_ctrl == 0 || controladores_ativados.get(&(id_ctrl as u32)).copied().unwrap_or(true)
        };

        // Agora que os controladores são conhecidos, cada seção vira `SpawnInstance` de
        // verdade — mesma lógica de antes, só que a decisão de "ativa no boot" usa
        // `esta_ativa` em vez do antigo `id_ctrl == 0`.
        for area in &areas_pendentes {
            if !area.b_init_gen || !esta_ativa(area.id_ctrl) {
                continue;
            }
            for g in &area.geradores {
                // **Sem teto.** Isto era `count.min(10)`, e o de recurso `count.min(5)` —
                // números sem origem no original, que usa `dwNum`/`dwNumber` como veio.
                // Medido no mundo do 155BR: os dois tetos escondiam 5.811 monstros e NPCs
                // (337 geradores declaram mais de dez) e 271 recursos.
                for c in 0..g.quantidade {
                    instance_counter += 1;
                    // No Perfect World oficial, IDs de NPCs/Monstros possuem o bit 31 ativo (ISNPCID: (id & 0x80000000) && !(id & 0x40000000))
                    let npc_nid = (0x80000000u32 | (instance_counter & 0x3FFFFFFF)) as i32;
                    let pos = posicao_na_area(area.pos, area.exts, npc_nid, c);
                    let spawn = SpawnInstance {
                        instance_id: npc_nid,
                        template_id: g.tid,
                        spawn_type: if g.tid >= 10000 { SpawnType::Npc } else { SpawnType::Monster },
                        pos,
                        dir: area.dir,
                        respawn_sec: g.refresh.max(1) as u32,
                        aggressive: g.agressivo,
                        centro_da_area: area.pos,
                        extensao_da_area: area.exts,
                        tipo_de_area: area.tipo,
                        acima_do_chao: g.acima_do_chao,
                        acima_da_agua: g.acima_da_agua,
                        caminho: g.caminho,
                        corre_no_caminho: g.corre,
                    };
                    grid.insert(spawn.clone());
                    instances.push(spawn);
                }
            }
        }
        for area in &res_areas_pendentes {
            if !area.b_init_gen || !esta_ativa(area.id_ctrl) {
                continue;
            }
            for &(template_id, refresh, count, hei_off) in &area.geradores {
                for c in 0..count {
                    instance_counter += 1;
                    let matter_id = (0xC0000000u32 | (instance_counter & 0x3FFFFFFF)) as i32;
                    let spawn = SpawnInstance {
                        instance_id: matter_id,
                        template_id,
                        spawn_type: SpawnType::ResourceMine,
                        // Espalhado na caixa `fExtX × fExtZ`, como o original. Antes ficava
                        // tudo em `area.pos`.
                        pos: posicao_na_area(area.pos, area.extensao, matter_id, c),
                        dir: Vector3::new(0.0, 0.0, 1.0),
                        // `reborn_time = max(BASE_REBORN_TIME + dwRefreshTime, 15)`
                        // (`npcgenerator.cpp:3920-3921`); o piso de 5 que estava aqui não
                        // tinha origem.
                        respawn_sec: refresh.max(15),
                        aggressive: 0,
                        centro_da_area: area.pos,
                        extensao_da_area: area.extensao,
                        tipo_de_area: TipoDeArea::NoChao,
                        acima_do_chao: hei_off,
                        acima_da_agua: 0.0,
                        caminho: 0,
                        corre_no_caminho: false,
                    };
                    grid.insert(spawn.clone());
                    instances.push(spawn);
                }
            }
        }
        for obj in &dynobjs_pendentes {
            if !esta_ativa(obj.id_ctrl) {
                continue;
            }
            instance_counter += 1;
            let dynobj_nid = (0xA0000000u32 | (instance_counter & 0x3FFFFFFF)) as i32;
            let spawn = SpawnInstance {
                instance_id: dynobj_nid,
                template_id: obj.id,
                spawn_type: SpawnType::DynamicObject,
                pos: obj.pos,
                dir: Vector3::new(0.0, 0.0, 1.0),
                respawn_sec: 0,
                aggressive: 0,
                // Objeto dinâmico (prédio, ponte) não é gerado numa área: a posição do
                // arquivo é a dele, e não há dispersão para reassentar. É `NaCaixa` com
                // caixa nula para que `init_spawns` só levante o que estiver abaixo do chão.
                centro_da_area: obj.pos,
                extensao_da_area: Vector3::new(0.0, 0.0, 0.0),
                tipo_de_area: TipoDeArea::NaCaixa,
                acima_do_chao: 0.0,
                acima_da_agua: 0.0,
                caminho: 0,
                corre_no_caminho: false,
            };
            grid.insert(spawn.clone());
            instances.push(spawn);
        }

        info!(
            "npcgen.data carregado com sucesso: {} entidades indexadas no Grid Espacial de 64m",
            instances.len()
        );

        Ok(Self {
            version,
            instances,
            grid,
        })
    }

    pub fn query_nearby(&self, pos: Vector3, radius: f32) -> Vec<&SpawnInstance> {
        self.grid.query_radius(pos, radius)
    }
}

/// Compacta a direção horizontal em um byte (0..255), idêntico a glb_CompressDirH da engine oficial
pub fn compress_dir_h(x: f32, z: f32) -> u8 {
    const INV_INTER: f32 = 256.0 / 360.0;
    if x.abs() < 0.00001 {
        if z > 0.0 {
            64
        } else {
            192
        }
    } else {
        let deg = z.atan2(x).to_degrees();
        let deg_norm = if deg < 0.0 { deg + 360.0 } else { deg };
        (deg_norm * INV_INTER) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(tipo: TipoDeArea, y: f32, acima: f32) -> SpawnInstance {
        SpawnInstance {
            instance_id: 1,
            template_id: 1,
            spawn_type: SpawnType::Monster,
            pos: Vector3::new(0.0, y, 0.0),
            dir: Vector3::new(0.0, 0.0, 1.0),
            respawn_sec: 1,
            aggressive: 0,
            centro_da_area: Vector3::new(0.0, y, 0.0),
            extensao_da_area: Vector3::new(0.0, 0.0, 0.0),
            tipo_de_area: tipo,
            acima_do_chao: acima,
            acima_da_agua: 0.0,
            caminho: 0,
            corre_no_caminho: false,
        }
    }

    /// Área de chão ignora o `y` do arquivo: nasce no terreno, mais o `fOffsetTrn`.
    #[test]
    fn area_de_chao_nasce_no_terreno() {
        // O `y` do arquivo (250) está 30 m acima do chão (220): não importa.
        assert_eq!(spawn(TipoDeArea::NoChao, 250.0, 0.0).altura_resolvida(Some(220.0)), 220.0);
        assert_eq!(spawn(TipoDeArea::NoChao, 250.0, 1.5).altura_resolvida(Some(220.0)), 221.5);
    }

    /// Área em caixa respeita o `y` sorteado — o terreno é piso, nunca teto.
    #[test]
    fn area_em_caixa_usa_o_terreno_como_piso() {
        // Acima do chão: fica onde está (gerador aéreo).
        assert_eq!(spawn(TipoDeArea::NaCaixa, 300.0, 0.0).altura_resolvida(Some(220.0)), 300.0);
        // Abaixo do chão: sobe para o chão.
        assert_eq!(spawn(TipoDeArea::NaCaixa, 150.0, 0.0).altura_resolvida(Some(220.0)), 220.0);
        // O deslocamento soma depois do piso, como no original.
        assert_eq!(spawn(TipoDeArea::NaCaixa, 150.0, 2.0).altura_resolvida(Some(220.0)), 222.0);
    }

    /// Fora do mapa de alturas não há o que assentar: vale o `y` que veio.
    #[test]
    fn sem_chao_vale_o_y_do_arquivo() {
        assert_eq!(spawn(TipoDeArea::NoChao, 250.0, 0.0).altura_resolvida(None), 250.0);
        assert_eq!(spawn(TipoDeArea::NaCaixa, 250.0, 3.0).altura_resolvida(None), 250.0);
    }

    /// Monta um `npcgen.data` sintético em memória: cabeçalho v10 + N áreas de IA (uma
    /// só gerador cada) + M controladores, byte a byte, do jeito que
    /// `NpcGenData::load_from_bytes` espera. `id_ctrl == 0` faz a área ignorar o mapa de
    /// controladores; um `id_ctrl != 0` procura o controlador daquele id.
    fn area_de_ia(pos: (f32, f32, f32), id_ctrl: i32, tid: u32) -> Vec<u8> {
        let mut b = Vec::with_capacity(71 + 60);
        b.extend((1i32).to_le_bytes()); // area_type
        b.extend((1i32).to_le_bytes()); // num_gen
        b.extend(pos.0.to_le_bytes());
        b.extend(pos.1.to_le_bytes());
        b.extend(pos.2.to_le_bytes());
        b.extend(0f32.to_le_bytes()); // dir_x
        b.extend(0f32.to_le_bytes()); // dir_y
        b.extend(1f32.to_le_bytes()); // dir_z
        b.extend([0u8; 12]); // ext x/y/z
        b.extend((0i32).to_le_bytes()); // npc_type
        b.extend((0i32).to_le_bytes()); // grp_type
        b.push(1); // b_init_gen = true
        b.push(0); // b_auto_revive
        b.push(0); // b_valid_once
        b.extend((0u32).to_le_bytes()); // dw_gen_id
        b.extend(id_ctrl.to_le_bytes());
        b.extend((0i32).to_le_bytes()); // life_time
        b.extend((0i32).to_le_bytes()); // max_num
        assert_eq!(b.len(), 71);
        // gerador (NPCGENFILEAIGEN10, versão < 11: 60 bytes)
        b.extend(tid.to_le_bytes());
        b.extend((1u32).to_le_bytes()); // count
        b.extend((60i32).to_le_bytes()); // refresh
        b.extend((0u32).to_le_bytes()); // died_times
        b.extend((0u32).to_le_bytes()); // aggressive
        b.extend([0u8; 40]); // resto do registro (v < 11)
        b
    }

    fn controlador(id: u32, ativado: bool) -> Vec<u8> {
        let mut b = Vec::with_capacity(199);
        b.extend(id.to_le_bytes());
        b.extend((0i32).to_le_bytes()); // controller_id
        b.extend([0u8; 128]); // nome
        b.push(if ativado { 1 } else { 0 });
        b.extend([0u8; 62]); // espera/parar/flags/ActiveTime/StopTime/faixa
        assert_eq!(b.len(), 199);
        b
    }

    /// Achado em 2026-09-04: uma área com `id_ctrl != 0` cujo controlador está
    /// `ativado=1` DEVE spawnar — era tratada como "sempre inativa" antes deste
    /// conserto, o que sumia com NPCs permanentes (o "Ancião" da cidade inicial,
    /// achado batendo com `data/realm_155BR/config/world/npcgen.data` real).
    #[test]
    fn area_com_controlador_ativado_spawna() {
        let mut buf = Vec::new();
        buf.extend((10u32).to_le_bytes()); // version
        buf.extend((1i32).to_le_bytes()); // num_ai_gen
        buf.extend((0i32).to_le_bytes()); // num_res_area
        buf.extend((0i32).to_le_bytes()); // num_dyn_obj
        buf.extend((1i32).to_le_bytes()); // num_npc_ctrl
        buf.extend(area_de_ia((10.0, 0.0, 20.0), 2077, 5001));
        buf.extend(controlador(2077, true));

        let dados = NpcGenData::load_from_bytes(&buf).expect("deveria parsear sem sobra");
        assert_eq!(dados.instances.len(), 1, "controlador ativado deveria deixar a área spawnar");
        assert_eq!(dados.instances[0].template_id, 5001);
    }

    /// Espelho do teste acima: controlador `ativado=0` mantém a área desligada — não é
    /// que `id_ctrl` deixou de importar, é que agora ele é checado de verdade.
    #[test]
    fn area_com_controlador_desativado_nao_spawna() {
        let mut buf = Vec::new();
        buf.extend((10u32).to_le_bytes());
        buf.extend((1i32).to_le_bytes());
        buf.extend((0i32).to_le_bytes());
        buf.extend((0i32).to_le_bytes());
        buf.extend((1i32).to_le_bytes());
        buf.extend(area_de_ia((10.0, 0.0, 20.0), 2007, 5001));
        buf.extend(controlador(2007, false));

        let dados = NpcGenData::load_from_bytes(&buf).expect("deveria parsear sem sobra");
        assert!(dados.instances.is_empty(), "controlador desativado deveria manter a área desligada");
    }

    /// `id_ctrl == 0` nunca dependeu de controlador nenhum — continua igual.
    #[test]
    fn area_sem_controlador_sempre_spawna() {
        let mut buf = Vec::new();
        buf.extend((10u32).to_le_bytes());
        buf.extend((1i32).to_le_bytes());
        buf.extend((0i32).to_le_bytes());
        buf.extend((0i32).to_le_bytes());
        buf.extend((0i32).to_le_bytes()); // num_npc_ctrl = 0
        buf.extend(area_de_ia((10.0, 0.0, 20.0), 0, 5001));

        let dados = NpcGenData::load_from_bytes(&buf).expect("deveria parsear sem sobra");
        assert_eq!(dados.instances.len(), 1);
    }
}
