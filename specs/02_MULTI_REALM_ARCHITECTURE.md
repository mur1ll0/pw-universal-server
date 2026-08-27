# Especificação 02: Arquitetura Multi-Realm Concorrente

## 1. Topologia de Rede e Portas

Para rodar simultaneamente múltiplos servidores em versões distintas (ex: **Realm 1.2.6** e **Realm 1.5.3**) na mesma máquina/servidor host, o roteamento de portas é segregado por Realm:

```
+-------------------------------------------------------------------------------------------------+
|                                    MAPEAMENTO DE PORTAS HOST                                    |
+-------------------------------------------------------------------------------------------------+
| SERVIÇO GLOBAL (Compartilhado)                                                                  |
|   • PostgreSQL Database:          localhost:5432                                                |
|   • DragonflyDB Cache:            localhost:6379                                                |
|   • pw-auth (Global Auth API):    localhost:29200 (Interno)                                     |
|   • pw-admin-web (Painel Web):    localhost:3000 (UI) / localhost:8000 (API)                    |
+-------------------------------------------------------------------------------------------------+
| REALM 1: Classic (Versão 1.2.6)                                                                 |
|   • pw-link-126 (Client Gateway):  0.0.0.0:29000  (Porta pública no client serverlist.txt)     |
|   • pw-delivery-126 (Broker):      localhost:29100 (Interno ao container)                       |
|   • pw-gs-world-126 (Game Server): localhost:29400 (Interno ao container)                       |
+-------------------------------------------------------------------------------------------------+
| REALM 2: Eclipse (Versão 1.5.3)                                                                 |
|   • pw-link-153 (Client Gateway):  0.0.0.0:29001  (Porta pública no client serverlist.txt)     |
|   • pw-delivery-153 (Broker):      localhost:29101 (Interno ao container)                       |
|   • pw-gs-world-153 (Game Server): localhost:29401 (Interno ao container)                       |
+-------------------------------------------------------------------------------------------------+
```

---

## 2. Fluxo de Conexão do Jogador (Login Flow)

1. **Seleção de Servidor pelo Cliente**:
   - O jogador que abre o **ElementClient 1.2.6** possui seu `serverlist.txt` apontando para a porta `29000`.
   - O jogador que abre o **ElementClient 1.5.3** possui seu `serverlist.txt` apontando para a porta `29001`.

2. **Validação de Credenciais**:
   - O `pw-link` de cada realm recebe a conexão e encaminha o pacote de login para o `pw-auth` global.
   - O `pw-auth` valida o usuário no PostgreSQL e verifica se a conta está ativa e sem banimento.

3. **Carregamento de Personagens do Realm**:
   - A requisição solicita a lista de personagens para `realm_id = 'realm_126'` (se conectou na porta 29000) ou `realm_id = 'realm_153'` (se conectou na porta 29001).
   - O cliente recebe apenas os personagens válidos para aquela versão, impedindo qualquer crash de modelo 3D ou item incompatível.

---

## 3. Isolamento de Recursos no Docker

Cada Realm opera como um grupo de containers com limites de memória RAM definidos:

```yaml
# Exemplo de limites por Realm
realm_126_gs:
  deploy:
    resources:
      limits:
        cpus: '2.0'
        memory: 1024M

realm_153_gs:
  deploy:
    resources:
      limits:
        cpus: '3.0'
        memory: 1536M
```
Total estimado para os dois mundos simultâneos: **~2.5 GB a 3.0 GB de RAM** (muito abaixo do consumo de 12+ GB do servidor legado).
