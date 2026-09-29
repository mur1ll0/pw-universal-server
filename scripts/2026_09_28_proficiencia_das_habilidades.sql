-- B145 — a proficiência das habilidades de produção (`ability`).
--
-- Cada item produzido soma 1 ou 2 à proficiência da habilidade de produção
-- (`IncSkillAbility`, `gs/player.cpp:16555-16567`), até o `GetMaxability` do nível
-- (`SkillWrapper::IncAbility`, `cskill/skill/skillwrapper.cpp:1325-1350`); o valor viaja no
-- `SKILL_DATA` (`short ability`, `EC_GPDataType.h:2216-2226`) e no `SKILL_ABILITY` (187).
ALTER TABLE character_skills
    ADD COLUMN IF NOT EXISTS ability INT NOT NULL DEFAULT 0;

-- O mesmo no schema `test`, onde a suíte isola tudo.
ALTER TABLE test.character_skills
    ADD COLUMN IF NOT EXISTS ability INT NOT NULL DEFAULT 0;
