-- Corrige as habilidades e a arma inicial dos personagens dos realms 1.5.5.
--
-- Motivo (achado em 2026-09-07, cruzando os stubs do ElementSkill do client 1.5.5 com o
-- elements.data do realm):
--
--   * As habilidades gravadas vinham de uma lista chutada. Os sacerdotes tinham a 11
--     (passiva, nível 29), a 117 (nível 29), a 118 (39) e a 119 (49) — o cliente monta a
--     barra com elas e recusa todas.
--   * A arma inicial era o "Graveto de Madeira" (2867), tipo maior 5 (Acha). Toda
--     habilidade de Sacerdote só aceita o tipo 292 (Magia) ou nenhuma arma, e
--     `ElementSkill::Condition` recusa a conjuração já na primeira linha
--     (`if (!ValidWeapon(info.weapon)) return 1;`). De quebra, o modelo 3D do graveto
--     não existe na instalação dos dois clients — a "arma não funcional" relatada.
--
-- Os valores abaixo são os mesmos de `CharacterClass::default_skills()` e
-- `default_weapon_id()` em `crates/pw-core/src/types.rs`, cobertos pelos testes
-- `as_habilidades_iniciais_sao_as_da_arvore_da_classe` (pw-core) e
-- `armas_iniciais_batem_com_o_elements` (pw-data-loader).
--
-- Só mexe em realm_155 e realm_155BR. O realm_126 é de outra versão e fica de fora.

BEGIN;

CREATE TEMP TABLE inicio_por_classe (cls int, skill_id int, arma_id int) ON COMMIT DROP;
INSERT INTO inicio_por_classe (cls, skill_id, arma_id) VALUES
  (0,     1, 2097),  -- Guerreiro   — 流水诀            / Espada de Madeira   (tipo 1)
  (1,    81, 2251),  -- Mago        — 烈火符            / Varinha             (tipo 292)
  (2,  1125, 26332), -- Espiritual. — ataque de orbe    / Pequena Esfera      (tipo 25333)
  (2,  1126, 26332),
  (3,   299, 2251),  -- Feiticeira  — 剧毒蛊            / Varinha             (tipo 292)
  (4,   102, 2258),  -- Bárbaro     — 重击              / Porrete c/ Espinhos (tipo 9)
  (5,  1111, 26331), -- Mercenário  — ataque de adaga   / Faca de Limpar Osso (tipo 23749)
  (6,   234, 2250),  -- Arqueiro    — 引而不发          / Arco de Madeira     (tipo 13)
  (6,   235, 2250),
  (7,   125, 2251),  -- Sacerdote   — 羽箭              / Varinha             (tipo 292)
  (7,   113, 2251),
  (8,  1350, 2097),  -- Arcano                          / Espada de Madeira   (tipo 1)
  (9,  1374, 2251),  -- Místico                         / Varinha             (tipo 292)
  (9,  1381, 2251),
  (10, 2547, 44937), -- Retalhador                      / Sabre de Bronze     (tipo 44878)
  (11, 2571, 45020); -- Tormentador                     / Foice de Ferro      (tipo 44879)

CREATE TEMP TABLE alvos ON COMMIT DROP AS
  SELECT id, cls FROM characters
   WHERE realm_id IN ('realm_155', 'realm_155BR') AND NOT is_deleted;

-- 1. Habilidades: apaga o que estava lá e grava a árvore da classe + a 167 (Portal da
--    Cidade, cls 255, vale para todas).
DELETE FROM character_skills WHERE character_id IN (SELECT id FROM alvos);

INSERT INTO character_skills (character_id, skill_id, level)
SELECT a.id, i.skill_id, 1 FROM alvos a JOIN inicio_por_classe i ON i.cls = a.cls;

INSERT INTO character_skills (character_id, skill_id, level)
SELECT a.id, 167, 1 FROM alvos a;

-- 2. Arma equipada (container 1, slot 0): troca pela arma do tipo maior da classe.
UPDATE character_items ci
   SET item_id  = i.arma_id,
       updated_at = now()
  FROM alvos a
  JOIN inicio_por_classe i ON i.cls = a.cls
 WHERE ci.character_id = a.id
   AND ci.container_type = 1
   AND ci.slot = 0
   AND ci.item_id <> i.arma_id;

-- 3. Quem estava sem arma nenhuma ganha a da classe.
INSERT INTO character_items
       (character_id, container_type, slot, item_id, count, durability, max_durability)
SELECT DISTINCT a.id, 1, 0, i.arma_id, 1, 2800, 2800
  FROM alvos a JOIN inicio_por_classe i ON i.cls = a.cls
 WHERE NOT EXISTS (
   SELECT 1 FROM character_items ci
    WHERE ci.character_id = a.id AND ci.container_type = 1 AND ci.slot = 0);

COMMIT;
