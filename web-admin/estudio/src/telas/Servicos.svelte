<script lang="ts">
  /* B392: área Serviços — os serviços de NPC por fora de qualquer NPC (um serviço serve a vários). O mesmo painel do
     modal do NPC, sem o vínculo: no editor, "Usado por" leva aos NPCs. Rota `#/servicos?tabela=&indice=`. */
  import { ir, rota } from "../lib/estado.svelte";
  import Icone from "../componentes/Icone.svelte";
  import PainelDeServicos from "../componentes/PainelDeServicos.svelte";

  const tabela = rota.params.get("tabela") ?? "";
  const indice = rota.params.get("indice");
  function lembrar(t: string, i: number) {
    const p = new URLSearchParams(rota.params);
    p.set("tabela", t); p.set("indice", String(i));
    history.replaceState(null, "", `#/servicos?${p}`);
  }
</script>

<div class="barra">
  <button class="botao" type="button" onclick={() => ir("")}><Icone nome="voltar" /> Voltar</button>
  <h2>Serviços de NPC</h2>
  <span class="mini">Lojas, forjas, teleportes, treinadores, missões… — o que um NPC oferece. Para ligar a um NPC, abra o NPC (aba Serviços).</span>
</div>
<section class="caixa">
  <PainelDeServicos tabelaInicial={tabela} indiceInicial={indice != null ? Number(indice) : null} aoAbrir={lembrar} />
</section>

<style>
  .barra { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-bottom: 14px; }
  h2 { margin: 0; font-size: 18px; }
</style>
