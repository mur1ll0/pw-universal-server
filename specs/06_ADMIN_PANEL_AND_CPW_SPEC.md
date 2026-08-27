# Especificação 06: Painel Web de Administração (pwAdmin Moderno) e CPW Patcher

## 1. Painel Web de Administração (`pw-admin-web`)

O painel administrativo substitui as ferramentas antigas em PHP/Perl por uma aplicação web responsiva moderna:
- **Frontend**: Next.js 14 (App Router) + TailwindCSS + Lucide Icons + Shadcn UI (Dark Theme nativo).
- **Backend API**: FastAPI (Python) ou Actix-web (Rust) com autenticação JWT e controle de permissões por nível (RBAC).

```
+---------------------------------------------------------------------------------------------------+
|                                 PW-ADMIN-WEB: MAPA DE FUNCIONALIDADES                             |
+---------------------------------------------------------------------------------------------------+
| 1. Dashboard em Tempo Real                                                                        |
|    • Jogadores online por Realm (Gráficos ao vivo via WebSocket).                                 |
|    • Ticks por segundo (TPS) e uso de memória RAM/CPU dos mundos.                                 |
|    • Mapa de calor (Heatmap) de aglomeração de jogadores nas cidades.                             |
|                                                                                                   |
| 2. Gestão de Contas e Usuários                                                                    |
|    • Criar contas, resetar senhas com 1 clique.                                                   |
|    • Banir/Desbanir contas com motivo e data de expiração.                                        |
|    • Conceder/Revogar permissões de Game Master (GM níveis 1 a 32).                               |
|                                                                                                   |
| 3. Gestão Econômica e Billing                                                                     |
|    • Injetar CUBI / GOLD / Cash por conta ou em lote (com histórico de auditoria).                |
|    • Relatórios de circulação de moedas e transações de leilão.                                   |
|                                                                                                   |
| 4. Inspetor & Editor de Personagens ao Vivo                                                        |
|    • Busca de personagem por nome, ID ou conta.                                                   |
|    • Editor visual de Inventário e Armazém (arrastar/soltar itens, alterar refino +1..+12).       |
|    • Edição de Cultivo, Nível, HP/MP, Moedas e Reputação.                                         |
|    • Botão de Teletransporte de Emergência (reseta posição para Cidade do Dragão).                |
|                                                                                                   |
| 5. Controle de Eventos de Servidor (Multiplicadores ao Vivo)                                      |
|    • Toggle dinâmico de Double EXP, Double SP, Double Drop e Double Gold por Realm.               |
|                                                                                                   |
| 6. Gerenciador Dinâmico de Mapas e Instâncias                                                     |
|    • Listagem de todos os mapas e dungeons (World, a01..a33, b01..b35, Morai, etc.).             |
|    • Ativação/Desativação de instâncias sob demanda com 1 clique para economizar memória.         |
|                                                                                                   |
| 7. Transmissão Global e Mensagens de Sistema                                                      |
|    • Envio de Avisos Globais de Sistema (anúncios amarelos no topo da tela dos jogadores).        |
|    • Envio de Correio em Massa (SysMail com itens anexados para todos os jogadores online).       |
+---------------------------------------------------------------------------------------------------+
```

---

## 2. Modernização do CPW (Gerador de Patches e Auto-Updater CDN)

A ferramenta `pw-patch-tool` moderniza o processo arcaico de geração de arquivos `.cup`:

### 2.1 Gerador de Patches Diferenciais
- Compara a pasta do cliente base com a pasta do cliente atualizado.
- Descompacta e compara internamente os arquivos `.pck` (`surfaces.pck`, `models.pck`, `configs.pck`).
- Empacota apenas as diferenças em um arquivo `.cup` comprimido com **Zstandard (zstd)** de alta taxa de compressão e descompressão instantânea.

### 2.2 Manifesto de Atualização para CDN / HTTP
Gera um arquivo de manifesto `patch_manifest.json` para ser hospedado em qualquer servidor web ou CDN (Cloudflare, AWS S3, Nginx):

```json
{
  "current_version": 153,
  "min_supported_version": 145,
  "patches": [
    {
      "from_version": 145,
      "to_version": 153,
      "file_name": "ec_patch_145-153.cup",
      "file_size": 48920150,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "download_url": "https://cdn.meupw.com/patches/ec_patch_145-153.cup"
    }
  ]
}
```

### 2.3 Auto-Patcher Inteligente
- Suporte nativo a *HTTP Range Requests* (download pausável e resumível).
- Verificação de integridade automática por SHA-256 antes da aplicação.
- Interface moderna e leve em Rust (`egui`/`tauri`) ou C#.
