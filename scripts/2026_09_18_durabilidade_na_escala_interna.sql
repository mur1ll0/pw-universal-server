-- B61 — a durabilidade dos itens já gerados, para a escala interna.
--
-- O original multiplica a durabilidade pelo `DURABILITY_UNIT_COUNT` (100) no fim da
-- geração do item (`update_require_data`, `gs/item/item_addon.h:454-458`, chamado em
-- `gs/template/generate_item_temp.h:367`), e o cliente divide de volta arredondando para
-- cima (`EC_IvtrEquip.cpp:281`). Nossa geração gravava o número do `elements.data` cru, e
-- por isso o ★Arco Real (50) aparecia como **1/1** em jogo (relato de 2026-09-18).
--
-- Os itens vindos de `class_template_items` já estavam certos (2800 = 28 na tela); os
-- gerados por nós ficaram 100 vezes menores. O corte em 100 separa os dois casos: nenhum
-- equipamento do `elements.data` tem `durability_min` de 100 ou mais.
--
-- O bloco `extra_data` não precisa de conserto: desde o B61 o `item_info` regrava a
-- durabilidade do bloco com a da coluna antes de mandar (`BusServer::info_de`).
--
-- Aplicado uma vez, em 2026-09-18. Não reaplicar: multiplicaria de novo.

BEGIN;

SELECT count(*) AS itens_na_escala_do_arquivo
  FROM character_items
 WHERE max_durability > 0 AND max_durability < 100;

UPDATE character_items
   SET durability = durability * 100,
       max_durability = max_durability * 100,
       updated_at = CURRENT_TIMESTAMP
 WHERE max_durability > 0 AND max_durability < 100;

COMMIT;
