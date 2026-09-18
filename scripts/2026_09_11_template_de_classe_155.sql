-- Reescreve o molde de personagem novo do realm 155.
--
-- # O que estava errado
--
-- A tabela tinha **seis** das doze classes, e os ids não batiam com os nomes: a linha
-- `cls = 4` chamava-se "Feiticeira" (4 é o Bárbaro) e a `cls = 3` chamava-se "Bárbaro"
-- (3 é a Feiticeira). Como `create_character` procura por `cls`, o Bárbaro criado em
-- 2026-09-11 recebeu o molde da Feiticeira: **Graveto de Madeira** (2867, arma de magia)
-- no lugar do Porrete com Espinhos.
--
-- Os itens de bolsa eram os mesmos para todas as classes, e um deles não serve a
-- personagem nenhum de nível 1: a **Poção Perfeita de Cura** (1801) tem
-- `require_level = 30` no `elements.data`.
--
-- # De onde vem cada valor
--
-- | campo | fonte |
-- | :--- | :--- |
-- | `cls`, `name` | `pw_core::CharacterClass` (`types.rs:47-60`) |
-- | arma inicial | `CharacterClass::default_weapon_id`, conferida contra o `WEAPON_ESSENCE` pelo teste `armas_iniciais_batem_com_o_elements` |
-- | poções e pergaminho | `MEDICINE_ESSENCE`/`TOWNSCROLL_ESSENCE`, escolhidos por `require_level = 0` |
-- | nascimento | `CharacterClass::default_spawn_position`, por **raça** |
--
-- Os quatro atributos e a vida/mana **não saem daqui**: vêm do `ptemplate.conf` do realm,
-- pela mesma conta que o mundo usa (`BaseDaClasse::ficha_inicial`). As colunas de atributo
-- desta tabela existem e são ignoradas — está assim de propósito, e o teste
-- `template_do_realm_bate_com_o_codigo` cobra a coerência do resto.
--
-- # O que ainda é palpite, e como corrigir
--
-- **As coordenadas de nascimento.** No servidor original elas vêm do `clsconfig` — um
-- personagem-molde por classe gravado no próprio banco (`GameDBManager::GetClsDetail`,
-- `cnet/gamedbd/gamedbmanager.cpp:404-460`) — e aquele arquivo é um banco binário que
-- ainda não foi decodificado. As três primeiras raças usam coordenadas que caem no chão;
-- Abissais, Guardiões e Sombrios usam palpites (ver o item 42e do ESTADO_E_RETOMADA).
--
-- Para corrigir uma: ande até o ponto certo em jogo e leia a posição do personagem,
--
--   SELECT pos_x, pos_y, pos_z FROM characters WHERE name = 'SeuPersonagem';
--
-- e grave-a para as duas classes daquela raça:
--
--   UPDATE class_templates SET spawn_x = ?, spawn_y = ?, spawn_z = ?
--    WHERE realm_id = 'realm_155' AND cls IN (4, 3);   -- Selvagens
--
-- Idempotente: apaga e regrava as doze linhas do realm.

BEGIN;

DELETE FROM class_template_items
 WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_155');
DELETE FROM class_template_skills
 WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_155');
DELETE FROM class_templates WHERE realm_id = 'realm_155';

INSERT INTO class_templates
    (realm_id, cls, name, initial_level, initial_cultivation, initial_money, initial_sp,
     strength, agility, vitality, energy, spawn_world_id, spawn_x, spawn_y, spawn_z)
VALUES
  -- Humanos — Vale das Espadas
  ('realm_155',  0, 'Guerreiro',    1, 0, 0, 0, 10, 10, 10, 10, 1,   976.0, 219.2,  4187.3),
  ('realm_155',  1, 'Mago',         1, 0, 0, 0, 10, 10, 10, 10, 1,   976.0, 219.2,  4187.3),
  -- Abissais — coordenada por confirmar
  ('realm_155',  2, 'Psíquico',     1, 0, 0, 0, 10, 10, 10, 10, 1,   650.0, 201.1,   130.0),
  ('realm_155',  5, 'Assassino',    1, 0, 0, 0, 10, 10, 10, 10, 1,   650.0, 201.1,   130.0),
  -- Selvagens — coordenada por confirmar (o Murillo caiu no "campo da expedição")
  ('realm_155',  3, 'Feiticeira',   1, 0, 0, 0, 10, 10, 10, 10, 1, -1445.6, 219.3,  2642.0),
  ('realm_155',  4, 'Bárbaro',      1, 0, 0, 0, 10, 10, 10, 10, 1, -1445.6, 219.3,  2642.0),
  -- Alados — Vale das Plumas
  ('realm_155',  6, 'Arqueiro',     1, 0, 0, 0, 10, 10, 10, 10, 1,  -741.5, 219.1, -1234.8),
  ('realm_155',  7, 'Sacerdote',    1, 0, 0, 0, 10, 10, 10, 10, 1,  -741.5, 219.1, -1234.8),
  -- Guardiões — coordenada por confirmar
  ('realm_155',  8, 'Guardião',     1, 0, 0, 0, 10, 10, 10, 10, 1,   380.0, 219.3,   230.0),
  ('realm_155',  9, 'Místico',      1, 0, 0, 0, 10, 10, 10, 10, 1,   380.0, 219.3,   230.0),
  -- Sombrios — coordenada por confirmar
  ('realm_155', 10, 'Ceifador',     1, 0, 0, 0, 10, 10, 10, 10, 1,   150.0, 210.1,   250.0),
  ('realm_155', 11, 'Tormentador',  1, 0, 0, 0, 10, 10, 10, 10, 1,   150.0, 210.1,   250.0);

-- A arma de cada classe, no slot 0 do equipamento.
--
-- `container_type = 1` é equipamento; o slot 0 é `EQUIPIVTR_WEAPON`. A durabilidade é a de
-- fábrica do `elements.data` — o servidor multiplica por 100 ao mandar ao cliente.
INSERT INTO class_template_items (template_id, container_type, slot, item_id, count, durability, max_durability)
SELECT t.id, 1, 0, a.item_id, 1, 2800, 2800
  FROM class_templates t
  JOIN (VALUES
        ( 0,  2097),   -- Guerreiro   Espada de Madeira
        ( 1,  2251),   -- Mago        Varinha
        ( 2, 26332),   -- Psíquico    Pequena Esfera
        ( 3,  2251),   -- Feiticeira  Varinha
        ( 4,  2258),   -- Bárbaro     Porrete com Espinhos
        ( 5, 26331),   -- Assassino   Faca de Limpar Osso
        ( 6,  2250),   -- Arqueiro    Arco de Madeira
        ( 7,  2251),   -- Sacerdote   Varinha
        ( 8,  2097),   -- Guardião    Espada de Madeira
        ( 9,  2251),   -- Místico     Varinha
        (10, 44937),   -- Ceifador    Sabre de Bronze
        (11, 45020)    -- Tormentador Foice de Ferro
       ) AS a(cls, item_id) ON a.cls = t.cls
 WHERE t.realm_id = 'realm_155';

-- O kit de bolsa, igual para todas as classes.
--
-- Só itens de `require_level = 0`: a Poção Perfeita de Cura (1801) que estava aqui exige
-- nível 30 e não servia a ninguém recém-criado.
INSERT INTO class_template_items (template_id, container_type, slot, item_id, count, durability, max_durability)
SELECT t.id, 0, k.slot, k.item_id, k.count, 0, 0
  FROM class_templates t
  JOIN (VALUES
        (0, 2100,  5),   -- Portal da Cidade   (TOWNSCROLL_ESSENCE)
        (1, 1796, 10),   -- Poção Pequena de Cura      (require_level 0)
        (2, 1804, 10)    -- Poção Pequena do Espírito  (require_level 0)
       ) AS k(slot, item_id, count) ON true
 WHERE t.realm_id = 'realm_155';

COMMIT;

-- Conferência rápida:
--
--   SELECT t.cls, t.name, i.container_type, i.slot, i.item_id, i.count
--     FROM class_templates t JOIN class_template_items i ON i.template_id = t.id
--    WHERE t.realm_id = 'realm_155' ORDER BY t.cls, i.container_type, i.slot;
