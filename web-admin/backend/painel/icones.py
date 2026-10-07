"""Ícones de itens recortados do atlas do cliente (B188).

O cliente guarda os ícones de inventário num arquivo agrupado: `Surfaces\\IconSet\\
IconList_Ivtr{M,F}.dds` (masculino/feminino) com um `.txt` de largura, altura, linhas, colunas
e o nome de cada célula, linha a linha (`CECGameUIMan`, `EC_GameUIMan.cpp:588-640`). Um item
acha a célula pelo nome do arquivo do seu `file_icon` em minúsculas (`SetCover`, `:6204-6208`).

O atlas usado é o do cliente 1.5.5 (extraído do `surfaces.pck` para `data/icones/`): cobre
98,3% dos ícones do realm 1.5.5 e 96,1% dos do 1.2.6 — o mesmo que o atlas do próprio 1.2.6.
A textura é DXT1 4096×4096 sem mipmaps; cada ícone é recortado sob demanda (64 blocos de 8 B)
e devolvido em PNG, sem dependência de imagem: DXT1 e PNG à mão, com `zlib`.

B195: o mesmo para os outros conjuntos do `CECGameUIMan` — `IconList_Skill` (DXT1 2048×1024,
24 × 64 células de 32 px) e `IconList_Pet` (**DXT3** 512×1024, 18 × 16), extraídos do mesmo
`surfaces.pck`. DXT3 = 8 bytes de alfa (4 bits por pixel) + o bloco de cor do DXT1 sempre com
quatro cores.
"""
import functools
import os
import struct
import zlib
from pathlib import Path

def _pasta_padrao():
    # Contêiner: /app/painel/icones.py com ../data montado em /app/data. Repositório:
    # web-admin/backend/painel/icones.py, com data/ na raiz.
    aqui = Path(__file__).resolve()
    # `parents[3]` só existe no repositório (no contêiner o arquivo está em /app/painel).
    niveis = [n for n in (1, 3) if n < len(aqui.parents)]
    for candidata in (aqui.parents[n] / "data" / "icones" for n in niveis):
        if candidata.is_dir():
            return candidata
    return aqui.parents[1] / "data" / "icones"


PASTA = Path(os.getenv("ICONES_DIR") or _pasta_padrao())
CABECALHO_DDS = 128


# Conjunto do painel → nome do atlas do cliente (`IconList_<nome>`, `EC_GameUIMan.cpp:588-640`).
CONJUNTOS = {"m": "ivtrm", "f": "ivtrf", "habilidade": "skill", "mascote": "pet"}


class Atlas:
    def __init__(self, nome):
        texto = (PASTA / f"iconlist_{nome}.txt").read_bytes().split(b"\n")
        self.largura, self.altura, self.linhas, self.colunas = (int(texto[i].strip()) for i in range(4))
        self.indice = {}
        for n, linha in enumerate(texto[4:4 + self.linhas * self.colunas]):
            celula = linha.strip(b"\r").decode("gbk", "replace").lower()
            if celula:
                self.indice.setdefault(celula, n)
        self.dds = (PASTA / f"iconlist_{nome}.dds").read_bytes()
        altura, largura = struct.unpack_from("<II", self.dds, 12)
        self.formato = self.dds[84:88]
        if self.formato not in (b"DXT1", b"DXT3") or largura < self.colunas * self.largura \
                or altura < self.linhas * self.altura:
            raise ValueError("atlas de ícones inesperado (esperado DXT1/DXT3 do tamanho do índice)")
        self.bytes_por_bloco = 8 if self.formato == b"DXT1" else 16
        self.blocos_por_linha = largura // 4

    def png(self, titulo):
        n = self.indice.get(titulo.lower())
        if n is None:
            return None
        x0 = (n % self.colunas) * self.largura
        y0 = (n // self.colunas) * self.altura
        pixels = bytearray(self.largura * self.altura * 4)
        for by in range(self.altura // 4):
            for bx in range(self.largura // 4):
                desloc = CABECALHO_DDS + ((y0 // 4 + by) * self.blocos_por_linha + (x0 // 4 + bx)) * self.bytes_por_bloco
                bruto = self.dds[desloc:desloc + self.bytes_por_bloco]
                bloco = decodificar_bloco_dxt1(bruto) if self.formato == b"DXT1" else decodificar_bloco_dxt3(bruto)
                for py in range(4):
                    for px in range(4):
                        d = ((by * 4 + py) * self.largura + bx * 4 + px) * 4
                        pixels[d:d + 4] = bloco[py * 4 + px]
        return codificar_png(self.largura, self.altura, bytes(pixels))


def rgb565(c):
    r, g, b = (c >> 11) & 31, (c >> 5) & 63, c & 31
    return (r << 3 | r >> 2, g << 2 | g >> 4, b << 3 | b >> 2)


def decodificar_bloco_dxt1(b):
    """Um bloco 4×4 de DXT1: duas cores RGB565 e 2 bits por pixel. `c0 > c1` = quatro cores
    opacas; senão três cores e o índice 3 transparente (formato S3TC/DXT1)."""
    c0, c1, bits = struct.unpack("<HHI", b)
    a, z = rgb565(c0), rgb565(c1)
    if c0 > c1:
        cores = [a + (255,), z + (255,),
                 tuple((2 * p + q) // 3 for p, q in zip(a, z)) + (255,),
                 tuple((p + 2 * q) // 3 for p, q in zip(a, z)) + (255,)]
    else:
        cores = [a + (255,), z + (255,), tuple((p + q) // 2 for p, q in zip(a, z)) + (255,), (0, 0, 0, 0)]
    return [bytes(cores[(bits >> (2 * i)) & 3]) for i in range(16)]


def decodificar_bloco_dxt3(b):
    """Um bloco 4×4 de DXT3: 16 alfas de 4 bits (`a × 17` em 8 bits) e o bloco de cor DXT1,
    que no DXT3 usa sempre as quatro cores interpoladas (S3TC)."""
    alfas = int.from_bytes(b[:8], "little")
    c0, c1, bits = struct.unpack("<HHI", b[8:16])
    a, z = rgb565(c0), rgb565(c1)
    cores = [a, z, tuple((2 * p + q) // 3 for p, q in zip(a, z)), tuple((p + 2 * q) // 3 for p, q in zip(a, z))]
    return [bytes(cores[(bits >> (2 * i)) & 3] + (((alfas >> (4 * i)) & 15) * 17,)) for i in range(16)]


def codificar_png(largura, altura, rgba):
    def pedaco(tipo, dados):
        return struct.pack(">I", len(dados)) + tipo + dados + struct.pack(">I", zlib.crc32(tipo + dados))
    linhas = b"".join(b"\x00" + rgba[y * largura * 4:(y + 1) * largura * 4] for y in range(altura))
    return (b"\x89PNG\r\n\x1a\n" + pedaco(b"IHDR", struct.pack(">IIBBBBB", largura, altura, 8, 6, 0, 0, 0))
            + pedaco(b"IDAT", zlib.compress(linhas, 9)) + pedaco(b"IEND", b""))


@functools.lru_cache(maxsize=4)
def atlas(conjunto):
    return Atlas(CONJUNTOS[conjunto])


@functools.lru_cache(maxsize=8192)
def png_do_icone(conjunto, titulo_hex):
    """PNG do ícone cujo arquivo (bytes GBK, em hexadecimal — como o GS manda) é `titulo_hex`."""
    try:
        titulo = bytes.fromhex(titulo_hex).decode("gbk")
    except (ValueError, UnicodeDecodeError):
        return None
    return atlas(conjunto).png(titulo)
