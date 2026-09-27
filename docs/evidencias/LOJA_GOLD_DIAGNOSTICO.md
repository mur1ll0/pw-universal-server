# Loja Gold (gshop / `MALL_SHOPPING`) — diagnóstico de 2026-09-26

> **Implementado no B125** (mesmo dia). O plano do fim foi seguido; o estado atual está na
> spec 05 §8 (linha "Loja Gold") e na spec 03 §3.8. Este arquivo fica como a evidência.

Relato do Murillo (Tsuko, realm_126): comprar item de voo na Loja Gold não faz nada.

## Estado no código

- Não implementada em **nenhum** realm. C2S 106 `MALL_SHOPPING` removido no A51
  (`crates/pw-link/src/gateway.rs:1177`); o `pw-gs` não o trata.
- `crates/pw-data-loader/src/gshop.rs` **não lê o formato real** (registro de 24 B inventado);
  só o carimbo (1º `u32`) é usado.
- `PLAYER_CASH` (253) é mandado com o **dinheiro do personagem** (`world.dinheiro`) em
  `bus_server.rs` ~3645, ~3658, ~3917 — deveria ser o cash da conta. A conta tem
  `accounts.gold_balance` (Tsuko/admin = 1.000.000); só o `pw-auth` usa.
- `S2CGamedataSend::mall_item_buy_failed(i32)` escreve 4 B; a struct é
  `{short index; char reason}` = 3 B (IR, `gs/player.cpp:5773-5777`). Está em `LAYOUT_DIVERGE`.

## Arquivo (juiz: carregador do cliente, fecha no último byte)

`u32 timestamp, i32 n, n × GSHOP_ITEM, 8 × {wchar[64], i32 nsub, nsub × wchar[64]}`
(`EvolvedPWClient/ElementClient/CCommon/globaldataman.cpp:614-660`).

| realm | arquivo | n | registro |
| :--- | :--- | ---: | ---: |
| 155 | `gshop.data` (= cliente BR, byte a byte) | 1726 | 1436 (layout VIP de `globaldataman.h:101-138`) |
| 155 | `gshop1.data` / `gshop2.data` | 205 / 6 | 1436 |
| 126 | `gshop.data` (= cliente 1.2.6 e = `files1.2.6/pwserver/gamed/config`) | 668 | 1288 |

- 1.5.5, 1436 B: `local_id@0 main@4 sub@8 icon[128]@12 id@140 num@144`,
  `buy[4]` de 36 B em 148 `{price,end_time,time,start_time,i32 type,day,status,flag,min_vip}`,
  `desc wchar[512]@292 name wchar[32]@1316 idGift@1380 iGiftNum iGiftTime iLogPrice`,
  `owner_npcs[8]@1396 buy_times_limit@1428 mode@1432`.
  Medido: `type` sempre −1, `flag` sempre 0, 55 com brinde, 37 exigem VIP (1–6),
  20 com limite (mode 3), nenhum com dono.
- 1.2.6, 1288 B: mesmo cabeçalho até `num@144`; `buy[4]` de **12 B** em 148
  `{price, data_absoluta, time}`; `u32 status@196`; `desc@200 name@1224`. Sem brinde/VIP/limite.
  `load_malldata` do `gs` 1.2.6 (VA 0x81e877c): `cash_need=price`; `data_absoluta≠0` →
  validade absoluta, senão `time` relativo.
- `gshopsev.data` do realm 155 (2262 ofertas, 2023) **não** bate com a lista do cliente —
  o mall tem de sair do `gshop.data` do cliente (o índice que o cliente manda é dele).
  O `gs` 1.2.6 original lê o próprio `gshop.data` do cliente.
- O cliente 1.2.6 só abre `Data\gshop.data` (string no `elementclient.exe`); o `gshop2.data`
  do realm_126 não é usado por ele.

## Protocolo

| | 1.5.5 | 1.2.6 (`gs` 1.2.6) |
| :--- | :--- | :--- |
| C2S | 106, `u32 count` + `count × {i32 goods_id, i32 goods_index, i32 goods_slot}`; `size == 6+12·count` (`playercmd.cpp:3299-3318`); só `list[0]` é comprado | 106 (tabela de salto 0x84f4ce4), `u32 count` + `count × {i16 goods_id, i16 index, i16 slot}`; `size == 6+6·count` (VA 0x80d1631); `PlayerDoShopping(unsigned, const short*)`; id comparado com sinal (`movsx`) |
| regra | `gplayer_imp::PlayerDoShopping`, `player.cpp:15709-16010` | VA 0x807f8e0: estado, bolsa, slot ≤ 3, `QueryGoods`+id, `cash_need>0`, saldo |
| erros | 7 bolsa cheia, 94 pedido inválido, 16 sem saldo, 161 proibido, 226 VIP | 7, 0x5e=94, 0x10=16 |
| resposta | `obtain_item(id, 0, n, no_slot, 0, slot)` por item (e brinde), depois `player_cash(saldo)` | igual (sem brinde) |

Validade: 1.5.5 relativa `agora+time`; o `obtain_item` leva o `expire_date` do **molde** (0).
A bolsa do `pw-gs` não guarda validade — fica `falta`. Item: `get_item_for_sell`
(`ADDON_LIST_SHOP`, `itemdataman.cpp:1337-1400`); o `pw-gs` não tem essa variante — usar
`empilhar_gerado` para voo/ovo e registrar a de equipamento como `falta`.

## Plano (próxima sessão)

1. `gshop.rs`: leitor real por versão (1436/1288), fechando no último byte; `Vec` por índice.
2. `comandos.rs`: `MallShopping` com a entrada de 12 B (155) ou 6 B (126) — diferença no
   `WorldProtocol` do v126.
3. `pw-storage`: `gastar_cash(account_id, n)` atômico
   (`UPDATE ... SET gold_balance = gold_balance - $1 WHERE id=$2 AND gold_balance >= $1 RETURNING`).
4. `pw-gs`: `comprar_na_loja_gold` portando as conferências acima; `PLAYER_CASH` passa a
   mandar `gold_balance` nos três lugares.
5. Corrigir `mall_item_buy_failed(i16, u8)` e tirá-lo do `LAYOUT_DIVERGE`.
6. Testes: leitor dos dois arquivos, C2S dos dois tamanhos, compra no mundo; specs 03, 04, 05.
