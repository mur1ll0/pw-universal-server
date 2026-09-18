-- Corrige os atributos de personagens que nasceram com 10/10/10/10.
--
-- Até 2026-09-09 o `create_character` não mencionava as colunas `strength`, `agility`,
-- `vitality` e `energy` no `INSERT`, e o banco usava o `DEFAULT 10` do esquema
-- (`specs/01_DATABASE_SCHEMA_POSTGRES.sql:103-106`) para toda classe. O original tira
-- esses quatro valores do `ptemplate.conf`, por classe.
--
-- O código já grava o valor certo em personagem novo. Este script é para os que já
-- existem.
--
-- # A cláusula que protege quem já jogou
--
-- Só toca em personagem que **ainda está no ponto de partida**: nível 1, sem pontos
-- livres guardados, e com os quatro atributos exatamente em 10. Quem distribuiu pontos
-- pode ter chegado a 10 em algum deles de propósito, e sobrescrever isso seria roubar
-- progresso. Um personagem de nível 1, com 10/10/10/10 e nenhum ponto livre, não
-- distribuiu nada — é o estado que só o defeito produz.
--
-- Idempotente: rodar de novo não muda mais nada, porque os corrigidos deixam de casar
-- com o `WHERE`.
--
-- # De onde saem os números
--
-- `data/realm_155/config/ptemplate.conf`, um `[SECAO]` por classe, campos `strength`,
-- `agility`, `vitality` e `energy`, lidos em 2026-09-09. A ordem das seções **é** o
-- `character_class_id` (0 a 11) — é a mesma ordem que o `pw_data_loader::ptemplate` usa
-- para indexar o arquivo (`SECOES`, `ptemplate.rs:38-41`), e a mesma do `CharacterClass`
-- do `pw-core` (`types.rs:47-60`). Os nomes das seções são os do servidor chinês e não
-- batem com os nomes ocidentais das classes: [ANGEL] é o Sacerdote (`Cleric`, id 7), e
-- [NEC] é o `Psychomancer` (id 2) — o par de colunas abaixo é o que vale, não o nome.
--
-- Confira o alcance antes de rodar:
--
--   SELECT cls, count(*) FROM characters
--    WHERE level = 1 AND potential_points = 0
--      AND strength = 10 AND agility = 10 AND vitality = 10 AND energy = 10
--    GROUP BY cls ORDER BY cls;
--
-- Cuidado: os valores abaixo são os deste realm. Outro realm pode ter outro
-- `ptemplate.conf`, e o script não os lê do arquivo — extraia de novo se for rodar
-- noutro lugar.

BEGIN;

UPDATE characters c
   SET strength  = t.forca,
       agility   = t.agilidade,
       vitality  = t.vitalidade,
       energy    = t.energia
  FROM (VALUES
        -- cls  força  agi  vit  energia      seção do ptemplate.conf / classe
        ( 0,    15,    10,  20,   5),      -- [SWORDSMAN]  Blademaster  (Guerreiro)
        ( 1,    10,    10,  10,  20),      -- [MAGE]       Wizard       (Mago)
        ( 2,    10,    10,  15,  15),      -- [NEC]        Psychomancer
        ( 3,    15,     5,  15,  15),      -- [HAG]        Venomancer   (Feiticeira)
        ( 4,    15,     5,  25,   5),      -- [ORGE]       Barbarian    (Bárbaro)
        ( 5,     5,    15,   8,  22),      -- [ASN]        Assassin
        ( 6,     5,    10,  15,  20),      -- [ARCHER]     Archer       (Arqueiro)
        ( 7,    10,    10,  10,  20),      -- [ANGEL]      Cleric       (Sacerdote)
        ( 8,    10,    10,  10,  20),      -- [BLADE]      Seeker
        ( 9,    10,    10,  10,  20),      -- [GENIE]      Mystic
        (10,    10,    10,  10,  20),      -- [SHADOW]     Duskblade
        (11,    10,    10,  10,  20)       -- [FAIRY]      Stormbringer
       ) AS t(cls, forca, agilidade, vitalidade, energia)
 WHERE c.cls = t.cls
   AND c.level = 1
   AND c.potential_points = 0
   AND c.strength = 10 AND c.agility = 10 AND c.vitality = 10 AND c.energy = 10;

COMMIT;

-- Nota sobre as quatro últimas linhas: [BLADE], [GENIE], [SHADOW] e [FAIRY] trazem
-- 10/10/10/20 neste arquivo, exatamente o mesmo que [MAGE] e [ANGEL]. Não é engano de
-- transcrição — é o que o arquivo diz. As classes novas do 1.5.x parecem ter sido
-- copiadas do molde do Mago e nunca ajustadas neste `ptemplate.conf`.
