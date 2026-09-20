-- B68 — o item de voo que a missão entregou sem bloco de dados.
--
-- "Glória de Shalim" (45782, `FLYSWORD_ESSENCE`) entrou na bolsa **sem conteúdo**, e o
-- cliente lê a máscara de classes (`character_combo_id`) de dentro dele
-- (`IVTR_ESSENCE_FLYSWORD`, `EC_IvtrTypes.h:269-280`): sem o bloco a máscara é zero e o
-- item não pode ser usado por classe nenhuma.
--
-- Os 30 bytes abaixo saíram do gerador do próprio servidor, que porta o
-- `generate_flysword` (`gs/template/generate_item_temp.h:1126-1165`):
--   `cargo run -p pw-gs --example octetos_do_voo -- data/realm_155/config 45782`
-- Decodificados: cur_time 450, max_time 900, nível exigido 1, level 1, refino 0,
-- classes 192, +15 s por elemento, velocidade +1,0 e arrancada +2,0.
--
-- Aplicado uma vez, em 2026-09-20.

UPDATE character_items
   SET extra_data = decode('c20100008403000001000100c00000000f0000000000803f000000400000', 'hex'),
       updated_at = CURRENT_TIMESTAMP
 WHERE character_id = 5491 AND item_id = 45782 AND (extra_data IS NULL OR octet_length(extra_data) = 0);

SELECT slot, item_id, octet_length(extra_data) AS bloco
  FROM character_items WHERE character_id = 5491 AND item_id = 45782;
