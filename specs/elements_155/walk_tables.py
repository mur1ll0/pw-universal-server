"""
Caminhador genérico de `elements.data` (1.5.5, build v156).

Le a especificacao de tabelas do `PW_1.5.5_v156.cfg` (via `parse_seledit_cfg.py`) e
percorre o arquivo real tabela por tabela, tentando alinhar cada uma pela posicao
"ingenua" (logo apos a tabela anterior). Quando o `count` lido ali nao da um primeiro
registro plausivel (campos int32/float fora de faixa, texto wstring/string ilegivel),
faz busca em janela (positiva e negativa) por um deslocamento (`skip`) que produza um
primeiro registro plausivel, e reporta esse `skip` -- isso e o que resolveu o mistrio
da tabela 20 (`SKILLTOME_SUB_TYPE`, `skip=19`, `count=22`, nao `count=7` como uma
hipotese anterior chegou a supor por coincidencia).

Uso:
    python specs/elements_155/walk_tables.py [caminho/para/elements.data]

Sem argumento, usa `data/realm_155/config/elements.data` a partir da raiz do repo.

IMPORTANTE (ver README.md): a busca em janela usa so o PRIMEIRO registro de cada
tabela para pontuar. Isso e deliberado -- uma versao anterior que fazia media de
varias amostras (registro 0, meio, ultimo) piorou o resultado: um `count` errado por
sorte podia evitar as amostras "distantes" que o denunciariam, roubando o slot correto
que so errava por sorte no meio da tabela. So caia para a busca em janela quando a
posicao ingenua (skip=0) falha -- os resultados de skip=0 confirmados por conteudo
legivel (tabelas 0-24) sao MUITO mais confiaveis que qualquer resultado vindo da busca.

Confianca dos resultados, por faixa (ver README.md para o detalhe tabela a tabela):
  - Tabelas 0-25: ALTA. skip=0 na grande maioria, conteudo legivel conferido a olho.
  - Tabelas 26-38: BAIXA/PROVISORIA. A janela de busca encontra *algum* alinhamento
    plausivel, mas nem todo `sample` bate semanticamente com o nome da tabela (ex.:
    REVIVESCROLL_ESSENCE mostrando "Садовая лопатка" = "pa de jardim" e suspeito).
    Precisa de conferencia manual tabela a tabela, como foi feito para a tabela 20.
  - Tabela 39 em diante (NPC_TALK_SERVICE): a busca falha (nenhum alinhamento no
    range testado deu score aceitavel) -- bloqueio real, nao investigado.
  - Tabela 58 (TALK_PROC): tamanho variavel (`byte:AUTO` no cfg), tem loop de leitura
    manual no `elementdataman.cpp` original -- este caminhador para ali de proposito.
"""
import math
import os
import struct
import sys

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, REPO_ROOT)
from specs.elements_155.parse_seledit_cfg import field_size

CFG_PATH = os.path.join(REPO_ROOT, "specs", "elements_155", "PW_1.5.5_v156.cfg")
DEFAULT_DATA_PATH = os.path.join(REPO_ROOT, "data", "realm_155", "config", "elements.data")

# O `count` gravado no arquivo pode simplesmente estar errado, e as vezes o `skip` que
# `try_table()` acha sozinho tambem esta errado (achado nesta sessao -- ver README.md).
# `extend_declared_count()` automatiza uma primeira tentativa de correcao, mas nao e
# confiavel sozinha (uma heuristica de plausibilidade generica confunde preenchimento
# zerado com dado real, e nao sabe quando trocar de tabela). Cada entrada aqui e um
# `skip`/`count` real, CONFIRMADO A MAO por conteudo legivel (ver README.md, secao "Tabelas
# com override manual"), nao calculado automaticamente -- quando presente, `main()` ignora
# o resultado de `try_table()` para essa tabela e usa `skip`/`count` diretamente.
TABLE_OVERRIDES = {
    # FACE_HAIR_ESSENCE: skip=944 (achado por try_table) esta certo, mas o `count` gravado
    # (1) esta errado -- ha 428 registros legiveis (nomes de penteados de eventos "Мужская
    # прическа.../ГМ 2015..."), confirmados pelo prefixo de caminho esperado (file_hair_skin
    # comeca com "facedata\", file_icon com "Surfaces\") ate o registro 427; a tabela
    # seguinte (FACE_MOUSTACHE_ESSENCE, "Мужская борода 01..16") comeca exatamente em
    # seguida com seu proprio count=16 correto, sem precisar de ajuste.
    63: {"skip": 944, "count": 428},
    # ENEMY_FACTION_CONFIG: CORRIGIDO nesta rodada -- a entrada anterior aqui
    # (skip=142/count=1) estava ERRADA. Motivo do erro original: em skip=142 o registro
    # decodifica com um ID pequeno e pontuacao ok, e a tabela seguinte parecia "resolver
    # como vazia" -- mas eu nunca conferi o CONTEUDO da tabela seguinte, so que o numero
    # era pequeno. Cruzando contra o `elements.data` do CLIENT 1.5.5 original (build v159,
    # `F:\PW\1.5.5\1.5.5.EN\...`), a tabela equivalente resolve limpo em skip=0/count=1 com
    # ID=1 e Name="Opponent list 1" -- voltando pro NOSSO arquivo, skip=0/count=1 da
    # exatamente ID=1/Name="Список противников 1" (traducao literal do ingles do client!),
    # e a tabela seguinte (CHARRACTER_CLASS_CONFIG) resolve sozinha (sem override) com
    # count=12 = as 12 classes do jogo, todas legiveis (Воин/Маг/Шаман/Друид/Оборотень/
    # Убийца/Лучник/Жрец/Страж/Дух демона/Призрак/Жнец). O `skip=142` antigo lia so 1
    # registro plausivel por sorte e comia bytes que pertenciam a essa tabela 71 real.
    # Licao: "a proxima tabela parece plausivel" nao e evidencia forte o bastante sozinha --
    # precisa decodificar o CONTEUDO da proxima tabela, nao so olhar se o numero e pequeno.
    70: {"skip": 0, "count": 1},
    # PARAM_ADJUST_CONFIG: na verdade **nao precisa mais de override nenhum** -- confirmado
    # rodando `_try_table()` sem override depois de corrigir a tabela 70: acha sozinho
    # `(44335619, 1, 680)`, a mesma posicao. O motivo de eu ter "precisado" de um override
    # aqui antes era colateral do bug da tabela 70 (o `off` que chegava aqui vinha errado);
    # depois de consertar a 70, a 72 nunca teve problema proprio. Mantive a entrada mesmo
    # assim, agora ancorada por posicao ABSOLUTA (nao por `skip`, que se mostrou fragil a
    # mudanca de offset acumulado -- foi exatamente isso que quebrou esta entrada em
    # silencio quando a tabela 70 foi corrigida da primeira vez) -- serve de checagem de
    # regressao barata, nao porque a tabela seja dificil. Confirmado por fonte externa
    # tambem: export do `elements.data` do CLIENT 1.5.5 original via ADMVAL
    # (`tabela@linha@campo@valor`), impressao digital binaria dos 152 campos numericos
    # batendo 100% (so `Name`, traduzido, difere).
    72: {"abs_count_off": 44335619, "count": 1},
    # As 3 seguintes vieram da mesma tecnica (fingerprint binario contra o export do
    # cliente), mas ancoradas por posicao ABSOLUTA (`abs_count_off`), nao por `skip` a
    # partir do offset acumulado -- as tabelas entre 72 e cada uma destas (73-76, 78-85,
    # 87-92) ainda NAO foram resolvidas manualmente, entao o offset que o caminhador
    # acumula ali e lixo; ancorar por posicao absoluta faz cada uma resolver certo mesmo
    # assim, e o que vier DEPOIS de cada uma resume do lugar certo.
    77: {"abs_count_off": 48163839, "count": 2},  # PLAYER_LEVELEXP_CONFIG
    86: {"abs_count_off": 51261899, "count": 4},  # FACETICKET_ESSENCE
    93: {"abs_count_off": 51335459, "count": 6},  # PET_TYPE
    # FORCE_CONFIG: try_table() achou skip=732/count=5 (score mais alto, mas ilegivel).
    # skip=0/count=3 tem score mais baixo mas conteudo perfeito: "Орден Солнца"/"Орден
    # Мрака"/"Армия Зари" (Ordem do Sol / Ordem das Trevas / Exercito do Amanhecer) -- as 3
    # faccoes de guerra territorial do PW, com descricoes completas e coerentes (cada uma
    # menciona "opedir a Ordem X e capturar o Disco da Eternidade"). Prova de que score
    # sozinho nao basta -- aqui o candidato "melhor pontuado" era o errado.
    150: {"skip": 0, "count": 3},
    # ASTROLABE_APPEARANCE_CONFIG: confirmado com export do ADMVAL (formato
    # `tabela@linha@campo@valor`) -- os 22 campos batem 100%, incluindo os 10 caminhos
    # "gfx\...\星盘N.gfx" e o nome chines "星盘外观配置表". O `count` real e 1; a posicao
    # ingenua (skip=0) ja estava certa, so precisava ser aceita (o score dela e baixo pq a
    # tabela tem so 1 registro, mas o conteudo bate perfeito). A tabela seguinte
    # (EQUIP_MAKE_HOLE_CONFIG) resolve sozinha em skip=0/count=1 assim que esta e corrigida
    # -- tambem confirmada 100% (242 campos) contra o export do ADMVAL.
    204: {"skip": 0, "count": 1},
    # SOLO_TOWER_CHALLENGE_SCORE_COST_CONFIG: score baixo (tabela grande, maioria dos campos
    # zerados) fez o guloso rejeitar a posicao certa. Conteudo em skip=0 e perfeito: "单人爬
    # 塔副本积分消耗配置表" (tabela de configuracao de consumo de pontos da masmorra de
    # escalada solo) -- bate exatamente com o nome da tabela.
    209: {"skip": 0, "count": 1},
}


def parse_cfg_full(path):
    """Como `parse_seledit_cfg.parse_cfg`, mas tambem devolve nomes de campo e a
    lista de tokens de tipo crua (a versao original so devolvia o tamanho somado)."""
    lines = open(path, encoding="latin-1").read().splitlines()
    i = 2
    out = []
    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        name = line
        i += 1
        flag = lines[i].strip()
        i += 1
        header = lines[i] if i < len(lines) else ""
        i += 1
        types_line = lines[i] if i < len(lines) else ""
        i += 1
        fields = [t for t in header.strip().split(";") if t != ""]
        tokens = [t for t in types_line.strip().split(";") if t]
        out.append((name, flag, fields, tokens))
    return out


def table_size(tokens):
    if len(tokens) == 1 and tokens[0].startswith("byte:"):
        return None
    try:
        return sum(field_size(t) for t in tokens)
    except ValueError:
        return None


def decode_record(rec, fields, tokens):
    """Decodifica um registro segundo a especificacao de campos; devolve
    (valores, pontuacao_de_plausibilidade, bytes_consumidos)."""
    off = 0
    score = 0
    values = []
    for _fname, tok in zip(fields, tokens):
        tok = tok.strip()
        if tok == "int32":
            v = struct.unpack_from("<i", rec, off)[0]
            off += 4
            if -1_000_000 <= v <= 100_000_000:
                score += 1
            elif v == 0:
                score += 0.5
            else:
                score -= 2
            values.append(v)
        elif tok == "float":
            v = struct.unpack_from("<f", rec, off)[0]
            off += 4
            if v == 0.0:
                score += 0.5
            elif math.isfinite(v) and abs(v) < 1e7:
                score += 1
            else:
                score -= 2
            values.append(v)
        elif tok.startswith("wstring:"):
            n = int(tok.split(":")[1])
            raw = rec[off:off + n]
            off += n
            txt = raw.decode("utf-16le", errors="replace")
            nul = txt.find("\x00")
            head = txt[:nul] if nul >= 0 else txt
            tail = txt[nul + 1:] if nul >= 0 else ""
            if nul == -1:
                score -= 1
            else:
                score += 1 if all(c == "\x00" for c in tail) else -1
                if len(head) == 0:
                    score += 0.2
                elif all(c.isprintable() or ord(c) > 0x3000 for c in head):
                    score += 1.5
                else:
                    score -= 1.5
            values.append(head)
        elif tok.startswith("string:"):
            n = int(tok.split(":")[1])
            raw = rec[off:off + n]
            off += n
            nul_idx = raw.find(b"\x00")
            head = raw[:nul_idx] if nul_idx >= 0 else raw
            tail = raw[nul_idx + 1:] if nul_idx >= 0 else b""
            if nul_idx == -1:
                score -= 1
            elif all(b == 0 for b in tail):
                score += 0.5
            else:
                score -= 1
            try:
                htxt = head.decode("ascii")
                if all(32 <= ord(c) < 127 for c in htxt):
                    score += 1
            except Exception:
                if len(head) > 0:
                    score -= 1
            values.append(head)
        else:
            raise ValueError(f"tipo de campo desconhecido: {tok}")
    return values, score, off


def _read_wstr(buf, off, nchars):
    """Le uma string UTF-16LE de largura fixa (nchars caracteres = nchars*2 bytes),
    devolvendo o texto ate o primeiro terminador nulo."""
    raw = buf[off:off + nchars * 2]
    txt = raw.decode("utf-16le", errors="replace")
    nul = txt.find("\x00")
    return (txt[:nul] if nul >= 0 else txt), off + nchars * 2


def read_talk_proc_table(buf, off):
    """Le a tabela 58 (TALK_PROC), a unica de tamanho variavel em `elements.data`.
    Formato confirmado lendo `elementdataman.cpp::load_data` +
    `exptypes.h::talk_proc/window/option` (EvolvedPWServer): um `size_t` (aqui 4 bytes,
    build de 32 bits) com o total de arvores de dialogo, e cada `talk_proc` e
    [id_talk: u32][text: wchar[64]][num_window: i32][windows...], cada `window` e
    [id: u32][id_parent: i32][talk_text_len: i32][talk_text: wchar[len]]
    [num_option: i32][options...], e cada `option` e [id: u32][text: wchar[64]][param: u32].
    Devolve (count, novo_offset, texto_de_amostra)."""
    count = struct.unpack_from("<I", buf, off)[0]
    cur = off + 4
    sample = ""
    for i in range(count):
        id_talk = struct.unpack_from("<I", buf, cur)[0]
        cur += 4
        text, cur = _read_wstr(buf, cur, 64)
        if i == 0:
            sample = text
        num_window = struct.unpack_from("<i", buf, cur)[0]
        cur += 4
        for _w in range(num_window):
            cur += 8  # id, id_parent
            talk_text_len = struct.unpack_from("<i", buf, cur)[0]
            cur += 4
            cur += talk_text_len * 2
            num_option = struct.unpack_from("<i", buf, cur)[0]
            cur += 4
            cur += num_option * (4 + 64 * 2 + 4)  # id + text[64] + param, por opcao
    return count, cur, sample


def extend_declared_count(buf, rec_start, declared_count, size, fields, tokens, max_extra=50_000, bad_streak_limit=3):
    """O `count` gravado no arquivo às vezes MENTE (achado nesta sessão: `FACE_HAIR_ESSENCE`
    declara `count=1` mas tem pelo menos 647 registros legíveis depois). Esta função
    experimenta ler além do `count` declarado, mantendo os extras só se decodificarem com
    evidência de conteúdo real -- não basta `score>=0`, porque um registro totalmente zerado
    (ex.: o preenchimento entre o fim de uma tabela e o início da próxima) também pontua
    >=0 no `decode_record` (campos zerados/strings vazias dão pontos positivos por
    "plausibilidade", não por serem dado de verdade) e isso fez uma primeira versão desta
    função **estender tabelas que já estavam certas** (destruiu o alinhamento correto de
    `EQUIPMENT_ADDON`, confirmado por texto legível, ao aceitar lixo de padding como
    registro 2978+). Por isso exige pelo menos um campo string/wstring não vazio, ou um id
    positivo -- evidência de que ali tem dado real, não só zeros.

    **NÃO é chamada automaticamente por `main()`** -- é uma ferramenta pra investigar uma
    tabela específica manualmente (como foi usada pra achar os 648 registros reais de
    `FACE_HAIR_ESSENCE`, tabela 63, que o `count` do arquivo dizia ter só 1), não pra
    corrigir o caminhador inteiro sem supervisão. Rodar automaticamente pra todas as 231
    tabelas sem validação manual dos resultados corre o mesmo risco que já se provou real."""
    count = declared_count
    i = declared_count
    bad_streak = 0
    while i - declared_count < max_extra:
        start = rec_start + i * size
        if start + size > len(buf):
            break
        rec = buf[start:start + size]
        try:
            vals, sc, _ = decode_record(rec, fields, tokens)
        except Exception:
            break
        idv = vals[0] if vals else None
        has_text = any(isinstance(v, str) and v.strip() for v in vals)
        plausible = isinstance(idv, int) and sc >= 0 and (idv > 0 or has_text)
        if plausible:
            bad_streak = 0
            count = i + 1
        else:
            bad_streak += 1
            if bad_streak >= bad_streak_limit:
                break
        i += 1
    return count


def try_table(buf, off, fields, tokens, size, window=1024, neg_window=64, max_count=200_000):
    """Tenta ler uma tabela a partir de `off`. Ve o skip=0 primeiro (posicao ingenua);
    se o primeiro registro ali for plausivel, aceita direto (guloso). So cai para busca
    em janela (positiva e negativa) quando skip=0 falha -- ver o docstring do modulo
    para o porque de so pontuar pelo primeiro registro."""
    filesize = len(buf)

    def eval_at(cand):
        c_off = off + cand
        if c_off < 0 or c_off + 4 > filesize:
            return None
        count = struct.unpack_from("<I", buf, c_off)[0]
        if count > max_count:
            return None
        rec_start = c_off + 4
        if rec_start + size * count > filesize:
            return None
        if count == 0:
            return (0.1, None)
        rec = buf[rec_start:rec_start + size]
        if len(rec) < size:
            return None
        try:
            vals, sc, _ = decode_record(rec, fields, tokens)
        except Exception:
            return None
        return (sc, vals)

    r0 = eval_at(0)
    good_bar = len(tokens) * 0.6
    r0_count = struct.unpack_from("<I", buf, off)[0] if off + 4 <= filesize else None
    # count==0 at skip=0 (an empty table) is always accepted outright: there is no
    # record to falsify it, and in practice it is a common, legitimate outcome (e.g.
    # UNIONSCROLL_ESSENCE in this realm has zero entries) -- rejecting it just because
    # its score can't clear good_bar sent every later table on a wild goose chase in
    # an earlier version of this walker.
    if r0 is not None and (r0_count == 0 or r0[0] > good_bar):
        sc, values0 = r0
        return (off, r0_count, 4 + r0_count * size, 0, sc, values0)

    best = None
    for cand in range(-neg_window, window):
        r = eval_at(cand)
        if r is None:
            continue
        sc, vals = r
        if best is None or sc > best[0]:
            best = (sc, cand, vals)
    if best is None:
        return None
    sc, cand, values0 = best
    c_off = off + cand
    count = struct.unpack_from("<I", buf, c_off)[0]
    return (c_off, count, 4 + count * size, cand, sc, values0)


def main():
    data_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_DATA_PATH
    tables_full = parse_cfg_full(CFG_PATH)
    buf = open(data_path, "rb").read()

    off = 8  # 4 bytes de versao (ex.: 0x3000009c) + 4 bytes de timestamp de build
    report = []
    for idx, (name, _flag, fields, tokens) in enumerate(tables_full):
        size = table_size(tokens)
        if size is None:
            if idx == 58:  # 059 - TALK_PROC, unica tabela de tamanho variavel do arquivo
                count, new_off, sample = read_talk_proc_table(buf, off)
                report.append(
                    f"[{idx:3d}] {name:35s} off={off} count={count} (tamanho variavel) "
                    f"consumed={new_off - off} sample={sample!r}"
                )
                off = new_off
                continue
            report.append(f"[{idx:3d}] {name:35s} TAMANHO VARIAVEL nao tratada -- PARANDO aqui")
            break
        override_note = ""
        if idx in TABLE_OVERRIDES:
            ov = TABLE_OVERRIDES[idx]
            count = ov["count"]
            if "abs_count_off" in ov:
                # Ancorado numa posicao ABSOLUTA do arquivo (achada por impressao digital
                # binaria contra uma fonte externa, ver README), nao relativa ao offset que
                # o caminhador vinha acumulando -- por isso funciona mesmo que as tabelas
                # entre a ultima correcao conhecida e esta ainda nao tenham sido resolvidas.
                c_off = ov["abs_count_off"]
                skip = c_off - off
            else:
                skip = ov["skip"]
                c_off = off + skip
            consumed = 4 + count * size
            sc = float("nan")
            rec0 = buf[c_off + 4: c_off + 4 + size] if count > 0 else b""
            values0 = None
            if len(rec0) == size:
                try:
                    values0, sc, _ = decode_record(rec0, fields, tokens)
                except Exception:
                    values0 = None
            override_note = " [skip/count com OVERRIDE manual -- ver README]"
        else:
            res = try_table(buf, off, fields, tokens, size)
            if res is None:
                report.append(f"[{idx:3d}] {name:35s} FALHOU -- nenhum alinhamento plausivel perto de off={off}")
                break
            c_off, count, consumed, skip, sc, values0 = res
        snippet = ""
        if values0:
            for v in values0:
                if isinstance(v, str) and v.strip():
                    snippet = v[:30]
                    break
        report.append(
            f"[{idx:3d}] {name:35s} off={off} skip={skip} count={count} size={size} "
            f"score={sc:.1f} consumed={consumed} sample={snippet!r}{override_note}"
        )
        off = c_off + consumed

    report.append(f"\noffset final alcancado: {off} / tamanho do arquivo {len(buf)}")
    text = "\n".join(report)
    out_path = sys.argv[2] if len(sys.argv) > 2 else None
    if out_path:
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(text)
    else:
        sys.stdout.buffer.write(text.encode("utf-8"))
        sys.stdout.buffer.write(b"\n")


if __name__ == "__main__":
    main()
