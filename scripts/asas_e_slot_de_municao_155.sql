-- Asas iniciais para os Alados e as flechas no slot certo.
--
-- Os slots de equipamento seguem `EQUIPIVTR_*` do `EC_IvtrTypes.h` do client 1.5.5:
-- 0 arma, 11 munição (`EQUIPIVTR_PROJECTILE`), 12 item de voo (`EQUIPIVTR_FLYSWORD`).
--
-- Dois defeitos corrigidos aqui:
--
--   * A munição do Arqueiro estava no slot **12** — o slot de voo. Ela ocupava o lugar
--     das asas, e nenhum dos dois funcionava direito.
--   * Arqueiro (6) e Sacerdote (7) são Alados e nasciam sem item de voo. A asa é o item
--     2096 ("Asa"), a única linha da tabela `WINGMANWING_ESSENCE` do `elements.data`,
--     nível 1. O cliente confere a classe sozinho: `CanUseEquipment` recusa `ICID_WING`
--     para quem não for `PROF_ARCHOR` nem `PROF_ANGEL` (`EC_HostPlayer.cpp:4927`).
--
-- Mesmos valores de `ClassTemplateRepository`. Só o realm_155.

BEGIN;

CREATE TEMP TABLE alados ON COMMIT DROP AS
  SELECT id, cls FROM characters
   WHERE realm_id = 'realm_155'
     AND NOT is_deleted
     AND cls IN (6, 7);

-- 1. Tira o que estiver ocupando o slot de voo (as flechas mal colocadas).
DELETE FROM character_items ci
 USING alados a
 WHERE ci.character_id = a.id AND ci.container_type = 1 AND ci.slot = 12;

-- 2. Munição do Arqueiro no slot 11, de onde ela nunca deveria ter saído.
INSERT INTO character_items
       (character_id, container_type, slot, item_id, count, durability, max_durability)
SELECT a.id, 1, 11, 2271, 1000, 0, 0
  FROM alados a
 WHERE a.cls = 6
   AND NOT EXISTS (
     SELECT 1 FROM character_items ci
      WHERE ci.character_id = a.id AND ci.container_type = 1 AND ci.slot = 11);

-- 3. A asa, no slot de voo.
INSERT INTO character_items
       (character_id, container_type, slot, item_id, count, durability, max_durability)
SELECT a.id, 1, 12, 2096, 1, 0, 0 FROM alados a;

COMMIT;
