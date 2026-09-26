-- B117 — itens que as entregas do 1.2.6 deveriam ter tirado da bolsa de missão da Tsuko (11455).
-- O leitor v55 não lia o `m_bClearAcquired` (+0xae, `libtask.so` 1.2.6: `RecursiveAward` 0xabee,
-- `RecursiveClearTask` 0xd723), então o `RemoveAcquiredItem` nunca rodava no 1.2.6.
-- Todos estes vêm de missões com `m_bClearAcquired` e que não estão mais ativas (ativas: 2650,
-- 2669, 9374, nenhuma pede estes itens):
--   11538, 11537, 11543, 11544, 11545, 11546, 11547 — cadeia 5919 "Capturar Fera Psíquica"
--     (5920/5922/5925-5927/5933; a 5933 foi entregue em 2026-09-26 00:22 UTC);
--   11531 — 5913 "Certif. do Lobo" (cai do monstro 1047);
--   3268  — "O Surgimento de Lingxu" (dado pela 915/966, recolhido pela 917/967).
BEGIN;
DELETE FROM character_items
 WHERE character_id = 11455
   AND container_type = 5
   AND item_id IN (11538, 11537, 11543, 11544, 11545, 11546, 11547, 11531, 3268);
COMMIT;
