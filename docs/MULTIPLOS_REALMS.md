# Vários realms, e vários mundos dentro de um realm

> Duas coisas diferentes que costumam ser chamadas do mesmo jeito. Confundi-las neste
> documento uma vez (`COMO_TESTAR.md` dizia "vários servidores de mundo por realm" quando
> a pergunta era outra), e a distinção muda a resposta inteira.

---

## Os dois eixos

| | **Outro realm** | **Outro mundo no mesmo realm** |
| :--- | :--- | :--- |
| O que é | outro jogo, independente | outro mapa/instância do mesmo jogo |
| Personagens | separados | os mesmos, andando entre mapas |
| Config e `.data` | próprios | os mesmos |
| Nomes de personagem | podem repetir entre realms | únicos, é o mesmo realm |
| Variável | `REALM_ID` | `WORLD_TAG` |
| Estado hoje | **funciona** | estrutura existe, falta roteamento entre mundos |

**Dois servidores 1.2.6, cada um com sua config e seus `.data`, é o primeiro caso.** São
dois realms que por acaso rodam a mesma versão do protocolo. A versão (`GAME_VERSION`) diz
só qual dialeto o cliente fala; ela não junta nem separa nada.

O segundo eixo é o `WORLD_TAG`, que vira `world_id` no `pw-gs` e escolhe de qual
`npcgen.data` aquele processo carrega os spawns (`WorldInstance::init_spawns`). É o mesmo
número da coluna `characters.world_id`, que diz em que mapa o personagem está. Serve para
partir um realm grande em processos por mapa — não é o que você perguntou, e ainda falta a
travessia de um mundo para o outro.

---

## Receita: um segundo realm 1.2.6

Exemplo: o realm existente `realm_126` (porta 29000) mais um novo `realm_126b` na 29003.

### 1. A linha na tabela `realms`

A tabela `characters` tem chave estrangeira para `realms(id)`; sem a linha, nenhum
personagem é criado.

O `specs/01_DATABASE_SCHEMA_POSTGRES.sql` só roda **uma vez**, quando o volume do Postgres
está vazio. Num banco que já existe, insira à mão:

```sql
INSERT INTO realms (id, name, version, host, port, max_players, config)
VALUES ('realm_126b', 'Classic — Servidor 2', '1.2.6', '127.0.0.1', 29003, 3000,
        '{"enabled_classes": [0,1,2,3,4,5], "max_level": 105}'::jsonb);
```

Os templates de classe (`class_templates`) **não** precisam de nada: o `pw-link` chama
`ensure_default_templates(realm_id)` ao subir e cria os do realm novo se não existirem.

### 2. A pasta de dados

```
data/realm_126b/config/
```

Cópia independente dos `.data` — `elements.data`, `tasks.data`, `npcgen.data`,
`gshop*.data`, os mapas. É o que torna os dois servidores realmente diferentes: mudar
drop, spawn ou missão em um não toca o outro.

### 3. O par de serviços no compose

Cada realm são **dois** contêineres. Copie um bloco existente trocando quatro coisas: o
`REALM_ID`, a porta pública, a pasta de dados e os nomes.

```yaml
  pw-realm-126b:
    build:
      context: ..
      dockerfile: docker/Dockerfile.core
      args:
        CRATE_NAME: pw-link
    container_name: pw-realm-126b
    restart: unless-stopped
    environment:
      REALM_ID: realm_126b          # <- precisa existir na tabela `realms`
      GAME_VERSION: "1.2.6"         # <- o dialeto; pode repetir entre realms
      GATEWAY_PORT: 29003
      DATABASE_URL: postgresql://pw_admin:pw_secure_password_2026@pw-postgres:5432/pw_database
      REDIS_URL: redis://pw-dragonfly:6379
      GS_BUS: pw-world-126b:29100   # <- o mundo DESTE realm
    ports:
      - "29003:29003"
    volumes:
      - ../data/realm_126b/config:/app/data/config:ro
    depends_on:
      - pw-auth
      - pw-world-126b

  pw-world-126b:
    build:
      context: ..
      dockerfile: docker/Dockerfile.core
      args:
        CRATE_NAME: pw-gs
    container_name: pw-world-126b
    restart: unless-stopped
    environment:
      REALM_ID: realm_126b
      GAME_VERSION: "1.2.6"
      WORLD_TAG: "1"
      BUS_LISTEN: 0.0.0.0:29100     # <- a MESMA porta em todos: são redes internas
      CONFIG_DIR: /app/data/config
      RUST_LOG: info,pw_gs=debug
      DATABASE_URL: postgresql://pw_admin:pw_secure_password_2026@pw-postgres:5432/pw_database
      REDIS_URL: redis://pw-dragonfly:6379
    volumes:
      - ../data/realm_126b/config:/app/data/config:ro
    depends_on:
      pw-postgres:
        condition: service_healthy
      pw-dragonfly:
        condition: service_started
```

O `29100` se repete em todos os mundos de propósito e **não** conflita: cada um está no
próprio contêiner, e a porta não é publicada. É por isso que o `GS_BUS` usa o nome do
serviço — `pw-world-126b:29100` — e não `localhost`.

O teste `pw-bus/tests/topologia_do_compose.rs` confere isso sozinho: se você esquecer o
`GS_BUS`, apontar para o mundo errado, ou publicar a porta do barramento, o `cargo test`
falha antes de você subir nada.

### 4. Confira

```bash
cargo test -p pw-bus --test topologia_do_compose
docker compose up -d --build pw-world-126b pw-realm-126b
docker compose logs pw-realm-126b | grep barramento
```

---

## O que é compartilhado e o que não é

| | Escopo | Por quê |
| :--- | :--- | :--- |
| Contas (`accounts`) | **globais** | uma conta, todos os realms — como no PW original |
| Personagens | por realm | `characters.realm_id`, com FK |
| Nomes de personagem | por realm | `uq_character_name_per_realm` |
| Facções | por realm | `uq_faction_name_per_realm` |
| Templates de classe | por realm | `uq_class_template_per_realm` |
| Sessões e contagem de online | por realm | chaves `session:<realm>:<role>` e `online:<realm>` no cache |
| Banco e cache (os processos) | compartilhados | um Postgres e um Dragonfly atendem todos |
| Arquivos `.data` | por realm | pasta própria montada em `/app/data/config` |

A conta ser global e o personagem ser por realm é o ponto delicado: **o mesmo jogador
logado tem personagens nos dois mundos**. Foi aí que apareceu a falha descrita a seguir.

---

## A falha que este assunto revelou

`SelectRole`, `EnterWorld`, `DeleteRole` e `UndoDeleteRole` recebem o `role_id` dentro de
um pacote do cliente e o mandavam ao banco com `WHERE id = $1` — **sem conta e sem
realm**. O `role_id` é sequencial. Então, com um cliente logado, dava para entrar no mundo
como qualquer personagem do servidor e apagar o personagem de qualquer outro jogador.

Isso valia com **um** realm só; dois realms apenas tornam o vazamento óbvio. Corrigido em
duas camadas:

1. **No repositório**, onde não dá para esquecer: `get_details`, `delete_character` e
   `restore_character` passaram a exigir `account_id` e `realm_id` na assinatura *e* na
   cláusula `WHERE`. Nenhuma variante sem escopo existe.
2. **No `dispatch_packet`**, uma barreira única: sem conta na sessão, nada que toque dados
   de personagem passa. A lista de isentos é por inclusão, então um pacote novo já nasce
   exigindo login.

`crates/pw-storage/tests/autorizacao_de_personagem.rs` prova isso contra um PostgreSQL de
verdade, montando exatamente o cenário deste documento: dois realms 1.2.6, a mesma conta
com personagem nos dois. Com a correção revertida, quatro dos seis testes falham.

---

## Limites conhecidos

- **Não há travessia entre realms.** São jogos separados; um personagem não muda de realm.
- **`WORLD_TAG` ainda não particiona nada de verdade.** Dois `pw-gs` no mesmo realm com
  tags diferentes carregam spawns diferentes, mas não trocam jogadores entre si.
- **O `realms` só é semeado na criação do banco.** Realm novo em banco existente é
  `INSERT` manual (passo 1).
- **Nada balanceia carga.** Um realm, um `pw-link`, um `pw-gs`.
