-- O WRA (roleid 11457, Guerreiro do realm_126) nasceu antes dos moldes do `clsconfig` 1.2.6
-- (`2026_09_24_moldes_do_clsconfig_126.sql`, que tem de rodar antes deste): ficou com a
-- configuração vazia que o cliente salvou (barras sem ataque básico nem habilidades) e na
-- posição antiga dos humanos. Recebe a configuração e o ponto de nascimento do molde da classe
-- 0 — o que o `gamedbd` teria copiado na criação. Mesmo conserto do eaa no 1.5.5
-- (`2026_09_17_configuracao_do_eaa_pelo_molde.sql`).

BEGIN;

UPDATE character_client_config
   SET ui_config = (SELECT ui_config FROM class_templates WHERE realm_id = 'realm_126' AND cls = 0),
       updated_at = CURRENT_TIMESTAMP
 WHERE character_id = 11457
   AND (SELECT ui_config FROM class_templates WHERE realm_id = 'realm_126' AND cls = 0) IS NOT NULL;

UPDATE characters AS c
   SET world_id = t.spawn_world_id, pos_x = t.spawn_x, pos_y = t.spawn_y, pos_z = t.spawn_z
  FROM class_templates AS t
 WHERE c.id = 11457 AND t.realm_id = 'realm_126' AND t.cls = 0 AND c.level = 1;

COMMIT;
