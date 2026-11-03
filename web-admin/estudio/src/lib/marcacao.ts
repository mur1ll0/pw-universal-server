/* B392: a marcação de texto do cliente (falas de NPC, conversas, nomes de missão). Regra de quem desenha o texto,
   `AUInterface/AUICommon.cpp:870-910` (a mesma no traço e na medida):
     ^^ e ^&   → o caractere em si (^ ou &)
     ^U + 6    → sublinhado com a cor dos 6 caracteres seguintes (pula 8 no total)
     ^u ^o ^N  → liga/desliga sublinhado e afins (pula 2)
     ^Onnn     → fonte nnn (pula 5)
     ^ + 6     → cor RRGGBB (pula 7; o cliente não confere se são hexadecimais)
     &         → sozinho, não é desenhado (marca de citação, `nNumQuote++`)
   Os códigos ficam no arquivo e ocupam espaço: a opção de conversa cabe em 63 unidades (`conversas.rs::texto_fixo`). */

export interface Trecho {
  texto: string;
  cor: string | null;
  sublinhado: boolean;
}

export interface Leitura {
  trechos: Trecho[];
  /** posições (em unidades) de códigos `^` com cor que não é hexadecimal — o cliente pinta com lixo */
  invalidos: number[];
}

const HEX = /^[0-9a-fA-F]{6}$/;

export function ler(s: string): Leitura {
  const trechos: Trecho[] = [];
  const invalidos: number[] = [];
  let cor: string | null = null;
  let sublinhado = false;
  let buf = "";
  const sai = () => {
    if (!buf) return;
    trechos.push({ texto: buf, cor, sublinhado });
    buf = "";
  };
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === "&") continue;
    if (c !== "^") { buf += c; continue; }
    const n = s[i + 1];
    if (n === undefined) break;
    if (n === "^" || n === "&") { buf += n; i++; continue; }
    if (n === "U") { sai(); sublinhado = true; i += 7; continue; }
    if (n === "u") { sai(); sublinhado = !sublinhado; i += 1; continue; }
    if (n === "o" || n === "N") { i += 1; continue; }
    if (n === "O" && i + 4 < s.length) { i += 4; continue; }
    const hex = s.substr(i + 1, 6);
    sai();
    if (HEX.test(hex)) cor = hex.toLowerCase();
    else invalidos.push(i);
    i += 6;
  }
  sai();
  return { trechos, invalidos };
}

/** O texto como o jogador lê, sem os códigos. */
export const semCodigos = (s: string) => ler(s).trechos.map((t) => t.texto).join("");

/** Unidades UTF-16 (o que o arquivo guarda; os códigos contam). */
export const unidades = (s: string) => s.length;

/** Pinta o trecho [ini, fim) com a cor e volta à `depois` no fim (sem seleção, insere só o código). */
export function aplicarCor(s: string, ini: number, fim: number, cor: string, depois = "ffffff"): { texto: string; cursor: number } {
  const c = cor.replace(/^#/, "").toLowerCase();
  if (!HEX.test(c)) throw new Error(`cor inválida: ${cor}`);
  if (ini === fim) return { texto: s.slice(0, ini) + `^${c}` + s.slice(ini), cursor: ini + 7 };
  const meio = s.slice(ini, fim);
  const fecha = /\^[0-9a-fA-F]{6}$/.test(meio) ? "" : `^${depois}`;
  const texto = s.slice(0, ini) + `^${c}` + meio + fecha + s.slice(fim);
  return { texto, cursor: ini + 7 + meio.length + fecha.length };
}

/** Tira os códigos de cor `^RRGGBB` do trecho [ini, fim) (do texto inteiro quando não há seleção). */
export function tirarCores(s: string, ini = 0, fim = s.length): string {
  if (ini === fim) { ini = 0; fim = s.length; }
  return s.slice(0, ini) + s.slice(ini, fim).replace(/\^[0-9a-fA-F]{6}/g, "") + s.slice(fim);
}

/** Cores de partida da paleta: as que os textos do cliente 1.5.5 mais usam ficam em `CORES_DO_ARQUIVO` quando medidas. */
export const PALETA: { cor: string; nome: string }[] = [
  { cor: "ffffff", nome: "Branco (padrão)" },
  { cor: "ffcb4a", nome: "Ouro" },
  { cor: "ff4040", nome: "Vermelho" },
  { cor: "00ff00", nome: "Verde" },
  { cor: "66ccff", nome: "Azul-claro" },
  { cor: "ffff00", nome: "Amarelo" },
  { cor: "ff9933", nome: "Laranja" },
  { cor: "cc99ff", nome: "Lilás" },
  { cor: "aaaaaa", nome: "Cinza" },
];
