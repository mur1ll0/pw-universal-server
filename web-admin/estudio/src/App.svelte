<script lang="ts">
  /* Estúdio (B366, fase 1: Equipáveis/Armas). Rotas por hash:
     #/                               tela principal (categorias)
     #/equipaveis/<tabela>/<indice>   edição (Armas: tela própria)
     #/itens/novo?retorno=<código>    item novo para a aba de origem (BroadcastChannel) */
  import { onMount } from "svelte";
  import { contexto, sessao, versaoDoRealm } from "./lib/api";
  import { avisos, definirRascunho, estudio, rota } from "./lib/estado.svelte";
  import Topo from "./componentes/Topo.svelte";
  import Principal from "./telas/Principal.svelte";
  import EdicaoArma from "./telas/EdicaoArma.svelte";
  import NovoItem from "./telas/NovoItem.svelte";

  let pronto = $state(false);

  onMount(async () => {
    try {
      const s = await sessao();
      if (!s) { estudio.erro = "Sessão do painel expirada: entre no painel e abra o Estúdio de novo."; return; }
      estudio.usuario = s.usuario ?? "";
      const realm = rota.params.get("realm") || "";
      if (!realm) { estudio.erro = "Abra o Estúdio pelo menu do painel, com um realm escolhido."; return; }
      estudio.realm = realm;
      contexto.realm = realm;
      estudio.versao = await versaoDoRealm(realm);
      contexto.versao = estudio.versao;
      const r = Number(rota.params.get("rascunho")) || null;
      definirRascunho(r);
      pronto = true;
    } catch (e) {
      estudio.erro = (e as Error).message;
    }
  });
</script>

<Topo {pronto} />

<main>
  {#if estudio.erro}
    <p class="caixa erro">{estudio.erro} <a href="/">Voltar ao painel</a></p>
  {:else if !pronto}
    <p class="suave">Carregando…</p>
  {:else if rota.caminho[0] === "equipaveis" && rota.caminho[1] && rota.caminho[2] !== undefined}
    {#key `${rota.caminho[1]}/${rota.caminho[2]}/${estudio.rascunho}`}
      <EdicaoArma tabela={rota.caminho[1]} indice={Number(rota.caminho[2])} />
    {/key}
  {:else if rota.caminho[0] === "itens" && rota.caminho[1] === "novo"}
    <NovoItem />
  {:else}
    <Principal />
  {/if}
</main>

<div class="avisos" aria-live="polite">
  {#each avisos as a (a.id)}
    <div class="aviso {a.tipo}">{a.texto}</div>
  {/each}
</div>

<style>
  main { max-width: 1400px; margin: 0 auto; padding: 16px; }
  .avisos { position: fixed; right: 16px; bottom: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 80; max-width: min(420px, calc(100vw - 32px)); }
  .aviso { padding: 10px 14px; border-radius: 10px; background: var(--superficie-2); border: 1px solid var(--borda-forte); box-shadow: var(--sombra); }
  .aviso.ok { border-color: var(--verde); }
  .aviso.erro { border-color: var(--vermelho); color: var(--texto-erro); }
  .aviso.aviso { border-color: var(--ouro); }
</style>
