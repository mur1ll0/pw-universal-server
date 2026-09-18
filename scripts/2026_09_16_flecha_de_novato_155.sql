-- Troca a munição do Arqueiro do realm_155: Flecha de Iniciante (8543) → Flecha de Novato (43283).
--
-- O script de 2026-09-14 dava 8543, que pede **arma de nível 1 a 17**
-- (`require_weapon_level_min` 1). O Arco de Madeira (2250) do molde é **nível 0**
-- (`WEAPON_ESSENCE.level`, que o original copia para o `weapon_level` do item,
-- `generate_item_temp.h:329`). O cliente compara os dois em `CanUseProjectile`
-- (`EC_HostPlayer.cpp:5009-5016`) e pinta o arco de vermelho quando não cabem.
--
-- A 43283 "Flecha de Novato" é do mesmo tipo (8546) e aceita arma de nível 0 a 17.
-- Continua sendo decisão do projeto dar munição no molde (ver o script de 2026-09-14).

BEGIN;

UPDATE class_template_items cti
   SET item_id = 43283
  FROM class_templates t
 WHERE cti.template_id = t.id AND t.realm_id = 'realm_155' AND t.cls = 6
   AND cti.container_type = 1 AND cti.slot = 11 AND cti.item_id = 8543;

UPDATE character_items ci
   SET item_id = 43283
  FROM characters c
 WHERE ci.character_id = c.id AND c.realm_id = 'realm_155' AND c.cls = 6
   AND ci.container_type = 1 AND ci.slot = 11 AND ci.item_id = 8543;

COMMIT;
