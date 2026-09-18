-- Atributos dos personagens 1.5.5 já criados: de volta a 5/5/5/5, com os pontos da subida
-- de nível para distribuir.
--
-- # Por quê
--
-- De 2026-09-09 a 2026-09-16 a criação gravava os atributos do `ptemplate.conf`
-- (Arqueiro: vitalidade 15, energia 20, força 5, agilidade 10). O `gamed` original lê
-- esses números mas não os dá a jogador: a ficha nasce do molde do `clsconfig` do
-- `gamedbd`, que traz **5/5/5/5 para as 12 classes** (roles 16..31 do
-- `pwserver_155v156/home/pwserver/gamedbd/clsconfig`, `GRoleStatus.property`), e cada
-- nível dá 5 pontos livres (`LevelUp`, `player.cpp:2627`).
--
-- # O que muda
--
-- Até esta data não havia como distribuir ponto (`SET_STATUS_POINT` não existia), então
-- tudo acima de 5 veio do molde errado. Cada personagem dos realms 1.5.5 fica com 5/5/5/5
-- e `potential_points = 5 × (nível − 1)`. Vida e mana máximas são recalculadas pelo mundo
-- na entrada; a vida atual é limitada ao novo máximo lá.

BEGIN;

UPDATE characters
   SET strength = 5, agility = 5, vitality = 5, energy = 5,
       potential_points = 5 * GREATEST(level - 1, 0)
 WHERE realm_id = 'realm_155' AND NOT is_deleted;

UPDATE class_templates
   SET strength = 5, agility = 5, vitality = 5, energy = 5
 WHERE realm_id = 'realm_155';

COMMIT;
