# Especificação Mestre 00: Arquitetura do PW-Universal-Server

## 1. Visão Geral do Projeto
O **PW-Universal-Server** substitui o ecossistema original do Perfect World por uma arquitetura em **Rust**, **PostgreSQL 16** e **DragonflyDB**, projetada para rodar simultaneamente múltiplos Realms (versões de jogo diferentes como 1.2.6 e 1.5.3) a partir de uma base compartilhada de contas e infraestrutura.

---

## 2. Princípios de Engenharia (Spec-Driven Development)

1. **Separação Estrita de Responsabilidades**:
   - **Camada de Borda (Gateway / `pw-link`)**: Escuta conexões públicas de clientes, lida com criptografia RC4 e pacotes binários brutos.
   - **Camada de Roteamento (Delivery / `pw-delivery`)**: Mantém o estado global dos jogadores, chat, mensagens e distribuição entre servidores de mapa.
   - **Camada de Simulação (World Server / `pw-gs`)**: Executa o loop espacial 3D em ticks de 50ms, cálculos de combate, IA e física.
   - **Camada de Dados (Storage / `pw-storage`)**: Centraliza operações com PostgreSQL e DragonflyDB, eliminando dependência do Berkeley DB.

2. **Multi-Versão por Design**:
   - O núcleo não possui structs de pacotes codificadas de forma fixa. O formato das mensagens binárias de cada versão é carregado via arquivo de codec de versão (ex: `v126.toml`, `v153.toml`).
   - O `elements.data` é interpretado por um motor dinâmico com identificação automática de versão de cabeçalho.

3. **Multi-Realm em Docker**:
   - Uma única infraestrutura de banco de dados (`postgres` + `dragonfly`) atende a $N$ realms ao mesmo tempo.
   - Cada Realm roda seus próprios contêineres de Link, Delivery e World Server em portas distintas.

---

## 3. Matriz de Módulos (Crates)

| Crate | Responsabilidade Principal |
| :--- | :--- |
| `pw-core` | Tipos fundamentais, vetores 3D, octrees, AABB, enums de raças/classes e constantes globais. |
| `pw-crypto` | Cifra de fluxo RC4, algoritmos de hash (MD5, Argon2, SHA256) e tabelas de chaves de rede do PW. |
| `pw-protocol` | Codecs de serialização/deserialização para os pacotes binários do cliente (C2S e S2C). |
| `pw-data-loader` | Leitor dinâmico de `elements.data`, `tasks.data`, `aipolicy.data` e mapas 3D (`.clv`/`.clt`). |
| `pw-storage` | Repositórios de acesso a dados no PostgreSQL e camada de cache em memória no DragonflyDB. |
| `pw-auth` | Serviço de autenticação de contas, geração de tickets de sessão e controle de saldo de Gold. |
| `pw-uniquename` | Serviço de validação de unicidade de nomes de personagens e facções por Realm. |
| `pw-link` | Gateway TCP assíncrono de alto throughput que atende as conexões dos clientes de jogo. |
| `pw-delivery` | Roteador central de mensagens, canais de chat, amigos, correio in-game e instâncias. |
| `pw-gs` | Motor de simulação do mundo (World Server) com loop de 50ms, IA e fórmulas de combate. |
