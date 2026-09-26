-- B119 — o teto de chi (`max_ap`) que o prêmio da missão 973 "Missão Bem-Sucedida!" deveria ter
-- dado à Tsuko (11455, realm_126): o leitor v55 não lia o `m_ulFuryULimit` (+40 do prêmio;
-- `libtask.so` 1.2.6, `DeliverByAwardData` 0xb4b3-0xb4d2). A 973 é filha da 970, que está na
-- lista de concluídas dela, e o log do mundo 1.2.6 registra a entrega em 2026-09-26 00:28 UTC.
-- As de 199/299/399 (922, 925, 1888, 2804, 2818) pedem nível 29+; ela tem 18.
BEGIN;
UPDATE characters SET max_ap = 99 WHERE id = 11455 AND max_ap < 99;
COMMIT;
