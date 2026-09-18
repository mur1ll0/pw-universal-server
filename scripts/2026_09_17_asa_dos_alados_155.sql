-- A Asa (2096, WINGMANWING_ESSENCE) no slot 12 (EQUIPIVTR_FLYSWORD) de Arqueiro (6) e
-- Sacerdote (7), como no molde original: o equipamento dos roles de molde no `clsconfig` do
-- `pwserver_155v156` traz `2096 pos 12 count 1 proc 19` junto do arco (2250) e das flechas
-- (43283, pos 11), e junto do cajado do Sacerdote (2251) — lido em 2026-09-17 com o
-- GRoleInventory (id, pos, count, max_count, data, proc_type, expire, guid1, guid2, mask).
-- O molde do realm tinha perdido a asa; o personagem eaa (5491) recebe a dele.
INSERT INTO class_template_items (template_id, container_type, slot, item_id, count, durability, max_durability)
SELECT t.id, 1, 12, 2096, 1, 0, 0
  FROM class_templates t
 WHERE t.realm_id = 'realm_155' AND t.cls IN (6, 7)
   AND NOT EXISTS (SELECT 1 FROM class_template_items i WHERE i.template_id = t.id AND i.container_type = 1 AND i.slot = 12);

INSERT INTO character_items (character_id, container_type, slot, item_id, count, durability, max_durability)
SELECT c.id, 1, 12, 2096, 1, 0, 0
  FROM characters c
 WHERE c.realm_id = 'realm_155' AND c.cls IN (6, 7) AND c.is_deleted = FALSE
   AND NOT EXISTS (SELECT 1 FROM character_items i WHERE i.character_id = c.id AND i.container_type = 1 AND i.slot = 12);
