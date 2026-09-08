-- Acrescenta `characters.last_login_at`.
--
-- Duas coisas dependiam dela:
--
--   * O autosave escrevia `last_login_at = CURRENT_TIMESTAMP` numa tabela que não tinha a
--     coluna. O `UPDATE` falhava inteiro, o erro era engolido por um `let _ =` e o log
--     dizia "executado com sucesso" — nada de posição, experiência, dinheiro ou nível era
--     gravado, desde sempre.
--   * O campo `lastlogin_time` do `RoleInfo` ia zerado para todos os personagens, e é ele
--     que o cliente usa para escolher qual vem selecionado na tela de seleção
--     (`EC_LoginUIMan.cpp:809-818`, fica com o de maior valor). Com zero em todos, caía
--     sempre no primeiro da lista.
--
-- Idempotente: pode rodar de novo sem estragar nada.

ALTER TABLE characters ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMP WITH TIME ZONE;
