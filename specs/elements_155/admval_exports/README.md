# Exports do editor ADMVAL (ground truth externo)

Exportados pelo Murillo em 2026-09-02 com o
`D:\PROJETOS\PWPRIVATE\Tools\EDITOR DE ELEMENTS 1.5.5 ADMVAL`, carregando o `elements.data`
do **client** original (`F:\PW\1.5.5\1.5.5.EN\Perfect World 1.5.5 EN\element\data`, build
vizinha à do nosso `data/realm_155/config/elements.data`) e usando a função de export do
editor para as 4 tabelas que estavam sem confirmação nesta fase (sem campo de texto pra
validar por conteúdo legível).

Formato: texto (convertido de UTF-16LE do `.data` original para UTF-8 aqui), uma linha por
campo: `indice_tabela@linha@indice_campo@valor`. Veja
`specs/elements_155/crossref_admval.py` e o README.md principal (seção "Avanço decisivo")
para como isso foi usado para achar a posição exata dessas tabelas no arquivo real por
"impressão digital" binária.
