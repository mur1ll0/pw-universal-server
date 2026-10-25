/* B383: como o cliente monta cada equipamento no personagem, para o Estúdio exportar e importar o modelo certo.
   - Modelo próprio (.ecm pendurado num gancho): arma (`file_model_right/left`, gancho `CC_weapon` ou, no orbe, `CC_qiu`;
     `EC_Player.cpp:9972-9993`, carregado com as peles do .smd, `:4547-4583`), asa e voo (`file_model`/`file_model2`,
     `CC_chibang` nas asas, `CC_feijian` na espada voadora e nas rodas de fogo, `:1940-1966`), arma de moda (v156).
   - Pele sobre o esqueleto do corpo: armadura e moda não têm modelo — o cliente monta
     `Models\Players\装备\{男|女}\<realname>\<grupo><realname>{一级,二级,三级}.ski` (`ChangeArmor`; tabela do binário em
     `Estudio::GRUPOS_DE_PELE`) e liga a pele ao esqueleto de `Models\Players\形象\<classe>\躯干\<classe>.ecm`. */

export interface CampoDeModelo { rotulo: string; campo: string }

/** Os campos de modelo próprio (.ecm) de cada tabela, na ordem em que aparecem. `file_matter` (no chão) fica no Visual 3D. */
export function camposDeModelo(tabela: string, existe: (c: string) => boolean): CampoDeModelo[] {
  const todos: CampoDeModelo[] = [
    { rotulo: "Mão direita", campo: "file_model_right" },
    { rotulo: "Mão esquerda", campo: "file_model_left" },
    { rotulo: tabela === "FLYSWORD_ESSENCE" ? "Modelo (pé esquerdo nas rodas)" : "Modelo", campo: "file_model" },
    { rotulo: "Modelo 2 (pé direito nas rodas)", campo: "file_model2" },
  ];
  return todos.filter((c) => existe(c.campo));
}

/** Os ganchos que o esqueleto do modelo novo tem de ter (um basta), pela tabela e pelo modo de voo. */
export function ganchosExigidos(tabela: string, campo: string, modoDeVoo: number): string[] {
  if (campo === "file_model_right" || campo === "file_model_left") return ["CC_weapon", "CC_qiu"];
  if (tabela === "WINGMANWING_ESSENCE") return ["CC_chibang"];
  if (tabela === "FLYSWORD_ESSENCE") return modoDeVoo === 1 ? ["CC_chibang"] : ["CC_feijian"];
  return [];
}

/** A pasta sugerida para o modelo novo ("Tudo"). */
export function pastaSugerida(tabela: string): string {
  if (tabela === "FLYSWORD_ESSENCE" || tabela === "WINGMANWING_ESSENCE") return "models\\players\\通用装备\\estudio";
  return "models\\weapons\\estudio";
}

/** Armadura e moda: a peça é pele sobre o corpo (sem `file_model`). */
export const ehPeleNoCorpo = (tabela: string) => tabela === "ARMOR_ESSENCE" || tabela === "FASHION_ESSENCE";

const NIVEIS = ["一级", "二级", "三级"];
/** Os três `.ski` da peça (o mesmo que `Estudio::caminhos_da_pele`). */
export function caminhosDaPele(realname: string, genero: number, grupo: string): string[] {
  const g = genero === 0 ? "男" : "女";
  return NIVEIS.map((n) => `Models\\Players\\装备\\${g}\\${realname}\\${grupo}${realname}${n}.ski`);
}

/** A pele do corpo e a da cabeça da classe (sem equipamento), para vestir a peça sobre o corpo na prévia. */
export function pelesDoCorpo(nome: string): string[] {
  return [`Models\\Players\\形象\\${nome}\\躯干\\${nome}.ski`, `Models\\Players\\形象\\${nome}\\头\\${nome}头.ski`];
}

/** Nomes legíveis dos grupos de pele do binário. */
export const NOME_DO_GRUPO: Record<string, string> = {
  "男通用": "Masculino comum (todas as classes masculinas, menos o Bárbaro)",
  "女通用": "Feminino comum (todas as classes femininas, menos a Feiticeira)",
  "妖兽": "Bárbaro (masculino)",
  "妖精": "Feiticeira (feminino)",
};
