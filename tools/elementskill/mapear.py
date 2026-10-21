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


def relocacoes(dados):
    """B311: alvo (RVA) → sítios que o referenciam, pela tabela de relocações (`.reloc`, IMAGE_REL_BASED_HIGHLOW).
    Num DLL x86 toda referência absoluta a um dado está aqui — é como se prova quem lê uma constante."""
    import struct
    pe = struct.unpack_from("<I", dados, 0x3C)[0]
    nsec = struct.unpack_from("<H", dados, pe + 6)[0]
    opt = pe + 24
    imagem = struct.unpack_from("<I", dados, opt + 28)[0]
    rrva, rtam = struct.unpack_from("<II", dados, opt + 96 + 5 * 8)
    tam_opt = struct.unpack_from("<H", dados, pe + 20)[0]
    secoes = [struct.unpack_from("<IIII", dados, opt + tam_opt + 40 * i + 8) for i in range(nsec)]

    def arq(rva):
        for vs, va, rs, rp in secoes:
            if va <= rva < va + rs:
                return rva - va + rp
        raise ValueError(f"RVA {rva:#x} fora do arquivo")

    alvos = collections.defaultdict(list)
    p, fim = arq(rrva), arq(rrva) + rtam
    while p < fim:
        pagina, tam_bloco = struct.unpack_from("<II", dados, p)
        if tam_bloco == 0:
            break
        for k in range((tam_bloco - 8) // 2):
            e = struct.unpack_from("<H", dados, p + 8 + 2 * k)[0]
            if e >> 12 == 3:
                sitio = pagina + (e & 0xFFF)
                alvos[struct.unpack_from("<I", dados, arq(sitio))[0] - imagem].append(sitio)
        p += tam_bloco
    return alvos


def juntar(enderecos, dll, destino):
    dados = open(dll, "rb").read()
    alvos = relocacoes(dados)
    somente_leitura_recusadas = 0
    mapa = collections.defaultdict(dict)
    nao = collections.Counter()
    por_posicao = collections.defaultdict(list)
    for linha in open(enderecos, encoding="utf-8"):
        r = json.loads(linha)
        if r.get("igual_ao_arquivo") and r.get("somente_leitura"):
            # B311: em `.rdata` o compilador funde constantes iguais. Só entra a tabela lida por exatamente um sítio
            # (na base, `base` ou `base − 4` = `array[nível − 1]`) e por nenhum pelo meio; a mana da 121, que derrubou o
            # DLL no B305, é uma constante lida por 3 sítios.
            x, n = r["rva"], r["n"]
            na_base = sum(len(alvos.get(x + d, [])) for d in (0, -4))
            no_meio = sum(len(alvos.get(x + 4 * k, [])) for k in range(1, n))
            if na_base != 1 or no_meio:
                somente_leitura_recusadas += 1
                nao[r["campo"]] += 1
                continue
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
    # B311: posição lida por mais de uma habilidade = funções idênticas que o compilador fundiu (no 1.2.6, vários
    # `GetMpcost` que devolvem a mesma constante viram uma função só). Editar uma mudaria as outras: fica fora.
    for p, quem in compartilhadas.items():
        for i, campo in quem:
            del mapa[str(i)][campo]
            nao[campo] += 1
    for i in [i for i, cs in mapa.items() if not cs]:
        del mapa[i]
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
    print(f"  posições compartilhadas por mais de uma habilidade (excluídas): {len(compartilhadas)}")
    print(f"  em .rdata recusadas (constante lida por mais de um sítio ou pelo meio): {somente_leitura_recusadas}")


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


CAMPOS_MEDIDOS = ("nivel_exigido", "sp_exigido", "dinheiro_exigido", "item_exigido", "mp", "recarga_ms", "execucao_ms",
                  "apcost", "arrowcost", "cls", "max_level")


def provar(mapa, dll, destino):
    """Prova de ponta a ponta: +1 em todos os elementos de todas as tabelas numa cópia (medir depois com numeros.exe)."""
    import struct
    m = json.load(open(mapa, encoding="utf-8"))
    d = bytearray(open(dll, "rb").read())
    if hashlib.sha256(d).hexdigest() != m["dll_sha256"]:
        sys.exit("o mapa é de outro elementskill.dll")
    for cs in m["habilidades"].values():
        for t in cs.values():
            for k in range(t["n"]):
                p = t["arquivo"] + 4 * k
                if t["tipo"] == "f":
                    struct.pack_into("<f", d, p, struct.unpack_from("<f", d, p)[0] + 1)
                else:
                    struct.pack_into("<i", d, p, struct.unpack_from("<i", d, p)[0] + 1)
    open(destino, "wb").write(d)
    print(f"+1 em {sum(len(c) for c in m['habilidades'].values())} tabelas -> {destino}")


def conferir(mapa, original, copia):
    """Compara a medição do DLL original com a da cópia do `provar`: só o que o mapa diz pode ter subido 1."""
    m = json.load(open(mapa, encoding="utf-8"))["habilidades"]
    a = [json.loads(x) for x in open(original, encoding="utf-8")]
    b = [json.loads(x) for x in open(copia, encoding="utf-8")]
    esperado = certo = errado = 0
    for x, y in zip(a, b):
        for c in CAMPOS_MEDIDOS:
            k = "s_" + c if "s_" + c in x else c
            if k not in x:
                continue
            t = m.get(str(x["id"]), {}).get(c)
            # Nível além da tabela (`le_alem`) lê a seguinte, que também subiu 1 se for mapeada.
            sobe = 1 if t else 0
            esperado += sobe
            if y.get(k) == x[k] + sobe:
                certo += 1
            else:
                errado += 1
                if errado <= 10:
                    print("  errado:", x["id"], x["nivel"], c, x[k], y.get(k))
    print(f"linhas {len(a)} / {len(b)} | +1 esperados {esperado} | certos {certo} | errados {errado}")


if __name__ == "__main__":
    {"entrada": entrada, "juntar": juntar, "aplicar": aplicar, "provar": provar, "conferir": conferir}[sys.argv[1]](*sys.argv[2:])
