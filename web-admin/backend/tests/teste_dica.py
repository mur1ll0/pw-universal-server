"""B189: dica de item montada com os textos reais do cliente 1.5.5 BR (data/textos)."""
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
            "furos": [{"id": 0, "nome": ""}], "efeitos": [{"id": 101, "tipo": 101, "args": [5]}], "fabricante": "Murillo",
            "arma": {"nivel": 2, "dano": [20, 30], "dano_magico": [0, 0], "velocidade": 20, "alcance": 2.5,
                     "alcance_curto": 0.0, "tipo": 0, "municao": 0}}}
        linhas = dica.linhas_da_dica(item)
        self.assertTrue(linhas[0].startswith("^") and linhas[0].endswith("Lâmina (1 Slots) +3"), linhas[0])
        self.assertIn("^ffffffNv. 2", linhas)
        self.assertIn("^ffffffFreqüência de ataque (vezes/s) 1.00", linhas)
        self.assertIn("^ffffffAlcance 2.50", linhas)
        self.assertIn("^ffffffAtaque físico 20-30", linhas)
        self.assertIn("^ff0000Durabilidade 0/51", linhas, "zero em vermelho; 5001 internos = 51 na tela")
        self.assertIn("^ffffffNv. necessário: 10", linhas)
        self.assertIn("^ffffffFor. necessária: 12", linhas)
        self.assertIn("^8080ffEfeito 101 (5)", linhas)
        self.assertIn("^ffffffFeito por Murillo", linhas)
        self.assertIn("^ffffffPreço 1.500", linhas)

    def test_descricao_e_cor_do_item_comum(self):
        linhas = dica.linhas_da_dica({"id": 10042, "nome": "Carruagem", "quantidade": 3})
        self.assertEqual(linhas[1], "^ffffff×3")
        self.assertTrue(any("carruagem de cerco" in l for l in linhas), linhas)


if __name__ == "__main__":
    unittest.main()
