//! Como o monstro de chão anda sobre o mapa de movimento: desvia de obstáculo, contorna
//! estrutura e cerca o alvo — porte de `cgame/gs/pathfinding/` do servidor 1.5.5.
//!
//! # Quem é quem no original
//!
//! | papel | classe | onde |
//! | :--- | :--- | :--- |
//! | perseguir e voltar para casa | `follow_target` → `CreateNPCChaseAgent(mapa, chão)` com o modo **padrão** `CHASE_DISPERSE_ONCE` → `CNPCDisperseChaseOnGroundAgent` | `pathfinding.h:34-134`, `NPCMoveAgent.cpp:98-101`, `NPCMoveAgent.h:58` |
//! | a base dessa perseguição | `CNPCChaseOnGroundNoBlockAgent` — **não** o `CNPCChaseOnGroundAgent`: `NPCDisperseChaseOnGroundAgent.h:21` define `CHASE_WITHOUT_BLOCK` | `NPCChaseOnGroundNoBlockAgent.{h,cpp}` |
//! | a busca dela | `CPf2DBfs`: gulosa por distância de Manhattan, 8 vizinhos, em fatias de N pixels | `Pf2DBfs.{h,cpp}`, `PathFinding2D.h` |
//! | o trajeto | `CPathFollowing`: segmentos entre centros de pixel | `PathFollowing.{h,cpp}` |
//! | passear | `cruise` → `CNPCRambleOnGroundAgent`, que por dentro usa o `CNPCChaseOnGroundAgent` (modo `CHASE_NORMAL`) com 200 pixels de busca | `NPCRambleOnGroundAgent.{h,cpp}`, `NPCMove.h:499-624` |
//!
//! Tudo em pixels do `movemap` (1 m no 1.5.5). A altura de cada posição é a do
//! `AdjustCurPos` do agente de chão: terreno no ponto, ou, onde o piso fica acima do terreno,
//! terreno no **centro do pixel** + a altura do piso (`NPCChaseOnGroundNoBlockAgent.h:31-45`).
//!
//! Sem mapa de movimento (mapa vazio), tudo é alcançável e os agentes andam em linha reta.
use pw_data_loader::MapaDeMovimento;
use rand::Rng;
use std::collections::HashMap;

/// `RELAX_ERROR`, `SQR_RELAX_ERROR`, `ZERO_DIST_ERROR`, `MAX_GENERATE_GOAL_TIMES`,
/// `MAX_BLOCK_TIMES` (`NPCMove.h:26-56`).
const RELAX_ERROR: f32 = 0.1;
const SQR_RELAX_ERROR: f32 = 0.01;
const ZERO_DIST_ERROR: f32 = 0.01;
const MAX_GENERATE_GOAL_TIMES: i32 = 11;
const MAX_BLOCK_TIMES: i32 = 3;
/// `MAX_NODES` da lista aberta do `CNPCChaseOnGroundAgent` (`SortVectorPathNode.h`).
const MAX_NODES: usize = 30;
/// `PATHFINDING_SEARCH_PIXELS` do passeio (`NPCRambleOnGroundAgent.cpp`).
const PIXELS_DO_PASSEIO: i32 = 200;
/// `SetDisperseRadianRange(2*PI/3)` (`NPCDisperseChaseOnGroundAgent.h:38`).
const ANGULO_DE_DISPERSAO: f32 = 2.0 * std::f32::consts::PI / 3.0;
/// `PF2D_INVALID_NODE` (`Pf2DBfs.cpp`).
const NO_INVALIDO: i16 = -30000;
/// `follow_target::_detail_param` (`pathfinding.cpp:9`): teto de pixels somados antes de
/// desistir, pixels de busca por passo, e pixels de busca quando bloqueado.
const DETALHE: [[i32; 3]; 3] = [[300, 20, 50], [600, 40, 90], [900, 60, 120]];

type Pixel = (i32, i32);

/// Um `A3DVECTOR3`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl V3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn sub(self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn add(self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn mul(self, k: f32) -> V3 {
        V3::new(self.x * k, self.y * k, self.z * k)
    }
    fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn sqr(self) -> f32 {
        self.dot(self)
    }
    fn xz(self) -> V3 {
        V3::new(self.x, 0.0, self.z)
    }
    /// `A3DVECTOR3::Normalize`: normaliza e devolve o comprimento (zero fica zero).
    fn normalizar(&mut self) -> f32 {
        let m = self.sqr().sqrt();
        if m > 0.0 {
            *self = self.mul(1.0 / m);
        }
        m
    }
    fn e_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0
    }
}

/// `RAND(x)` = `x * rand() / RAND_MAX` (`NPCMove.h:50`).
fn rand_ate(x: f32) -> f32 {
    rand::thread_rng().gen::<f32>() * x
}

/// O mapa que os agentes consultam: terreno (`GetTerrainHeight`) e movimento.
pub struct Mapa<'a> {
    pub terreno: &'a dyn Fn(f32, f32) -> Option<f32>,
    pub movimento: &'a MapaDeMovimento,
}

impl Mapa<'_> {
    fn pixel(&self, p: V3) -> Pixel {
        self.movimento.pixel_de(p.x, p.z)
    }
    fn alcancavel(&self, p: Pixel) -> bool {
        self.movimento.alcancavel(p.0, p.1)
    }
    fn centro(&self, p: Pixel) -> V3 {
        let (x, z) = self.movimento.centro_do_pixel(p.0, p.1);
        V3::new(x, 0.0, z)
    }
    fn reta(&self, de: Pixel, ate: Pixel) -> (bool, Pixel) {
        self.movimento.reta_livre(de, ate)
    }
    fn tamanho(&self) -> f32 {
        self.movimento.tamanho_do_pixel()
    }
    /// `AdjustCurPos` dos agentes de chão (`NPCChaseOnGroundNoBlockAgent.h:31-45`,
    /// `NPCChaseOnGroundAgent.h:41-55`). Sem terreno sob o ponto, a altura fica.
    fn assentar(&self, p: &mut V3) {
        let px = self.pixel(*p);
        let acima = self.movimento.acima_no_pixel(px.0, px.1);
        if acima == 0.0 {
            if let Some(h) = (self.terreno)(p.x, p.z) {
                p.y = h;
            }
        } else {
            let c = self.centro(px);
            if let Some(h) = (self.terreno)(c.x, c.z) {
                p.y = h + acima;
            }
        }
    }
}

// =============================================================================
// CPf2DBfs + CPathFollowing
// =============================================================================

#[derive(Debug, Clone, Copy)]
struct NoBfs {
    x: i16,
    z: i16,
    ant_x: i16,
    ant_z: i16,
    custo: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EstadoBusca {
    Buscando,
    Achou,
    SemCaminho,
}

/// `CPf2DBfs` (`Pf2DBfs.cpp`).
#[derive(Debug, Clone)]
struct Bfs {
    aberta: Vec<NoBfs>,
    fechada: HashMap<(i16, i16), (i16, i16)>,
    atual: NoBfs,
    meta: Pixel,
    alcance: f32,
    estado: EstadoBusca,
}

impl Bfs {
    fn manhattan(x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
        ((x1 - x2).abs() + (z1 - z2).abs()) as f32
    }

    /// `Init` + `CPathFinding2D::Init` (`m_fGoalRange = fRange + 0.001f`).
    fn new(inicio: Pixel, meta: Pixel, alcance: f32) -> Self {
        let atual = NoBfs {
            x: inicio.0 as i16,
            z: inicio.1 as i16,
            ant_x: NO_INVALIDO,
            ant_z: NO_INVALIDO,
            custo: Self::manhattan(inicio.0, inicio.1, meta.0, meta.1),
        };
        Self { aberta: vec![atual], fechada: HashMap::new(), atual, meta, alcance: alcance + 0.001, estado: EstadoBusca::Buscando }
    }

    /// `StepSearch(nSteps)`.
    fn buscar(&mut self, passos: i32, mapa: &Mapa) {
        if self.estado != EstadoBusca::Buscando {
            return;
        }
        // `NeighborD`: esquerda, direita, cima, baixo e as quatro diagonais.
        const VIZINHOS: [(i16, i16); 8] = [(-1, 0), (1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];
        let mut contador = 0;
        while self.estado == EstadoBusca::Buscando && !self.aberta.is_empty() && contador < passos {
            // `PopMinCost`: o primeiro de menor custo; o último da lista ocupa o lugar dele.
            let mut i_min = 0;
            for i in 1..self.aberta.len() {
                if self.aberta[i].custo < self.aberta[i_min].custo {
                    i_min = i;
                }
            }
            self.atual = self.aberta.swap_remove(i_min);
            let (dx, dz) = (self.atual.x as i32 - self.meta.0, self.atual.z as i32 - self.meta.1);
            if (((dx * dx + dz * dz) as f32).sqrt()) < self.alcance {
                self.estado = EstadoBusca::Achou;
                self.fechada.insert((self.atual.x, self.atual.z), (self.atual.ant_x, self.atual.ant_z));
                break;
            }
            contador += 1;
            for (vx, vz) in VIZINHOS {
                let (x, z) = (self.atual.x + vx, self.atual.z + vz);
                if !mapa.alcancavel((x as i32, z as i32))
                    || self.fechada.contains_key(&(x, z))
                    || self.aberta.iter().any(|n| n.x == x && n.z == z)
                {
                    continue;
                }
                self.aberta.push(NoBfs {
                    x,
                    z,
                    ant_x: self.atual.x,
                    ant_z: self.atual.z,
                    custo: Self::manhattan(x as i32, z as i32, self.meta.0, self.meta.1),
                });
            }
            self.fechada.insert((self.atual.x, self.atual.z), (self.atual.ant_x, self.atual.ant_z));
        }
        if self.aberta.is_empty() && self.estado != EstadoBusca::Achou {
            self.estado = EstadoBusca::SemCaminho;
        }
    }

    /// `GeneratePath`: do início até o nó **atual** (o melhor visto), mesmo com a busca em
    /// andamento.
    fn caminho(&self) -> Vec<Pixel> {
        let mut volta = Vec::new();
        let (mut x, mut z) = (self.atual.x, self.atual.z);
        while x != NO_INVALIDO && z != NO_INVALIDO {
            volta.push((x as i32, z as i32));
            match self.fechada.get(&(x, z)) {
                Some(&(ax, az)) => (x, z) = (ax, az),
                None => break, // o `ASSERT(0)` do original
            }
            if volta.len() > 100_000 {
                break;
            }
        }
        volta.reverse();
        volta
    }
}

/// `CPathFollowing` (`PathFollowing.{h,cpp}`).
#[derive(Debug, Clone, Default)]
struct Trajeto {
    inicio: V3,
    fim: V3,
    nos: i32,
    segmentos: Vec<(V3, f32)>,
    atual: V3,
    seg: usize,
    andado: f32,
}

impl Trajeto {
    fn limpar(&mut self) {
        self.segmentos.clear();
        self.nos = 0;
    }
    fn adicionar(&mut self, p: V3) {
        if self.nos == 0 {
            self.inicio = p;
        } else {
            let mut dir = p.sub(self.fim);
            if dir.e_zero() {
                return;
            }
            let comprimento = dir.normalizar();
            self.segmentos.push((dir, comprimento));
        }
        self.fim = p;
        self.nos += 1;
    }
    fn comecar(&mut self) {
        self.seg = 0;
        self.atual = self.inicio;
        self.andado = 0.0;
    }
    fn fim_do_trajeto(&self) -> bool {
        self.seg == self.segmentos.len()
    }
    fn andar(&mut self, mut passo: f32) {
        if self.seg >= self.segmentos.len() {
            return;
        }
        let mut ate_o_no = self.segmentos[self.seg].1 - self.andado;
        while passo > ate_o_no {
            passo -= ate_o_no;
            self.atual = self.atual.add(self.segmentos[self.seg].0.mul(ate_o_no));
            self.andado = 0.0;
            self.seg += 1;
            if self.seg < self.segmentos.len() {
                ate_o_no = self.segmentos[self.seg].1;
            } else {
                return;
            }
        }
        self.andado += passo;
        self.atual = self.atual.add(self.segmentos[self.seg].0.mul(passo));
    }
}

// =============================================================================
// CNPCChaseOnGroundNoBlockAgent + CNPCDisperseChaseOnGroundAgent
// =============================================================================

/// `CChaseInfo`: a direção de dispersão que o monstro guarda entre sessões
/// (`NPCSessionUpdateChaseInfo`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InfoDePerseguicao {
    pub dispersou: bool,
    pub direcao: V3,
}

/// A perseguição que o 1.5.5 roda no chão.
#[derive(Debug, Clone, Default)]
pub struct Perseguicao {
    pos: V3,
    passo: f32,
    meta: V3,
    min: f32,
    min2: f32,
    direcao: V3,
    chegou: bool,
    bloqueado_alem: bool,
    achou: bool,
    bloqueado: bool,
    meta_alcancavel: bool,
    pf_pixels: i32,
    max_bloqueios: i32,
    p_inicio: Pixel,
    p_meta: Pixel,
    reto: bool,
    gerar: bool,
    caminho: Vec<Pixel>,
    busca: Option<Bfs>,
    trajeto: Option<Trajeto>,
    bloqueios: i32,
}

impl Perseguicao {
    fn init(&mut self, pos: V3, passo: f32) {
        self.pos = pos;
        self.passo = passo;
    }

    /// `SetMoveDir` do agente de chão: só no plano.
    fn apontar(&mut self, alvo: V3) {
        let mut d = alvo.sub(self.pos);
        d.y = 0.0;
        d.normalizar();
        self.direcao = d;
    }

    fn perto(&self, delta: V3) -> bool {
        delta.sqr() <= self.min2 + 2.0 * RELAX_ERROR * self.min + SQR_RELAX_ERROR
    }

    /// `CurPosGetToGoal` do NoBlock.
    fn chegou_agora(&self) -> bool {
        let mut d = self.meta.sub(self.pos);
        d.y = 0.0;
        self.perto(d) || (self.reto && d.dot(self.direcao) < 0.0)
    }

    /// `CNPCChaseAgent::SetGoal`.
    fn meta_base(&mut self, meta: V3, min: f32) {
        self.meta = meta;
        self.min = min;
        self.min2 = min * min;
        self.apontar(meta);
        self.chegou = self.chegou_agora();
        self.bloqueado_alem = false;
    }

    /// `CNPCDisperseChaseOnGroundAgent::SetGoal` (`NPCDisperseChaseOnGroundAgent.cpp`).
    fn meta_dispersa(&mut self, meta: V3, min: f32, info: Option<&mut InfoDePerseguicao>, mapa: &Mapa) {
        self.meta_base(meta, min);
        self.chegou = self.chegou_agora();
        if self.chegou {
            return;
        }
        let dispersa = match info {
            Some(info) => {
                if !info.dispersou {
                    let d = self.meta_disperso(meta, min, mapa);
                    let mut dir = d.sub(meta);
                    dir.normalizar();
                    info.direcao = dir;
                    info.dispersou = true;
                    d
                } else {
                    let mut d = meta.add(info.direcao.mul(min));
                    let mut para_meta = meta.sub(self.pos);
                    if para_meta.dot(info.direcao) > 0.0 {
                        // `CHalfSpace::SetNV(dir, meta)` + `Mirror`: reflete no plano que
                        // passa pela meta com normal na direção da perseguição.
                        para_meta.normalizar();
                        let dist = para_meta.dot(d) - para_meta.dot(meta);
                        d = d.sub(para_meta.mul(2.0 * dist));
                    }
                    if !mapa.alcancavel(mapa.pixel(d)) {
                        d = self.meta_disperso(meta, min, mapa);
                    }
                    d
                }
            }
            None => self.meta_disperso(meta, min, mapa),
        };
        let (p_atual, p_meta) = (mapa.pixel(self.pos), mapa.pixel(dispersa));
        if mapa.reta(p_atual, p_meta).0 {
            self.meta_base(dispersa, 0.0);
        }
        if !mapa.alcancavel(p_meta) {
            self.bloqueado_alem = true;
        }
    }

    /// `GetDispersedGoal`: um ponto a `min` da meta, num ângulo sorteado de ±60° em torno da
    /// direção de quem persegue, até 12 tentativas por um pixel alcançável.
    fn meta_disperso(&self, meta: V3, min: f32, mapa: &Mapa) -> V3 {
        if min < ZERO_DIST_ERROR {
            return meta;
        }
        let mut dir = self.pos.sub(meta);
        dir.y = 0.0;
        dir.normalizar();
        let (c, s) = (dir.x, dir.z);
        let mut tentativas = 0;
        loop {
            tentativas += 1;
            let r = rand_ate(ANGULO_DE_DISPERSAO) - ANGULO_DE_DISPERSAO * 0.5;
            let (cr, sr) = (r.cos(), r.sin());
            let d = V3::new(min * (c * cr - s * sr), 0.0, min * (s * cr + c * sr)).add(meta);
            if tentativas > MAX_GENERATE_GOAL_TIMES || mapa.alcancavel(mapa.pixel(d)) {
                return d;
            }
        }
    }

    /// `StartPathFinding` do NoBlock.
    fn comecar(&mut self, pixels: i32, mapa: &Mapa) {
        if self.chegou {
            return;
        }
        self.caminho.clear();
        self.busca = None;
        self.trajeto = None;
        self.p_inicio = mapa.pixel(self.pos);
        self.p_meta = mapa.pixel(self.meta);
        let (reto, parada) = mapa.reta(self.p_inicio, self.p_meta);
        self.reto = reto;
        let v_parada = mapa.centro(parada);
        // `PosGetToGoal(vStopPos)` do NoBlock mede a posição **atual**, não a de parada
        // (`NPCChaseOnGroundNoBlockAgent.h:52-57`) — assim no original.
        let mut d = self.meta.sub(self.pos);
        d.y = 0.0;
        if self.reto || self.perto(d) {
            self.bloqueado = false;
            self.achou = true;
            return;
        }
        let _ = v_parada;
        self.pf_pixels = pixels;
        self.gerar = parada == self.p_inicio;
        self.busca = Some(Bfs::new(parada, self.p_meta, self.min));
        let mut t = Trajeto::default();
        t.adicionar(self.pos.xz());
        t.adicionar(mapa.centro(parada));
        t.comecar();
        self.trajeto = Some(t);
        self.achou = false;
        self.bloqueado = false;
        self.meta_alcancavel = mapa.movimento.vizinhos_alcancaveis(self.p_meta, 1);
        self.max_bloqueios = MAX_BLOCK_TIMES;
        self.bloqueios = 0;
    }

    /// `CNPCChaseAgent::MoveOneStep` — a reta.
    fn passo_reto(&mut self, mapa: &Mapa) {
        if self.chegou || self.bloqueado_alem {
            return;
        }
        self.pos = self.pos.add(self.direcao.mul(self.passo));
        mapa.assentar(&mut self.pos);
        self.chegou = self.chegou_agora();
        if self.chegou {
            // `AdjustGetToGoalPos`.
            let antes = self.pos;
            self.pos = self.meta.sub(self.direcao.mul(self.min));
            mapa.assentar(&mut self.pos);
            if !self.chegou_agora() {
                self.pos = antes;
            }
        } else {
            self.apontar(self.meta);
        }
    }

    /// `MoveOneStep` do NoBlock.
    fn andar(&mut self, mapa: &Mapa) {
        if self.chegou {
            return;
        }
        if self.reto {
            self.passo_reto(mapa);
            return;
        }
        if (self.pf_pixels as f32) < self.passo {
            self.pf_pixels = self.passo as i32 + 1;
        }
        let Some(busca) = self.busca.as_mut() else { return };
        match busca.estado {
            EstadoBusca::Buscando => busca.buscar(self.pf_pixels, mapa),
            EstadoBusca::Achou => self.achou = true,
            EstadoBusca::SemCaminho => {}
        }
        let mut caminho_final = false;
        if self.gerar {
            let anterior = std::mem::take(&mut self.caminho);
            self.caminho = self.busca.as_ref().map(|b| b.caminho()).unwrap_or_default();
            self.atualizar_trajeto(&anterior, mapa);
            if self.achou {
                caminho_final = true;
            }
        }
        let Some(t) = self.trajeto.as_mut() else { return };
        t.andar(self.passo);
        if t.fim_do_trajeto() {
            self.bloqueios += 1;
            self.bloqueado = true;
        }
        if self.bloqueios > self.max_bloqueios || (self.bloqueado && self.achou) {
            self.gerar = true;
            self.bloqueado = false;
            self.bloqueios = 0;
        } else {
            self.gerar = false;
        }
        self.pos = t.atual;
        self.chegou = caminho_final && t.fim_do_trajeto();
        if self.chegou {
            // `AdjustCurPosGetToGoal` do NoBlock.
            let mut d = self.pos.sub(self.meta);
            d.y = 0.0;
            d.normalizar();
            self.pos = self.meta.add(d.mul(self.min));
        }
        mapa.assentar(&mut self.pos);
    }

    /// `UpdateFollowPath`.
    fn atualizar_trajeto(&mut self, anterior: &[Pixel], mapa: &Mapa) {
        let Some(t) = self.trajeto.as_mut() else { return };
        if self.caminho.is_empty() {
            return;
        }
        t.limpar();
        if anterior.is_empty() {
            for p in &self.caminho {
                t.adicionar(mapa.centro(*p));
            }
        } else {
            let (de, ate) = (anterior[anterior.len() - 1], self.caminho[self.caminho.len() - 1]);
            if mapa.reta(de, ate).0 {
                t.adicionar(mapa.centro(de));
                t.adicionar(mapa.centro(ate));
            } else {
                let mut iguais = 0usize;
                for i in 1..anterior.len() {
                    if self.caminho.get(i) == Some(&anterior[i]) {
                        iguais = i;
                    } else {
                        break;
                    }
                }
                for i in (iguais + 1..anterior.len()).rev() {
                    t.adicionar(mapa.centro(anterior[i]));
                }
                for p in &self.caminho[iguais..] {
                    t.adicionar(mapa.centro(*p));
                }
            }
        }
        t.comecar();
    }
}

/// `path_finding::follow_target` (`pathfinding.h:34-134`): a perseguição com os três níveis
/// de detalhe pela distância inicial.
#[derive(Debug, Clone, Default)]
pub struct SeguirAlvo {
    agente: Perseguicao,
    alvo: V3,
    contador: i32,
    detalhe: usize,
    nivel_de_passo: bool,
}

impl SeguirAlvo {
    /// `Start(source, target, speed, range, cur_distance, pInfo)`. `distancia_ao_quadrado`
    /// é o `range` do chamador — o original passa o **quadrado** da distância
    /// (`squared_distance`, `npcsession.cpp:179`) e compara com 100 e 400.
    pub fn comecar(&mut self, de: V3, alvo: V3, passo: f32, alcance: f32, distancia_ao_quadrado: f32, info: Option<&mut InfoDePerseguicao>, mapa: &Mapa) {
        self.detalhe = if distancia_ao_quadrado <= 100.0 { 0 } else if distancia_ao_quadrado <= 400.0 { 1 } else { 2 };
        self.contador = 0;
        self.nivel_de_passo = false;
        self.alvo = alvo;
        self.agente.init(de, passo);
        self.agente.meta_dispersa(alvo, alcance, info, mapa);
        self.agente.comecar(DETALHE[self.detalhe][1], mapa);
    }

    pub fn chegou(&self) -> bool {
        self.agente.chegou
    }

    pub fn bloqueado(&self) -> bool {
        self.agente.bloqueado
    }

    pub fn alvo(&self) -> V3 {
        self.alvo
    }

    pub fn posicao(&self) -> V3 {
        self.agente.pos
    }

    /// `MoveOneStep(speed)`: `false` quando desistiu — preso além do ambiente, ou o teto de
    /// pixels somados passou sem caminho achado.
    pub fn andar(&mut self, passo: f32, mapa: &Mapa) -> bool {
        self.agente.passo = passo;
        self.agente.andar(mapa);
        self.contador += self.agente.pf_pixels;
        if self.agente.bloqueado {
            if !self.nivel_de_passo {
                self.nivel_de_passo = true;
                self.agente.pf_pixels = DETALHE[self.detalhe][2];
            }
        } else {
            self.nivel_de_passo = false;
            self.agente.pf_pixels = DETALHE[self.detalhe][1];
        }
        if self.agente.bloqueado_alem {
            return false;
        }
        self.agente.achou || self.contador < DETALHE[self.detalhe][0]
    }
}

// =============================================================================
// CNPCChaseOnGroundAgent (o do passeio)
// =============================================================================

#[derive(Debug, Clone, Copy)]
struct NoGrade {
    u: i32,
    v: i32,
    h: i32,
    anterior: Option<usize>,
}

/// `CNPCChaseOnGroundAgent` (`NPCChaseOnGroundAgent.{h,cpp}`), com a lista aberta de até 30
/// nós ordenada pela distância de Manhattan.
#[derive(Debug, Clone, Default)]
struct BuscaNaGrade {
    pos: V3,
    passo: f32,
    meta: V3,
    min: f32,
    min2: f32,
    direcao: V3,
    chegou: bool,
    achou: bool,
    bloqueado: bool,
    meta_alcancavel: bool,
    pf_pixels: i32,
    reto: bool,
    nos: Vec<NoGrade>,
    aberta: Vec<usize>,
    fechada: Vec<usize>,
    achado: Vec<Pixel>,
    previsto: Vec<Pixel>,
    p_inicio: Pixel,
    p_meta: Pixel,
    du: i32,
    dv: i32,
    pixels_por_passo: i32,
    no_achado: i32,
    passo_pequeno: bool,
    passos_por_pixel: i32,
    passo_atual: i32,
    passinho: V3,
}

impl BuscaNaGrade {
    fn init(&mut self, pos: V3, passo: f32, mapa: &Mapa) {
        self.pos = pos;
        self.definir_passo(passo, mapa);
    }

    /// `SetMoveStep`.
    fn definir_passo(&mut self, passo: f32, mapa: &Mapa) {
        self.passo = passo;
        self.pixels_por_passo = (passo / mapa.tamanho() + 0.5) as i32;
        if self.pixels_por_passo == 0 {
            self.passo_pequeno = true;
            self.passos_por_pixel = (mapa.tamanho() / passo + 0.5) as i32;
            self.passo_atual = 0;
            self.pixels_por_passo = 1;
        } else {
            self.passo_pequeno = false;
        }
    }

    fn apontar(&mut self, alvo: V3) {
        let mut d = alvo.sub(self.pos);
        d.y = 0.0;
        d.normalizar();
        self.direcao = d;
    }

    fn chegou_agora(&self) -> bool {
        let mut d = self.meta.sub(self.pos);
        d.y = 0.0;
        d.sqr() <= self.min2 + 2.0 * RELAX_ERROR * self.min + SQR_RELAX_ERROR || (self.reto && d.dot(self.direcao) < 0.0)
    }

    fn definir_meta(&mut self, meta: V3, min: f32) {
        self.meta = meta;
        self.min = min;
        self.min2 = min * min;
        self.apontar(meta);
        self.chegou = self.chegou_agora();
    }

    fn pixel_chegou(&self, (u, v): Pixel) -> bool {
        let (du, dv) = (u - self.p_meta.0, v - self.p_meta.1);
        ((du * du + dv * dv) as f32) <= self.min2
    }

    fn manhattan(&self, u: i32, v: i32) -> i32 {
        (u - self.p_meta.0).abs() + (v - self.p_meta.1).abs()
    }

    /// `StartPathFinding`.
    fn comecar(&mut self, pixels: i32, mapa: &Mapa) {
        if self.chegou {
            return;
        }
        self.nos.clear();
        self.aberta.clear();
        self.fechada.clear();
        self.achado.clear();
        self.previsto.clear();
        self.p_inicio = mapa.pixel(self.pos);
        self.p_meta = mapa.pixel(self.meta);
        if self.pixel_chegou(self.p_inicio) {
            self.chegou = true;
            return;
        }
        if mapa.reta(self.p_inicio, self.p_meta).0 {
            self.reto = true;
            self.bloqueado = false;
            self.achou = true;
            return;
        }
        self.reto = false;
        self.du = (self.p_meta.0 - self.p_inicio.0).signum();
        self.dv = (self.p_meta.1 - self.p_inicio.1).signum();
        let h = self.manhattan(self.p_inicio.0, self.p_inicio.1);
        self.nos.push(NoGrade { u: self.p_inicio.0, v: self.p_inicio.1, h, anterior: None });
        self.inserir_ordenado(0);
        self.achou = false;
        self.chegou = false;
        self.bloqueado = false;
        self.meta_alcancavel = mapa.movimento.vizinhos_alcancaveis(self.p_meta, 1);
        self.pf_pixels = pixels;
        self.no_achado = -1;
    }

    /// `CSortVectorPathNode::SortPush`: entra antes do primeiro de `h` maior; passou de 30,
    /// o último sai.
    fn inserir_ordenado(&mut self, i: usize) -> bool {
        let h = self.nos[i].h;
        if let Some(k) = self.aberta.iter().position(|&j| h < self.nos[j].h) {
            self.aberta.insert(k, i);
            if self.aberta.len() > MAX_NODES {
                self.aberta.pop();
            }
            return true;
        }
        if self.aberta.len() < MAX_NODES {
            self.aberta.push(i);
            return true;
        }
        false
    }

    /// `InsertPathNode`.
    fn inserir(&mut self, u: i32, v: i32, anterior: usize, mapa: &Mapa) {
        if !mapa.alcancavel((u, v)) {
            return;
        }
        let ja = |l: &Vec<usize>, nos: &Vec<NoGrade>| l.iter().any(|&j| nos[j].u == u && nos[j].v == v);
        if ja(&self.aberta, &self.nos) || ja(&self.fechada, &self.nos) {
            return;
        }
        let h = self.manhattan(u, v);
        self.nos.push(NoGrade { u, v, h, anterior: Some(anterior) });
        let i = self.nos.len() - 1;
        if !self.inserir_ordenado(i) {
            self.nos.pop();
        }
    }

    /// `BestFirstSearchPath`.
    fn buscar(&mut self, mapa: &Mapa) {
        if self.achou {
            return;
        }
        let mut buscados = 0;
        let mut ultimo = None;
        while !self.aberta.is_empty() && buscados < self.pf_pixels {
            let i = self.aberta.remove(0);
            ultimo = Some(i);
            let (u, v) = (self.nos[i].u, self.nos[i].v);
            if self.pixel_chegou((u, v)) {
                self.achou = true;
                break;
            }
            buscados += 1;
            for (du, dv) in [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)] {
                self.inserir(u + du, v + dv, i, mapa);
            }
            self.fechada.push(i);
        }
        if self.achou {
            if let Some(mut i) = ultimo {
                // `GeneratePath`: da meta de volta ao início.
                loop {
                    self.achado.push((self.nos[i].u, self.nos[i].v));
                    match self.nos[i].anterior {
                        Some(a) => i = a,
                        None => break,
                    }
                }
            }
        }
    }

    /// `PredictPath`: sem caminho, anda em diagonal para a meta até bater.
    fn prever(&mut self, mapa: &Mapa) {
        if self.bloqueado {
            return;
        }
        let mut ultimo = *self.previsto.last().unwrap_or(&self.p_inicio);
        for _ in 0..self.pixels_por_passo {
            let atual = (
                if ultimo.0 == self.p_meta.0 { ultimo.0 } else { ultimo.0 + self.du },
                if ultimo.1 == self.p_meta.1 { ultimo.1 } else { ultimo.1 + self.dv },
            );
            if !mapa.alcancavel(atual) {
                self.bloqueado = true;
                return;
            }
            self.pos.x += (atual.0 - ultimo.0) as f32 * mapa.tamanho();
            self.pos.z += (atual.1 - ultimo.1) as f32 * mapa.tamanho();
            if self.pixel_chegou(atual) {
                self.chegou = true;
                self.ajustar_chegada(atual, ultimo, mapa);
                return;
            }
            self.previsto.push(atual);
            ultimo = atual;
        }
    }

    /// `FollowFoundPath`.
    fn seguir_achado(&mut self, mapa: &Mapa) {
        let mut movidos = 0;
        while self.no_achado != 0 && movidos < self.pixels_por_passo {
            let (atual, ultimo);
            if self.no_achado == -1 {
                ultimo = *self.previsto.last().unwrap_or(&self.p_inicio);
                if let Some(i) = self.achado.iter().position(|p| *p == ultimo) {
                    self.no_achado = if i == 0 { 0 } else { i as i32 - 1 };
                }
                if self.no_achado == -1 {
                    self.previsto.pop();
                    atual = *self.previsto.last().unwrap_or(&self.p_inicio);
                } else {
                    atual = self.achado[self.no_achado as usize];
                }
            } else {
                ultimo = self.achado[self.no_achado as usize];
                self.no_achado -= 1;
                atual = self.achado[self.no_achado as usize];
            }
            movidos += 1;
            self.pos.x += (atual.0 - ultimo.0) as f32 * mapa.tamanho();
            self.pos.z += (atual.1 - ultimo.1) as f32 * mapa.tamanho();
        }
        if self.no_achado == 0 && self.achado.len() > 1 {
            self.chegou = true;
            let (a, b) = (self.achado[0], self.achado[1]);
            self.ajustar_chegada(a, b, mapa);
        }
    }

    /// `AdjustCurPosGetToGoal(cur, last)`: para exatamente a `min` da meta, entre os dois
    /// centros de pixel.
    fn ajustar_chegada(&mut self, atual: Pixel, ultimo: Pixel, mapa: &Mapa) -> bool {
        if self.min < ZERO_DIST_ERROR && atual == self.p_meta {
            self.pos = self.meta;
            return true;
        }
        let (c, l) = (mapa.centro(atual), mapa.centro(ultimo));
        let mut d = self.meta.sub(c);
        d.y = 0.0;
        if d.sqr() >= self.min2 {
            self.pos = c;
            return true;
        }
        let mut d = self.meta.sub(l);
        d.y = 0.0;
        if d.sqr() <= self.min2 {
            self.pos = l;
            return true;
        }
        let (dx, dz) = (l.x - c.x, l.z - c.z);
        let (cx, cz) = (c.x - self.meta.x, c.z - self.meta.z);
        let a = dx * dx + dz * dz;
        let b = 2.0 * (dx * cx + dz * cz);
        let cc = cx * cx + cz * cz - self.min2;
        let det = b * b - 4.0 * a * cc;
        if det < 0.0 {
            return false;
        }
        let mut r = (-b - det.sqrt()) / (2.0 * a);
        if !(r > 0.0 && r < 1.0) {
            r = (-b / a) - r;
            if !(r > 0.0 && r < 1.0) {
                return false;
            }
        }
        self.pos = V3::new(c.x + dx * r, 0.0, c.z + dz * r);
        true
    }

    /// `CNPCChaseAgent::MoveOneStep` com o `AdjustCurPos` de chão.
    fn passo_reto(&mut self, mapa: &Mapa) {
        if self.chegou {
            return;
        }
        self.pos = self.pos.add(self.direcao.mul(self.passo));
        mapa.assentar(&mut self.pos);
        self.chegou = self.chegou_agora();
        if self.chegou {
            let antes = self.pos;
            self.pos = self.meta.sub(self.direcao.mul(self.min));
            mapa.assentar(&mut self.pos);
            if !self.chegou_agora() {
                self.pos = antes;
            }
        } else {
            self.apontar(self.meta);
        }
    }

    fn buscar_ou_prever(&mut self, mapa: &Mapa) {
        if self.meta_alcancavel {
            self.buscar(mapa);
            if self.achou {
                self.seguir_achado(mapa);
                self.bloqueado = false;
            } else {
                self.prever(mapa);
            }
        } else {
            self.prever(mapa);
        }
    }

    /// `MoveOneStep`.
    fn andar(&mut self, mapa: &Mapa) {
        if self.chegou {
            return;
        }
        if self.reto {
            self.passo_reto(mapa);
            return;
        }
        if self.passo_pequeno {
            if self.passo_atual == 0 {
                let antes = self.pos;
                self.buscar_ou_prever(mapa);
                if self.chegou {
                    return;
                }
                let mut p = self.pos.sub(antes);
                p.y = 0.0;
                self.pos = antes;
                self.passinho = p.mul(1.0 / self.passos_por_pixel.max(1) as f32);
            }
            self.pos = self.pos.add(self.passinho);
            self.passo_atual += 1;
            if self.passo_atual == self.passos_por_pixel {
                self.passo_atual = 0;
            }
        } else {
            self.buscar_ou_prever(mapa);
        }
        mapa.assentar(&mut self.pos);
    }
}

/// `path_finding::cruise` + `CNPCRambleOnGroundAgent` (`NPCRambleOnGroundAgent.cpp`,
/// `NPCMove.h:499-624`).
#[derive(Debug, Clone, Default)]
pub struct Passeio {
    centro: V3,
    raio: f32,
    meta: V3,
    agente: BuscaNaGrade,
}

impl Passeio {
    /// `Start(source, center, speed, range)`: `Init` + `SetRambleRange` + `StartRamble`.
    pub fn comecar(&mut self, de: V3, centro: V3, passo: f32, raio: f32, mapa: &Mapa) {
        self.agente = BuscaNaGrade::default();
        self.agente.init(de, passo, mapa);
        self.centro = centro;
        self.raio = raio;
        self.gerar_meta(mapa);
        self.agente.definir_meta(self.meta, 0.0);
        // `StartRamble` chama `StartPathFinding()` (10 pixels) e depois `SetPFPixels(200)`.
        self.agente.comecar(10, mapa);
        self.agente.pf_pixels = PIXELS_DO_PASSEIO;
    }

    /// `GeneratePosInMoveRange` de chão: um ponto no disco, na altura do centro.
    fn ponto_no_disco(&self) -> V3 {
        let r = rand_ate(self.raio);
        let t = rand_ate(2.0 * std::f32::consts::PI);
        self.centro.add(V3::new(r * t.cos(), 0.0, r * t.sin()))
    }

    /// `GenerateTmpGoal` de chão: primeiro um ponto alcançável **em linha reta**; depois um
    /// só alcançável; senão, fica onde está.
    fn gerar_meta(&mut self, mapa: &Mapa) {
        let atual = mapa.pixel(self.agente.pos);
        for _ in 0..=MAX_GENERATE_GOAL_TIMES {
            self.meta = self.ponto_no_disco();
            let p = mapa.pixel(self.meta);
            if mapa.alcancavel(p) && mapa.reta(atual, p).0 {
                return;
            }
        }
        for _ in 0..=MAX_GENERATE_GOAL_TIMES {
            self.meta = self.ponto_no_disco();
            if mapa.alcancavel(mapa.pixel(self.meta)) {
                return;
            }
        }
        self.meta = self.agente.pos;
    }

    pub fn parou(&self) -> bool {
        self.agente.chegou
    }

    pub fn posicao(&self) -> V3 {
        self.agente.pos
    }

    /// `MoveOneStep(speed)`.
    pub fn andar(&mut self, passo: f32, mapa: &Mapa) {
        if self.parou() {
            return;
        }
        self.agente.definir_passo(passo, mapa);
        self.agente.andar(mapa);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Um mapa de 64×64 pixels com uma parede em `u = 32` de `v = 10` a `v = 54`, feito à mão
    /// no formato do `movemap` e lido pelo leitor de verdade.
    fn mapa_com_parede() -> MapaDeMovimento {
        let dir = std::env::temp_dir().join(format!("pw_navegacao_{}", std::process::id()));
        let pasta = dir.join("movemap");
        std::fs::create_dir_all(&pasta).unwrap();
        std::fs::write(pasta.join("movemap.conf"), "Map Width = 1\nMap Length = 1\nSubmap Width = 64\nSubmap Length = 64\nPixel Size = 1.0\n").unwrap();
        // `.rmap`: 8 bytes por linha, 64 linhas.
        let mut bits = vec![0xFFu8; 8 * 64];
        for v in 10..=54 {
            bits[v * 8 + 4] &= !1; // u = 32
        }
        let mut corpo = Vec::new();
        for x in [8i32, 64, 64, 64] {
            corpo.extend_from_slice(&x.to_le_bytes());
        }
        corpo.extend_from_slice(&1.0f32.to_le_bytes());
        corpo.extend_from_slice(&bits);
        let mut r = 1u32.to_le_bytes().to_vec();
        r.extend_from_slice(&(corpo.len() as u32).to_le_bytes());
        r.extend_from_slice(&corpo);
        std::fs::write(pasta.join("1.rmap"), r).unwrap();
        // `.dhmap`: 1×1 bloco nulo de 64 (expoente 6), altura zero.
        let mut corpo = Vec::new();
        for x in [1i32, 1, 6, 64, 64] {
            corpo.extend_from_slice(&x.to_le_bytes());
        }
        corpo.extend_from_slice(&1.0f32.to_le_bytes());
        corpo.extend_from_slice(&(-1i32).to_le_bytes());
        corpo.extend_from_slice(&0i32.to_le_bytes());
        let mut d = 1u32.to_le_bytes().to_vec();
        d.extend_from_slice(&(corpo.len() as u32).to_le_bytes());
        d.extend_from_slice(&corpo);
        std::fs::write(pasta.join("1.dhmap"), d).unwrap();
        let m = MapaDeMovimento::ler(1, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        m
    }

    fn plano(_: f32, _: f32) -> Option<f32> {
        Some(0.0)
    }

    #[test]
    fn a_perseguicao_contorna_a_parede_em_vez_de_atravessar() {
        let mov = mapa_com_parede();
        assert!(mov.tem_dados() && !mov.alcancavel(32, 30) && mov.alcancavel(32, 60));
        let mapa = Mapa { terreno: &plano, movimento: &mov };
        // Origem no centro: pixel (u, v) = (x + 32, z + 32). De (20, 30) a (44, 30).
        let (de, ate) = (V3::new(20.5 - 32.0, 0.0, 30.5 - 32.0), V3::new(44.5 - 32.0, 0.0, 30.5 - 32.0));
        let mut s = SeguirAlvo::default();
        s.comecar(de, ate, 2.0, 1.0, de.sub(ate).sqr(), None, &mapa);
        let mut atravessou = false;
        for _ in 0..200 {
            if s.chegou() {
                break;
            }
            if !s.andar(2.0, &mapa) {
                s.comecar(s.posicao(), ate, 2.0, 1.0, s.posicao().sub(ate).sqr(), None, &mapa);
            }
            let (u, v) = mov.pixel_de(s.posicao().x, s.posicao().z);
            atravessou |= !mov.alcancavel(u, v);
        }
        assert!(!atravessou, "passou por dentro da parede");
        let fim = s.posicao();
        assert!(fim.sub(ate).xz().sqr() <= 2.5 * 2.5, "não chegou perto do alvo: {fim:?}");
    }

    #[test]
    fn sem_obstaculo_a_perseguicao_e_uma_reta() {
        let vazio = MapaDeMovimento::vazio();
        let mapa = Mapa { terreno: &plano, movimento: &vazio };
        let mut s = SeguirAlvo::default();
        let (de, ate) = (V3::new(0.0, 0.0, 0.0), V3::new(10.0, 0.0, 0.0));
        s.comecar(de, ate, 2.0, 1.0, 100.0, None, &mapa);
        assert!(s.andar(2.0, &mapa));
        // A meta é dispersa: um ponto a 1 m do alvo, até ±60° em torno da direção de quem vem
        // — o passo de 2 m sai quase reto, mas não exatamente.
        let p = s.posicao();
        assert!((p.sub(de).xz().sqr().sqrt() - 2.0).abs() < 1e-3, "{p:?}");
        assert!(p.x > 1.9 && p.z.abs() < 0.2, "{p:?}");
    }

    #[test]
    fn o_passeio_nao_escolhe_meta_dentro_da_parede() {
        let mov = mapa_com_parede();
        let mapa = Mapa { terreno: &plano, movimento: &mov };
        for _ in 0..50 {
            let mut p = Passeio::default();
            let centro = V3::new(0.5, 0.0, 0.0); // o pixel da parede é (32, 32)
            p.comecar(centro, centro, 1.0, 10.0, &mapa);
            let (u, v) = mov.pixel_de(p.meta.x, p.meta.z);
            assert!(mov.alcancavel(u, v), "meta ({u},{v}) na parede");
            for _ in 0..30 {
                p.andar(1.0, &mapa);
                let (u, v) = mov.pixel_de(p.posicao().x, p.posicao().z);
                assert!(mov.alcancavel(u, v) || p.parou(), "passeou por dentro da parede em ({u},{v})");
            }
        }
    }
}
