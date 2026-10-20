"""B303: correções do catálogo de habilidades pelo `elementskill.dll` do cliente 1.5.5 BR (v156).

O `habilidades.json` sai dos stubs do fonte (`EvolvedPWServer/cskill/skills`), mas o binário é o juiz: o
`elementskill.dll` instalado e o `gs` do pacote de servidor da mesma versão (`pwserver_155v156/home/pwserver/gamed/gs`)
trazem as mesmas tabelas de dinheiro — e nenhuma das do fonte (ex.: a sequência int32 da habilidade 8,
`0,0,0,6980,…,771980`, aparece 30× no `gs` e 29× no DLL; a do fonte para a habilidade 1, `0,30,130,…`, 0× nos dois).

Uso:
    tools/elementskill/compilar.bat
    (na pasta `element` do cliente) numeros.exe elementskill.dll > cliente.jsonl
    python medir_elementskill.py cliente.jsonl

Grava `elementskill_v156.json` ao lado: `{id: {campo: valor}}` só com o que o binário diz diferente do fonte (ou o
que o fonte não deu). O `extrair_habilidades.py` aplica esse arquivo no fim da extração.
"""
import collections
import json
import os
import sys

AQUI = os.path.dirname(os.path.abspath(__file__))
# Valor por nível, lido pela vtable do objeto (`numeros.cpp`). `mp` o DLL devolve como int (`GetMpCost`, trunca):
# só se corrige quando a parte inteira difere.
POR_NIVEL = ("mp", "recarga_ms", "execucao_ms", "nivel_exigido", "sp_exigido", "dinheiro_exigido", "item_exigido")
# Requisitos: sem sobrescrita no stub, a base devolve 0 (`ElementSkill/skill.h:170`); o fonte deixou `null` e o
# servidor recusava aprender. Para eles o DLL preenche o `null`. Custos com `null` dependem de mais que o nível — o
# DLL foi chamado sem jogador, então não se usa.
PREENCHE_NULL = ("nivel_exigido", "sp_exigido", "dinheiro_exigido", "item_exigido")
ESCALARES = ("apcost", "arrowcost", "cls")


def main():
    cli = collections.defaultdict(dict)
    with open(sys.argv[1], encoding="utf-8") as f:
        for linha in f:
            r = json.loads(linha)
            cli[r["id"]][r["nivel"]] = r
    with open(os.path.join(AQUI, "habilidades.json"), encoding="utf-8") as f:
        catalogo = json.load(f)["habilidades"]
    # O catálogo já pode ter sido corrigido: compara com o fonte puro guardado em `do_fonte`.
    correcoes = {}
    conta = collections.Counter()
    so_relato = []
    for sid, h in sorted(catalogo.items(), key=lambda x: int(x[0])):
        i = int(sid)
        if i not in cli:
            continue
        fonte = dict(h)
        fonte.update(h.get("do_fonte") or {})
        niveis = cli[i]
        c = {}
        if niveis[1]["max_level"] != fonte["max_level"]:
            # Mudar o nível máximo pede todos os campos dos níveis novos (dano, alcance…); fica só no relato.
            so_relato.append((i, fonte["max_level"], niveis[1]["max_level"]))
        for k in ESCALARES:
            if fonte.get(k) is not None and fonte[k] != niveis[1][k]:
                c[k] = niveis[1][k]
        n = fonte["max_level"]
        if all(m in niveis for m in range(1, n + 1)):
            for k in POR_NIVEL:
                v = fonte.get(k)
                dll = [niveis[m][k] for m in range(1, n + 1)]
                if v is None:
                    if k in PREENCHE_NULL:
                        c[k] = dll
                    continue
                if any(x is not None and int(x) != y for x, y in zip(v, dll)):
                    c[k] = [y if x is not None and int(x) != y else x for x, y in zip(v, dll)]
        if c:
            correcoes[sid] = c
            for k in c:
                conta[k] += 1
    destino = os.path.join(AQUI, "elementskill_v156.json")
    with open(destino, "w", encoding="utf-8") as f:
        json.dump({
            "fonte": "elementskill.dll do cliente 1.5.5 BR v156 (tools/elementskill/numeros.cpp)",
            "nivel_maximo_diferente": [{"id": i, "fonte": a, "dll": b} for i, a, b in so_relato],
            "correcoes": correcoes,
        }, f, ensure_ascii=False, separators=(",", ":"))
    print(f"{len(correcoes)} habilidades corrigidas -> {destino}")
    for k, v in sorted(conta.items()):
        print(f"  {k}: {v}")
    print(f"  nível máximo diferente (só relato): {len(so_relato)}")


if __name__ == "__main__":
    main()
