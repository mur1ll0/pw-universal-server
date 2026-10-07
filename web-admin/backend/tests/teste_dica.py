"""B189/B192/B193: dica de item montada com os textos reais do cliente 1.5.5 BR (data/textos)."""
import unittest

from painel import dica, textos


class TesteDica(unittest.TestCase):
    def setUp(self):
        try:
            textos.frases()
        except OSError:
            self.skipTest("textos do cliente ausentes em data/textos")

    def test_arma_com_furo_refino_e_durabilidade_zero(self):
        item = {"id": 6, "nome": "Lâmina", "quantidade": 1, "preco": 1500, "equipamento": {
            "durabilidade": 0, "durabilidade_maxima": 5001, "refino": 3,
            "requisitos": {"nivel": 10, "forca": 12, "agilidade": 0, "vitalidade": 0, "energia": 0, "classes": 1},
            "furos": [{"id": 0, "nome": ""}], "fabricante": "Murillo",
            # B192: 341 é do tipo 0 (`ADDPHYDAMAGE`), 2362 do 63 (frase além do 112, conferida no binário no B193), 2363
            # do 100 (afiador: sem linha aqui) e 341 com `0x8000` é de pedra (sem linha aqui).
            "efeitos": [{"id": 341, "tipo": 341 | 1 << 13, "args": [25]}, {"id": 2362, "tipo": 2362, "args": [3]},
                        {"id": 2363, "tipo": 2363, "args": [1]}, {"id": 341, "tipo": 341 | 0x8000, "args": [9]}],
            "arma": {"nivel": 2, "dano": [20, 30], "dano_magico": [0, 0], "velocidade": 20, "alcance": 2.5,
                     "alcance_curto": 0.0, "tipo": 0, "municao": 0}}}
        linhas = dica.linhas_da_dica(item, versao="1.5.5", classe=1)
        self.assertTrue(linhas[0].startswith("^") and linhas[0].endswith("Lâmina (1 Slots) +3"), linhas[0])
        self.assertIn("^ffffffNv. 2", linhas)
        self.assertIn("^ffffffFreqüência de ataque (vezes/s) 1.00", linhas)
        self.assertIn("^ffffffAlcance 2.50", linhas)
        self.assertIn("^ffffffAtaque físico 20-30", linhas)
        self.assertIn("^ff0000Durabilidade 0/51", linhas, "zero em vermelho; 5001 internos = 51 na tela")
        self.assertIn("^ffffffNv. necessário: 10", linhas)
        self.assertIn("^ffffffFor. necessária: 12", linhas)
        efeitos = [l for l in linhas if l.startswith("^8080ff")]
        self.assertEqual(efeitos, ["^8080ffAtaque físico +25", "^8080ffForça da Alma +3"], "pedra e afiador sem linha")
        # Classe: só Guerreiro (máscara 1); o personagem é Mago → vermelho, antes do nível exigido.
        classe = "^ff0000" + textos.frase("ITEMDESC_PROFESSIONREQ") + " Guerreiro"
        self.assertLess(linhas.index(classe), linhas.index("^ffffffNv. necessário: 10"))
        # Ordem do cliente (`EC_IvtrWeapon.cpp:375-460`): efeitos, preço, fabricante.
        self.assertLess(linhas.index("^8080ffAtaque físico +25"), linhas.index("^ffffffPreço 1.500"))
        self.assertLess(linhas.index("^ffffffPreço 1.500"), linhas.index("^ffffffFeito por Murillo"))

    def test_efeitos_com_float_percentual_e_desconhecido(self):
        import struct
        bits = lambda x: struct.unpack("<i", struct.pack("<f", x))[0]
        self.assertEqual(dica.texto_do_efeito({"id": 331, "tipo": 331, "args": [-4]}), ["Intervalo de Ataque 0.20 segundos"])
        self.assertEqual(dica.texto_do_efeito({"id": 471, "tipo": 471, "args": [bits(0.5)]}), ["Alcance +0.50"])
        self.assertEqual(dica.texto_do_efeito({"id": 527, "tipo": 527, "args": [bits(0.05), bits(0.03)]}),
                         ["Def Metal 5%", "Def Fogo -3%"])
        self.assertEqual(dica.texto_do_efeito({"id": 9999, "tipo": 9999, "args": [1]}),
                         [textos.frase("ITEMDESC_ERRORPROP", 9999)], "fora do item_ext_prop: o `default:`")

    def test_frases_alem_do_112_pelo_binario_br(self):
        """B193: posições conferidas no ElementClient BR; tipos que ele não tem = o `default:`."""
        texto = lambda i: dica.texto_do_efeito({"id": i, "tipo": i, "args": [3]})
        self.assertEqual(texto(2029), ["Nível de Ataque +3"], "tipo 59, ATK_DEGREE → 202")
        self.assertEqual(texto(2843), ["Nível de Matança: +3"], "tipo 90, PENETRATION → 276")
        self.assertEqual(texto(2239), ["Resistência Elemental  +3%"], "tipo 61, TOTAL_DEFENCE_ADD → 255")
        self.assertEqual(texto(3193), ["Penet. Física +3"], "tipo 175, PPEN → 397")
        self.assertEqual(texto(3160), [textos.frase("ITEMDESC_ERRORPROP", 3160)], "tipo 161: o BR manda ao default")
        self.assertIsNone(texto(445), "habilidade (55) ainda crua")
        self.assertNotIn("ITEMDESC_PDEF", textos.frases(), "além do 112 sem conferência: fora")

    def test_classe_que_cobre_todas_some(self):
        self.assertIsNone(dica._linha_de_classe(0xFFF, "1.5.5", 0))
        self.assertIsNone(dica._linha_de_classe(0xFF, "1.2.6", 0), "o 1.2.6 tem 8 classes")
        self.assertTrue(dica._linha_de_classe(0xFF, "1.5.5", 0).startswith("^ffffff"))

    def test_descricao_e_cor_do_item_comum(self):
        linhas = dica.linhas_da_dica({"id": 10042, "nome": "Carruagem", "quantidade": 3})
        self.assertEqual(linhas[1], "^ffffff×3")
        self.assertTrue(any("carruagem de cerco" in l for l in linhas), linhas)


if __name__ == "__main__":
    unittest.main()
