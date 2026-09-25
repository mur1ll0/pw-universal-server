-- Atributos iniciais do realm 1.2.6: 5/5/5/5, como no 1.5.5
-- (`2026_09_16_atributos_iniciais_5_155.sql`).
--
-- # Por quê
--
-- O `gamedbd/clsconfig` do servidor 1.2.6 (`files1.2.6/pwserver/gamedbd/clsconfig`) traz,
-- no `GRoleStatus.property` (`extend_prop`: vitality, energy, strength, agility, max_hp,
-- max_mp — `cgame/gs/property.h:35-45`), **5/5/5/5** para os moldes das seis classes do 1.2.6
-- (0, 1, 3, 4, 6, 7; os que estão no mundo 1), com vida/mana = `vit_hp`/`eng_mp` × 5 da
-- `CHARRACTER_CLASS_CONFIG` v7 (Feiticeira: 60/60). Cada nível dá 5 pontos livres.
--
-- Os moldes do `realm_126` estavam com os números do `ptemplate.conf` (Feiticeira
-- 15/5/15/15 = a seção [HAG]). O `ptemplate.conf` é a base do cálculo de vida/mana/dano,
-- não a ficha do personagem novo. E como o leitor recusava o `ptemplate.conf` do 1.2.6
-- (procurava a seção [NEC] do 1.5.5; B101), o `pw-link` criava o personagem sem ficha e o
-- banco caía nesses moldes.
--
-- # O que muda
--
-- Moldes do `realm_126` com 5/5/5/5, e os personagens já criados com 5/5/5/5 e
-- `potential_points = 5 × (nível − 1)`. Vida e mana máximas são recalculadas pelo mundo na
-- entrada.

BEGIN;

UPDATE characters
   SET strength = 5, agility = 5, vitality = 5, energy = 5,
       potential_points = 5 * GREATEST(level - 1, 0)
 WHERE realm_id = 'realm_126' AND NOT is_deleted;

UPDATE class_templates
   SET strength = 5, agility = 5, vitality = 5, energy = 5
 WHERE realm_id = 'realm_126';

COMMIT;
