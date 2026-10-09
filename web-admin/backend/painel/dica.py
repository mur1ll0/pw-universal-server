"""Dica (tooltip) de item, montada como o cliente monta (B189, parte 1).

Ordem e cores de `CECIvtrWeapon::GetNormalDesc` (`EC_IvtrWeapon.cpp:271-400`) e afins: nome na
cor do `item_color.txt` (com furos, `ITEMDESC_NAMESOCKET`; refino ` +N`), valores da arma/
armadura/ornamento, durabilidade (vermelha se zero; a tela mostra a escala interna ÷ 100,
arredondada para cima — `VisualizeEndurance`, `EC_IvtrEquip.cpp:281`), requisitos, pedras,
efeitos, fabricante, preço e a descrição do `item_ext_desc.txt`.

Cada linha é texto com códigos de cor `^RRGGBB`, como no cliente; o navegador pinta.
Só usa frases `ITEMDESC_*` cuja posição foi conferida contra o `item_desc.txt` BR (até
`ITEMDESC_WEAKDIST`, índice 112): dali em diante o arquivo do cliente diverge do cabeçalho
do fonte.

Parte 2 (B192): o texto de cada efeito como `AddOneAddOnPropDesc` (`EC_IvtrEquip.cpp:971-2610`)
o escreve para o item do jogador (ramo não local), pela tabela gerada em
`efeitos_do_cliente.json`; o tipo vem do `item_ext_prop.txt`. B193: as frases além do índice 112
com a posição conferida no binário BR, e os tipos que o BR não tem saem como o `default:` dele.
Só a habilidade (caso 55, `CECSkill::GetDesc`) ainda sai como id e parâmetros. E a
linha de classe (`AddProfReqDesc`, `EC_IvtrItem.cpp:646-664`). Ordem da arma
(`EC_IvtrWeapon.cpp:375-460`): classe, requisitos, efeitos, pedras, preço, fabricante, descrição.
"""
import struct

from . import textos

BRANCO, AZUL_CLARO, VERMELHO = "^ffffff", "^8080ff", "^ff0000"


def _durabilidade(v):
    return -(-int(v) // 100) if v > 0 else 0


def _f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def _bits(p):
    """`*(float*)&p`: os bits do int como float."""
    return struct.unpack("<f", struct.pack("<i", p))[0]


def _vp(p):
    """`VisualizeFloatPercent` (`EC_IvtrItem.h:370`): `(int)(*(float*)&p * 100.0f + 0.5f)`.
    Bits que não são um float finito: o `(int)` do C é indefinido; aqui 0."""
    v = _f32(_f32(_bits(p) * 100.0) + 0.5)
    return int(v) if v == v and abs(v) < 2**31 else 0


_EXPRESSOES = {
    "p0": lambda a, b: a, "p1": lambda a, b: b, "-p0": lambda a, b: -a, "-p1": lambda a, b: -b,
    "p0/2": lambda a, b: int(a / 2), "-p0*0.05": lambda a, b: _f32(-a * _f32(0.05)),
    "f(p0)": lambda a, b: _bits(a), "vp(p0)": lambda a, b: _vp(a), "-vp(p0)": lambda a, b: -_vp(a),
    "-vp(p1)": lambda a, b: -_vp(b),
}


def _valor(expr, p0, p1):
    return _EXPRESSOES[expr](p0, p1)


def _printf(modelo, *args):
    """`glb_vsnprintf` com os `%d`/`%+d`/`%.2f`/`%%` das frases; argumento a mais é ignorado."""
    try:
        return modelo % args
    except (TypeError, ValueError):
        try:
            return modelo % ()
        except (TypeError, ValueError):
            return modelo


def texto_do_efeito(efeito):
    """Linhas do efeito como o cliente; `None` quando ainda não há texto conferido (sai cru)."""
    tipo = int(efeito.get("tipo") or 0)
    if tipo & (0x8000 | 0x10000 | 0x20000):
        return []  # pedra, conjunto e gravação têm descrição própria (`BuildAddOnPropDesc`, `:2610-2630`)
    idprop = int(efeito.get("id") or 0)
    args = [int(a) for a in (efeito.get("args") or [])] + [0, 0, 0]
    byte = textos.tipos_de_efeito().get(idprop, 0xFF)
    if 100 <= byte <= 115:
        return []  # afiador: `IsSharpenerProperty`, descrito por `AddSharpenerDesc`
    caso = textos.efeitos_do_cliente().get(str(byte))
    if caso is None or caso.get("erro"):
        # `default:` — também os tipos que o cliente BR não tem (B193, `efeitos_no_binario_br.json`).
        return [_printf(textos.frases().get("ITEMDESC_ERRORPROP", ""), idprop)]
    if not caso["seguro"]:
        return None
    linhas, linha = [], ""
    for frase, literal, expr, quebra in caso["partes"]:
        modelo = textos.frases().get(frase, "") if frase else literal
        linha += _printf(modelo, _valor(expr, args[0], args[1])) if expr else _printf(modelo)
        if quebra:
            linhas.append(linha)
            linha = ""
    if linha:
        linhas.append(linha)
    return linhas


_CATALOGO_DE_EFEITOS = None


def catalogo_de_efeitos():
    """B206: `[(id, texto)]` de todo efeito que o cliente descreve (`item_ext_prop.txt` → tipo →
    frase), com parâmetros 0, para a busca na edição do item. Calculado uma vez."""
    global _CATALOGO_DE_EFEITOS
    if _CATALOGO_DE_EFEITOS is None:
        lista = []
        for idprop in sorted(textos.tipos_de_efeito()):
            linhas = texto_do_efeito({"id": idprop, "args": [0, 0, 0]})
            if linhas:
                lista.append((idprop, " ".join(linhas)))
        _CATALOGO_DE_EFEITOS = lista
    return _CATALOGO_DE_EFEITOS


def _linha_de_classe(classes, versao, classe):
    """`AddProfReqDesc`: some se a máscara cobre todas as classes da versão (`(1 << NUM_PROFESSION)
    - 1`, 12 no 1.5.5 `ExpTypes.h:20`; 8 no 1.2.6, classes 0–7); branca se a classe do personagem
    está nela, vermelha se não."""
    total = 8 if str(versao or "").startswith("1.2") else 12
    todas = (1 << total) - 1
    if classes is None or (classes & todas) == todas:
        return None
    nomes = textos.nomes_das_classes()
    cor = BRANCO if classe is not None and classes & (1 << classe) else VERMELHO
    partes = [textos.frase("ITEMDESC_PROFESSIONREQ")]
    partes += [nomes[i] for i in range(total) if classes & (1 << i) and i < len(nomes)]
    return cor + " ".join(partes)


def linhas_da_dica(item, versao=None, classe=None):
    t = textos.frase
    linhas = []
    equip = item.get("equipamento") or {}
    furos = equip.get("furos") or []
    nome = item.get("nome") or f"Item {item.get('id')}"
    cabeca = "^" + textos.cor_do_nome(int(item.get("id") or 0))
    cabeca += t("ITEMDESC_NAMESOCKET", nome, len(furos)) if furos else t("ITEMDESC_NAME", nome)
    if equip.get("refino"):
        cabeca += f" +{equip['refino']}"
    linhas.append(cabeca)
    if (item.get("quantidade") or 1) > 1:
        linhas.append(f"{BRANCO}×{item['quantidade']}")

    arma = equip.get("arma")
    if arma:
        linhas.append(BRANCO + t("ITEMDESC_LEVEL", arma["nivel"]))
        if arma.get("velocidade"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_ATKSPEED')} {1.0 / (arma['velocidade'] * 0.05):.2f}")
        linhas.append(BRANCO + t("ITEMDESC_ATKDISTANCE", float(arma["alcance"])))
        if arma.get("alcance_curto"):
            linhas.append(BRANCO + t("ITEMDESC_WEAKDIST", float(arma["alcance_curto"])))
        if any(arma["dano"]):
            linhas.append(f"{BRANCO}{t('ITEMDESC_PHYDAMAGE')} {arma['dano'][0]}-{arma['dano'][1]}")
        if any(arma["dano_magico"]):
            linhas.append(f"{BRANCO}{t('ITEMDESC_MAGICDAMAGE')} {arma['dano_magico'][0]}-{arma['dano_magico'][1]}")
    for chave in ("armadura", "ornamento"):
        bloco = equip.get(chave)
        if not bloco:
            continue
        if bloco.get("dano"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_PHYDAMAGE')} {bloco['dano']}")
        if bloco.get("dano_magico"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_MAGICDAMAGE')} {bloco['dano_magico']}")
        if bloco.get("defesa"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_PHYDEFENCE')} {bloco['defesa']}")
        if bloco.get("hp"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_ADDHP')} {bloco['hp']:+d}")
        if bloco.get("mp"):
            linhas.append(f"{BRANCO}{t('ITEMDESC_ADDMP')} {bloco['mp']:+d}")
        for frase, valor in zip(("ITEMDESC_GOLDDEFENCE", "ITEMDESC_WOODDEFENCE", "ITEMDESC_WATERDEFENCE",
                                 "ITEMDESC_FIREDEFENCE", "ITEMDESC_EARTHDEFENCE"), bloco.get("resistencias") or []):
            if valor:
                linhas.append(f"{BRANCO}{t(frase)} {valor}")
    if equip:
        atual, maxima = equip.get("durabilidade", 0), equip.get("durabilidade_maxima", 0)
        cor = VERMELHO if atual <= 0 else BRANCO
        linhas.append(f"{cor}{t('ITEMDESC_ENDURANCE')} {_durabilidade(atual)}/{_durabilidade(maxima)}")
        req = equip.get("requisitos") or {}
        classes = _linha_de_classe(req.get("classes"), versao, classe)
        if classes:
            linhas.append(classes)
        for chave, frase in (("nivel", "ITEMDESC_LEVELREQ"), ("forca", "ITEMDESC_STRENGTHREQ"),
                             ("agilidade", "ITEMDESC_AGILITYREQ"), ("vitalidade", "ITEMDESC_VITALITYREQ"),
                             ("energia", "ITEMDESC_ENERGYREQ")):
            if req.get(chave):
                linhas.append(BRANCO + t(frase, req[chave]))
        for efeito in equip.get("efeitos") or []:
            texto = texto_do_efeito(efeito)
            if texto is None:
                args = ", ".join(str(a) for a in efeito.get("args") or [])
                texto = [f"Efeito {efeito['id']}" + (f" ({args})" if args else "")]
            linhas.extend(AZUL_CLARO + linha for linha in texto)
        for pedra in furos:
            if pedra.get("id"):
                linhas.append(AZUL_CLARO + (pedra.get("nome") or f"Pedra {pedra['id']}"))
    if item.get("preco"):
        linhas.append(f"{BRANCO}{t('ITEMDESC_PRICE')} {int(item['preco']):,}".replace(",", "."))
    if equip.get("fabricante"):
        linhas.append(BRANCO + t("ITEMDESC_MADEFROM", equip["fabricante"]))
    descricao = textos.descricoes().get(int(item.get("id") or 0))
    if descricao:
        linhas.extend(parte for parte in descricao.replace("\\r", "\r").split("\r") if parte)
    return linhas
