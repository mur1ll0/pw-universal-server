-- B217 (E8, Templates do painel) — o template de criação passa a ter tudo o que o molde do
-- `clsconfig` copia para o personagem novo (`DBCreateRole`, `cnet/gamedbd/dbcreaterole.hrp:46-60`
-- e `:320-360`: `GRoleBase` + `GRoleStatus` + bolsa + corpo + armazém do molde da classe).
--
-- 1. EXP e pontos livres iniciais (`GRoleStatus.exp` e `.pp`), que a tabela não tinha.
-- 2. As habilidades iniciais do realm_155 passam a morar no template, como no realm_126 (B108).
--    Até aqui o template do 1.5.5 não tinha nenhuma e o personagem recebia, na primeira carga,
--    as de `CharacterClass::default_skills()` (`crates/pw-core/src/types.rs:145`). Gravamos
--    exatamente essas — o personagem novo nasce com o mesmo de antes, agora editável.
-- Idempotente.

BEGIN;

ALTER TABLE class_templates ADD COLUMN IF NOT EXISTS initial_exp BIGINT NOT NULL DEFAULT 0;
ALTER TABLE class_templates ADD COLUMN IF NOT EXISTS initial_points INT NOT NULL DEFAULT 0;
ALTER TABLE test.class_templates ADD COLUMN IF NOT EXISTS initial_exp BIGINT NOT NULL DEFAULT 0;
ALTER TABLE test.class_templates ADD COLUMN IF NOT EXISTS initial_points INT NOT NULL DEFAULT 0;

CREATE TEMP TABLE habilidades_do_template (cls int, skill_id int) ON COMMIT DROP;
INSERT INTO habilidades_do_template VALUES
  (0, 1), (1, 81), (2, 1125), (2, 1126), (3, 299), (4, 102), (5, 1111),
  (6, 234), (6, 235), (7, 125), (7, 113), (8, 1350), (9, 1374), (9, 1381),
  (10, 2547), (11, 2571);
INSERT INTO habilidades_do_template SELECT c, 167 FROM generate_series(0, 11) c;

INSERT INTO class_template_skills (template_id, skill_id, level)
SELECT t.id, h.skill_id, 1
  FROM habilidades_do_template h
  JOIN class_templates t ON t.realm_id = 'realm_155' AND t.cls = h.cls
ON CONFLICT DO NOTHING;

-- O mesmo no schema de teste, que tem a cópia dos templates do realm_155.
INSERT INTO test.class_template_skills (template_id, skill_id, level)
SELECT t.id, h.skill_id, 1
  FROM habilidades_do_template h
  JOIN test.class_templates t ON t.realm_id = 'realm_155' AND t.cls = h.cls
ON CONFLICT DO NOTHING;

COMMIT;
