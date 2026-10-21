//! B320: a classe e o nível de cada modelo de item para **organizar a bolsa** como o cliente 1.5.5 organiza
//! (`CECHostPlayer::SortPack` + `DefaultPackSorter`, `EC_HostPlayer.cpp:1700-1870`).
//!
//! A classe é o `ICID` que o cliente dá ao item: o `switch` de `CECIvtrItem::CreateItem` (`EC_IvtrItem.cpp`) escolhe a
//! classe pela tabela do `elements.data` (`DT_*`), e o construtor de cada classe grava `m_iCID = ICID_*`; o número é a
//! posição no enum de `EC_IvtrItem.h` (`ICID_ARMOR` = 0). Item sem tabela conhecida vira `CECIvtrUnknown`
//! (`ICID_ERRORITEM`). O nível é o `level` do `WEAPON_ESSENCE`/`ARMOR_ESSENCE` (o comparador desempata arma e
//! armadura por ele); o resto da regra está em `pw_gs::organizar`.

use std::collections::HashMap;

use crate::generic_elements::GenericElementsData;

/// `ICID_ERRORITEM` (`CECIvtrUnknown`).
pub const CLASSE_DESCONHECIDA: i32 = 21;
pub const CLASSE_ARMA: i32 = 15;
pub const CLASSE_ARMADURA: i32 = 0;
/// `ICID_GENERALCARD` (`POKER_ESSENCE`): desempate pelos octetos da instância (`IVTR_ESSENCE_GENERALCARD`).
pub const CLASSE_CARTA: i32 = 57;

/// Tabela do `elements.data` → `ICID` (gerada do `CreateItem` e dos construtores do cliente 1.5.5; no 1.2.6 só
/// existem 40 destas tabelas, as outras simplesmente não aparecem).
pub const CLASSE_POR_TABELA: [(&str, i32); 66] = [
    ("WEAPON_ESSENCE", 15), // CECIvtrWeapon: ICID_WEAPON
    ("PROJECTILE_ESSENCE", 2), // CECIvtrArrow: ICID_ARROW
    ("ARMOR_ESSENCE", 0), // CECIvtrArmor: ICID_ARMOR
    ("DECORATION_ESSENCE", 3), // CECIvtrDecoration: ICID_DECORATION
    ("FASHION_ESSENCE", 6), // CECIvtrFashion: ICID_FASHION
    ("MEDICINE_ESSENCE", 9), // CECIvtrMedicine: ICID_MEDICINE
    ("MATERIAL_ESSENCE", 8), // CECIvtrMaterial: ICID_MATERIAL
    ("DAMAGERUNE_ESSENCE", 4), // CECIvtrDmgRune: ICID_DMGRUNE
    ("ARMORRUNE_ESSENCE", 1), // CECIvtrArmorRune: ICID_ARMORRUNE
    ("SKILLTOME_ESSENCE", 11), // CECIvtrSkillTome: ICID_SKILLTOME
    ("FLYSWORD_ESSENCE", 7), // CECIvtrFlySword: ICID_FLYSWORD
    ("TOWNSCROLL_ESSENCE", 13), // CECIvtrTownScroll: ICID_TOWNSCROLL
    ("UNIONSCROLL_ESSENCE", 14), // CECIvtrUnionScroll: ICID_UNIONSCROLL
    ("REVIVESCROLL_ESSENCE", 10), // CECIvtrRevScroll: ICID_REVSCROLL
    ("ELEMENT_ESSENCE", 5), // CECIvtrElement: ICID_ELEMENT
    ("TOSSMATTER_ESSENCE", 12), // CECIvtrTossMat: ICID_TOSSMAT
    ("TASKMATTER_ESSENCE", 16), // CECIvtrTaskItem: ICID_TASKITEM
    ("STONE_ESSENCE", 17), // CECIvtrStone: ICID_STONE
    ("WINGMANWING_ESSENCE", 18), // CECIvtrWing: ICID_WING
    ("TASKDICE_ESSENCE", 19), // CECIvtrTaskDice: ICID_TASKDICE
    ("TASKNORMALMATTER_ESSENCE", 20), // CECIvtrTaskNmMatter: ICID_TASKNMMATTER
    ("FACETICKET_ESSENCE", 22), // CECIvtrFaceTicket: ICID_FACETICKET
    ("FACEPILL_ESSENCE", 23), // CECIvtrFacePill: ICID_FACEPILL
    ("GM_GENERATOR_ESSENCE", 24), // CECIvtrGMGenerator: ICID_GM_GENERATOR
    ("RECIPE_ESSENCE", 25), // CECIvtrRecipe: ICID_RECIPE
    ("PET_EGG_ESSENCE", 26), // CECIvtrPetEgg: ICID_PETEGG
    ("PET_FOOD_ESSENCE", 27), // CECIvtrPetFood: ICID_PETFOOD
    ("PET_FACETICKET_ESSENCE", 28), // CECIvtrPetFaceTicket: ICID_PETFACETICKET
    ("FIREWORKS_ESSENCE", 29), // CECIvtrFirework: ICID_FIREWORK
    ("FIREWORKS2_ESSENCE", 66), // CECIvtrFirework2: ICID_FIREWORK2
    ("WAR_TANKCALLIN_ESSENCE", 30), // CECIvtrTankCallin: ICID_TANKCALLIN
    ("SKILLMATTER_ESSENCE", 31), // CECIvtrSkillMat: ICID_SKILLMATTER
    ("INC_SKILL_ABILITY_ESSENCE", 47), // CECIvtrIncSkillAbility: ICID_INCSKILLABILITY
    ("REFINE_TICKET_ESSENCE", 32), // CECIvtrRefineTicket: ICID_REFINETICKET
    ("DESTROYING_ESSENCE", 33), // CECIvtrDestroyingEssence: ICID_DESTROYINGESSENCE
    ("BIBLE_ESSENCE", 34), // CECIvtrBible: ICID_BIBLE
    ("SPEAKER_ESSENCE", 35), // CECIvtrSpeaker: ICID_SPEAKER
    ("AUTOHP_ESSENCE", 36), // CECIvtrAutoHP: ICID_AUTOHP
    ("AUTOMP_ESSENCE", 37), // CECIvtrAutoMP: ICID_AUTOMP
    ("DOUBLE_EXP_ESSENCE", 38), // CECIvtrDoubleExp: ICID_DOUBLEEXP
    ("DYE_TICKET_ESSENCE", 40), // CECIvtrDyeTicket: ICID_DYETICKET
    ("TRANSMITSCROLL_ESSENCE", 39), // CECIvtrTransmitScroll: ICID_TRANSMITSCROLL
    ("GOBLIN_ESSENCE", 41), // CECIvtrGoblin: ICID_GOBLIN
    ("GOBLIN_EQUIP_ESSENCE", 42), // CECIvtrGoblinEquip: ICID_GOBLIN_EQUIP
    ("GOBLIN_EXPPILL_ESSENCE", 43), // CECIvtrGoblinExpPill: ICID_GOBLIN_EXPPILL
    ("SELL_CERTIFICATE_ESSENCE", 44), // CECIvtrCertificate: ICID_CERTIFICATE
    ("TARGET_ITEM_ESSENCE", 45), // CECIvtrTargetItem: ICID_TARGETITEM
    ("LOOK_INFO_ESSENCE", 46), // CECIvtrLookInfoItem: ICID_LOOKINFOITEM
    ("WEDDING_BOOKCARD_ESSENCE", 48), // CECIvtrWeddingBookCard: ICID_WEDDINGBOOKCARD
    ("SHARPENER_ESSENCE", 50), // CECIvtrShapener: ICID_SHARPENER
    ("FACTION_MATERIAL_ESSENCE", 51), // CECIvtrFactionMaterial: ICID_FACTIONMATERIAL
    ("CONGREGATE_ESSENCE", 52), // CECIvtrCongregate: ICID_CONGREGATE
    ("FORCE_TOKEN_ESSENCE", 53), // CECIvtrForceToken: ICID_FORCETOKEN
    ("DYNSKILLEQUIP_ESSENCE", 54), // CECIvtrDynSkillEquip: ICID_DYNSKILLEQUIP
    ("MONEY_CONVERTIBLE_ESSENCE", 55), // CECIvtrMoneyConvertible: ICID_MONEYCONVERTIBLE
    ("MONSTER_SPIRIT_ESSENCE", 56), // CECIvtrMonsterSpirit: ICID_MONSTERSPIRIT
    ("POKER_ESSENCE", 57), // CECIvtrGeneralCard: ICID_GENERALCARD
    ("POKER_DICE_ESSENCE", 58), // CECIvtrGeneralCardDice: ICID_GENERALCARD_DICE
    ("SHOP_TOKEN_ESSENCE", 59), // CECIvtrShopToken: ICID_SHOPTOKEN
    ("UNIVERSAL_TOKEN_ESSENCE", 60), // CECIvtrUniversalToken: ICID_UNIVERSAL_TOKEN
    ("ASTROLABE_ESSENCE", 61), // CECIvtrAstrolabe: ICID_ASTROLABE
    ("ITEM_PACKAGE_BY_PROFESSION_ESSENCE", 62), // CECIvtrOccupPackage: ICID_OCCUP_PACKAGE
    ("ASTROLABE_INC_EXP_ESSENCE", 63), // CECIvtrAstrolabeExpPill: ICID_ASTROLABE_EXP_ITEM
    ("ASTROLABE_INC_INNER_POINT_VALUE_ESSENCE", 64), // CECIvtrAstrolabePointIncPill: ICID_ASTROLABE_POINT_INC_ITEM
    ("ASTROLABE_RANDOM_ADDON_ESSENCE", 65), // CECIvtrAstrolabeRandomAddonPill: ICID_ASTROLABE_RANDOM_ADDON_ITEM
    ("FIX_POSITION_TRANSMIT_ESSENCE", 62), // CECIvtrFixPositionTransmit: ICID_OCCUP_PACKAGE
];

/// A ordem das classes do `DefaultPackSorter::GetClassIDOrder` (`s_CIDs`): quem está antes vale mais
/// (`count − i`); classe fora da lista vale 0.
pub const ORDEM_DAS_CLASSES: [i32; 23] = [15, 2, 12, 0, 3, 34, 7, 18, 41, 42, 6, 36, 37, 9, 31, 45, 17, 26, 32, 40, 43, 57, 58];

pub fn ordem_da_classe(classe: i32) -> i32 {
    ORDEM_DAS_CLASSES.iter().position(|&c| c == classe).map_or(0, |i| (ORDEM_DAS_CLASSES.len() - i) as i32)
}

/// Modelo → `(classe, nível)`.
pub fn carregar(g: &GenericElementsData) -> HashMap<u32, (i32, i32)> {
    let mut m = HashMap::new();
    for (tabela, classe) in CLASSE_POR_TABELA {
        for r in g.get(tabela) {
            let Some(id) = r.get("ID").and_then(|v| v.as_i32()).filter(|&i| i > 0) else { continue };
            let nivel = if classe == CLASSE_ARMA || classe == CLASSE_ARMADURA {
                r.get("level").and_then(|v| v.as_i32()).unwrap_or(0)
            } else {
                0
            };
            m.entry(id as u32).or_insert((classe, nivel));
        }
    }
    m
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ordem_das_classes_como_no_cliente() {
        assert_eq!(ordem_da_classe(CLASSE_ARMA), 23);
        assert_eq!(ordem_da_classe(58), 1);
        assert_eq!(ordem_da_classe(CLASSE_DESCONHECIDA), 0);
        assert_eq!(CLASSE_POR_TABELA.iter().find(|x| x.0 == "FIX_POSITION_TRANSMIT_ESSENCE").map(|x| x.1), Some(62), "ICID_OCCUP_PACKAGE");
    }
}
