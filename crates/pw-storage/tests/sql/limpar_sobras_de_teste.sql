-- Apaga o que os testes de integração deixaram no banco em execuções anteriores.
--
-- Os testes que precisam de PostgreSQL (`TEST_DATABASE_URL`) criam realms `t_*`, contas e
-- personagens de verdade, porque o que eles conferem mora em chaves estrangeiras e
-- colunas reais. Até 2026-09-14 nada disso era apagado: o banco local chegou a 3.129 realms
-- de teste, 3.129 contas, 1.327 personagens e 18.774 moldes de classe.
--
-- Cada arquivo de teste roda isto uma vez por processo, antes de criar os próprios dados
-- (`include_str!` deste arquivo em `pw-storage/tests` e `pw-gs/tests`). Não dá para apagar
-- no fim de cada teste — um teste que falha não chega ao fim, e é justamente o que mais
-- deixaria sobra.
--
-- # O que é "de teste"
--
-- Só o que os testes criam, pelos nomes exatos que eles geram:
--
--   realm   t_gs_*, t_at_*, t_it_*, t_a_*, t_b_*    (e qualquer outro t_*)
--   conta   gs_<n>_<n>, at_<n>_<n>, it_<n>_<n>, teste_a_<n>_<n>, teste_b_<n>_<n>
--
-- Conferido em 2026-09-14 antes da primeira limpeza: nenhum personagem de conta real em
-- realm t_*, nenhuma conta de teste com personagem fora de t_*, nenhuma facção, correio ou
-- registro de auditoria ligado a eles.
--
-- # A margem de 15 minutos
--
-- Dois `cargo test` podem rodar ao mesmo tempo (duas sessões, um terminal). Apagar o que
-- outro processo acabou de criar derrubaria os testes dele. Nada que tenha menos de 15
-- minutos é tocado, e a suíte inteira leva bem menos que isso.
--
-- # Ordem
--
-- `characters.account_id` apaga em cascata (e com ele itens, habilidades e missões),
-- `characters.realm_id` **não** — por isso as contas vão primeiro, e o realm só sai quando
-- não sobrou personagem nele. `class_templates` sai em cascata com o realm.
--

DELETE FROM accounts
 WHERE (username ~ '^(gs|at|it)_[0-9]+(_[0-9]+)?$'
        OR username ~ '^teste_[ab]_[0-9]+(_[0-9]+)?$')
   AND created_at < now() - interval '15 minutes';

DELETE FROM realms r
 WHERE r.id LIKE 't\_%'
   AND r.created_at < now() - interval '15 minutes'
   AND NOT EXISTS (SELECT 1 FROM characters c WHERE c.realm_id = r.id);
