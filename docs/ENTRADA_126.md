# Entrada no mundo 1.2.6 — B74

Base `2dca19e`, árvore `../pw-126`, sem publicação.

| comando | captura / validação do cliente | resultado local |
|---|---|---|
| SELF_INFO_00 38 | `evidencias/126/s2c-38.txt:1`; cliente VA 0x5849bf retorna 36 | amostra reproduzida byte a byte |
| OWN_EXT_PROP 50 | `s2c-50.txt:1`; VA 0x584997 retorna 152 | offsets dos campos recebidos pelo trait conferidos; ataque mágico/resistências continuam zeros, como na base |
| OWN_IVTR_DATA 42 | `s2c-42.txt:1`; VA 0x584951 valida 6+len | bolsa de missão vazia de 32 slots reproduzida; amostra não prova bolsa preenchida |
| OWN_ITEM_INFO 40 | `s2c-40.txt:1`; VA 0x58490e valida 22+u16(offset20) | envelope compatível; conteúdo de todos os tipos de item não auditado |
| SKILL_DATA 90 | `s2c-90.txt:1`; VA 0x584b08 valida 4+5×count | fórmula comum compatível |
| TASK_DATA 105 | `s2c-105.txt:1`; VA 0x584b24 percorre três blocos | envelope v126 compatível; estruturas internas das missões pertencem à camada seguinte |
| EQUIP_DATA 66 | `s2c-66.txt:8`; VA 0x584a1d valida 10+4×popcount(mask32) | corrigido override v126; eram 14+4×n, quatro bytes extras |
| GetUIConfig_Re GNET 105 | `gnet-ui-105.txt:1`: 323 bytes, blob 309 bytes, versão 3 + zlib | resposta reproduzida integralmente; barras salvas devolvidas sem reescrever versão |
| SERVER_TIME 114 | `s2c-114.txt:2`: lua_version=102 | valor atual confirmado, sem correção |

O binário e seu SHA256 estão em `evidencias/126/cliente-validacao-entrada.txt:1`.
Seu dispatcher em VA 0x584610 rejeita ids acima de 260 (`cmp eax,0x104`).
Isso comprova que 390 e os seis avisos tardios de entrada não pertencem ao cliente.

## Correções

- `V126Protocol::equip_data` escreve máscara de 32 bits. Os itens de slots maiores
  que 31, ordenados pelo mundo, não são anexados ao pacote. O trait já existia.
- `WorldProtocol::scene_service_npc_list` é opcional: padrão Some preserva o 155;
  v126 devolve None. O mundo apenas trata a opção.
- `WorldProtocol::initial_status_notifications` preserva como padrão os 14 avisos
  anteriores do link, na mesma ordem. V126 fornece oito avisos suportados.
  Não há `if versão` no mundo/link nem mudança de regra de jogo.

## Provas e limites

Os testes novos falharam antes das correções (logs `equip-data-antes.log`,
`equip-data-limite-antes.log`, `scene-390-antes.log`, `avisos-antes.log`).
`entrada-depois.log`: 19 testes de layouts aprovados, zero falhas, incluindo
regressões 155 de máscara de 64 bits, lista 390 e sequência de avisos.
Fechamento em 2026-09-21: `camada2-focado.log:17`, **11 aprovados, zero falhas,
8 filtrados**, com `TEST_DATABASE_URL` definido. Filtros: `entrada_`, `equip_data_`,
`own_ext_prop_tem`, `o_126_nao_anuncia`, `o_155_continua`.

A suíte ampla iniciada antes da restrição do usuário terminou com uma falha:
`aceitar_e_entregar_missao_no_npc_mexe_nas_listas_e_premia`,
`subcomandos_no_mundo.rs:1807`, listas não gravadas (63/64 naquele arquivo).
Não repetida nem investigada nesta retomada. Não há aprovação da suíte completa.
Escopo fechado para revisão local: somente Camada 2; Camadas 3/4 não iniciadas.

Não há relato novo de teste em jogo. O sintoma previsto do EQUIP_DATA antigo é
rejeição do pacote e equipamento/modelo de outro jogador incompleto; a causa é
comprovada no validador, mas o efeito visual ainda precisa de teste.

## Roteiro quando houver publicação autorizada

1. Entrar com um personagem 126: HP/MP e bolsa devem aparecer. Abrir C e conferir
   atributos. A ausência de erro no overlay não prova valores de ataque mágico.
2. Arrastar item/habilidade para a barra, relogar e conferir a posição.
3. Com dois clientes 126, aproximar personagens equipados e verificar os modelos.
4. No overlay (`##debug`, Shift+tecla à esquerda do 1, `d_rtdebug 1`), não deve
   aparecer Invalid EQUIP_DATA nem Unknown GAMEDATA_390/avisos acima de 260.

Logs estreitos (PowerShell):

```powershell
docker logs --since 5m pw-world-126 2>&1 | Select-String 'get_other_equip|equip_data|GET_ALL_DATA|TASK_DATA'
docker logs --since 5m pw-realm-126 2>&1 | Select-String 'GetUIConfig|SetUIConfig|configuração|TASK_DATA'
```

Não publicar nem reiniciar sem pedido.

## Subir esta worktree após aprovação

Em `F:/Python_C_Projects/PWSource1.5.3/pw-126`, sem trocar a branch da árvore
original. Projeto Compose existente conferido: `docker`. Os serviços compartilhados
já estão ativos, e `data` nesta worktree é uma junction para os dados existentes.
Reconstruir ambos, pois o link e o mundo usam os codificadores alterados:

```powershell
docker compose -p docker -f docker/docker-compose.yml build pw-world-126 pw-realm-126 *> build-126.log
if ($LASTEXITCODE -ne 0) { Get-Content build-126.log -Tail 20; throw 'Build 126 falhou' }
docker compose -p docker -f docker/docker-compose.yml up -d --no-deps pw-world-126 pw-realm-126
```

O cliente continua na porta 29000. Os comandos selecionam somente o 126;
não usar `down`, `--remove-orphans` ou `up` sem nomes de serviço nesta operação.
