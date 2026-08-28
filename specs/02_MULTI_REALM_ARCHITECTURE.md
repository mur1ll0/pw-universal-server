# Especificação 02: Arquitetura Multi-Realm & Adaptadores de Protocolo

## 1. Decisão Arquitetural: Abordagem de Adaptadores no Servidor Universal

Para viabilizar múltiplos servidores em versões distintas (1.2.6, 1.4.8, 1.5.3, etc.) com a máxima fidelidade e facilidade de distribuição para os jogadores, optou-se pela **Abordagem A (Servidor Polimórfico com Adaptadores de Versão)** ao invés de recompilar os clientes:

### Por que NÃO recompilar os clientes (`elementclient.exe`)?
1. **Incompatibilidade Gráfica e de Shaders**: O cliente 1.2.6 foi compilado com a engine Angelica 2.0 (DirectX 8/9 legado), enquanto o 1.5.3 utiliza Angelica 2.2 com múltiplos passes de shaders e modelos 3D avançados.
2. **Distribuição Transparente para a Comunidade**: Permite que jogadores usem qualquer cliente oficial de sua preferência sem precisar de executáveis modificados.
3. **Isolamento e Segurança**: Toda a inteligência de tradução de versões reside no servidor Rust em alta performance.

---

## 2. Topologia de Rede e Portas por Realm

Cada Realm roda em seu próprio container com uma porta pública dedicada no `serverlist.txt`:

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
|   • pw-realm-126 (Client Gateway): 0.0.0.0:29000  (serverlist.txt v1.2.6)                      |
|   • Código de Versão de Rede:     0x00010206 (Decimal: 66054)                                   |
|   • Classes Suportadas:           6 classes (WR, MG, EA, EP, WB, WF)                            |
+-------------------------------------------------------------------------------------------------+
| REALM 2: Tides / Genesis (Versão 1.4.8)                                                         |
|   • pw-realm-148 (Client Gateway): 0.0.0.0:29002  (serverlist.txt v1.4.8)                      |
|   • Código de Versão de Rede:     0x00010408 (Decimal: 66568)                                   |
|   • Classes Suportadas:           10 classes (+ MC, ES, ME, GD)                                 |
+-------------------------------------------------------------------------------------------------+
| REALM 3: Eclipse (Versão 1.5.3)                                                                 |
|   • pw-realm-153 (Client Gateway): 0.0.0.0:29001  (serverlist.txt v1.5.3)                      |
|   • Código de Versão de Rede:     0x00010503 (Decimal: 66819)                                   |
|   • Classes Suportadas:           12 classes (+ TM, RT) com meridianos e reencarnação           |
+-------------------------------------------------------------------------------------------------+
```

---

## 3. Especificação Binária dos Pacotes por Versão

### 3.1 Codificação do Código de Versão (`GAME_VERSION`)
A engine Wanmei calcula a versão a partir dos 4 octetos `(major, minor, release, patch)`:
$$\text{version\_code} = (\text{major} \ll 24) \mid (\text{minor} \ll 16) \mid (\text{release} \ll 8) \mid \text{patch}$$

| Versão | Octetos | Valor Hex | Valor Decimal |
| :--- | :---: | :---: | :---: |
| **1.2.6** | `(0, 1, 2, 6)` | `0x00010206` | **66054** |
| **1.4.8** | `(0, 1, 4, 8)` | `0x00010408` | **66568** |
| **1.5.3** | `(0, 1, 5, 3)` | `0x00010503` | **66819** |

### 3.2 Sequência de Handshake de Login Completa

1. **S2C: Challenge (Opcode 1)**:
   - `nonce`: 16 bytes (bytes 0..3: `server_attr`, bytes 4..7: `free_creatime`, bytes 8..15: random)
   - `version`: `u32` (código de versão exato do Realm)
   - `algo`: `i8` (`0`)
   - *(Apenas v1.4.8 e v1.5.3)*: `edition`: Octets vazios, `exp_rate`: `u8` (`1`).

2. **C2S: Response (Opcode 2)**:
   - `username`: Octets (UTF-8 ou UTF-16LE)
   - `password_response`: Octets (MD5 Response Hash)
   - *(Apenas v1.4.8 e v1.5.3)*: `use_token`, `cli_fingerprint`.

3. **S2C: OnlineAnnounce (Opcode 4)**:
   - Enviado diretamente pelo servidor para autenticar e comutar o cliente para o estado `_state_GSelectRoleClient`.
   - `userid`: `i32`, `localsid`: `u32`, `remain_time`: `i32`, `zoneid`: `i8`, `free_time_left`: `i32`, `free_time_end`: `i32`, `creatime`: `i32`
   - *(Apenas v1.4.8 e v1.5.3)*: `referrer_flag`, `passwd_flag`, `usbbind`, `accountinfo_flag`.

4. **C2S: RoleList (Opcode 0x52 / 82)**:
   - Disparado pelo cliente em `state_GSelectRoleClient` solicitando a lista de personagens.

5. **S2C: RoleList_Re (Opcode 0x53 / 83)**:
   - Resposta com a lista de personagens (`RoleInfo`), fechando o diálogo modal de carregamento e liberando a tela de seleção/criação de personagens.

### 3.3 Pacotes Contínuos de Sessão, Criação, Exclusão e Entrada no Mundo
- **C2S: KeepAlive / Heartbeat (Opcode 0x5A / 90)**:
  - Enviado periodicamente pelo cliente para manter o socket TCP vivo.
  - Payload: `code: char` (1 byte `i8`).
- **C2S: CreateRole (Opcode 0x54 / 84)** / **S2C: CreateRole_Re (Opcode 0x55 / 85)**:
  - `userid`: `i32`, `localsid`: `u32`, `roleinfo`: `RoleInfo` estruturado.
- **C2S: DeleteRole (Opcode 0x56 / 86)** / **S2C: DeleteRole_Re (Opcode 0x57 / 87)**:
  - C2S: `roleid`: `i32`, `localsid`: `u32`.
  - S2C: `result`: `i32` (0 = Sucesso), `roleid`: `i32`, `localsid`: `u32`.
- **C2S: SelectRole (Opcode 0x46 / 70)** / **S2C: SelectRole_Re (Opcode 0x47 / 71)**:
  - C2S: `roleid`: `i32`, `flag`: `i8`.
  - S2C: `result`: `i32` (0 = Sucesso), `auth`: `ByteVector` (permissões/GM). Aciona `LaunchLoading()` no cliente.
- **C2S: EnterWorld (Opcode 0x48 / 72)**:
  - Disparado pelo cliente após carregar os recursos do mapa e transicionar para `state_GDataExchgClient`.
  - `roleid`: `i32`, `provider_link_id`: `i32`, `locktime`: `i32`, `timeout`: `i32`, `settime`: `i32`, `localsid`: `u32`.

---

## 4. Isolamento de Personagens e Dados
- Cada personagem está estritamente vinculado à sua chave composta `(account_id, realm_id)`.
- Personagens criados no Realm 1.2.6 nunca colidem com dados do 1.4.8 ou 1.5.3 no banco unificado PostgreSQL.
