-- Moldes de personagem novo do realm 1.2.6 (Classic).
--
-- # Classes do 1.2.6
--
-- O 1.2.6 possui apenas 6 classes (Humanos, Selvagens e Alados):
--   0: Guerreiro (Humano)
--   1: Mago (Humano)
--   3: Feiticeira (Selvagem)
--   4: Bárbaro (Selvagem)
--   6: Arqueiro (Alado)
--   7: Sacerdote (Alado)
--
-- # Coordenadas de nascimento no Mapa 1
--
--   Os pontos dos moldes do `gamedbd/clsconfig` 1.2.6, junto ao Guia de cada raça
--   (`2026_09_24_moldes_do_clsconfig_126.sql`, que também grava a configuração do cliente):
--   Humanos (0, 1) junto ao Guia 3517; Selvagens (3, 4) ao 3518; Alados (6, 7) ao 3519.
--
-- Idempotente: recria os moldes do realm_126. Atributos 5/5/5/5 do `clsconfig` 1.2.6
-- (ver `2026_09_24_atributos_iniciais_5_126.sql`).

BEGIN;

DELETE FROM class_template_items
 WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_126');
DELETE FROM class_template_skills
 WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_126');
DELETE FROM class_templates WHERE realm_id = 'realm_126';

INSERT INTO class_templates
    (realm_id, cls, name, initial_level, initial_cultivation, initial_money, initial_sp,
     strength, agility, vitality, energy, spawn_world_id, spawn_x, spawn_y, spawn_z)
VALUES
  -- Humanos — Vale das Espadas
  ('realm_126', 0, 'Guerreiro',   1, 0, 0, 0, 5, 5, 5, 5, 1,   217.2983, 218.5191,  2838.3706),
  ('realm_126', 1, 'Mago',        1, 0, 0, 0, 5, 5, 5, 5, 1,   217.2983, 218.5191,  2838.3706),
  -- Selvagens — Cidade das Feras (coordenada medida da captura da VM)
  ('realm_126', 3, 'Feiticeira',  1, 0, 0, 0, 5, 5, 5, 5, 1, -1441.9996, 242.1628,  1383.1140),
  ('realm_126', 4, 'Bárbaro',     1, 0, 0, 0, 5, 5, 5, 5, 1, -1442.1249, 242.1220,  1383.5858),
  -- Alados — Vila dos Alados
  ('realm_126', 6, 'Arqueiro',    1, 0, 0, 0, 5, 5, 5, 5, 1,  -317.4324, 218.0879, -910.9881),
  ('realm_126', 7, 'Sacerdote',   1, 0, 0, 0, 5, 5, 5, 5, 1,  -318.4770, 218.1214, -910.7966);

-- Arma inicial de cada classe no slot 0 do equipamento
INSERT INTO class_template_items (template_id, container_type, slot, item_id, count, durability, max_durability)
SELECT t.id, 1, 0, a.item_id, 1, 2800, 2800
  FROM class_templates t
  JOIN (VALUES
        (0, 2097),   -- Guerreiro   Espada de Madeira
        (1, 2251),   -- Mago        Varinha
        (3, 2251),   -- Feiticeira  Varinha
        (4, 2258),   -- Bárbaro     Porrete com Espinhos
        (6, 2250),   -- Arqueiro    Arco de Madeira
        (7, 2251)    -- Sacerdote   Varinha
       ) AS a(cls, item_id) ON a.cls = t.cls
 WHERE t.realm_id = 'realm_126';

-- Habilidades iniciais de nível 1 de cada classe — as do molde do `clsconfig` 1.2.6, com o
-- Portal da Cidade (167) em todas (`2026_09_24_habilidades_do_clsconfig_126.sql`, B108).
INSERT INTO class_template_skills (template_id, skill_id, level)
SELECT t.id, s.skill_id, s.level
  FROM class_templates t
  JOIN (VALUES
        (0,   1, 1),   -- Guerreiro: Golpe Básico
        (0, 167, 1),   -- todas: Portal da Cidade
        (1, 167, 1),
        (3, 167, 1),
        (4, 167, 1),
        (6, 167, 1),
        (7, 167, 1),
        (1,  81, 1),   -- Mago: Piromancia
        (3, 299, 1),   -- Feiticeira: Veneno
        (4, 102, 1),   -- Bárbaro: Golpe de Martelo
        (6, 234, 1),   -- Arqueiro: Tiro
        (6, 235, 1),   -- Arqueiro: Flecha Sucessiva
        (7, 113, 1),   -- Sacerdote: Cura
        (7, 125, 1)    -- Sacerdote: Flecha Emplumada
       ) AS s(cls, skill_id, level) ON s.cls = t.cls
 WHERE t.realm_id = 'realm_126';

COMMIT;
