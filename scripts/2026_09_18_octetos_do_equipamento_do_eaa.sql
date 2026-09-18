-- O set Halo (e as outras peças de missão) do eaa nascem com o bloco de dados que o
-- original gera na entrega: `DeliverCommonItem` → `generate_item_for_drop`
-- (`task/taskman.cpp:281-303`), a mesma geração do drop de monstro.
--
-- As peças dele tinham sido criadas **sem** bloco (defeito corrigido no B60): o cliente
-- mostrava a faixa do modelo no tooltip ("Destreza +1~2") e o servidor não somava nada.
-- Os blocos abaixo saíram do gerador do próprio servidor:
--   cargo run -p pw-gs --example gerar_octetos -- data/realm_155/config <ids>
--
-- item 36121: durabilidade 50/50, propriedades: 753[2], 1109[1], 1312[22]
-- item 333: durabilidade 9/18, propriedades: nenhum
-- item 28830: durabilidade 95/95, propriedades: 1130[1], 652[8]
-- item 28819: durabilidade 36/36, propriedades: nenhum
-- item 28790: durabilidade 85/85, propriedades: 1130[1], 652[8]
-- item 28803: durabilidade 70/70, propriedades: 1140[2], 652[8]
-- item 28794: durabilidade 70/70, propriedades: 1140[1], 652[8]

BEGIN;
UPDATE character_items SET extra_data = decode('01004000050000000500000032000000320000002c000000010000000d0000000100000062210000550000009c00000000000000000000001e0000000000a041000000000000000003000000f12200000200000055240000010000002025000016000000','hex'), durability = 50, max_durability = 50
 WHERE character_id = 5491 AND item_id = 36121 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('0100ff0f05000000000000000900000012000000240000000000000000000000050000000000000000000000000000000000000000000000000000000000000000000000','hex'), durability = 9, max_durability = 18
 WHERE character_id = 5491 AND item_id = 333 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('0100600400000000000000005f0000005f00000024000000b9000000000000000000000000000000130100001301000013010000130100001301000000000000020000006a240000010000008c22000008000000','hex'), durability = 95, max_durability = 95
 WHERE character_id = 5491 AND item_id = 28830 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('0100600400000000000000002400000024000000240000000000000000000000170000000000000000000000000000000000000000000000000000000000000000000000','hex'), durability = 36, max_durability = 36
 WHERE character_id = 5491 AND item_id = 28819 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('010060040000000000000000550000005500000024000000a0000000000000000000000000000000eb000000eb000000eb000000eb000000eb00000000000000020000006a240000010000008c22000008000000','hex'), durability = 85, max_durability = 85
 WHERE character_id = 5491 AND item_id = 28790 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('0100600400000000000000004600000046000000240000006d0000000000000000000000000000009b0000009b0000009b0000009b0000009b000000000000000200000074240000020000008c22000008000000','hex'), durability = 70, max_durability = 70
 WHERE character_id = 5491 AND item_id = 28803 AND octet_length(coalesce(extra_data,'')) = 0;
UPDATE character_items SET extra_data = decode('010060040000000000000000460000004600000024000000540000000000000000000000000000007800000078000000780000007800000078000000000000000200000074240000010000008c22000008000000','hex'), durability = 70, max_durability = 70
 WHERE character_id = 5491 AND item_id = 28794 AND octet_length(coalesce(extra_data,'')) = 0;
COMMIT;

-- Conferência: nenhuma peça de equipamento sem bloco
-- SELECT slot, item_id, octet_length(extra_data) FROM character_items
--  WHERE character_id = 5491 AND container_type = 1 ORDER BY slot;
