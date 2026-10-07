"""Dica (tooltip) de item, montada como o cliente monta (B189, parte 1).

Ordem e cores de `CECIvtrWeapon::GetNormalDesc` (`EC_IvtrWeapon.cpp:271-400`) e afins: nome na
cor do `item_color.txt` (com furos, `ITEMDESC_NAMESOCKET`; refino ` +N`), valores da arma/
armadura/ornamento, durabilidade (vermelha se zero; a tela mostra a escala interna ÷ 100,
arredondada para cima — `VisualizeEndurance`, `EC_IvtrEquip.cpp:281`), requisitos, pedras,
efeitos, fabricante, preço e a descrição do `item_ext_desc.txt`.

Cada linha é texto com códigos de cor `^RRGGBB`, como no cliente; o navegador pinta.
Só usa frases `ITEMDESC_*` cuja posição foi conferida contra o `item_desc.txt` BR (até
`ITEMDESC_WEAKDIST`, índice 112): dali em diante o arquivo do cliente diverge do cabeçalho
do fonte. Parte 2: o texto de cada efeito (`FormatPropDesc`) — hoje sai id e parâmetros.
"""
from . import textos

BRANCO, AZUL_CLARO, VERMELHO = "^ffffff", "^8080ff", "^ff0000"


def _durabilidade(v):
    return -(-int(v) // 100) if v > 0 else 0


def linhas_da_dica(item):
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
        for chave, frase in (("nivel", "ITEMDESC_LEVELREQ"), ("forca", "ITEMDESC_STRENGTHREQ"),
                             ("agilidade", "ITEMDESC_AGILITYREQ"), ("vitalidade", "ITEMDESC_VITALITYREQ"),
                             ("energia", "ITEMDESC_ENERGYREQ")):
            if req.get(chave):
                linhas.append(BRANCO + t(frase, req[chave]))
        for pedra in furos:
            if pedra.get("id"):
                linhas.append(AZUL_CLARO + (pedra.get("nome") or f"Pedra {pedra['id']}"))
        for efeito in equip.get("efeitos") or []:
            args = ", ".join(str(a) for a in efeito.get("args") or [])
            linhas.append(f"{AZUL_CLARO}Efeito {efeito['id']}" + (f" ({args})" if args else ""))
        if equip.get("fabricante"):
            linhas.append(BRANCO + t("ITEMDESC_MADEFROM", equip["fabricante"]))
    if item.get("preco"):
        linhas.append(f"{BRANCO}{t('ITEMDESC_PRICE')} {int(item['preco']):,}".replace(",", "."))
    descricao = textos.descricoes().get(int(item.get("id") or 0))
    if descricao:
        linhas.extend(parte for parte in descricao.replace("\\r", "\r").split("\r") if parte)
    return linhas
