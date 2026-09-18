-- Onde cada classe nasce no realm 155: o mapa 161 (`a61`), pelos moldes do servidor
-- original.
--
-- # De onde vem
--
-- `F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamedbd\clsconfig`, lido por
-- `specs/clsconfig_155/ler_clsconfig.py`. O `gamedbd` original copia o personagem novo do
-- molde da classe (`GameDBManager::GetClsDetail`, `cnet/gamedbd/gamedbmanager.cpp:345`),
-- e o molde de cada classe sai de `GetDataRoleId` (`gamedbmanager.cpp:208`):
--
--   cls  0 → role 16   cls  1 → role 19   cls  2 → role 20   cls  3 → role 23
--   cls  4 → role 24   cls  5 → role 27   cls  6 → role 28   cls  7 → role 31
--   cls  8 → role 18   cls  9 → role 17   cls 10 → role 21   cls 11 → role 22
--
-- Escolha do Murillo em 2026-09-12: o pacote `pwserver_155v156` (a mesma build v156 do
-- `elements.data` do realm), em que **todas** as classes nascem no mapa 161 — e não o
-- `home155`, que devolve as raças antigas às vilas do mundo 1 (item 47).
--
-- # Conferido
--
-- Cada ponto fica 1 cm acima do chão do `a61/map/*.hmap` e a poucos metros do Guia da
-- raça no `a61/npcgen.data`: Guia Selvagem (44699) em (-709, -362) para o Bárbaro e a
-- Feiticeira, Guia Alado (44698) em (-824, -260), Guia Guardião (44700) em (-807, -328),
-- Guia Abissal (44701) em (-647, -223), Guia dos Sombrios (44702) em (-759, -216).
--
-- Só as colunas de nascimento mudam. Idempotente.

BEGIN;

UPDATE class_templates AS t
   SET spawn_world_id = 161,
       spawn_x = v.x, spawn_y = v.y, spawn_z = v.z,
       updated_at = now()
  FROM (VALUES
    ( 0, -848.3218::real, 40.5060::real, -181.9892::real),  -- Guerreiro    (role 16)
    ( 1, -847.4388, 40.5051, -182.3745),                    -- Mago         (role 19)
    ( 2, -651.0889, 41.0100, -225.2057),                    -- Psíquico     (role 20)
    ( 3, -712.7958, 35.0153, -364.2174),                    -- Feiticeira   (role 23)
    ( 4, -712.8714, 35.0153, -364.3938),                    -- Bárbaro      (role 24)
    ( 5, -651.8218, 41.0100, -225.4892),                    -- Assassino    (role 27)
    ( 6, -821.6534, 44.9115, -259.6685),                    -- Arqueiro     (role 28)
    ( 7, -821.8543, 44.9129, -260.2281),                    -- Sacerdote    (role 31)
    ( 8, -800.5193, 44.9111, -314.2396),                    -- Guardião     (role 18)
    ( 9, -800.6604, 44.9111, -315.2453),                    -- Místico      (role 17)
    (10, -760.7382, 44.8697, -218.2820),                    -- Ceifador     (role 21)
    (11, -760.2554, 44.8826, -218.3112)                     -- Tormentador  (role 22)
  ) AS v(cls, x, y, z)
 WHERE t.realm_id = 'realm_155' AND t.cls = v.cls;

COMMIT;

SELECT cls, name, spawn_world_id, spawn_x, spawn_y, spawn_z
  FROM class_templates WHERE realm_id = 'realm_155' ORDER BY cls;
