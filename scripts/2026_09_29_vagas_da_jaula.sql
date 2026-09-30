-- B153 — as vagas da jaula de mascotes (`pet_manager::_active_pet_slot`, `pets.capacity` gravado
-- em `userlogin.cpp:782` e lido em `:133`): começa em 1 (`petman.cpp:1276`) e só cresce, até
-- `MAX_PET_CAPACITY` 20 (`petman.h:114, 170-176`), pelo prêmio de missão `m_ulPetInventorySize`
-- (`TaskProcess.cpp:1292` → `SetPetSlotCapacity`).
--
-- Até o B153 o servidor inventava a capacidade (mascotes na jaula + 1) e a incubação ocupava
-- qualquer slot. Para não perder mascote de quem já tem, a capacidade inicial de cada personagem
-- é a maior entre 1 e o último slot ocupado + 1.
ALTER TABLE characters ADD COLUMN IF NOT EXISTS pet_slots INTEGER NOT NULL DEFAULT 1;
ALTER TABLE test.characters ADD COLUMN IF NOT EXISTS pet_slots INTEGER NOT NULL DEFAULT 1;

UPDATE characters c
   SET pet_slots = LEAST(20, GREATEST(c.pet_slots, j.ultimo + 1))
  FROM (SELECT character_id, MAX(slot) AS ultimo
          FROM character_items WHERE container_type = 4 GROUP BY character_id) j
 WHERE j.character_id = c.id;
