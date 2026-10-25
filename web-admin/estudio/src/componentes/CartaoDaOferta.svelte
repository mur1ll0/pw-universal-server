<script lang="ts">
  /* Cartão da vitrine como o da loja do cliente (`CDlgQShopItem`): o ícone do ITEM (atlas `iconlist_ivtr*`,
     `DlgQShopItem.cpp:380`), o `szName` da oferta (:449), o preço da 1ª opção em gold (:447-448, `GetCashText`) e o
     selo (novo/promo/hot/desconto/VIP, :276-296, 455-462). Passar o mouse no ícone mostra a dica do item. */
  import { esconderDica, mostrarDica } from "../lib/dica.svelte";
  import { precoCheio, precoEmGold, selo as seloDe, type Oferta } from "../lib/gshop";
  import IconeItem from "./IconeItem.svelte";

  let { oferta, escolhida = false, problemas = 0, avisos = 0, grande = false, aoEscolher }:
    { oferta: Oferta; escolhida?: boolean; problemas?: number; avisos?: number; grande?: boolean; aoEscolher?: () => void } = $props();
  const s = $derived(seloDe(oferta.selo));
  const desativada = $derived(oferta.mostrado === 0);
  const cheio = $derived(precoCheio(oferta.mostrado, oferta.selo));
</script>

<button type="button" class="cartao" class:escolhida class:desativada class:grande onclick={aoEscolher}
  title={desativada ? "Desativada: nenhuma opção tem preço (o cliente não mostra)" : `#${oferta.indice} · item ${oferta.id}`}>
  {#if s.cor}<span class="selo {s.cor}">{s.curto}</span>{/if}
  <span class="indice">#{oferta.indice}</span>
  <span class="icone" role="presentation" onmouseenter={(e) => mostrarDica(oferta.id, oferta.item ?? oferta.nome, e.currentTarget as HTMLElement)} onmouseleave={esconderDica}>
    <IconeItem hex={oferta.icone} nome={oferta.item ?? ""} tamanho={grande ? 48 : 36} />
  </span>
  <span class="nome">{oferta.nome || oferta.item || `Item ${oferta.id}`}{#if oferta.quantidade > 1} ×{oferta.quantidade}{/if}</span>
  <span class="preco">
    {#if desativada}<span class="mini">sem preço</span>
    {:else}
      {#if cheio !== null}<s class="mini">{precoEmGold(cheio)}</s>{/if}
      {precoEmGold(oferta.mostrado)} <span class="mini">gold</span>
    {/if}
  </span>
  <span class="marcas">
    {#if oferta.vip}<span class="marca vip" title="VIP mínimo da 1ª opção">VIP {oferta.vip}</span>{/if}
    {#if oferta.brinde}<span class="marca" title="Tem brinde (item {oferta.brinde})">🎁</span>{/if}
    {#if oferta.opcoes > 1}<span class="marca" title="Opções de compra que o GS vende">{oferta.opcoes} opções</span>{/if}
    {#if problemas}<span class="marca erro" title="Erros na conferência">⚠ {problemas}</span>
    {:else if avisos && !desativada}<span class="marca aviso" title="Avisos na conferência">! {avisos}</span>{/if}
  </span>
</button>

<style>
  .cartao { position: relative; display: grid; grid-template-columns: auto 1fr; grid-template-rows: auto auto auto; column-gap: 10px; row-gap: 2px;
    align-items: center; text-align: left; padding: 10px; min-height: 86px; border-radius: var(--raio); border: 1px solid var(--borda);
    background: var(--superficie-2); color: var(--texto); font: inherit; cursor: pointer; width: 100%; }
  .cartao:hover { border-color: var(--ouro); }
  .cartao.escolhida { border-color: var(--ouro); box-shadow: 0 0 0 2px var(--ouro-suave); }
  .cartao.desativada { opacity: .55; }
  .cartao.grande { min-height: 110px; padding: 14px; cursor: default; }
  .icone { grid-row: 1 / span 3; display: inline-flex; }
  .nome { font-weight: 600; font-size: 13px; line-height: 1.25; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .preco { color: var(--ouro); font-family: var(--fonte-numero); font-size: 13px; font-variant-numeric: tabular-nums; }
  .preco s { margin-right: 4px; }
  .marcas { display: flex; gap: 4px; flex-wrap: wrap; }
  .marca { font-size: 11px; padding: 0 5px; border-radius: 5px; background: var(--fundo-2); color: var(--texto-2); }
  .marca.vip { color: var(--azul); }
  .marca.erro { color: var(--texto-erro); background: var(--vermelho-suave); }
  .marca.aviso { color: var(--ouro); background: var(--ouro-suave); }
  .indice { position: absolute; right: 8px; bottom: 6px; font-size: 11px; color: var(--texto-3); font-variant-numeric: tabular-nums; }
  .selo { position: absolute; top: -1px; right: -1px; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 0 var(--raio) 0 8px; letter-spacing: .03em; }
  .selo.novo { background: var(--verde); color: #08130d; }
  .selo.promo { background: var(--azul); color: #0a1220; }
  .selo.hot { background: var(--vermelho); color: #1c0909; }
  .selo.desconto { background: var(--ouro); color: #16120a; }
  .selo.relampago { background: #c27cff; color: #170a22; }
  .selo.vip { background: var(--azul-suave); color: var(--azul); border: 1px solid var(--azul); }
</style>
