import { describe, expect, it } from "vitest";
import { ajusteDeLealdade, atributosNoNivel, chanceDeAcerto, danoDoGolpe, esperado, lutar, mascoteNoNivel, monstroDaFicha, reducaoPorDefesa, sementeira, simular } from "./mascote";

/* Campos reais do `elements.data` do realm_126 (leitor `specs/elements_layouts/pw_elements_reader.py`, 2026-11-03). */
const VESPAO = { ID: 10521, id_type: 8782, Name: "Vespão Pequeno", level_max: 150, level_require: 8,
  hp_a: 25, hp_b: 0.10000000149011612, hp_c: 2, hp_gen_a: 0.5555559992790222, hp_gen_b: 0.10000000149011612, hp_gen_c: 2,
  damage_a: 2.3450000286102295, damage_b: 0.1080000028014183, damage_c: 3.259200096130371, damage_d: 3.7555201053619385,
  speed_a: 7.940000057220459, speed_b: 0.009999999776482582, attack_a: 21.66670036315918, attack_b: 0.10000000149011612, attack_c: 2,
  armor_a: 13.33329963684082, armor_b: 0.10000000149011612, armor_c: 2,
  physic_defence_a: 1.6666699647903442, physic_defence_b: 40, physic_defence_c: 0.10000000149011612, physic_defence_d: -15,
  magic_defence_a: 1.6666699647903442, magic_defence_b: 40, magic_defence_c: 0.10000000149011612, magic_defence_d: -15,
  size: 1, damage_delay: 1.2000000476837158, attack_range: 3, attack_speed: 1.6749999523162842, inhabit_type: 2 };
const FILHOTE = { ID: 10386, id_type: 8782, Name: "Filhote de Lobo Feroz", level_max: 150, level_require: 2,
  hp_a: 27.5, hp_b: 0.10000000149011612, hp_c: 2, hp_gen_a: 0.7222219705581665, hp_gen_b: 0.10000000149011612, hp_gen_c: 2,
  damage_a: 1.625, damage_b: 0.1080000028014183, damage_c: 3.3887999057769775, damage_d: 5.749919891357422,
  speed_a: 6.710000038146973, speed_b: 0.009999999776482582, attack_a: 21.66670036315918, attack_b: 0.10000000149011612, attack_c: 2,
  armor_a: 13.33329963684082, armor_b: 0.10000000149011612, armor_c: 2,
  physic_defence_a: 1.6666699647903442, physic_defence_b: 40, physic_defence_c: 0.10000000149011612, physic_defence_d: -15,
  magic_defence_a: 1.8333300352096558, magic_defence_b: 40, magic_defence_c: 0.10000000149011612, magic_defence_d: -15,
  size: 0.8999999761581421, damage_delay: 1.149999976158142, attack_range: 3, attack_speed: 1.25, inhabit_type: 0 };
const LOBO_REI = { ID: 4359, Name: "Lobo-rei de Hades", level: 42, life: 146772, defence: 1782,
  magic_defences_1: 1103, magic_defences_2: 1103, magic_defences_3: 1793, magic_defences_4: 1103, magic_defences_5: 644,
  immune_type: 6080, attack: 21850, armor: 204, damage_min: 925, damage_max: 2157, attack_range: 4.599999904632568,
  attack_speed: 1.75, damage_delay: 0.800000011920929, hp_regenerate: 36 };

describe("GenerateBaseProp (petdataman.h:23-76)", () => {
  it("dano no nível 30 = o Atq da janela do mascote no jogo (B135: Vespão 466, Filhote 332)", () => {
    expect(atributosNoNivel(VESPAO, 30).dano).toBe(466);
    expect(atributosNoNivel(FILHOTE, 30).dano).toBe(332);
  });
  it("as outras fórmulas com o nível exigido", () => {
    // vida = (int)(25 × (30 − 0,1 × 8 + 2)) = (int)(25 × 31,2) = 780
    expect(atributosNoNivel(VESPAO, 30).vida).toBe(780);
    // defesa = (int)(1,66667 × (40 × (30 − 0,8) − 15)) = (int)(1,66667 × 1153) = 1921
    expect(atributosNoNivel(VESPAO, 30).defesa).toBe(1921);
  });
  it("lealdade: −40/−20/0/+20 % (pet_filter.cpp:8) e divisão inteira do corpo_do_mascote", () => {
    expect([10, 100, 300, 999].map(ajusteDeLealdade)).toEqual([-40, -20, 0, 20]);
    expect(mascoteNoNivel(VESPAO, 30, 999).danoMin).toBe(Math.trunc(466 * 120 / 100));
    expect(mascoteNoNivel(VESPAO, 30, 300).intervalo).toBe(33); // (int)(1,675 × 20 + 0,1)
  });
});

describe("golpe (pw_gs::combat::resolver)", () => {
  it("acerto e defesa como o original", () => {
    expect(chanceDeAcerto(100, 300)).toBeCloseTo(100 / 250, 6);
    expect(chanceDeAcerto(1, 100000)).toBe(0.05);
    expect(reducaoPorDefesa(1782, 40)).toBeCloseTo(1782 / (1782 + 1600 - 25), 5);
    expect(reducaoPorDefesa(1e9, 1)).toBe(0.95);
  });
  it("o Lobo-rei é imune a físico? não: 6080 são estados — o golpe passa", () => {
    const a = mascoteNoNivel(VESPAO, 40, 300), b = monstroDaFicha(LOBO_REI);
    const d = danoDoGolpe(a, b, a.danoMin, [0, 0, 0, 0, 0]);
    expect(d).toBeGreaterThan(0);
  });
  it("imune a físico sem dano mágico: nada passa", () => {
    const a = mascoteNoNivel(VESPAO, 40, 300), b = { ...monstroDaFicha(LOBO_REI), imunidades: 1 };
    expect(danoDoGolpe(a, b, 500, [0, 0, 0, 0, 0])).toBeNull();
    expect(esperado(a, b).imune).toBe(true);
  });
});

describe("luta", () => {
  it("nenhum dos dois aguenta o Lobo-rei sozinho no nível 40", () => {
    const lobo = monstroDaFicha(LOBO_REI);
    for (const p of [VESPAO, FILHOTE]) {
      const r = simular(mascoteNoNivel(p, 40, 999), lobo, 50, sementeira(7));
      expect(r.vitorias[0]).toBe(0);
    }
  });
  it("a mesma semente dá a mesma luta, e quem morre fica com 0", () => {
    const a = mascoteNoNivel(VESPAO, 30, 300), b = mascoteNoNivel(FILHOTE, 30, 300);
    const x = lutar(a, b, sementeira(42), true), y = lutar(a, b, sementeira(42), true);
    expect(x).toEqual(y);
    expect(x.vencedor).not.toBe("empate");
    expect(Math.min(...x.vidaFinal)).toBe(0);
    expect(x.curva!.length).toBeGreaterThan(2);
  });
});
