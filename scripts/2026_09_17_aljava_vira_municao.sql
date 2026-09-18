-- Aljavas guardadas como item vão para a munição que contêm.
--
-- Até 2026-09-17 o drop entregava o id cru do `QUIVER_ESSENCE` (ex.: 1955 "Flecha Presa de
-- Lobo 1", do Espírito da Estrela 44578). O original nunca põe aljava na bolsa:
-- `generate_quiver` (`gs/template/generate_item_temp.h:650-667`) entrega o `id_projectile`
-- com `RandNormal(num_min, num_max)` unidades. Aqui, sem o sorteio que não houve, cada
-- aljava guardada vira `num_min` unidades — o menor valor que o original daria.
--
-- Mapeamento das aljavas do 155 que caem de monstro iniciante (conferido no
-- `elements.data` v156): 1955 → 410 × 50..150.

BEGIN;

UPDATE character_items ci
   SET item_id = 410, count = 50 * ci.count
  FROM characters c
 WHERE ci.character_id = c.id AND c.realm_id = 'realm_155' AND ci.item_id = 1955;

COMMIT;
