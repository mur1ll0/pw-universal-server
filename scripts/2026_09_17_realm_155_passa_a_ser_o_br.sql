-- Um realm por versão: o 1.5.5 fica só com os dados do cliente BR, sob o nome `realm_155`.
--
-- Até 2026-09-17 havia dois realms 1.5.5: `realm_155` (dados do cliente EN, elements v159,
-- porta 29003, fora dos testes desde 2026-09-05) e `realm_155BR` (cliente BR, elements v156,
-- porta 29004, o realm de teste). Pedido do Murillo: apagar o EN e renomear o BR para
-- `realm_155`, mantendo a porta **29004** (o `serverlist.txt` do cliente BR não muda).
--
-- As chaves estrangeiras para `realms(id)` não têm `ON UPDATE CASCADE`, então a troca é:
-- apagar o EN (moldes caem em cascata; ele não tinha personagem), criar a linha nova com os
-- valores do BR, repontar os filhos e apagar a linha antiga. Tudo numa transação.
--
-- **Aplicado uma vez, em 2026-09-17. Não reaplicar:** depois dele `realm_155` já é o BR, e o
-- passo 1 tentaria apagá-lo (a transação aborta pelo RESTRICT dos personagens, mas não conte
-- com isso num banco sem personagem).
--
-- Backup anterior: `data/_backups/pw_database_2026-09-17_antes_de_renomear_realm_155.sql`.

BEGIN;

-- 1. O realm EN sai. `characters` e `factions` são RESTRICT: se houver algo, a transação falha.
DELETE FROM mails            WHERE realm_id = 'realm_155';
DELETE FROM admin_audit_logs WHERE realm_id = 'realm_155';
DELETE FROM class_template_items  WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_155');
DELETE FROM class_template_skills WHERE template_id IN (SELECT id FROM class_templates WHERE realm_id = 'realm_155');
DELETE FROM class_templates  WHERE realm_id = 'realm_155';
DELETE FROM realms           WHERE id = 'realm_155';

-- 2. A linha nova, com a configuração do BR.
INSERT INTO realms (id, name, version, host, port, is_online, max_players,
                    double_exp_multiplier, double_sp_multiplier, double_drop_multiplier,
                    double_gold_multiplier, config, created_at)
SELECT 'realm_155', 'Perfect World Evolved (1.5.5)', version, host, port, is_online, max_players,
       double_exp_multiplier, double_sp_multiplier, double_drop_multiplier,
       double_gold_multiplier, config, created_at
  FROM realms WHERE id = 'realm_155BR';

-- 3. Os filhos passam para o nome novo.
UPDATE characters       SET realm_id = 'realm_155' WHERE realm_id = 'realm_155BR';
UPDATE factions         SET realm_id = 'realm_155' WHERE realm_id = 'realm_155BR';
UPDATE mails            SET realm_id = 'realm_155' WHERE realm_id = 'realm_155BR';
UPDATE class_templates  SET realm_id = 'realm_155' WHERE realm_id = 'realm_155BR';
UPDATE admin_audit_logs SET realm_id = 'realm_155' WHERE realm_id = 'realm_155BR';

-- 4. A linha antiga sai.
DELETE FROM realms WHERE id = 'realm_155BR';

COMMIT;

-- Conferência:
-- SELECT id, name, port FROM realms WHERE id LIKE 'realm_155%';
-- SELECT realm_id, count(*) FROM class_templates WHERE realm_id LIKE 'realm_155%' GROUP BY 1;  -- 12
-- SELECT id, name, realm_id FROM characters WHERE realm_id LIKE 'realm_155%';                  -- POTATO, eaa
