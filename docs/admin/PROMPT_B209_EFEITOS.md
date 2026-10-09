# Prompt — B209: efeitos de equipamento com o valor do id, por família e por realm

Continue a reforma do pw-admin no repositório F:\Python_C_Projects\PWSource1.5.3\pw-universal-server (branch main).
Objetivo deste bloco: **os efeitos de equipamento editados pelo painel funcionarem como no jogo** — valor certo
vindo do id, efeito só na peça certa, os efeitos parados passando a agir, e a ficha mostrando o que existe em cada
versão.

LEIA PRIMEIRO (só isto): skill pw-retomar-sessao; docs/ESTADO_E_RETOMADA.md §5, item "Efeitos de equipamento
(diagnóstico de 2026-10-08)"; docs/admin/MEMORIA_DA_REFORMA.md §6.14; specs/05 §7.11 (B194 editar item, B204
efeito de essência) e §5.1 (atributos do equipamento); specs/06 (modal Editar item, B206). Histórico: só B194,
B204 e B206, por grep.

ESTADO AO ABRIR
- A árvore tem trabalho **sem commit**: B199–B208 (troca de rosto, brilho das pedras, vínculo, regras de vestir,
  passivas comuns, efeito de essência, `bAutoRevive`, busca de efeitos/pedras e Furos, Aura de Aço, portais de
  região). Não commite nem publique sem o Murillo pedir; se precisar de base limpa, pergunte antes.
- Confirmado em jogo pelo Murillo: dar/mover/equipar/editar item, dinheiro, habilidades (1.5.5); EXP/SP, nível,
  cultivo, posição, pontos, dar missão (1.2.6); sair da Caverna das Sombras pelo altar (B208).

O DIAGNÓSTICO (já feito; confira o que for usar, com arquivo:linha)
- Teste do Murillo no 1.5.5, personagem RT (id 11456, `realm_155`), arma 44995 e peças do corpo. Decodificar:
  `cargo run -p pw-gs --example dump_item_octetos -- data/realm_155/config <item> <hex>` e o que soma ao vestir:
  `cargo run -p pw-gs --example bonus_do_equipamento -- data/realm_155/config <item> <slot> <hex>`. O hex sai do
  banco (só leitura): `SELECT encode(extra_data,'hex') FROM public.character_items WHERE character_id=11456 AND
  container_type=1 AND slot=N`.
- Efeitos que o Murillo pôs e não apareceram:
  | efeito | id e valor posto | situação |
  | :--- | :--- | :--- |
  | Acerto | 1317 = 999 | `enhance_attack_addon_2`: o GS soma em `attack_rate` e manda no `OWN_EXT_PROP` (a ficha mostra `rep.ak.attack`, `DlgCharacter.cpp:485`). O id vale **118 fixo** (`param1 = param2 = 118`). Conferir em jogo se o número muda; se não, investigar. |
  | Nível de ataque | 2029 = 30 | `enhance_attack_degree`: o GS soma (`attack_degree`), mas o `OWN_EXT_PROP` do 1.5.5 manda **zero fixo** nesse campo (`crates/pw-protocol/src/versions/v155/mod.rs`, `own_ext_prop`: attack_degree, defend_degree, crit_rate, crit_damage_bonus, invisible_degree, anti_invisible_degree, penetration, resilience, vigour, anti_defense_degree, anti_resistance_degree = 0). |
  | Tempo de conjuração | 332 = 50 | `reduce_cast_time_addon` **sem porte** (log `addons sem porte`). O id vale 0,03 em float → 3 (%) (`arg_addon<PERCENT>`). |
  | Movimento (bota) | 286 = 50 | `enhance_speed_addon` **sem porte**; o id vale 0,05 → **5** (%) e age em `_en_percent.walk_speed/run_speed` (`item/item_addon.cpp:322-355`). |
  | (elmo) | 831 | `enhance_weapon_max_magic_addon` — efeito de essência **de arma**; numa armadura não age nem no original. |
- Efeitos que **funcionaram** com valor livre (o Murillo quer manter editáveis): defesa +80 na armadura
  (652 `IA_EA_ESS<offsetof(armor_essence,defense)>`) e MP +999 no elmo (276
  `IA_EA_ESS<offsetof(armor_essence,mp_enhance)>`) — efeitos de essência aplicados pelo B204.
- Tamanho: dos 2.911 ids de `EQUIPMENT_ADDON` do 1.5.5 (`specs/addons_155/addons.json`, id → tratador), o GS
  soma 925 ao vestir (`BonusDeAddons::somar`, `crates/pw-gs/src/entity.rs`), 210 são refino, 493 essência
  (`geracao::aplicar_na_essencia_com`); **1.283 ids de 180 tratadores não fazem nada**. Os mais comuns:
  `item_skill_addon` 72, `SET_ADDON_MACRO(n, …)` (conjuntos) ~200, `reduce_cast_time_addon` 23, `IDMRA(…)` 90,
  `IAERA3(…)` 75, `enhance_penetration` 17, `enhance_vigour` 15, `enhance_resilience` 14,
  `item_armor_enhance_all_resistance` 13, `STONE_MAGIC_DMG_ADDON(…)`. Recontar com o script do diagnóstico
  (filtra os nomes do `somar`, `refine_*` e os prefixos de essência).

DECISÕES DO MURILLO (2026-10-08)
1. **Valor vem do id.** Ao pôr um efeito, o GS gera os argumentos como o original ao gerar o item
   (`GenerateParam` de cada `arg_addon<…>`, `item/item_addon.cpp`; o nosso porte é `geracao::gerar_addon` +
   `DadosDoAddon::sorteio`, `crates/pw-data-loader/src/addons.rs`, que hoje só cobre os tratadores com porte —
   estender a todos os tipos de parâmetro).
2. **Editável quando o efeito permite.** Os que somam um número inteiro direto (como os de essência
   `IA_EA_ESS`/`IA_ED_ESS`, e os de ponto `EPSA_addon<…, POINT|DOUBLE_POINT>`) continuam com o valor editável
   (o padrão vem do id). Os de codificação especial (float convertido, `PERCENT`/`DOUBLE_PERCENT` se a escala
   confundir, ids de habilidade em `item_skill_addon`, conjuntos `SET_ADDON_MACRO`, `DOUBLE_FIX_POINT`, os que
   gravam dois argumentos com significados diferentes) ficam com o valor **fixo do id**. **Identifique pelo tipo
   de parâmetro do tratador no fonte** (`arg_addon<POINT|DOUBLE_POINT|PERCENT|DOUBLE_PERCENT|DOUBLE_FIX_POINT|…>`
   e as classes próprias) e grave a classificação num mapa gerado (como `specs/addons_155/addons.json`), com o
   script versionado. **Se não for possível classificar com evidência, deixe a edição habilitada para todos**,
   com o valor do id preenchido e a faixa mostrada.
3. **Efeito só na família certa** (arma, armadura, acessório; os de essência já são por família). A busca filtra
   pela peça aberta; o GS recusa o resto (`efeito_de_outra_familia`). Fonte da família: o prefixo do tratador e
   as listas de efeitos que cada `WEAPON/ARMOR/DECORATION_ESSENCE` sorteia — conferir no `elements.data`.
4. **Por realm.** O 1.2.6 **não tem** nível de ataque/defesa, penetração, resiliência, vigor nem o bloco
   correspondente da ficha: o `OWN_EXT_PROP` do 1.2.6 vai direto de `status_point` para `ROLEEXTPROP_BASE`
   (`crates/pw-protocol/src/versions/v126/mod.rs`, `own_ext_prop`). Implemente só o que existe em cada versão:
   - 1.5.5: mandar no `OWN_EXT_PROP` os valores reais dos campos que hoje vão zero (conferir a ordem no
     `gs/player.cpp` que monta o `ROLEEXTPROP` e no cliente, `EC_HostMsg.cpp`/`DlgCharacter.cpp`);
   - 1.2.6: não mandar o que o pacote não tem; os efeitos desses tipos no 1.2.6 — conferir no
     `EQUIPMENT_ADDON` do v7 (`data/realm_126`) e no `gs` 1.2.6 (`files1.2.6/pwserver/gamed/gs`, ELF com símbolos)
     se existem; os que não existem não aparecem na busca nem são aceitos.
5. **Portar os tratadores parados em etapas**, cada um conferido no fonte (`item/item_addon*.cpp`):
   (a) atributo simples: `enhance_speed_addon` (velocidade %), `reduce_cast_time_addon` (`DecPrayTime`, ligar ao
   tempo de conjuração das habilidades), `enhance_crit_rate`/`SET_ADDON_MACRO(..., enhance_crit_rate)`,
   penetração, resiliência, vigor, `IDMRA`/`IAERA3` (resistências), `item_armor_enhance_all_resistance`;
   (b) conjuntos (`SET_ADDON_MACRO`: precisam do reconhecimento das peças do conjunto vestidas);
   (c) `item_skill_addon` (habilidade dada pelo item). Pare e pergunte antes de (b) e (c) se o porte for grande.

O PAINEL
- Modal Editar item (`web-admin/backend/static/painel.js`, `linhaDeEfeito`, busca `ei-busca-efeito`; API
  `GET /api/efeitos?busca`, `GET /api/efeitos/{id}?args` em `painel/consultas.py`, `catalogo_de_efeitos` em
  `painel/dica.py`). Hoje a busca mostra o texto com parâmetros 0, então ids diferentes parecem iguais.
  Mudar para: a busca pede ao GS (canal administrativo, consulta nova) os ids com o valor gerado e o tipo de
  parâmetro, filtrados pela família da peça; mostra "Acerto +118", "Acerto +138"; ao escolher, a linha vem com o
  valor do id; editável só quando o efeito permite (campo bloqueado e com a faixa ao lado, senão).
- O quadro de resposta do canal tem 64 KiB: pagine ou limite a 30 resultados.
- `EdicaoDeItem` (`crates/pw-gs/src/bus_server/item_editado.rs`): validar no GS — valor de efeito fixo diferente
  do id → `efeito_valor_invalido`; efeito de outra família → `efeito_de_outra_familia`; tratador inexistente no
  realm → `efeito_inexistente`.
- As peças da RT já editadas com valores errados (999, 50) ficam como estão; documentar que basta reabrir e
  salvar.

REGRAS QUE VALEM
- Evidência do original com arquivo:linha; o binário do cliente é o juiz; S2C de tamanho errado é descartado em
  silêncio — o `OWN_EXT_PROP` passa pela skill pw-protocolo-cliente.
- Testes Rust só com `TEST_DATABASE_URL=postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database`.
  Painel: venv `C:\Users\Murillo\AppData\Local\Temp\pw-admin-reforma-venv`, dentro de `web-admin/backend/tests`,
  `TEST_REDIS_URL=redis://localhost:6379`, `PW_VALIDADOR_CREDENCIAIS=<repo>\target\debug\pw-validar-credenciais.exe`,
  `python -m unittest teste_base teste_dica`; Node `node --test web-admin/backend/tests/teste_fluxo_contas.cjs`.
- Não rode a suíte do painel junto com a do cargo (disputam o banco e o validador).
- Suíte inteira só no fim, em segundo plano, filtrada; intermitentes conhecidos: convite de grupo, roupa,
  `o_guia_selvagem…1177`, `o_primeiro_teste_do_guerreiro_126`.
- Scripts de edição em arquivo no rascunho, rodados com `python <arquivo>`.
- Uma fatia por vez, com roteiro de tela no fim; specs 05 §5.1/§7.11, 04 (OWN_EXT_PROP), 06, estado, memória e
  histórico (B209 em diante) atualizados; tabela de consumo por etapa.

PRIMEIRA ENTREGA: a classificação dos tratadores (editável × fixo × família × existe no 1.2.6), com a evidência
de cada grupo e os números, e a ordem das fatias. Implementar depois da resposta do Murillo se sobrar decisão.

ROTEIRO DE TESTE QUE O MURILLO VAI FAZER (para orientar o fim de cada fatia)
- 1.5.5, RT: armadura com Def editável (+80) e elmo com MP (+999) continuam somando; Acerto 1317 entra com +118
  fixo e a ficha sobe; bota com 286 → +5% de velocidade; arma com 332 → conjuração mais curta; nível de ataque
  aparece na ficha; 831 não aparece na busca de efeitos de um elmo.
- 1.2.6: a busca não oferece nível de ataque nem penetração; os efeitos que existem lá funcionam.
