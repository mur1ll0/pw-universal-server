#!/usr/bin/env bash
# Hook de fim de turno (Stop): lembra de atualizar as specs quando há código alterado e
# nenhuma spec nem o estado foram tocados. Regra em specs/README.md e na skill
# pw-atualizar-specs.
#
# Avisa uma vez por estado do diff: guarda o hash do diff do código em .git/ e não repete o
# lembrete para o mesmo conjunto de mudanças (se a mudança não exige spec, o agente diz isso
# e segue sem ser cobrado de novo).

entrada=$(cat)
case "$entrada" in
  *'"stop_hook_active":true'* | *'"stop_hook_active": true'*) exit 0 ;;
esac

cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0
git rev-parse --git-dir >/dev/null 2>&1 || exit 0

alterados=$(git status --porcelain 2>/dev/null | awk '{print $NF}')
codigo=$(printf '%s\n' "$alterados" | grep -E '^(crates|tools|docker|scripts|web-admin)/' | grep -vE '/tests/|\.md$')
[ -z "$codigo" ] && exit 0

documentos=$(printf '%s\n' "$alterados" | grep -E '^(specs/|docs/ESTADO_E_RETOMADA\.md|docs/HISTORICO_DE_SESSOES\.md)')
[ -n "$documentos" ] && exit 0

marca="$(git rev-parse --git-dir)/pw-lembrete-specs"
hash=$( { printf '%s\n' "$codigo"; git diff -- $codigo 2>/dev/null; } | sha1sum | cut -d' ' -f1)
[ -f "$marca" ] && [ "$(cat "$marca")" = "$hash" ] && exit 0
printf '%s' "$hash" > "$marca"

lista=$(printf '%s' "$codigo" | head -8 | tr '\n' ' ')
printf '{"decision":"block","reason":"Há código alterado (%s) e nenhuma spec, nem docs/ESTADO_E_RETOMADA.md, foi atualizada. Siga a skill pw-atualizar-specs: atualize a spec da área (mapa em specs/README.md) e o estado. Se a mudança não altera comportamento descrito (teste, comentário, refatoração), diga isso ao usuário em uma linha e encerre."}\n' "$lista"
exit 0
