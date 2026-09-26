//! Estruturas e layouts oficiais de mascotes e ovos de mascote do Perfect World 1.5.5.
//!
//! Layouts verificados contra os fontes C++ oficiais:
//! - `item_petegg.h:12-35` (`struct pe_essence`, tamanho base 60 bytes)
//! - `EC_IvtrTypes.h:289` (`struct IVTR_ESSENCE_PETEGG`, tamanho base 60 bytes)
//! - `petman.h:22-65` (`struct pet_data`, 192 bytes)
//! - `EC_GPDataType.h:749-780` (`struct info_pet`, 192 bytes)

pub const TAMANHO_PE_ESSENCE_BASE: usize = 60;
pub const TAMANHO_INFO_PET: usize = 192;

pub const PET_CLASS_MOUNT: i32 = 0;
pub const PET_CLASS_COMBAT: i32 = 1;
pub const PET_CLASS_FOLLOW: i32 = 2;
pub const PET_CLASS_SUMMON: i32 = 3;
pub const PET_CLASS_PLANT: i32 = 4;
pub const PET_CLASS_EVOLUTION: i32 = 5;

pub const HUNGER_LEVEL_1: i32 = 1;

/// Essência do ovo de pet guardada em `ItemRecord.octets` (`pe_essence` no C++).
#[derive(Debug, Clone, PartialEq)]
pub struct PeEssence {
    pub require_level: i32,
    pub require_class: i32,
    pub honor_point: i32,
    pub pet_tid: i32,
    pub pet_vis_tid: i32,
    pub pet_egg_tid: i32,
    pub pet_class: i32,
    pub level: i16,
    pub color: u16,
    pub exp: i32,
    pub skill_point: i32,
    pub name_len: u16,
    pub skill_count: u16,
    pub name: [u8; 16],
    pub skills: Vec<(i32, i32)>, // (id_skill, level)
}

impl Default for PeEssence {
    fn default() -> Self {
        Self {
            require_level: 1,
            require_class: 0xFFFF, // Por padrão montarias permitem todas as classes
            honor_point: 0,
            pet_tid: 0,
            pet_vis_tid: 0,
            pet_egg_tid: 0,
            pet_class: PET_CLASS_MOUNT,
            level: 1,
            color: 0,
            exp: 0,
            skill_point: 0,
            name_len: 0,
            skill_count: 0,
            name: [0u8; 16],
            skills: Vec::new(),
        }
    }
}

impl PeEssence {
    pub fn para_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(TAMANHO_PE_ESSENCE_BASE + self.skills.len() * 8);
        buf.extend_from_slice(&self.require_level.to_le_bytes());
        buf.extend_from_slice(&self.require_class.to_le_bytes());
        buf.extend_from_slice(&self.honor_point.to_le_bytes());
        buf.extend_from_slice(&self.pet_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_vis_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_egg_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_class.to_le_bytes());
        buf.extend_from_slice(&self.level.to_le_bytes());
        buf.extend_from_slice(&self.color.to_le_bytes());
        buf.extend_from_slice(&self.exp.to_le_bytes());
        buf.extend_from_slice(&self.skill_point.to_le_bytes());
        buf.extend_from_slice(&self.name_len.to_le_bytes());
        buf.extend_from_slice(&(self.skills.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.name);
        for &(skill, lvl) in &self.skills {
            buf.extend_from_slice(&skill.to_le_bytes());
            buf.extend_from_slice(&lvl.to_le_bytes());
        }
        buf
    }

    pub fn de_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < TAMANHO_PE_ESSENCE_BASE {
            return None;
        }
        let require_level = i32::from_le_bytes(bytes[0..4].try_into().ok()?);
        let require_class = i32::from_le_bytes(bytes[4..8].try_into().ok()?);
        let honor_point = i32::from_le_bytes(bytes[8..12].try_into().ok()?);
        let pet_tid = i32::from_le_bytes(bytes[12..16].try_into().ok()?);
        let pet_vis_tid = i32::from_le_bytes(bytes[16..20].try_into().ok()?);
        let pet_egg_tid = i32::from_le_bytes(bytes[20..24].try_into().ok()?);
        let pet_class = i32::from_le_bytes(bytes[24..28].try_into().ok()?);
        let level = i16::from_le_bytes(bytes[28..30].try_into().ok()?);
        let color = u16::from_le_bytes(bytes[30..32].try_into().ok()?);
        let exp = i32::from_le_bytes(bytes[32..36].try_into().ok()?);
        let skill_point = i32::from_le_bytes(bytes[36..40].try_into().ok()?);
        let name_len = u16::from_le_bytes(bytes[40..42].try_into().ok()?);
        let skill_count = u16::from_le_bytes(bytes[42..44].try_into().ok()?);
        let mut name = [0u8; 16];
        name.copy_from_slice(&bytes[44..60]);

        let mut skills = Vec::new();
        let mut off = 60;
        for _ in 0..skill_count {
            if off + 8 <= bytes.len() {
                let s = i32::from_le_bytes(bytes[off..off+4].try_into().ok()?);
                let l = i32::from_le_bytes(bytes[off+4..off+8].try_into().ok()?);
                skills.push((s, l));
                off += 8;
            } else {
                break;
            }
        }

        Some(Self {
            require_level,
            require_class,
            honor_point,
            pet_tid,
            pet_vis_tid,
            pet_egg_tid,
            pet_class,
            level,
            color,
            exp,
            skill_point,
            name_len,
            skill_count,
            name,
            skills,
        })
    }
}

/// Estrutura `info_pet` / `pet_data` serializada em 192 bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct InfoPet {
    pub honor_point: i32,
    pub hunger: i32,
    pub feed_time: i32,
    pub pet_tid: i32,
    pub pet_vis_tid: i32,
    pub pet_egg_tid: i32,
    pub pet_class: i32,
    pub hp_factor: f32,
    pub level: i16,
    pub color: u16,
    pub exp: i32,
    pub skill_point: i32,
    pub is_bind: u8,
    pub unused: u8,
    pub name_len: u16,
    pub name: [u8; 16],
    pub skills: [(i32, i32); 8],
    pub evo_prop: [i32; 6],
    pub reserved: [i32; 10],
}

impl Default for InfoPet {
    fn default() -> Self {
        Self {
            honor_point: 0,
            hunger: HUNGER_LEVEL_1,
            feed_time: 0,
            pet_tid: 0,
            pet_vis_tid: 0,
            pet_egg_tid: 0,
            pet_class: PET_CLASS_MOUNT,
            hp_factor: 1.0,
            level: 1,
            color: 0,
            exp: 0,
            skill_point: 0,
            is_bind: 0,
            unused: 0,
            name_len: 0,
            name: [0u8; 16],
            skills: [(0, 0); 8],
            evo_prop: [0i32; 6],
            reserved: [0i32; 10],
        }
    }
}

impl InfoPet {
    pub fn de_essencia(ess: &PeEssence) -> Self {
        let mut pet = Self::default();
        pet.honor_point = ess.honor_point;
        pet.hunger = HUNGER_LEVEL_1;
        pet.pet_tid = ess.pet_tid;
        pet.pet_vis_tid = ess.pet_vis_tid;
        pet.pet_egg_tid = ess.pet_egg_tid;
        pet.pet_class = ess.pet_class;
        pet.level = ess.level;
        pet.color = ess.color;
        pet.exp = ess.exp;
        pet.skill_point = ess.skill_point;
        pet.name_len = ess.name_len;
        pet.name = ess.name;
        for (i, &(s, l)) in ess.skills.iter().take(8).enumerate() {
            pet.skills[i] = (s, l);
        }
        pet
    }

    /// O caminho de volta do [`Self::para_bytes`]: lê o bloco de 192 bytes guardado no item
    /// do mascote, **inteiro**. Até o B112 lia só os 40 primeiros bytes, e como o mundo grava a
    /// jaula a cada mudança do mascote ativo (experiência, fome, recolher), nome e habilidades
    /// voltavam zerados na primeira gravação.
    pub fn do_bloco(b: &[u8]) -> Option<Self> {
        if b.len() < 40 {
            return None;
        }
        let i32_em = |i: usize| b.get(i..i + 4).map(|x| i32::from_le_bytes(x.try_into().unwrap())).unwrap_or(0);
        let mut info = Self::cabecalho_do_bloco(b, &i32_em)?;
        if b.len() >= TAMANHO_INFO_PET {
            info.skill_point = i32_em(40);
            info.is_bind = b[44];
            info.unused = b[45];
            info.name_len = u16::from_le_bytes([b[46], b[47]]).min(16);
            info.name.copy_from_slice(&b[48..64]);
            for (k, s) in info.skills.iter_mut().enumerate() {
                *s = (i32_em(64 + k * 8), i32_em(68 + k * 8));
            }
            for (k, e) in info.evo_prop.iter_mut().enumerate() {
                *e = i32_em(128 + k * 4);
            }
            for (k, r) in info.reserved.iter_mut().enumerate() {
                *r = i32_em(152 + k * 4);
            }
        }
        Some(info)
    }

    fn cabecalho_do_bloco(b: &[u8], i32_em: &dyn Fn(usize) -> i32) -> Option<Self> {
        Some(Self {
            honor_point: i32_em(0),
            hunger: i32_em(4),
            feed_time: i32_em(8),
            pet_tid: i32_em(12),
            pet_vis_tid: i32_em(16),
            pet_egg_tid: i32_em(20),
            pet_class: i32_em(24),
            hp_factor: f32::from_le_bytes(b[28..32].try_into().ok()?),
            level: i16::from_le_bytes(b[32..34].try_into().ok()?),
            color: u16::from_le_bytes(b[34..36].try_into().ok()?),
            exp: i32_em(36),
            ..Default::default()
        })
    }


    pub fn para_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(TAMANHO_INFO_PET);
        buf.extend_from_slice(&self.honor_point.to_le_bytes());
        buf.extend_from_slice(&self.hunger.to_le_bytes());
        buf.extend_from_slice(&self.feed_time.to_le_bytes());
        buf.extend_from_slice(&self.pet_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_vis_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_egg_tid.to_le_bytes());
        buf.extend_from_slice(&self.pet_class.to_le_bytes());
        buf.extend_from_slice(&self.hp_factor.to_le_bytes());
        buf.extend_from_slice(&self.level.to_le_bytes());
        buf.extend_from_slice(&self.color.to_le_bytes());
        buf.extend_from_slice(&self.exp.to_le_bytes());
        buf.extend_from_slice(&self.skill_point.to_le_bytes());
        buf.push(self.is_bind);
        buf.push(self.unused);
        buf.extend_from_slice(&self.name_len.to_le_bytes());
        buf.extend_from_slice(&self.name);
        for &(s, l) in &self.skills {
            buf.extend_from_slice(&s.to_le_bytes());
            buf.extend_from_slice(&l.to_le_bytes());
        }
        for &e in &self.evo_prop {
            buf.extend_from_slice(&e.to_le_bytes());
        }
        for &r in &self.reserved {
            buf.extend_from_slice(&r.to_le_bytes());
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O bloco de 192 B vai e volta inteiro: nome, habilidades e o resto (B112 — o
    /// `do_bloco` antigo lia 40 bytes e a gravação da jaula zerava nome e habilidades).
    #[test]
    fn o_bloco_do_mascote_vai_e_volta_inteiro() {
        let mut a = InfoPet::default();
        a.pet_tid = 10386;
        a.level = 7;
        a.skill_point = 3;
        a.is_bind = 1;
        a.name_len = 8;
        a.name[..8].copy_from_slice(&[76, 0, 111, 0, 98, 0, 111, 0]);
        a.skills[0] = (747, 2);
        a.skills[1] = (748, 1);
        a.evo_prop[5] = 9;
        a.reserved[9] = 4;
        let b = a.para_bytes();
        assert_eq!(b.len(), TAMANHO_INFO_PET);
        assert_eq!(InfoPet::do_bloco(&b), Some(a));
    }
}
