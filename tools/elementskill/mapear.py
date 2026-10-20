"""B305: mapa (habilidade, campo) → posição da tabela no arquivo elementskill.dll do cliente.

    python mapear.py entrada <habilidades.json> <cliente.jsonl> <entrada.txt>
    enderecos.exe elementskill.dll entrada.txt > enderecos.jsonl          (na pasta `element` do cliente)
    python mapear.py juntar <enderecos.jsonl> <elementskill.dll> <saida.json>

`cliente.jsonl` é a medição do `numeros.exe` (B303/B304). Para os inteiros, a sequência por nível é a do próprio
DLL. A mana o DLL devolve truncada (`GetMpCost` é `int`), então a sequência `float` vem do catálogo, só onde a parte
inteira bate com o DLL em todos os níveis. A saída guarda o SHA-256 do DLL: o mapa só vale para aquele arquivo.
"""
import collections
import hashlib
import json
import sys

INTEIROS = ("nivel_exigido", "sp_exigido", "dinheiro_exigido", "item_exigido", "execucao_ms", "recarga_ms")


def valor(r, campo):
    # A estática (`s_*`) quando houver; a do slot é a mesma (conferido em 20.676 níveis no B303).
    return r.get("s_" + campo, r.get(campo))


def entrada(catalogo, medicao, destino):
    h = json.load(open(catalogo, encoding="utf-8"))["habilidades"]
    cli = collections.defaultdict(dict)
    for linha in open(medicao, encoding="utf-8"):
        r = json.loads(linha)
        cli[r["id"]][r["nivel"]] = r
    linhas = []
    for i in sorted(cli):
        niveis = cli[i]
        n = max(niveis)
        if sorted(niveis) != list(range(1, n + 1)):
            continue
        for campo in INTEIROS:
            vs = [valor(niveis[m], campo) for m in range(1, n + 1)]
            if None in vs:
                continue
            linhas.append(f"{i} {campo} i {n} " + " ".join(str(v) for v in vs))
        mp = (h.get(str(i)) or {}).get("mp")
        dll = [niveis[m].get("mp") for m in range(1, n + 1)]
        if isinstance(mp, list) and len(mp) == n and None not in mp and None not in dll \
                and all(int(x) == y for x, y in zip(mp, dll)):
            linhas.append(f"{i} mp f {n} " + " ".join(repr(float(x)) for x in mp))
    with open(destino, "w", encoding="ascii") as f:
        f.write("\n".join(linhas) + "\n")
    print(f"{len(linhas)} linhas -> {destino}")


def juntar(enderecos, dll, destino):
    dados = open(dll, "rb").read()
    mapa = collections.defaultdict(dict)
    nao = collections.Counter()
    por_posicao = collections.defaultdict(list)
    for linha in open(enderecos, encoding="utf-8"):
        r = json.loads(linha)
        if r.get("igual_ao_arquivo"):
            mapa[str(r["id"])][r["campo"]] = {"arquivo": r["arquivo"], "tipo": r["tipo"], "n": r["n"]}
            por_posicao[r["arquivo"]].append((r["id"], r["campo"]))
        else:
            nao[r["campo"]] += 1
    # Tabela mais curta que o nível máximo (a 187: 12 níveis, tabelas de 10 no DLL): os últimos níveis leem a tabela
    # seguinte — o que se vê é lixo do original. Edita-se só até onde a próxima começa.
    ordem = sorted((t["arquivo"], i, c) for i, cs in mapa.items() for c, t in cs.items())
    for (p, i, c), (q, _, _) in zip(ordem, ordem[1:]):
        t = mapa[i][c]
        if q > p and p + 4 * t["n"] > q:
            t["le_alem"] = t["n"] - (q - p) // 4
            t["n"] = (q - p) // 4
    # Tabela lida por mais de uma habilidade (o compilador fundiu funções idênticas): editar uma muda as outras.
    compartilhadas = {str(p): sorted(set(q)) for p, q in por_posicao.items() if len(q) > 1}
    for p, quem in compartilhadas.items():
        for i, campo in quem:
            mapa[str(i)][campo]["compartilhada_com"] = [x for x in quem if x != (i, campo)]
    # O "resto": o arquivo com as tabelas zeradas. Um DLL editado só dentro das tabelas tem o mesmo resto — é assim
    # que o servidor reconhece uma cópia já editada num rascunho (o SHA do arquivo inteiro muda).
    resto = bytearray(dados)
    for cs in mapa.values():
        for t in cs.values():
            resto[t["arquivo"]:t["arquivo"] + 4 * t["n"]] = bytes(4 * t["n"])
    json.dump({
        "dll_sha256": hashlib.sha256(dados).hexdigest(),
        "dll_tamanho": len(dados),
        "sha256_do_resto": hashlib.sha256(resto).hexdigest(),
        "habilidades": mapa,
    }, open(destino, "w", encoding="utf-8"), ensure_ascii=False, separators=(",", ":"))
    conta = collections.Counter(c for v in mapa.values() for c in v)
    print(f"{sum(conta.values())} tabelas em {len(mapa)} habilidades -> {destino}")
    for c in sorted(set(conta) | set(nao)):
        print(f"  {c}: {conta[c]} no arquivo, {nao[c]} não editáveis")
    print(f"  tabelas compartilhadas: {len(compartilhadas)} posições")


def aplicar(mapa, dll, destino, id_, campo, nivel, novo):
    """Prova: troca um número numa cópia do DLL (`python mapear.py aplicar mapa.json dll copia.dll 8 dinheiro_exigido 10 123`)."""
    import struct
    m = json.load(open(mapa, encoding="utf-8"))
    dados = bytearray(open(dll, "rb").read())
    if hashlib.sha256(dados).hexdigest() != m["dll_sha256"]:
        sys.exit("o mapa é de outro elementskill.dll")
    t = m["habilidades"][id_][campo]
    pos = t["arquivo"] + 4 * (int(nivel) - 1)
    dados[pos:pos + 4] = struct.pack("<f" if t["tipo"] == "f" else "<i", float(novo) if t["tipo"] == "f" else int(novo))
    open(destino, "wb").write(dados)
    print(f"{id_} {campo} nível {nivel} = {novo} no byte {pos} -> {destino}")


if __name__ == "__main__":
    {"entrada": entrada, "juntar": juntar, "aplicar": aplicar}[sys.argv[1]](*sys.argv[2:])
