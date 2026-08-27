# Guia do Gerador de Atualizações CDN (`pw-patch-tool`)

O **`pw-patch-tool`** é o substituto moderno do antigo utilitário **CPW (Cup Package Writer)** do Perfect World. Ele gera patches diferenciais leves, calcula somas de verificação SHA-256 de cada arquivo e gera manifestos JSON para distribuição via CDN / HTTP com suporte a resumo de download.

---

## 1. Estrutura de Diretórios para Criar um Patch

Crie uma pasta de trabalho (ex: `updates/`) para manter a versão base e a versão modificada:

```
updates/
├── v10_base/             <-- Cópia exata dos arquivos do cliente atual (v10)
│   ├── element/
│   │   └── data/
│   │       ├── elements.data
│   │       └── gshop.data
│   └── surfaces.pck
│
├── v11_modified/         <-- Arquivos com as suas alterações (v11)
│   ├── element/
│   │   └── data/
│   │       ├── elements.data  (novos itens ou balanceamento)
│   │       └── gshop.data     (novas promoções da loja)
│   └── surfaces.pck           (novas texturas/ícones)
│
└── patch_notes_v11.txt   <-- Arquivo de texto com o Changelog da versão
```

---

## 2. Como Fazer o Arquivo de Notas de Versão (`patch_notes_v11.txt`)

Crie um arquivo `.txt` ou `.md` simples e direto descrevendo o que mudou na atualização:

```text
[NOTAS DA ATUALIZAÇÃO v11]
• Adicionado novo conjunto de equipamentos de refino no elements.data.
• Atualizada a loja de Gold (gshop.data) com novas montarias e roupas de evento.
• Corrigidos diálogos de NPCs na Cidade do Dragão.
• Aplicada otimização de texturas na interface gráfica.
```

---

## 3. Passo a Passo: Gerando o Patch com o `pw-patch-tool`

### Passo 1: Executar o comando `create-patch`
No terminal, execute o `pw-patch-tool` informando as pastas e as versões:

```bash
pw-patch-tool create-patch ./updates/v10_base ./updates/v11_modified 10 11 ./updates/patch_notes_v11.txt
```

### Passo 2: O que o `pw-patch-tool` faz automaticamente:
1. **Varredura SHA-256**: Compara todos os arquivos entre `v10_base` e `v11_modified`.
2. **Identificação Diferencial**: Separa apenas os arquivos que foram modificados ou adicionados (ignorando arquivos idênticos).
3. **Empacotamento `.cup`**: Compacta os arquivos alterados no pacote `ec_patch_10-11.cup`.
4. **Geração do `patch_manifest.json`**: Cria o catálogo oficial de atualização para a CDN.

---

## 4. Estrutura do Manifesto CDN (`patch_manifest.json`)

O arquivo gerado fica pronto para ser hospedado no seu servidor web / CDN (Nginx, Cloudflare, AWS S3):

```json
{
  "current_version": 10,
  "latest_version": 11,
  "cdn_base_url": "https://patch.seuservidor.com/updates/",
  "patches": [
    {
      "from_version": 10,
      "to_version": 11,
      "release_date": "2026-08-27T13:20:00Z",
      "release_notes": "Adicionado novo conjunto de equipamentos e atualização do GShop.",
      "package_file": "ec_patch_10-11.cup",
      "package_size_mb": 14.5,
      "package_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "changed_files_count": 4
    }
  ]
}
```

---

## 5. Como o Launcher / Patcher do Jogo Atualiza

1. O cliente abre o `patcher.exe`.
2. O launcher consulta `https://patch.seuservidor.com/updates/patch_manifest.json`.
3. Se a versão local for menor que `latest_version`, ele baixa apenas o pacote `ec_patch_10-11.cup`.
4. Valida a integridade do pacote contra o `package_sha256`.
5. Extrai os arquivos na pasta do jogo e atualiza o número de versão local.
6. O jogador pode ver as **Notas de Versão (Changelog)** diretamente no painel web ou na tela do launcher.
