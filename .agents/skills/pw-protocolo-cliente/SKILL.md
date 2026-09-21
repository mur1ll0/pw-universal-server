---
name: pw-protocolo-cliente
description: Criar, corrigir ou diagnosticar um subcomando do mundo 3D (S2C ou C2S do GamedataSend) ou pacote GNET no pw-universal-server, conferindo o layout contra IR, EC_GPDataType.h e o overlay do cliente. Usar quando algo "não aparece na tela", ao escrever codificador/decodificador novo, ou ao mexer em `versions/`.
---

# Layout de protocolo sem descarte silencioso

O cliente descarta inteiro todo S2C cujo tamanho não é o `sizeof` que ele espera, sem erro em
log nenhum. Regra geral e fatos já medidos: `specs/04_PROTOCOLO_MUNDO_3D.md` — leia antes.

## 1. Levantar o layout pelas três fontes

```bash
python tools/pw-ir/consultar_ir.py s2c <NOME|id>       # ou c2s / struct
```

1. **IR** (`specs/protocol/gamedata_155.json`): campos, deslocamentos, tamanho-base.
2. **Fonte do cliente**: `F:\PW\1.5.5\EvolvedPWClient\ElementClient\Network\EC_GPDataType.h`
   (tudo dentro do `#pragma pack(1)` da linha 563) e o tratador em `EC_GameDataPrtc.cpp` /
   `EC_HostMsg.cpp` / `EC_ManPlayer.cpp` — o que o cliente **faz** com cada campo.
3. **Servidor original**: `F:\PW\1.5.5\EvolvedPWServer\cgame\common\protocol.h` e quem monta o
   comando em `cgame/gs/` (valores e unidades: ms, ×256, ticks de 50 ms, bits de `state`).

Discordância entre as três: **o binário decide** — só o overlay em jogo mostra o tamanho real
(ex.: `OWN_EXT_PROP` = 196, nem o 188 do IR nem o 228 do fonte). Diga ao Murillo que é
preciso medir.

Campos condicionais a bits de `state`/`state2`: mande os bits zerados salvo evidência, e o
tamanho vira o tamanho-base.

## 2. Escrever

- Codificador S2C em `crates/pw-protocol/src/packets/s2c.rs` (`S2CGamedataSend::*`), com o
  comentário citando a struct e a linha do `EC_GPDataType.h`.
- Diferença **medida** entre versões: variante na estratégia da versão (`crates/pw-protocol/src/versions/<versao>/`); nunca
  um segundo codificador paralelo.
- Decodificador C2S em `crates/pw-gs/src/comandos.rs` (`ids::*` + struct com `BYTES`), tratamento
  em `BusServer::tratar_subcomando`. Não criar braço novo no `gateway.rs` do `pw-link` — e se
  migrar um de lá, **remover** o braço antigo — `os_comandos_ja_migrados_nao_sobraram_no_gateway`
  (`pw-link/tests`) falha se ficar nos dois lados.

## 3. Travar com teste

| mudou | teste |
| :--- | :--- |
| S2C | `crates/pw-protocol/tests/subcomandos_s2c_contra_o_ir.rs`: entrada na tabela `INTENCAO`; se diverge do IR de propósito, `LAYOUT_DIVERGE` com o porquê |
| campo a campo | `crates/pw-protocol/tests/protocol_tests.rs` (bytes esperados escritos à mão a partir da struct) |
| variante 1.2.6 | `crates/pw-protocol/tests/layouts_do_126.rs` |
| C2S | `crates/pw-gs/tests/comandos_contra_o_ir.rs` (payload montado do IR) |
| efeito no mundo | `crates/pw-gs/tests/subcomandos_no_mundo.rs` |

Rodar com o banco (skill `pw-testar-e-publicar`).

## 4. Roteiro de verificação em jogo para o Murillo

- `##debug` no chat → `Shift` + tecla à esquerda do "1" → `d_rtdebug 1`.
- Se o comando for **recusado** aparece `SERVER - Invalid <NOME> size(Network:a, Client:b)`
  (o `Client:` é o tamanho que o binário quer) ou `Unknown GAMEDATA_n`.
- **Comando aceito não aparece no overlay** — silêncio é o esperado. A prova de aceite é o
  efeito na tela (e o log do realm).
- Diga o que deve acontecer na tela e em que ordem.

## 5. Fechar

Atualize `specs/04_PROTOCOLO_MUNDO_3D.md` (§4, §5 ou a tabela §6 de onde cada C2S é tratado)
pela skill `pw-atualizar-specs`.
