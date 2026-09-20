-- B65 — o nível de cultivo que a missão já concedida não deu.
--
-- A missão 32394 "Só um Pouco de Progresso" (NPC 44713, nível 9) tem a submissão 32416
-- "Adepto Espiritual", cujo prêmio traz `m_ulNewPeriod = 1` — o **nível de cultivo**
-- (`Task/TaskProcess.cpp:1284` → `PlayerTaskInterface::SetCurPeriod` →
-- `gplayer_imp::SetSecLevel`, `gs/task/taskman.cpp:251-254`, `gs/player_imp.h:2798-2804`).
--
-- Nosso leitor de `tasks.data` não lia esse campo do `AWARD_DATA` (deslocamento 25), então
-- o prêmio era descartado em silêncio: o eaa concluiu a cadeia, ganhou os 3.000 de chi (SP)
-- e ficou com `cultivation = 0`. Corrigido em B65; este script acerta quem já passou por lá.
--
-- Aplicado uma vez, em 2026-09-20.

UPDATE characters
   SET cultivation = 1
 WHERE id = 5491 AND cultivation = 0;

SELECT id, name, level, cultivation, sp FROM characters WHERE id = 5491;
