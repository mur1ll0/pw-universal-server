# Guia de Operação: Como Rodar o Servidor (Versão Única ou Multi-Realm Concorrente)

Este guia prático explica o passo a passo exato para subir o servidor em uma versão específica (ex: apenas 1.2.6 ou apenas 1.5.3) ou rodar **ambas as versões simultaneamente** compartilhando o mesmo banco de dados PostgreSQL e DragonflyDB.

---

## 1. Onde Ficam e Onde Copiar os Arquivos de Configuração?

Cada Realm possui sua pasta dedicada dentro de `data/`.

> [!NOTE]
> **Arquivos Globais vs Arquivos Específicos de Mapa**:
> - **Arquivos Globais** (`elements.data`, `tasks.data`, `gshop.data`, `aipolicy.data`): Ficam na raiz da pasta `config/`.
> - **Arquivos Específicos de Mapa** (`npcgen.data`, `.clv`, `.clt`, `precinct.sev`, `region.sev`, `path.sev`): **Ficam DENTRO da pasta de cada mapa** (`world/`, `a01/`, `a02/`, `b01/`, etc.). Cada mapa/dungeon possui seu próprio `npcgen.data` dedicado!

```
pw-universal-server/
└── data/
    ├── realm_126/
    │   └── config/      <-- COPIE AQUI OS ARQUIVOS DA VERSÃO 1.2.6
    │       ├── elements.data        (Templates globais de itens e monstros)
    │       ├── gshop.data           (Loja de Gold do 1.2.6)
    │       ├── tasks.data           (Missões do 1.2.6)
    │       ├── aipolicy.data        (Árvores de IA dos monstros do 1.2.6)
    │       │
    │       ├── world/               <-- PASTA DO MAPA-MÚNDI (Pan Gu)
    │       │   ├── npcgen.data      (Spawns de NPCs e monstros do Mundo Principal)
    │       │   ├── collision.clt    (Grid de relevo e altura do terreno)
    │       │   └── collision.clv    (Malhas 3D de construções e pontes)
    │       │
    │       ├── a01/                 <-- DUNGEON FB19 HUMANO
    │       │   └── npcgen.data      (Spawns específicos da Dungeon a01)
    │       ├── a02/                 <-- DUNGEON FB19 FERA
    │       │   └── npcgen.data      (Spawns específicos da Dungeon a02)
    │       ├── a03/                 <-- DUNGEON FB19 ELFO
    │       │   └── npcgen.data      (Spawns específicos da Dungeon a03)
    │       └── a04/ .. b35/         <-- DEMAIS DUNGEONS (FB29..FB99, Frost, Dusk)
    │           └── npcgen.data      (Spawns específicos de cada dungeon)
    │
    └── realm_153/
        └── config/      <-- COPIE AQUI OS ARQUIVOS DA VERSÃO 1.5.3
            ├── elements.data        (v145/v153)
            ├── gshop.data           (Loja de Gold do 1.5.3)
            ├── tasks.data           (Missões do 1.5.3)
            ├── aipolicy.data        (IA dos monstros do 1.5.3)
            │
            ├── world/               (Mapa-Múndi v1.5.3)
            │   └── npcgen.data      (Spawns do Mundo 1.5.3)
            ├── is01/ .. is05/       (Mundo Primitivo / Morai)
            │   └── npcgen.data      (Spawns de Morai / Mundo Primitivo)
            └── a01/ .. b35/         (Dungeons do 1.5.3)
                └── npcgen.data
```

---

## 2. Cenário A: Rodando Apenas 1 Servidor em uma Versão Específica

Se você quiser rodar apenas o servidor **1.2.6 Classic**:

1. **Copiar os arquivos**:
   ```bash
   mkdir -p ./data/realm_126/config
   cp -r ../files1.2.6/pwserver/gamed/config/* ./data/realm_126/config/
   ```
2. **Subir com Docker**:
   ```bash
   docker compose up -d pw-postgres pw-dragonfly pw-auth pw-admin-api pw-realm-126
   ```
3. **Configuração no Cliente 1.2.6**:
   - Abra o arquivo `patcher/server/serverlist.txt` dentro do seu cliente 1.2.6.
   - Configure o endereço IP e a porta **`29000`**:
     ```text
     "PW Classic 1.2.6"  "127.0.0.1"  29000  1
     ```

---

## 3. Cenário B: Rodando 2 Servidores SIMULTÂNEOS (Multi-Realm: 1.2.6 e 1.5.3)

Para rodar os dois servidores ao mesmo tempo na mesma máquina:

1. **Copiar os arquivos de ambas as versões**:
   ```bash
   # Copiar dados do 1.2.6
   mkdir -p ./data/realm_126/config
   cp -r ../files1.2.6/pwserver/gamed/config/* ./data/realm_126/config/

   # Copiar dados do 1.5.3
   mkdir -p ./data/realm_153/config
   cp -r ../pwclient_153v145/element/data/* ./data/realm_153/config/
   ```

2. **Iniciar todos os serviços com 1 comando**:
   ```bash
   docker compose up -d
   ```

3. **Mapeamento de Portas e Clientes**:
   - **Cliente 1.2.6 (`ElementClient.exe`)**: Conecta em `127.0.0.1:29000`
   - **Cliente 1.5.3 (`ElementClient.exe`)**: Conecta em `127.0.0.1:29001`
   - **Painel Web de Administração (`pw-admin-web`)**: Acesse no navegador em `http://localhost:8000` (API) ou abrindo `web-admin/frontend/index.html`.

---

## 4. Como Usar o Painel Web Administrativo (`pw-admin-web`)

1. Abra `web-admin/frontend/index.html` no seu navegador favorito (Chrome, Firefox, Edge).
2. O painel se conectará automaticamente à API local (`http://localhost:8000`).
3. **Operações Disponíveis**:
   - **Criar Contas**: Clique no botão "+ Criar Nova Conta", defina o usuário, senha e nível de GM.
   - **Dar Gold/CUBI**: Na lista de contas, clique no botão `+ Gold` e informe a quantidade. O saldo é creditado instantaneamente.
   - **Double EXP / Eventos**: Acesse a aba *Double Eventos*, ajuste o multiplicador (ex: `2.0x`) e clique em *Aplicar*. O servidor atualiza o ganho de EXP sem precisar reiniciar o jogo.
   - **Teletransporte de Emergência**: Na aba *Personagens*, pesquise o boneco e clique em *Teleportar CDD* para resgatá-lo para a Cidade do Dragão.
   - **Transmissão Amarela de Sistema**: Na aba *Anúncios*, digite o aviso e transmita para todos os jogadores do Realm selecionado.
