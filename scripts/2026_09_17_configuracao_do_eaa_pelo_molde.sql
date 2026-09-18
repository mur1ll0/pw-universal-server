-- O eaa (5491) só tem gravada a configuração vazia que o cliente salvou enquanto o
-- GetUIConfig_Re chegava corrompido (B53): barras vazias e bTraceAll = 0. Volta à do molde
-- do Arqueiro (barra com 235, 234 e 167; rastreador ligado).
UPDATE character_client_config
   SET ui_config = (SELECT ui_config FROM class_templates WHERE realm_id = 'realm_155' AND cls = 6),
       updated_at = CURRENT_TIMESTAMP
 WHERE character_id = 5491;
