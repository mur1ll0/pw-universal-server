# Prompt para a próxima sessão

Gerado em 2026-09-09, ao fim da sequência de sessões documentada nos itens 32–40 do
`docs/ESTADO_E_RETOMADA.md`. Copie o bloco abaixo inteiro como primeira mensagem.

---

Estou continuando o **pw-universal-server** (reimplementação em Rust do servidor de
Perfect World), em `F:\Python_C_Projects\PWSource1.5.3\pw-universal-server`, branch
`feat/aipolicy-reader`, último commit `83cca34`.

**Leia primeiro o item 40 do `docs/ESTADO_E_RETOMADA.md`.** Ele tem o estado atual e a fila
de trabalho, com a evidência já levantada para cada item — não reinvestigue o que já está
medido lá.

## Como este projeto trabalha

- **Evidência, nunca palpite.** Toda decisão de layout ou de número sai do fonte do cliente
  (`F:\PW\1.5.5\EvolvedPWClient`), do fonte do servidor (`F:\PW\1.5.5\EvolvedPWServer`), do
  IR (`specs/protocol/gamedata_153.json`) ou do `elements.data` do realm. Quando as fontes
  discordam, **quem manda é o binário do cliente** — ver a memória
  `pw_client_155_fonte_vs_binario`.
- **O overlay do cliente é o instrumento.** No jogo: `##debug` no chat, depois `Shift` + a
  tecla à esquerda do "1" para abrir a janela, e `d_rtdebug 1`. Ele imprime todo comando
  que o cliente descarta, com o tamanho que ele esperava. Um comando com tamanho errado não
  gera erro em log nenhum dos dois lados — foi o que escondeu quatro defeitos seguidos.
- **A suíte só testa de verdade com o banco:**
  ```bash
  TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
    cargo test --workspace
  ```
  Sem a variável os testes de integração **passam sem verificar nada**. Referência:
  67 suítes verdes, e as únicas falhas legítimas são as duas do 1.2.6 no `loader_tests`.
- **Publicar no realm de teste:**
  ```bash
  cd docker && docker compose build pw-world-155br pw-realm-155br \
    && docker compose up -d pw-world-155br pw-realm-155br
  ```
- Documente o que fizer como item numerado no `docs/ESTADO_E_RETOMADA.md`, no mesmo estilo
  dos itens 32–40: o sintoma, a causa com a referência ao fonte, a correção, e o que
  **continua** faltando. O idioma do projeto é português.

## O que fazer, em ordem

**1. A armadura vai aparecer vermelha — corrija antes de aparecer.**

É o mesmo defeito da Varinha (item 38), esperando a primeira peça de armadura. Hoje só
**arma** recebe bloco de dados no `OWN_ITEM_INFO`; armadura vai sem bloco, e o cliente cai
no `CECIvtrArmor::DefaultInfo` (`EC_IvtrArmor.cpp:188-196`), que **não** preenche
`m_iProfReq` — fica zero, e `CanUseEquipment` faz
`!(GetProfessionRequirement() & (1 << profissão))`, que recusa todas as classes.

Repita o que `crates/pw-data-loader/src/armas.rs` faz, para `ARMOR_ESSENCE` (e depois
`DECORATION_ESSENCE`). Atenção ao `id_sub_type`: é por ele que o cliente sabe em que slot a
peça entra.

**2. A visibilidade entre jogadores ainda é de uma vez só.**

O item 39 resolveu isso para NPC e monstro. Para jogador continua como estava: o
`gateway.rs` manda `PLAYER_ENTER_WORLD` mútuo no login e `PLAYER_LEAVE_WORLD` no logout,
sem raio e sem streaming. Dois jogadores que se afastam somem um para o outro para sempre.

Mova para `BusServer::atualizar_visiveis`, que já tem a grade espacial e já roda a cada
movimento. **Jogador entra com `PLAYER_ENTER_SLICE` (12)**, não com `NPC_ENTER_SLICE` (11)
— o cliente roteia pelo id e o comando errado joga o jogador no gerente de NPCs. O
`OBJECT_LEAVE_SLICE` (13) serve para os dois.

**3. Matéria: minério, ervas, os "recursos do mapa".**

Ninguém manda `MATTER_ENTER_WORLD` (18), nem no login nem no streaming. O `npcgen.data` já
traz as instâncias (`SpawnType::Matter`, `npcgen.rs:424`).

**4. O nível da habilidade é fixo em 1** (`NIVEL_DA_HABILIDADE`, `bus_server.rs`). O
`character_skills` guarda o nível; o `CastSkill` do cliente não o manda, quem deve saber é o
servidor.

**5. Personagem novo nasce com 10/10/10/10**, mas o `ptemplate.conf` dá os atributos por
classe. Os já existentes precisariam de migração.

Se algo nesta lista se mostrar errado ao ser investigado, **diga isso e corrija a lista** —
ela foi escrita com a evidência de 2026-09-09, não é dogma.

## O que eu vou testar em jogo

Tenho dois clientes 1.5.5 e uso as contas do realm `realm_155BR` (sacerdotes `HEal` id 40 e
`testesacer` id 42, ambos classe 7). Quando você terminar, me diga exatamente o que olhar na
tela e o que esperar do overlay.
