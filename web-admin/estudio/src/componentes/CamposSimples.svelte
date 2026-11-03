<script lang="ts">
  /* B392: campos de uma ficha (número ou texto), com o nome do registro citado e a explicação. Os valores são do pai
     (`valores`, ligável); `originais` marca o que mudou. Campo que a ficha não tem não aparece. */
  import type { Campo } from "../lib/api";
  import { rotuloGerado } from "../lib/catalogo";

  let { nomes, porNome, valores = $bindable(), originais, editavel, rotulos = {}, explicacoes = {} }:
    { nomes: string[]; porNome: Record<string, Campo>; valores: Record<string, number | string>; originais: Record<string, number | string>;
      editavel: boolean; rotulos?: Record<string, string>; explicacoes?: Record<string, string> } = $props();
  const lista = $derived(nomes.filter((n) => n in porNome));
  const rotulo = (n: string) => rotulos[n] || porNome[n]?.rotulo || rotuloGerado(n) || n;
</script>

{#if lista.length}
  <div class="grade-campos">
    {#each lista as n (n)}
      {@const c = porNome[n]}
      <label title={n}>
        <span>{rotulo(n)}</span>
        {#if c.tipo === "int32" || c.tipo === "float"}
          <input class="campo" class:alterado={valores[n] !== originais[n]} type="number" step={c.tipo === "float" ? "any" : "1"} disabled={!editavel}
            value={valores[n]} oninput={(e) => { const t = (e.target as HTMLInputElement).value; valores[n] = t === "" ? "" : Number(t); }} />
        {:else}
          <input class="campo" class:alterado={valores[n] !== originais[n]} disabled={!editavel} value={valores[n]}
            oninput={(e) => (valores[n] = (e.target as HTMLInputElement).value)} />
        {/if}
        {#if c.referencia?.nome && valores[n] === originais[n]}<span class="mini">{c.referencia.nome}</span>{/if}
        {#if c.referencia?.ausente && valores[n] === originais[n]}<span class="mini erro">id {c.referencia.id} não existe</span>{/if}
        {#if explicacoes[n]}<span class="mini">{explicacoes[n]}</span>{/if}
      </label>
    {/each}
  </div>
{/if}
