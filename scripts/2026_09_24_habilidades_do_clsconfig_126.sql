-- B108 — habilidades iniciais do realm_126 pelo `clsconfig` do gamedbd 1.2.6, com o Portal da
-- Cidade (167).
--
-- O `2026_09_18_templates_iniciais_realm_126.sql` gravou só a habilidade de ataque de cada
-- classe, e no banco sobrou uma por classe (sem a 235 do Arqueiro e a 125 do Sacerdote).
-- O molde original de cada classe (`GRoleStatus.skills`, formato de
-- `SkillWrapper::StoreDatabase`, `cskill/skill/skillwrapper.cpp:870-879`: contador + (id,
-- ability, level) little-endian) traz, em `files1.2.6/pwserver/gamedbd/clsconfig`:
--   role 16/17 cls 0: 1, 167        role 19 cls 1: 81, 167       role 23 cls 3: 167, 299
--   role 24 cls 4: 102, 167         role 28 cls 6: 167, 234, 235 role 31 cls 7: 113, 125, 167
-- todas no nível 1. É o que o `gamedbd` copia para o personagem novo.
--
-- 1. Os moldes do realm_126 passam a ter exatamente essas habilidades.
-- 2. Os personagens do realm_126 que já existem recebem as que faltam (nível 1), sem mexer
--    nas que já têm.

BEGIN;

CREATE TEMP TABLE habilidades_do_molde (cls int, skill_id int) ON COMMIT DROP;
INSERT INTO habilidades_do_molde VALUES
  (0, 1), (0, 167),
  (1, 81), (1, 167),
  (3, 167), (3, 299),
  (4, 102), (4, 167),
  (6, 167), (6, 234), (6, 235),
  (7, 113), (7, 125), (7, 167);

DELETE FROM class_template_skills
 WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_126');

INSERT INTO class_template_skills (template_id, skill_id, level)
SELECT t.id, h.skill_id, 1
  FROM class_templates t
  JOIN habilidades_do_molde h ON h.cls = t.cls
 WHERE t.realm_id = 'realm_126';

INSERT INTO character_skills (character_id, skill_id, level)
SELECT c.id, h.skill_id, 1
  FROM characters c
  JOIN habilidades_do_molde h ON h.cls = c.cls
 WHERE c.realm_id = 'realm_126' AND NOT c.is_deleted
ON CONFLICT (character_id, skill_id) DO NOTHING;

COMMIT;
