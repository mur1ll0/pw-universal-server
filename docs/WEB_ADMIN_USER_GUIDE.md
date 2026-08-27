# Manual do Usuário: Painel Web Administrativo (`pw-admin-web`)

O **`pw-admin-web`** é o painel de controle e gestão moderno que substitui completamente o antigo `pwAdmin` em PHP. Ele opera em tempo real comunicando-se com a API FastAPI (`porta 8000`), o banco **PostgreSQL 16** e a memória de cache **DragonflyDB**.

---

## 1. Como Acessar o Painel

1. Certifique-se de que o contêiner `pw-admin-api` está em execução:
   ```bash
   docker compose up -d pw-admin-api
   ```
2. Abra o arquivo `web-admin/frontend/index.html` em qualquer navegador (Chrome, Firefox, Edge, Safari) ou acesse `http://localhost:8000`.

---

## 2. Guia de Recursos por Aba

### 2.1 Visão Geral (Dashboard)
- **Métricas em Tempo Real**: Total de contas cadastradas, personagens criados e quantidade de jogadores online em cada Realm (Classic 1.2.6 e Eclipse 1.5.3).
- **Cards de Estado dos Servidores**: Mostra as portas públicas (29000 e 29001), multiplicadores ativos e status de conexão.

### 2.2 Gestão de Contas (pwAdmin)
- **Criar Nova Conta**: Cria uma conta de jogo global com usuário, senha, e-mail opcional e nível de GM.
- **Injeção de Gold/CUBI (`+ Gold`)**: Adiciona moedas de Cash/Gold imediatamente na conta, gravando no log de auditoria `admin_audit_logs`.
- **Reset de Senha**: Redefine a senha da conta com 1 clique, convertendo automaticamente o hash.
- **Banimento**: Aplica ban temporário (em horas) ou permanente, desconectando a sessão ativa.

### 2.3 Personagens & Itens
- **Busca Avançada**: Pesquise personagens por nome parcial e filtre por Realm (`realm_126` ou `realm_153`).
- **Inspetor de Inventário & Equipamentos**: Visualiza cada item em sua tabela normalizada (`character_items`), permitindo editar o refino (+0 a +12), quantidade, durabilidade e pedras espirituais sem risco de conflitar octets de outros slots.
- **Teletransporte de Emergência (CDD)**: Resgata personagens travados ou com bugs de colisão direto para a coordenada segura da Cidade do Dragão $(550.0, 200.0, 650.0)$.

### 2.4 Ligar / Desligar Mapas (Instâncias Dinâmicas)
- Permite ativar ou desativar em tempo real qualquer mapa do servidor (Mapa-Múndi, Dungeons FB19 a FB99, Frost, Dusk, Vale da Lua, Cubo, Morai).
- **Vantagem**: Reduz o consumo de CPU/RAM em servidores menores mantendo ativas apenas as dungeons que os jogadores estiverem utilizando.

### 2.5 Double Eventos & Multiplicadores
- Altere dinamicamente os multiplicadores de:
  - **EXP**: Experiência de monstros e missões (ex: 2.0x, 5.0x).
  - **Alma (SP)**: Pontos de habilidade.
  - **Drops**: Taxa de queda de itens dos monstros.
  - **Gold**: Moedas ganhas no chão.
- Os multiplicadores entram em vigor imediatamente no Realm sem precisar reiniciar o servidor!

### 2.6 Changelog & Patches CDN
- Exibe o histórico de todas as atualizações de cliente geradas pelo `pw-patch-tool`.
- Apresenta as notas de versão formatadas, tamanho do pacote em MB, data de lançamento e a soma de verificação SHA-256 oficial para integridade do download.

### 2.7 Anúncios de Sistema (Mensagem Amarela)
- Envia mensagens e avisos globais em amarelo para todos os jogadores conectados no Realm selecionado via DragonflyDB Pub/Sub.
