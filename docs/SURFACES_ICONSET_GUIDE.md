# 🎨 Guia de Integração e Decodificação de Ícones (Surfaces & Iconset)

Este documento detalha o funcionamento, arquitetura e instruções para o uso de ícones do cliente do Perfect World (**Surfaces & Iconset**) no ecossistema `PW-Universal-Server` e no painel administrativo **Web-Admin**.

---

## 📌 1. Visão Geral e Arquitetura

No cliente do Perfect World, os ícones de itens, armas, armaduras, consumíveis, montarias e habilidades residem dentro do arquivo binário `element/surfaces.pck`. 

Para que o servidor e a interface administrativa web exibam as texturas oficiais de 32x32 pixels sem a necessidade de carregar o arquivo `.pck` completo (que pesa centenas de MBs com telas de carregamento e texturas 3D), o sistema adota a estratégia de **Atlas de Sprites Extraídos (Iconset)**.

```
                  ┌───────────────────────────────┐
                  │  elements.data (do Realm)     │
                  │  - Item #6: icon="钢刀.dds"   │
                  └──────────────┬────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ SurfacesIconManager (Backend Python FastAPI)                   │
│                                                                 │
│ 1. Lê data/{realm}/surfaces/iconset/iconlist_ivtrm.txt          │
│    -> Localiza "钢刀.dds" no índice #237 (Col 25, Linha 4)       │
│                                                                 │
│ 2. Abre data/{realm}/surfaces/iconset/iconlist_ivtrm.dds       │
│    -> Recorta quadrante 32x32: box=(800, 128, 832, 160)         │
│                                                                 │
│ 3. Retorna PNG via HTTP: GET /api/elements/icon/{realm}/{id}.png│
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ Frontend Web-Admin (index.html)                                 │
│                                                                 │
│ - Exibe <img src="/api/elements/icon/realm_126/6.png" />        │
│ - Se não encontrar o arquivo DDS: fallback para ícone vetorial! │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📂 2. Estrutura de Pastas no Servidor

Para que um Realm carregue seus ícones automaticamente ao inicializar, organize a pasta `data/` com a seguinte estrutura:

```text
pw-universal-server/data/
  ├── realm_126/
  │    ├── config/
  │    │    └── elements.data            <-- Banco binário de itens e receitas
  │    └── surfaces/
  │         └── iconset/
  │              ├── iconlist_ivtrm.dds   <-- Atlas de itens masculinos / gerais (1024x2048)
  │              ├── iconlist_ivtrm.txt   <-- Mapeamento de nomes de arquivos .dds
  │              ├── iconlist_ivtrf.dds   <-- Atlas de itens femininos
  │              ├── iconlist_ivtrf.txt   <-- Mapeamento
  │              ├── iconlist_skill.dds   <-- Atlas de habilidades (1024x1024)
  │              ├── iconlist_skill.txt   <-- Mapeamento de habilidades
  │              ├── iconlist_pet.dds     <-- Atlas de mascotes e pets
  │              ├── iconlist_pet.txt
  │              ├── iconlist_guild.dds   <-- Atlas de clãs e guildas
  │              └── iconlist_guild.txt
  │
  ├── realm_148/
  │    ├── config/
  │    │    └── elements.data
  │    └── surfaces/
  │         └── iconset/
  │
  └── realm_153/
       ├── config/
       │    └── elements.data
       └── surfaces/
            └── iconset/
```

> [!TIP]
> O tamanho de toda a pasta `surfaces/iconset` é de apenas **~6 MB**, permitindo upload e sincronização ultrarrápidos em contêineres Docker e servidores remotos.

---

## ⚙️ 3. Como Funciona o Mapeamento (Decoder)

### Formato do Arquivo `iconlist_*.txt`
O arquivo de texto possui o seguinte cabeçalho:
- **Linha 1**: Largura do ícone em pixels (padrão: `32`)
- **Linha 2**: Altura do ícone em pixels (padrão: `32`)
- **Linha 3**: Número de colunas por linha no atlas (ex: `53` para itens, `19` para skills)
- **Linha 4**: Número de linhas no atlas (ex: `64` para itens, `32` para skills)
- **Linhas 5 em diante**: Lista sequencial dos nomes de arquivo `.dds` dos itens.

### Cálculo de Coordenadas
Para um ícone no índice $I$ da lista:
$$\text{Coluna} = I \pmod{\text{Colunas}}$$
$$\text{Linha} = \lfloor I / \text{Colunas} \rfloor$$
$$\text{Bounding Box} = ( \text{Coluna} \times 32, \ \text{Linha} \times 32, \ (\text{Coluna} + 1) \times 32, \ (\text{Linha} + 1) \times 32 )$$

---

## 🌐 4. Rotas e Endpoints da API

O backend FastAPI disponibiliza os seguintes endpoints de alto desempenho com cabeçalho de cache (`Cache-Control: public, max-age=86400`):

| Endpoint | Método | Descrição |
| :--- | :--- | :--- |
| `/api/elements/icon/{realm_id}/{item_id}.png` | `GET` | Retorna o PNG 32x32 do item decodificado a partir do ID |
| `/api/elements/skill-icon/{realm_id}/{skill_id}.png` | `GET` | Retorna o PNG 32x32 da habilidade |
| `/api/elements/raw-icon/{realm_id}/{icon_filename}` | `GET` | Retorna o PNG 32x32 pelo nome do arquivo `.dds` original |

---

## 🛡️ 5. Sistema de Fallback Gracioso

Caso um realm não possua os arquivos de iconset instalados, ou um item customizado não tenha textura no atlas:
1. A API retorna `icon_img: null` ou código HTTP 404.
2. O componente no frontend detecta a ausência ou erro (`onerror`) e ativa instantaneamente o **ícone vetorial categorizado (FontAwesome / SVG)** com a cor de qualidade correspondente.
3. A interface nunca fica quebrada nem exibe ícones corrompidos.
