# Especificação 06: Painel administrativo e patcher

> Verificada contra o código em 2026-09-14, commit `e6433ae`. Cobre `web-admin/` e
> `tools/pw-patch-tool/`. **Frente de prioridade 3** (Contextos G e H do roadmap): nada aqui
> foi revisado desde a mudança de base para o 1.5.5, exceto o leitor de `elements.data`.

## 1. Painel (`web-admin/`) — existe, sem revisão

| peça | o que é |
| :--- | :--- |
| `backend/main.py` | FastAPI (~1.400 linhas), acesso direto ao PostgreSQL; serviço `pw-admin-api`, porta **8000** |
| `backend/elements_decoder.py` | ícones e busca de itens/habilidades; usa `specs/elements_layouts/pw_elements_reader.py` (montado read-only no contêiner) com fallback para o formato v7 |
| `frontend/index.html` | página única estática (~3.200 linhas), servida pelo backend em `/` |
| manual | `docs/WEB_ADMIN_USER_GUIDE.md` (escrito para 1.2.6/1.5.3; desatualizado) |

Endpoints existentes (`/api/...`):

| grupo | rotas |
| :--- | :--- |
| contas | `accounts/create`, `reset-password`, `set-gm`, `grant-gold`, `ban`, `list` |
| personagens | `characters/search`, `{id}`, `{id}/edit-stats`, `{id}/teleport`, `{id}/teleport-cdd` |
| itens | `characters/{id}/items/add`, `move`, `unequip`; `items/{inst}/edit`, `DELETE items/{inst}` |
| habilidades | `characters/{id}/skills/add`, `edit`, `learn-all`, `import-hex`, `DELETE skills/{id}` |
| elements | `elements/search-items`, `item/{id}`, `search-skills`, ícones; `elements|skills/encode-octets`, `decode-octets` |
| realms e mapas | `realms/list`, `set-multipliers`, `broadcast`; `realms/{id}/maps`, `maps/toggle`, `toggle-all` |
| moldes de classe | `templates/list`, `templates/{realm}/{cls}`, `.../save` |
| outros | `metrics`, `patches/changelog` |

Limitações conhecidas:

- **Sem autenticação** — não expor a porta 8000 fora da máquina.
- Escreve direto no banco com o servidor rodando: personagem em jogo tem o estado no `pw-gs`
  e o autosave de 60 s sobrescreve edições de nível/exp/moedas/posição.
- "Multiplicadores" e "mapas ligados" gravam no banco e **nenhum daemon lê** (o `nonce` do
  `Challenge` vai zerado, spec 02 §3.4); "broadcast" publica no Redis `chat:<realm>:world`,
  que só o `pw-delivery` escuta — e ele não roda.
- Moldes de classe: as colunas de atributo de `class_templates` são ignoradas pelo servidor
  (quem manda é o `ptemplate.conf`) — decidir antes de expor edição.

`planejado` (visão original, não iniciado): dashboard em tempo real por WebSocket (online,
TPS), RBAC com JWT, correio em massa com anexos, auditoria navegável, frontend em framework.

## 2. Patcher (`tools/pw-patch-tool/`) — protótipo

CLI Rust de ~140 linhas (`docs/PW_PATCH_TOOL_GUIDE.md`):

| comando | faz |
| :--- | :--- |
| `scan <dir_cliente>` | catálogo SHA-256 dos arquivos |
| `create-patch <dir_v1> <dir_v2> <v1> <v2> [notas]` | pacote `.cup` + `patch_manifest.json` |
| `list-patches` | histórico |

`planejado`: diferença **dentro** dos `.pck` (atenção: `models.pck`/`litmodels.pck`/`building.pck`
passam de 2 GB e o motor não os abre — `tools/pw-pck-extract/` lê), compressão zstd,
download retomável com HTTP Range, verificação por SHA-256 antes de aplicar, launcher
gráfico, e atualizar `serverlist.txt` (UTF-16LE com BOM) e o `.bat` com `logiccheck:0`.
