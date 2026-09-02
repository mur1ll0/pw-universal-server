# Banco de dados: o que falta, o que sobra e o que está errado

Revisão do PostgreSQL e do Dragonfly com uma pergunta só: **o servidor consegue jogar o
jogo com o que está guardado hoje?** A resposta é não, em pontos específicos — e cada um
deles aparece abaixo com a evidência de onde a falta se manifesta, não como opinião de
modelagem.

O que saiu disto está em `specs/02_MIGRACAO_COMPATIBILIDADE_MULTI_REALM.sql`, que aplica
sobre o `01_DATABASE_SCHEMA_POSTGRES.sql` e pode rodar duas vezes sem reclamar.

---

## Como esta revisão foi feita

Três fontes, nesta ordem de autoridade:

1. **O que vai no fio.** Os `rpcdata` do cliente 1.5.3 (`roleinfo`, `groleinventory`) dizem
   exatamente quais campos o cliente espera receber. Um campo que o cliente lê e que o
   servidor preenche com zero é uma falta de coluna, não uma escolha.
2. **O que o código já pede.** Quando `CharacterDetails` tem `reputation` e o repositório
   escreve `reputation: 0` porque não tem de onde ler, a necessidade já está declarada — só
   não tem onde morar.
3. **O que o jogador faz.** O roteiro de 45 passos que o Murillo executou no servidor 1.2.6
   real é a lista de funcionalidades que precisam existir. Expandir a bolsa, expandir a
   gaiola de pets, montar lista de amigos: cada passo é um requisito medido.

Nada aqui vem de "num MMO normalmente tem". O que seria palpite está na seção
[O que ficou de fora](#o-que-ficou-de-fora-de-proposito), com o motivo.

---

## 1. O que o cliente recebe zerado

O `RoleInfo` tem 23 campos e viaja em três protocolos (`RoleList_Re`, `CreateRole_Re`,
`CreateRole`). O `write_role_info` (`crates/pw-protocol/src/packets/s2c.rs`) escreve **oito**
deles com constante, porque não há coluna:

| Campo do `RoleInfo` | O que o jogador perde | Coluna criada |
| :--- | :--- | :--- |
| `level2` | nível de renascimento sempre 0 | `characters.level2` |
| `create_time` | data de criação errada na tela | (já havia `created_at`; falta ligar) |
| `lastlogin_time` | "último acesso" sempre vazio | `characters.last_login_at` |
| `delete_time` | o `UndoDeleteRole` não sabe até quando dá para desfazer | `characters.delete_scheduled_at` |
| `custom_status` | nenhum ícone de estado sobre a cabeça | `characters.custom_status` |
| `charactermode` | idem | `characters.charactermode` |
| `reincarnation_data` | renascimento não persiste (1.4.8+) | `characters.reincarnation_data` |
| `realm_data` | idem | `characters.realm_data` |

E o item, no `GRoleInventory`, tem cinco campos que também vão zerados —
`proctype`, `expire_date`, `guid1`, `guid2`, `mask` — mais o `max_count`, que o
`ItemRepository` devolve como **`100` chumbado** (`repositories/item.rs:37`), inclusive para
equipamento, que empilha 1.

O par `guid1`/`guid2` merece um parágrafo próprio: é o identificador único da *instância* do
item. Sem ele, dois itens iguais são indistinguíveis para o servidor, e uma duplicação
vira uma discussão em vez de uma consulta. A migração cria a coluna, o índice e a sequência
`item_guid_seq` — única para o banco inteiro, para que dois realms nunca gerem o mesmo
número, o que passa a importar no dia em que houver transferência entre realms.

## 2. O que o servidor responde "ok" e joga fora

Em `crates/pw-link/src/gateway.rs`, três protocolos respondem `result: 0` e descartam o que
receberam; os `Get*` correspondentes devolvem vazio:

- `SetUIConfig` / `GetUIConfig` — a interface que o jogador arrumou;
- `SetHelpStates` / `GetHelpStates` — quais dicas ele já dispensou;
- `SetCustomData` — o blob que o cliente guarda por conta própria.

O sintoma é o pior tipo: **não há erro nenhum**. O jogador arruma tudo, desloga, volta, e
está no padrão de novo. Três tabelas (`character_ui_config`, `character_help_states`,
`character_custom_data`), uma linha por personagem, blob opaco — o servidor não interpreta,
guarda e devolve.

Separadas de `characters` de propósito: são escritas a cada mudança de janela e lidas uma
vez no login. Como coluna, cada `UPDATE` de posição do personagem reescreveria quilobytes.

O `GetFriendList` é o quarto caso, e o mais visível: devolve `groups`, `friends` e `status`
vazios, com o comentário honesto de que "a lista de amigos ainda não vem do armazenamento".
Os passos 38 a 41 do roteiro são exatamente isso. Daí `character_friends` e
`character_friend_groups`, com índice nos dois sentidos — o inverso é o que avisa os amigos
quando alguém entra.

## 3. O que o código já declarava precisar

`repositories/character.rs:419-423` monta o `CharacterDetails` com:

```rust
reputation: 0,
inventory_size: 64,
storehouse_size: 32,
```

Os três campos existem no tipo de domínio em `pw-core`. Expandir a bolsa (passo 43 do
roteiro) funciona até o logout; a reputação, que o `TaskReward` do `pw-data-loader` já sabe
conceder, é sempre zero. Viram colunas, junto com `petbag_size` (passo 44) e
`storehouse_money` — o banco tem saldo próprio, separado do dinheiro do personagem.

## 4. Restrições que faltavam

**Nome de personagem.** `UNIQUE(realm_id, name)` distingue maiúsculas: no mesmo realm cabem
`Murillo` e `murillo`, que na tela são duas pessoas com o mesmo nome. É assim que se
personifica alguém para um golpe de troca. Existia um índice sobre `LOWER(name)`, mas não
único — ele acelerava a busca e não impedia nada. A migração cria o índice **único**.

Antes de aplicar, vale conferir se o banco já tem casos assim:

```sql
SELECT realm_id, LOWER(name), COUNT(*)
FROM characters
GROUP BY realm_id, LOWER(name)
HAVING COUNT(*) > 1;
```

Se voltar alguma linha, é preciso renomear antes — a criação do índice falha, e é bom que
falhe.

**Versão do realm.** `pw-link` e `pw-gs` **abortam** com um `GAME_VERSION` desconhecido, em
vez de cair no 1.2.6 em silêncio (item 44). A tabela `realms` aceitava qualquer texto: um
`'1.53'` digitado errado subia e aparecia no painel. A migração alinha o banco ao código com
um `CHECK`.

**Uma facção por personagem.** `factions.members` é um JSONB com a lista inteira. Três
problemas concretos: não há integridade (um personagem excluído continua na lista), a
pergunta mais frequente do jogo — "de que facção é este?" — vira varredura de todas as
facções do realm, e dois membros entrando ao mesmo tempo reescrevem o mesmo documento, com
um dos dois sumindo sem erro. `faction_members` resolve os três, e a restrição
`uq_uma_faccao_por_personagem` é a que o JSONB não tinha como ter.

A coluna `members` **não foi removida**: quem já tem dados migra primeiro.

```sql
INSERT INTO faction_members (faction_id, character_id, rank)
SELECT f.id, (m->>'character_id')::int, COALESCE((m->>'rank')::smallint, 0)
FROM factions f, jsonb_array_elements(f.members) AS m
ON CONFLICT DO NOTHING;
```

(Confira o formato do seu JSONB antes: os nomes das chaves dependem de quem escreveu.)

As quatro restrições foram testadas tentando violá-las uma a uma; as quatro recusam.

---

## 5. Dragonfly: o que está lá e o que preocupa

O `CacheManager` (`crates/pw-storage/src/cache.rs`) tem seis métodos. Três deles —
`set_player_session`, `remove_player_session` e `get_online_count` — **não são chamados por
ninguém**. Não é um detalhe: significa que hoje **nada marca quem está online**, e qualquer
contagem de jogadores no painel é ficção.

Antes de ligarem, um defeito de desenho para corrigir:

```rust
conn.set_ex(format!("session:{realm}:{role}"), value, ttl)  // expira sozinho
conn.sadd(format!("online:{realm}"), role_id)               // NÃO expira
```

A chave da sessão expira; o conjunto `online:` não. Um `pw-link` que caia sem fechar a
sessão deixa o jogador no conjunto **para sempre**, e a contagem de online sobe sozinha até
não significar mais nada. É o tipo de erro que só aparece semanas depois, quando ninguém
liga mais o número ao dia da queda.

A correção é usar um `ZSET` com a hora como pontuação (`ZADD online:{realm} <agora>
<roleid>`), tratar como online quem tiver pontuação dentro da janela de TTL, e podar o resto
com `ZREMRANGEBYSCORE`. O jogador só continua online enquanto o link renovar a marca — que é
o que a palavra "online" quer dizer.

O que **falta** no cache, e o jogo vai precisar:

- **Bloqueio de nome durante a criação.** Hoje duas criações simultâneas do mesmo nome se
  resolvem no `UNIQUE` do Postgres — funciona, mas a segunda vira erro genérico. Um
  `SET NX` com TTL curto dá a mensagem certa.
- **Presença entre realms** para o chat global e a lista de amigos: quem está online, em que
  realm, em que mapa.
- **Tempos de recarga (cooldown) de habilidade e item**, que não podem ir ao Postgres a cada
  uso.
- **A sessão do `EnterWorld`**: hoje o `pw-link` guarda o `localsid` na memória do processo.
  Se o processo reinicia, todos caem — e o Dragonfly existe justamente para isso.

## 6. Fora do banco, mas do mesmo assunto: o `docker-compose`

Três coisas que valem mudar antes de qualquer máquina ficar exposta à internet:

```yaml
pw-postgres:
  ports: ["5432:5432"]     # o banco inteiro, aberto na máquina hospedeira
pw-dragonfly:
  ports: ["6379:6379"]     # sem senha nenhuma
```

Nenhum dos dois precisa ser alcançável de fora: os serviços se falam pela rede interna do
compose, pelo nome. Se for preciso acessar de fora para depurar, `127.0.0.1:5432:5432`
limita ao próprio host. O Dragonfly deveria subir com `--requirepass`, e a senha do Postgres
(`pw_secure_password_2026`) não deveria estar versionada — um `.env` fora do git resolve os
dois.

É o mesmo raciocínio que já governou a porta do barramento entre daemons, que não é exposta
ao jogador.

## 7. Compatibilidade entre realms

O modelo de hoje é **um esquema para as três versões**, com `realm_id` em cada linha. Está
certo, e a migração não muda isso. As colunas que só existem a partir do 1.4.8
(`reincarnation_data`, `realm_data`) ficam nulas num realm 1.2.6 — e o `write_role_info` já
corta os quatro últimos campos do `RoleInfo` para essa versão, então nada delas vaza para o
fio errado.

Onde isso vai apertar, e ainda não apertou: meridianos e cultivo estendido do 1.4.8+, e a
segunda página de banco. Quando chegar, a escolha será entre colunas nulas para dois terços
dos realms e uma tabela `character_version_data (character_id, chave, valor)`. Não dá para
decidir bem antes de ter o primeiro caso concreto — e o `realms.config JSONB` já resolve a
parte de configuração por realm.

Uma observação sobre os dados de disco, que a investigação do login trouxe e vale registrar
aqui: **as constantes de versão que o cliente confere estão dentro dos `.data` do realm**
(1ª palavra do `elements.data`, 2ª do `tasks.data`). Não é configuração de banco nem
constante de código — ver `crates/pw-protocol/src/edition.rs`.

---

## O que ficou de fora, de propósito

Cada um destes é necessário para o jogo completo e **não** entrou na migração, porque eu não
sei ainda o formato certo e chutar uma tabela é pior que não ter nenhuma:

- **Pets** (`character_pets`). A gaiola já ganhou `petbag_size`, mas o que há dentro de um
  pet — nível, experiência, fome, habilidades — tem formato próprio no `elements.data` e
  precisa ser medido antes.
- **Loja de jogador** (`OPEN_BOOTH`, subcomando 76). Está na lista de dívidas do protocolo e
  ainda não é tratado; a tabela vem junto com o tratamento.
- **Registro de trocas, correio com item e leilão.** Dependem do `guid` do item, que só agora
  passa a existir. É a ordem certa: primeiro a identidade da instância, depois o rastro.
- **Cônjuge, títulos, PK e karma.** Aparecem no roteiro (duelo, passo 36) mas ainda não
  passam pelo nosso protocolo; a coluna sem o campo no fio não faz nada.

---

## Ordem sugerida daqui

1. Aplicar a migração (`psql -f specs/02_...sql`) — ela é idempotente e não apaga nada.
2. Ligar as colunas no código, começando pelas que o cliente já lê e recebe zeradas:
   `create_time`/`lastlogin_time` no `RoleInfo` são duas linhas e aparecem na tela.
3. Guardar de verdade os três blobs (`UIConfig`, `HelpStates`, `CustomData`) — é o conserto
   com maior efeito visível por linha escrita.
4. Corrigir o `online:` do Dragonfly **antes** de alguém passar a chamá-lo.
5. Fechar as portas do `docker-compose`.
