# pw-crash-re — engenharia reversa do crash do elementclient, sem PDB e sem IDA

Ferramentas escritas em 2026-09-03 pra atacar o crash de render do client 1.5.5
(`ACCESS_VIOLATION` no primeiro frame depois de entrar no mundo). Dependem só de
`capstone` (já instalado nesta máquina: `python -c "import capstone"`).

## `stackwalk.py` — reconstrói a cadeia de chamadas de um minidump

```bash
python stackwalk.py <ec_buildNNNN.dmp> <ELEMENTCLIENT.EXE>
```

Lê o `MINIDUMP_EXCEPTION_STREAM` (registradores + endereço da falha), acha a região de
memória que contém o `ESP`, e varre a pilha. **Um DWORD só é aceito como endereço de
retorno se os bytes imediatamente antes dele, no `.exe` em disco, forem uma instrução
`CALL`** (`E8 rel32`, `FF 15 abs`, `FF D0-D7`, `FF 50+r disp8`, `FF 90+r disp32`,
`FF 10-17`). Isso é verificação, não palpite — o resultado saiu **idêntico frame a frame**
nos dois builds (BR 2569 e EN 2575), o que por si só já prova que é o mesmo caminho de
código nos dois.

## `disasm.py` / `dis2.py` — desmontagem com resolução de string

```bash
python disasm.py <exe> fn    <VA>        # tenta achar o início da função e desmontar
python disasm.py <exe> range <VA> <n>    # n instruções a partir do VA
python dis2.py   <exe> <VA> [antes] [depois]   # acha o alinhamento certo e mostra o contexto
```

`dis2.py` é o mais útil: dado um VA no meio de uma instrução conhecida, ele testa
alinhamentos até achar um fluxo de instruções que **cai exatamente** naquele endereço, e
desmonta a janela em volta anotando toda referência a string ASCII/UTF-16 do binário. Foi
assim que `CECGameRun::LoadConfigsFromServer` e `CECGameSession::OnPrtcGetConfigRe` foram
identificados por nome, sem símbolo nenhum.

Detalhe de formato: este binário usa **`0x90` (nop) como padding entre funções**, não
`0xCC` — a busca pelo início de função tem que procurar `90` seguido de não-`90`.

## `npcgen_ids.py` — inventário de `template_id` de um pacote de mapas

```bash
python npcgen_ids.py <pasta com a01/, a02/, world/...> [saida.json]
```

Espelha `crates/pw-data-loader/src/npcgen.rs` e **exige que o offset final bata exatamente
com o tamanho do arquivo** — se sobrar um byte, a leitura está errada e ele avisa. Usado
pra medir a compatibilidade de um pacote de servidor com o `elements.data` de um client
(ver item 12 de `docs/ESTADO_E_RETOMADA.md`).
