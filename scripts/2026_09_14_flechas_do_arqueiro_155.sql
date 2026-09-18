-- Flechas para o Arqueiro do realm_155 — molde e personagens já criados.
--
-- # Decisão, não evidência do original
--
-- O `clsconfig` do gamedbd 1.5.5 original dá ao Arqueiro só o Arco de Madeira (2250), sem
-- munição (conferido byte a byte em 2026-09-14: nenhum `GRoleInventory` com id de
-- `PROJECTILE_ESSENCE` no arquivo), e nenhuma missão do `tasks.data` entrega a Flecha de
-- Iniciante. No original o jogador compra a flecha. O Murillo pediu que o Arqueiro nasça
-- com flechas (teste em jogo de 2026-09-14), então elas entram aqui como escolha do
-- projeto.
--
-- # Qual flecha e onde
--
-- * 8543 "Flecha de Iniciante": `PROJECTILE_ESSENCE`, `type` 8546 — o mesmo
--   `require_projectile` do Arco de Madeira (2250) —, `require_weapon_level_min` 1,
--   `pile_num_max` 50000. O 2271 que o `ClassTemplateRepository` usava não existe na
--   `PROJECTILE_ESSENCE` do v156.
-- * Slot 11 do equipamento (`EQUIPIVTR_PROJECTILE`, `EC_IvtrTypes.h:67`).
-- * 1000 unidades, a quantidade que o projeto já usava.

BEGIN;

INSERT INTO class_template_items (template_id, container_type, slot, item_id, count, durability, max_durability)
SELECT t.id, 1, 11, 8543, 1000, 0, 0
  FROM class_templates t
 WHERE t.realm_id = 'realm_155' AND t.cls = 6
ON CONFLICT (template_id, container_type, slot) DO UPDATE
   SET item_id = EXCLUDED.item_id, count = EXCLUDED.count;

INSERT INTO character_items (character_id, container_type, slot, item_id, count, durability, max_durability)
SELECT c.id, 1, 11, 8543, 1000, 0, 0
  FROM characters c
 WHERE c.realm_id = 'realm_155' AND c.cls = 6 AND NOT c.is_deleted
   AND NOT EXISTS (
     SELECT 1 FROM character_items ci
      WHERE ci.character_id = c.id AND ci.container_type = 1 AND ci.slot = 11);

COMMIT;
