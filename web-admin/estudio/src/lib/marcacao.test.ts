import { describe, expect, it } from "vitest";
import { aplicarCor, ler, semCodigos, tirarCores } from "./marcacao";

describe("marcação de texto do cliente (AUICommon.cpp:870-910)", () => {
  it("cor ^RRGGBB pula 7 e troca a cor", () => {
    const l = ler("Olá ^ffcc00Armas^ffffff hoje");
    expect(l.trechos.map((t) => [t.texto, t.cor])).toEqual([["Olá ", null], ["Armas", "ffcc00"], [" hoje", "ffffff"]]);
    expect(l.invalidos).toEqual([]);
  });
  it("^^ e ^& viram o caractere; & sozinho some", () => {
    expect(semCodigos("a^^b^&c&d")).toBe("a^b&cd");
  });
  it("^U com cor pula 8, ^u/^o/^N pulam 2, ^Onnn pula 5", () => {
    expect(semCodigos("x^Uff0000y^uz^ow^NQ^O001r")).toBe("xyzwQr");
    expect(ler("x^Uff0000y").trechos[1].sublinhado).toBe(true);
  });
  it("cor que não é hexadecimal: o cliente pula igual, a tela avisa", () => {
    const l = ler("a^zzzzzzb");
    expect(semCodigos("a^zzzzzzb")).toBe("ab");
    expect(l.invalidos).toEqual([1]);
  });
  it("aplicar e tirar cor", () => {
    const r = aplicarCor("Bem-vindo, viajante", 11, 19, "#FFCB4A");
    expect(r.texto).toBe("Bem-vindo, ^ffcb4aviajante^ffffff");
    expect(tirarCores(r.texto)).toBe("Bem-vindo, viajante");
    expect(aplicarCor("ab", 1, 1, "00ff00").texto).toBe("a^00ff00b");
  });
});
