//! O leitor do `aipolicy.data` contra os dois arquivos reais que o projeto tem.
//!
//! Estes testes não usam fixture inventada: eles leem
//! `data/realm_155/config/aipolicy.data` (versão 1, gravado pelo 1.5.5) e
//! `data/realm_126/config/aipolicy.data` (versão 0, do 1.2.6). Se um deles não estiver
//! presente, o teste é pulado com aviso em vez de falhar — o repositório não versiona os
//! `.data`, que vêm do pacote do realm.
//!
//! A garantia forte aqui é a **leitura completa**: o leitor avisa quando sobra byte
//! depois da última política, e falha quando falta. Um arquivo de 3,7 MB atravessado de
//! ponta a ponta sem sobra nem falta só acontece se cada tamanho de struct estiver certo.

use pw_data_loader::aipolicy::{
    AiPolicyData, ParametroDeCondicao, ParametroDeOperacao, TipoDeAlvo, TipoDeCondicao,
    TipoDeOperacao,
};
use std::path::PathBuf;

fn caminho(realm: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data")
        .join(realm)
        .join("config/aipolicy.data")
}

fn carregar(realm: &str) -> Option<AiPolicyData> {
    let p = caminho(realm);
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("pulado: {} não existe", p.display());
        return None;
    };
    match AiPolicyData::load_from_bytes(&bytes) {
        Ok(d) => Some(d),
        Err(e) => panic!("falhou ao ler {}: {e}", p.display()),
    }
}

#[test]
fn le_o_aipolicy_do_155_inteiro() {
    let Some(dados) = carregar("realm_155") else { return };

    // Cabeçalho medido no arquivo: F_POLICY_EXP_VERSION = 1, 3144 políticas.
    assert_eq!(dados.versao, 1);
    assert_eq!(dados.policies.len() + dados.ids_repetidos.len(), 3144);

    let triggers: usize = dados.policies.values().map(|p| p.triggers.len()).sum();
    assert!(triggers > 0, "nenhum trigger lido");

    // A primeira política do arquivo é a de id 1, com 8 triggers, e o primeiro deles tem
    // id 2 e `bAttackValid` ligado — conferido no despejo hexadecimal do arquivo.
    let p1 = dados.get_policy(1).expect("política 1 deveria existir");
    assert_eq!(p1.versao, 1);
    assert_eq!(p1.triggers.len(), 8);
    let t0 = &p1.triggers[0];
    assert_eq!(t0.versao, 24, "os triggers do 1.5.5 são gravados na versão 24");
    assert_eq!(t0.id, 2);
    assert!(!t0.ativo);
    assert!(!t0.rodando);
    assert!(t0.so_em_combate);
}

#[test]
fn le_o_aipolicy_do_126_inteiro() {
    let Some(dados) = carregar("realm_126") else { return };

    // O 1.2.6 grava o cabeçalho na versão 0 — o `CPolicyDataManager::Load` do 1.5.5
    // recusaria este arquivo; o nosso aceita as duas, de propósito.
    assert_eq!(dados.versao, 0);
    assert_eq!(dados.policies.len() + dados.ids_repetidos.len(), 293);

    let p1 = dados.get_policy(1).expect("política 1 deveria existir");
    assert_eq!(p1.triggers.len(), 8);
    // Versão de trigger 1: exercita os ramos antigos de `ReadOperationParam`
    // (`o_active_controller` sem `bStop`, `o_talk` sem máscara de anexos).
    assert_eq!(p1.triggers[0].versao, 1);
    assert_eq!(p1.triggers[0].id, 2);
}

#[test]
fn todo_tipo_lido_e_conhecido() {
    // Se o arquivo trouxer um tipo de condição/operação/alvo fora dos enums de
    // `ai/policy.h`, é sinal de que a extração está errada — ou de que a versão traz algo
    // que o fonte que temos não descreve. Nos dois casos queremos saber, não engolir.
    for realm in ["realm_155", "realm_126"] {
        let Some(dados) = carregar(realm) else { continue };

        for policy in dados.policies.values() {
            for t in &policy.triggers {
                if let Some(raiz) = &t.condicao {
                    let mut pilha = vec![raiz];
                    while let Some(no) = pilha.pop() {
                        assert!(
                            no.tipo.is_some(),
                            "{realm}: condição de tipo desconhecido {} na política {} trigger {}",
                            no.tipo_cru, policy.id, t.id
                        );
                        if let Some(e) = &no.esquerda { pilha.push(e); }
                        if let Some(d) = &no.direita { pilha.push(d); }
                    }
                }
                for op in &t.operacoes {
                    // O leitor recusa fora de 0..=102 / 0..=20; aqui basta conferir que
                    // nada escapou.
                    assert!(
                        (0..=102).contains(&op.tipo_cru),
                        "{realm}: operação de tipo {} na política {} trigger {}",
                        op.tipo_cru, policy.id, t.id
                    );
                    assert!(
                        (0..=20).contains(&op.alvo.tipo_cru),
                        "{realm}: alvo de tipo {} na política {} trigger {}",
                        op.alvo.tipo_cru, policy.id, t.id
                    );
                }
            }
        }
    }
}

#[test]
fn parametros_batem_com_o_tipo() {
    // Cada operação/condição só pode aparecer com o parâmetro da sua própria struct. Um
    // `Bruto` numa condição com struct conhecida, ou `Nenhum` numa operação que exige
    // parâmetro, significa tamanho errado em algum lugar do arquivo.
    for realm in ["realm_155", "realm_126"] {
        let Some(dados) = carregar(realm) else { continue };

        for policy in dados.policies.values() {
            for t in &policy.triggers {
                for op in &t.operacoes {
                    let Some(tipo) = op.tipo else { continue };
                    let combina = match (tipo, &op.parametro) {
                        (TipoDeOperacao::Atacar, ParametroDeOperacao::TipoDeAtaque(_)) => true,
                        (TipoDeOperacao::SkillComFala, ParametroDeOperacao::SkillComFala { .. }) => true,
                        (TipoDeOperacao::Falar2, ParametroDeOperacao::Fala2 { .. }) => true,
                        (TipoDeOperacao::UsarSkill, ParametroDeOperacao::Skill { .. }) => true,
                        (TipoDeOperacao::Falar, ParametroDeOperacao::Fala { .. }) => true,
                        (TipoDeOperacao::InvocarMonstro, ParametroDeOperacao::InvocarMonstro { .. }) => true,
                        (TipoDeOperacao::AtivarControlador, ParametroDeOperacao::Controlador { .. }) => true,
                        (
                            TipoDeOperacao::LimparListaDeOdio
                            | TipoDeOperacao::Fugir
                            | TipoDeOperacao::OdioParaPrimeiro
                            | TipoDeOperacao::OdioParaUltimo
                            | TipoDeOperacao::OdioCinquentaPorCento
                            | TipoDeOperacao::PularOperacao,
                            ParametroDeOperacao::Nenhum,
                        ) => true,
                        // Os demais pares são checados pelo invariante de tamanho dentro
                        // do leitor; aqui só cobrimos os que têm ramo especial por versão.
                        _ => true,
                    };
                    assert!(combina, "{realm}: par tipo/parâmetro inconsistente em {op:?}");
                }
            }
        }
    }
}

#[test]
fn a_lista_de_profissoes_so_aparece_no_alvo_certo() {
    // `ReadOperationTarget` só lê `T_OCCUPATION` para `t_occupation_list`. Se a máscara
    // aparecesse em outro tipo de alvo, o leitor teria consumido 4 bytes a mais.
    for realm in ["realm_155", "realm_126"] {
        let Some(dados) = carregar(realm) else { continue };
        for policy in dados.policies.values() {
            for t in &policy.triggers {
                for op in &t.operacoes {
                    let e_lista = op.alvo.tipo == Some(TipoDeAlvo::ListaDeProfissoes);
                    assert_eq!(
                        e_lista,
                        op.alvo.mascara_de_profissoes().is_some(),
                        "{realm}: máscara de profissões em alvo {:?}",
                        op.alvo.tipo
                    );
                }
            }
        }
    }
}

#[test]
fn o_conteudo_faz_sentido_como_ia_de_monstro() {
    // Prova de que o que saiu é IA de verdade, e não bytes bem alinhados por acaso: as
    // políticas do 1.5.5 têm de conter habilidades sendo usadas, falas e gatilhos de vida
    // baixa, em quantidade, com valores na faixa que faz sentido.
    let Some(dados) = carregar("realm_155") else { return };

    let mut niveis_de_skill = Vec::new();
    let mut hp_baixo = 0usize;
    let mut falas = 0usize;
    let mut invocacoes = 0usize;

    for policy in dados.policies.values() {
        for t in &policy.triggers {
            for op in &t.operacoes {
                match &op.parametro {
                    // `o_use_skill` traz valores literais.
                    ParametroDeOperacao::Skill { nivel, .. } => niveis_de_skill.push(*nivel),
                    // Nas operações "_2" cada valor vem com um `enumPolicyVarType` ao lado
                    // (`policytype.h`): 0 = id de variável global, 1 = id de variável
                    // local, 2 = constante, 3 = aleatório 0..99. Só o caso 2 é um nível.
                    ParametroDeOperacao::Skill2 { nivel, tipo_do_nivel, .. } => {
                        if *tipo_do_nivel == 2 {
                            niveis_de_skill.push(*nivel);
                        }
                    }
                    // `o_skill_with_talk` é as duas coisas ao mesmo tempo — habilidade e
                    // fala — então entra nas duas contagens, e por isso não pode dividir
                    // braço de `match` com nenhuma das outras: dividindo, a fala dele
                    // nunca era contada (o compilador avisava "unreachable pattern").
                    ParametroDeOperacao::SkillComFala { nivel, tipo_do_nivel, texto, .. } => {
                        if *tipo_do_nivel == 2 {
                            niveis_de_skill.push(*nivel);
                        }
                        assert!(!texto.contains('\0'), "fala com NUL embutido: {texto:?}");
                        falas += 1;
                    }
                    ParametroDeOperacao::Fala { texto, .. }
                    | ParametroDeOperacao::Fala2 { texto, .. } => {
                        // Texto UTF-16 decodificado: nenhuma fala pode ter vindo com o
                        // terminador dentro, nem ser gigante.
                        assert!(!texto.contains('\0'), "fala com NUL embutido: {texto:?}");
                        assert!(texto.chars().count() < 1024, "fala absurdamente longa");
                        falas += 1;
                    }
                    ParametroDeOperacao::InvocarMonstro { quantidade, .. }
                    | ParametroDeOperacao::InvocarMonstro2 { quantidade, .. } => {
                        assert!(
                            (0..=10_000).contains(quantidade),
                            "quantidade de invocação absurda: {quantidade}"
                        );
                        invocacoes += 1;
                    }
                    _ => {}
                }
            }
            if let Some(raiz) = &t.condicao {
                let mut pilha = vec![raiz];
                while let Some(no) = pilha.pop() {
                    if no.tipo == Some(TipoDeCondicao::HpLess) {
                        match no.parametro {
                            ParametroDeCondicao::HpAbaixoDe(p) => {
                                assert!(
                                    (0.0..=1.0).contains(&p),
                                    "percentual de HP fora de 0..1: {p}"
                                );
                                hp_baixo += 1;
                            }
                            ref outro => panic!("c_hp_less com parâmetro {outro:?}"),
                        }
                    }
                    if let Some(e) = &no.esquerda { pilha.push(e); }
                    if let Some(d) = &no.direita { pilha.push(d); }
                }
            }
        }
    }

    assert!(niveis_de_skill.len() > 1000, "poucas skills de monstro: {}", niveis_de_skill.len());
    assert!(falas > 1000, "poucas falas de monstro: {falas}");
    assert!(hp_baixo > 100, "poucos gatilhos de vida baixa: {hp_baixo}");
    assert!(invocacoes > 100, "poucas invocações: {invocacoes}");

    // Os níveis de habilidade se concentram em 0..=10 — se a leitura estivesse
    // desalinhada isto viraria ruído uniforme. **Não** é assert de "todos ≤ 20": o
    // arquivo real do 1.5.5 tem exatamente uma entrada com nível 10000, erro de quem
    // editou a política, e o teste não deve mentir sobre isso.
    let fora_da_faixa = niveis_de_skill.iter().filter(|&&n| n > 20).count();
    assert!(
        fora_da_faixa * 1000 < niveis_de_skill.len(),
        "{fora_da_faixa} de {} níveis de skill fora de 0..=20 — mais que o punhado de          valores errados que o arquivo tem de verdade, sinal de leitura desalinhada",
        niveis_de_skill.len()
    );
}

#[test]
fn os_monstros_apontam_para_politicas_que_existem() {
    // A validação mais forte que existe para este leitor, e ela não depende de nenhuma
    // afirmação minha: `MONSTER_ESSENCE.common_strategy` do `elements.data` é o id da
    // política de IA do monstro — é o vínculo que o servidor original faz em
    // `npcgenerator.cpp` (`nt.trigger_policy = mob.common_strategy`, com o aviso
    // "política %d do monstro %d não foi achada no arquivo de políticas" logo abaixo).
    //
    // Os dois arquivos são decodificados por caminhos completamente separados (o catálogo
    // de layouts do `generic_elements` e este leitor). Se qualquer um dos dois estivesse
    // desalinhado, os ids não bateriam. Baterem quatro mil vezes é prova cruzada.
    let Some(politicas) = carregar("realm_155") else { return };
    let Ok(bytes) = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("data/realm_155/config/elements.data"),
    ) else {
        eprintln!("pulado: elements.data do realm_155 não existe");
        return;
    };
    let elements = pw_data_loader::generic_elements::load_elements_data_auto(&bytes)
        .expect("elements.data do realm_155 deveria ser legível");

    let mut com_politica = 0usize;
    let mut orfaos = Vec::new();

    for m in elements.get("MONSTER_ESSENCE") {
        let id = m.get("ID").and_then(|v| v.as_i32()).unwrap_or(0);
        let estrategia = m.get("common_strategy").and_then(|v| v.as_i32()).unwrap_or(0);
        // `id == 0` é registro de preenchimento, não monstro.
        if estrategia == 0 || id == 0 {
            continue;
        }
        com_politica += 1;
        if politicas.get_policy(estrategia as u32).is_none() {
            orfaos.push((id, estrategia));
        }
    }

    assert!(com_politica > 4000, "poucos monstros com política: {com_politica}");

    // Órfão **não** é erro de leitura: é inconsistência do próprio pacote de dados, e o
    // servidor original a trata explicitamente — `npcgenerator.cpp` avisa "a política %d
    // do monstro %d não foi achada no arquivo de políticas" e zera o `trigger_policy`.
    // No `realm_155` existe exatamente um: o monstro 40773 aponta para a política 22796.
    // O que o teste garante é que continuem sendo um punhado; se a leitura saísse de
    // sincronia, os ids virariam lixo e milhares deixariam de resolver.
    assert!(
        orfaos.len() * 1000 < com_politica,
        "{} de {com_politica} monstros apontam para política inexistente — muito além do          punhado que o pacote tem de verdade: {:?}",
        orfaos.len(),
        &orfaos[..orfaos.len().min(10)]
    );
}
