//! A string `edition` do `Challenge` — a segunda porta do login.
//!
//! O cliente compara **duas** coisas no `Challenge`, e reprova antes de olhar a senha se
//! qualquer uma falhar (`CElementClient/Network/EC_GameSession.cpp:4011`):
//!
//! ```cpp
//! AString str((const char *)p->edition.begin(), p->edition.size());
//! if (p->version != g_pGame->GetGameVersion() || stricmp(g_pGame->GetVersionString(), str))
//! ```
//!
//! O `version` é o [`GameVersion::server_version_code`]. O `edition` é a
//! `GetVersionString()`, montada em `CElementClient/EC_Game.cpp:646`:
//!
//! ```cpp
//! m_strAllVersion.Format("%x%x%x%x", ELEMENTDATA_VERSION, _task_templ_cur_version,
//!                        globaldata_getgshop_timestamp(), globaldata_getgshop_timestamp2());
//! ```
//!
//! São quatro números em hexadecimal minúsculo, **concatenados sem separador e sem
//! preenchimento**. O servidor original monta a mesma string em
//! `cgame/gs/global_manager.cpp:32`, com os mesmos quatro valores.
//!
//! # De onde vem cada valor
//!
//! | Valor | No cliente | O que o servidor usa |
//! | :--- | :--- | :--- |
//! | `ELEMENTDATA_VERSION` | constante de compilação (`CCommon/ExpTypes.h:16` → `0x3000007f`) | 1ª palavra do `elements.data` do realm |
//! | `_task_templ_cur_version` | constante de compilação (`Task/TaskTempl.cpp:5` → `121`) | 2ª palavra do `tasks.data` do realm |
//! | `gshop_timestamp` | `gshop.data` | 1º `u32` de `gshop.data`/`gshopsev.data` |
//! | `gshop_timestamp2` | `gshop1.data` | 1º `u32` de `gshop1.data`/`gshopsev1.data` |
//!
//! Os dois timestamps vêm de **arquivos diferentes** — não são dois campos do mesmo
//! arquivo. É por isso que o `pw-data-loader` passou a carregar o `gshop1.data`.
//!
//! As duas primeiras linhas são o assunto das duas seções seguintes: os números da coluna
//! do meio são de uma compilação do cliente que **não é** a que os jogadores rodam, e a
//! coluna da direita é como se descobre, para cada realm, o número que o cliente dele quer.
//!
//! # O cliente é a autoridade
//!
//! Os fontes do servidor 1.5.3 definem `ELEMENTDATA_VERSION` como **`0x30000080`**
//! (`cgame/gs/template/exptypes.h:16`), um a mais que o do cliente. As duas árvores de
//! fonte não são da mesma compilação — o mesmo desencontro que aparece nos três campos
//! de layout divergentes entre elas.
//!
//! Aqui isso não é um detalhe: quem valida é o cliente, então o valor que vale é o
//! **dele**. Usar o do servidor produz uma string que não bate, e o login falha com a
//! mesma mensagem genérica de versão errada.
//!
//! # E o cliente varia por *build*, não por versão de protocolo
//!
//! O `EC.log` do cliente que o Murillo roda (build **2552**, "1.5.3") registra a recusa com
//! as duas strings lado a lado:
//!
//! ```text
//! local ver: 300000917c571db3f456986c25
//! server ver: 3000007f7900
//! ```
//!
//! Separando a do cliente pelos quatro `%x` — e usando os dois timestamps como âncora, já
//! que eles foram conferidos byte a byte nos `gshopsev.data`/`gshopsev1.data` daquele realm
//! (`0x571db3f4` e `0x56986c25`) — o que sobra à esquerda é `30000091` e `7c`:
//!
//! | Valor | Fontes que temos | Cliente build 2552 |
//! | :--- | :--- | :--- |
//! | `ELEMENTDATA_VERSION` | `0x3000007f` | **`0x30000091`** |
//! | `_task_templ_cur_version` | `121` | **`124` (`0x7c`)** |
//!
//! Ou seja: as duas "constantes" são da compilação do cliente, e a árvore de fontes vazada
//! é de **outra**. Fixá-las no código como um número por versão de protocolo estava errado
//! por construção — o mesmo "1.5.3" pode ter builds diferentes, e cada realm serve um.
//!
//! # Onde os dois números realmente moram: nos `.data` do realm
//!
//! Os dois não precisam ser configurados nem deduzidos — eles **estão nos arquivos que a
//! pasta do realm já tem**, porque o cliente se recusa a carregar dados de outra versão:
//!
//! | Valor | Onde | O cliente recusa se não bater |
//! | :--- | :--- | :--- |
//! | `ELEMENTDATA_VERSION` | 1ª palavra de `elements.data` | `CCommon/elementdataman.cpp:3619` |
//! | `_task_templ_cur_version` | 2ª palavra de `tasks.data` (após o mágico `0x93858361`) | `Task/TaskTemplMan.cpp:1599` |
//!
//! Conferido nas pastas reais dos realms:
//!
//! ```text
//! realm_153/config/elements.data  →  30000091   (= o `30000091` do EC.log)
//! realm_153/config/tasks.data     →  93858361 0000007c   (0x7c = 124 = o `7c` do EC.log)
//! realm_126/config/elements.data  →  30000007
//! realm_126/config/tasks.data     →  93858361 00000037   (55)
//! ```
//!
//! Os dois números do cliente do Murillo saem inteiros dos arquivos dele. É por isso que
//! [`VersaoDoCliente::resolver`] lê os `.data` do realm antes de qualquer padrão nosso, e
//! `ELEMENTDATA_VERSION`/`TASK_TEMPL_VERSION` no ambiente ficam só como saída de emergência
//! para uma pasta de dados que não corresponda ao cliente instalado.

use crate::version::GameVersion;

/// As duas constantes de compilação **do cliente** que entram no `edition`.
///
/// Não são propriedades do protocolo: são do binário do cliente que aquele realm serve.
/// Ver a seção "E o cliente varia por *build*" na documentação do módulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersaoDoCliente {
    /// `ELEMENTDATA_VERSION` (`CCommon/ExpTypes.h`).
    pub elements_data: u32,
    /// `_task_templ_cur_version` (`Task/TaskTempl.cpp`).
    pub task_templ: u32,
}

/// Nome da variável de ambiente que sobrescreve o `ELEMENTDATA_VERSION` do realm.
pub const VAR_ELEMENTS: &str = "ELEMENTDATA_VERSION";
/// Nome da variável de ambiente que sobrescreve o `_task_templ_cur_version` do realm.
pub const VAR_TASK: &str = "TASK_TEMPL_VERSION";

impl VersaoDoCliente {
    /// O padrão para uma versão de protocolo: o valor medido do cliente que temos.
    pub fn padrao(version: GameVersion) -> Self {
        match version {
            // O 1.2.6 não manda `edition` nenhum ([`GameVersion::challenge_has_edition`]),
            // então estes dois valores não vão para o fio em realm nenhum dessa versão.
            // Ficam com o número dos fontes só para não inventar outro.
            GameVersion::V1_2_6 => Self { elements_data: 0x3000_007F, task_templ: 121 },
            // Sem medição: herda o do 1.5.3, que é o cliente mais próximo que medimos.
            GameVersion::V1_4_8 => Self { elements_data: 0x3000_0091, task_templ: 124 },
            // Medido no `EC.log` do cliente build 2552 — ver o módulo. Não é `0x3000007f`
            // /121: aquele par é da árvore de fontes vazada, de outra compilação.
            GameVersion::V1_5_3 => Self { elements_data: 0x3000_0091, task_templ: 124 },
            // Constantes de compilação lidas direto dos fontes do EvolvedPW (2026-09-02):
            // `ELEMENTDATA_VERSION` em `CCommon/ExpTypes.h:16` (mesmo valor do 1.5.3 —
            // parece que o formato de `elements.data` não mudou) e
            // `_task_templ_cur_version` em `Task/TaskTempl.cpp:5` = 125. Ainda não
            // cruzado contra o `elements.data`/`tasks.data` reais de `data/realm_155/`
            // (o `resolver()` abaixo lê o do realm por cima disto de qualquer forma).
            GameVersion::V1_5_5 => Self { elements_data: 0x3000_0091, task_templ: 125 },
        }
    }

    /// O padrão da versão, com `ELEMENTDATA_VERSION` e `TASK_TEMPL_VERSION` do ambiente
    /// sobrescrevendo quando presentes.
    ///
    /// `buscar` existe para o teste poder chamar isto sem mexer no ambiente do processo.
    ///
    /// # Formato
    ///
    /// - `ELEMENTDATA_VERSION`: **hexadecimal**, com ou sem `0x` — é como o cliente o
    ///   imprime e como os fontes o declaram (`30000091` ou `0x30000091`).
    /// - `TASK_TEMPL_VERSION`: **decimal** (`124`).
    ///
    /// Um valor que não parseia é erro de configuração e vira `Err`: adivinhar aqui
    /// produziria um `edition` errado, e o cliente responde a isso com a mesma mensagem
    /// genérica de "versão errada" que não aponta para lugar nenhum.
    pub fn do_ambiente_com(
        version: GameVersion,
        buscar: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let mut v = Self::padrao(version);
        let (elements, task) = Self::apenas_do_ambiente(buscar)?;
        if let Some(e) = elements {
            v.elements_data = e;
        }
        if let Some(t) = task {
            v.task_templ = t;
        }
        Ok(v)
    }

    /// O que o ambiente **de fato** define, sem misturar com padrão nenhum.
    ///
    /// `None` aqui significa "a variável não está definida" — e é diferente de "está
    /// definida com o mesmo valor do padrão". Confundir os dois faria um realm cujo
    /// `elements.data` diz uma coisa ignorar um ambiente que diz outra, sempre que o
    /// ambiente coincidisse com o padrão da versão.
    fn apenas_do_ambiente(
        buscar: impl Fn(&str) -> Option<String>,
    ) -> Result<(Option<u32>, Option<u32>), String> {
        let elements = match buscar(VAR_ELEMENTS) {
            None => None,
            Some(bruto) => {
                let limpo = bruto.trim();
                let sem_prefixo = limpo
                    .strip_prefix("0x")
                    .or_else(|| limpo.strip_prefix("0X"))
                    .unwrap_or(limpo);
                Some(u32::from_str_radix(sem_prefixo, 16).map_err(|_| {
                    format!(
                        "{VAR_ELEMENTS}={bruto:?} não é hexadecimal (esperado algo como 30000091)"
                    )
                })?)
            }
        };

        let task = match buscar(VAR_TASK) {
            None => None,
            Some(bruto) => Some(bruto.trim().parse::<u32>().map_err(|_| {
                format!("{VAR_TASK}={bruto:?} não é decimal (esperado algo como 124)")
            })?),
        };

        Ok((elements, task))
    }

    /// [`Self::do_ambiente_com`] lendo o ambiente do processo.
    pub fn do_ambiente(version: GameVersion) -> Result<Self, String> {
        Self::do_ambiente_com(version, |k| std::env::var(k).ok())
    }

    /// A escolha completa, em ordem de autoridade **decrescente**:
    ///
    /// 1. o ambiente (`ELEMENTDATA_VERSION`, `TASK_TEMPL_VERSION`) — a saída de emergência;
    /// 2. os cabeçalhos dos `.data` **do próprio realm**, quando existem;
    /// 3. o padrão da versão de protocolo.
    ///
    /// O item 2 é o que faz isto deixar de ser adivinhação: o cliente **recusa a carregar**
    /// um `elements.data` cuja primeira palavra não seja o seu `ELEMENTDATA_VERSION`
    /// (`elementdataman.cpp:3619`) e um `tasks.data` cuja `version` não seja o seu
    /// `_task_templ_cur_version` (`TaskTemplMan.cpp:1599`). Então, para um realm cujos
    /// dados aquele cliente consegue abrir, o número certo está dentro dos arquivos que já
    /// estão na pasta — não há o que configurar nem o que deduzir.
    pub fn resolver(
        version: GameVersion,
        elements_do_realm: Option<u32>,
        tasks_do_realm: Option<u32>,
        buscar: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let mut v = Self::padrao(version);
        if let Some(e) = elements_do_realm {
            v.elements_data = e;
        }
        if let Some(t) = tasks_do_realm {
            v.task_templ = t;
        }

        // O ambiente por último: é ele que ganha de tudo, para o caso de uma pasta de
        // dados que não corresponda ao cliente que os jogadores rodam.
        let (do_ambiente_elements, do_ambiente_task) = Self::apenas_do_ambiente(buscar)?;
        if let Some(e) = do_ambiente_elements {
            v.elements_data = e;
        }
        if let Some(t) = do_ambiente_task {
            v.task_templ = t;
        }
        Ok(v)
    }
}

/// As parcelas da string `edition` — quatro para a maioria das versões, cinco para
/// clientes compilados com `VIP` (achado no 1.5.5, ver [`Self::gshop3_timestamp`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub elements_version: u32,
    pub task_version: u32,
    pub gshop_timestamp: u32,
    pub gshop2_timestamp: u32,
    /// O quinto valor, só presente quando a versão o exige
    /// (`GameVersion::challenge_edition_tem_terceiro_gshop`). `EC_Game.cpp` do 1.5.5
    /// (`EvolvedPWClient`) tem um `#ifdef VIP` que troca o formato de `"%x%x%x%x"` para
    /// `"%x%x%x%x%x"`, acrescentando `globaldata_getgshop_timestamp3()` no fim — lido de
    /// `Data\gshop2.data` no cliente (`gshopsev2.data` do lado do servidor).
    pub gshop3_timestamp: Option<u32>,
}

impl Edition {
    /// Monta o `edition` para uma versão, com os timestamps lidos dos `.data` do realm
    /// (`gshop.data`, `gshop1.data` e, se a versão usar, `gshop2.data`) e as constantes
    /// **padrão** do cliente daquela versão.
    ///
    /// Um realm que sirva outro build do cliente usa [`Self::com_versao_do_cliente`].
    pub fn new(
        version: GameVersion,
        gshop_timestamp: u32,
        gshop2_timestamp: u32,
        gshop3_timestamp: Option<u32>,
    ) -> Self {
        Self::com_versao_do_cliente(
            VersaoDoCliente::padrao(version),
            gshop_timestamp,
            gshop2_timestamp,
            gshop3_timestamp,
        )
    }

    /// Como [`Self::new`], mas com as constantes do cliente vindas da configuração do
    /// realm em vez do padrão da versão.
    pub fn com_versao_do_cliente(
        cliente: VersaoDoCliente,
        gshop_timestamp: u32,
        gshop2_timestamp: u32,
        gshop3_timestamp: Option<u32>,
    ) -> Self {
        Self {
            elements_version: cliente.elements_data,
            task_version: cliente.task_templ,
            gshop_timestamp,
            gshop2_timestamp,
            gshop3_timestamp,
        }
    }

    /// A string como o cliente a monta: hexadecimal **minúsculo**, sem separador e sem
    /// preenchimento — é literalmente `"%x%x%x%x"` (ou `"%x%x%x%x%x"` com
    /// [`Self::gshop3_timestamp`]).
    ///
    /// Sem preenchimento significa que um timestamp pequeno ocupa menos dígitos, e a
    /// string encurta. Isso é fiel ao original: o cliente compara com `stricmp`, e não
    /// por tamanho.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut s = format!(
            "{:x}{:x}{:x}{:x}",
            self.elements_version, self.task_version, self.gshop_timestamp, self.gshop2_timestamp
        );
        if let Some(g3) = self.gshop3_timestamp {
            s.push_str(&format!("{g3:x}"));
        }
        s.into_bytes()
    }
}

impl GameVersion {
    /// `ELEMENTDATA_VERSION` padrão do cliente daquela versão.
    ///
    /// Atalho para [`VersaoDoCliente::padrao`]; um realm pode sobrescrever.
    pub fn elements_data_version(&self) -> u32 {
        VersaoDoCliente::padrao(*self).elements_data
    }

    /// `_task_templ_cur_version` padrão do cliente daquela versão.
    ///
    /// Atalho para [`VersaoDoCliente::padrao`]; um realm pode sobrescrever.
    pub fn task_template_version(&self) -> u32 {
        VersaoDoCliente::padrao(*self).task_templ
    }

    /// Se o `Challenge` desta versão carrega `edition` e `exp_rate`.
    ///
    /// O 1.2.6 encerra o pacote no `algo`; as versões seguintes acrescentam os dois
    /// campos no fim.
    pub fn challenge_has_edition(&self) -> bool {
        !matches!(self, GameVersion::V1_2_6)
    }

    /// Se o `edition` desta versão carrega um quinto valor (`gshop_time_stamp3`).
    ///
    /// Achado no 1.5.5 (`EvolvedPWClient/EC_Game.cpp:655`, ramo `#ifdef VIP`) — não é
    /// leitura direta do binário (não temos o executável compilado), é inferência apoiada
    /// em dois fatos concordando: o ramo `VIP` lê um terceiro arquivo de gshop
    /// (`Data\gshop2.data`), e a pasta `data/realm_155/config` que o Murillo extraiu tem
    /// os três (`gshopsev.data`, `gshopsev1.data`, **`gshopsev2.data`**) — o realm_153 só
    /// tinha os dois primeiros. Documentado como inferência, não como medição, porque é o
    /// que de fato é.
    pub fn challenge_edition_tem_terceiro_gshop(&self) -> bool {
        matches!(self, GameVersion::V1_5_5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_e_hexadecimal_minusculo_sem_separador() {
        let e = Edition {
            elements_version: 0x3000_007F,
            task_version: 121,
            gshop_timestamp: 0x4A2B_1C3D,
            gshop2_timestamp: 0x4A2B_1C40,
            gshop3_timestamp: None,
        };
        // "%x%x%x%x": 3000007f + 79 + 4a2b1c3d + 4a2b1c40
        assert_eq!(e.to_wire(), b"3000007f794a2b1c3d4a2b1c40");
    }

    #[test]
    fn nao_ha_preenchimento_a_esquerda() {
        // `%x` de 1 é "1", não "00000001". Preencher mudaria a string e o cliente
        // recusaria o login com a mensagem genérica de versão errada.
        let e = Edition {
            elements_version: 1,
            task_version: 2,
            gshop_timestamp: 3,
            gshop2_timestamp: 4,
            gshop3_timestamp: None,
        };
        assert_eq!(e.to_wire(), b"1234");
    }

    #[test]
    fn o_153_usa_as_constantes_do_cliente_e_nao_as_do_servidor() {
        // Os fontes do servidor dizem 0x30000080; nenhuma árvore de fonte manda aqui.
        // Quem valida é o cliente, então o padrão é o que o cliente **mediu**.
        let e = Edition::new(GameVersion::V1_5_3, 0, 0, None);
        assert_ne!(e.elements_version, 0x3000_0080, "esse é o valor do servidor");
        assert_eq!(e.elements_version, 0x3000_0091);
        assert_eq!(e.task_version, 124);
    }

    #[test]
    fn o_padrao_do_153_reproduz_a_string_do_ec_log() {
        // O teste que fecha o caso: o `EC.log` do cliente build 2552 imprimiu
        // `local ver: 300000917c571db3f456986c25`. Com os dois timestamps lidos dos
        // `gshopsev*.data` daquele realm, é exatamente isso que temos que escrever.
        let e = Edition::new(GameVersion::V1_5_3, 0x571d_b3f4, 0x5698_6c25, None);
        assert_eq!(
            String::from_utf8(e.to_wire()).unwrap(),
            "300000917c571db3f456986c25"
        );
    }

    #[test]
    fn o_155_tem_cinco_valores_quando_o_terceiro_gshop_e_passado() {
        // Achado no 1.5.5 (EvolvedPWClient, EC_Game.cpp, ramo `#ifdef VIP`): o `edition`
        // ganha um quinto valor no fim. `Edition::new` só o inclui quando o chamador passa
        // `Some` — a decisão de qual versão precisa disso é do `GameVersion`, testada em
        // `challenge_edition_tem_terceiro_gshop` logo abaixo.
        let e = Edition::new(GameVersion::V1_5_5, 0x11, 0x22, Some(0x33));
        // elements=30000091, task=125(0x7d), gshop=11, gshop2=22, gshop3=33
        assert_eq!(String::from_utf8(e.to_wire()).unwrap(), "300000917d112233");
    }

    #[test]
    fn sem_o_terceiro_gshop_o_155_tambem_aceita_quatro_valores() {
        // `Edition::new` não força o quinto valor pela versão sozinha — quem decide é o
        // chamador (`gateway.rs`, olhando `challenge_edition_tem_terceiro_gshop`). Um
        // `None` aqui continua produzindo a string de quatro valores, sem erro.
        let e = Edition::new(GameVersion::V1_5_5, 0x11, 0x22, None);
        assert_eq!(String::from_utf8(e.to_wire()).unwrap(), "300000917d1122");
    }

    #[test]
    fn so_o_155_precisa_do_terceiro_gshop() {
        assert!(GameVersion::V1_5_5.challenge_edition_tem_terceiro_gshop());
        assert!(!GameVersion::V1_5_3.challenge_edition_tem_terceiro_gshop());
        assert!(!GameVersion::V1_2_6.challenge_edition_tem_terceiro_gshop());
        assert!(!GameVersion::V1_4_8.challenge_edition_tem_terceiro_gshop());
    }

    #[test]
    fn o_ambiente_sobrescreve_as_duas_constantes() {
        // O caso do realm que serve outro build: nem recompilar, nem editar código.
        let v = VersaoDoCliente::do_ambiente_com(GameVersion::V1_5_3, |k| match k {
            VAR_ELEMENTS => Some("0x30000080".into()),
            VAR_TASK => Some("121".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(v.elements_data, 0x3000_0080);
        assert_eq!(v.task_templ, 121);
    }

    #[test]
    fn o_elements_do_ambiente_e_lido_como_hexadecimal() {
        // Sem prefixo `0x` — é a forma em que o valor aparece no `EC.log` e nos fontes.
        // Ler `30000091` como decimal daria 30.000.091 e um `edition` errado, com o
        // cliente recusando o login sem dizer por quê.
        let v = VersaoDoCliente::do_ambiente_com(GameVersion::V1_5_3, |k| {
            (k == VAR_ELEMENTS).then(|| "30000091".to_string())
        })
        .unwrap();
        assert_eq!(v.elements_data, 0x3000_0091);
    }

    #[test]
    fn um_valor_ilegivel_no_ambiente_e_erro_e_nao_silencio() {
        let erro = VersaoDoCliente::do_ambiente_com(GameVersion::V1_5_3, |k| {
            (k == VAR_TASK).then(|| "cento e vinte e quatro".to_string())
        })
        .unwrap_err();
        assert!(erro.contains(VAR_TASK), "a mensagem tem que nomear a variável: {erro}");

        let erro = VersaoDoCliente::do_ambiente_com(GameVersion::V1_5_3, |k| {
            (k == VAR_ELEMENTS).then(|| "3000009z".to_string())
        })
        .unwrap_err();
        assert!(erro.contains(VAR_ELEMENTS), "{erro}");
    }

    #[test]
    fn os_dados_do_realm_ganham_do_padrao_da_versao() {
        // O caso do realm_153 do Murillo: `elements.data` começa com 0x30000091 e
        // `tasks.data` traz 124. Não importa o que o nosso padrão ache.
        let v = VersaoDoCliente::resolver(
            GameVersion::V1_5_3,
            Some(0x3000_0091),
            Some(124),
            |_| None,
        )
        .unwrap();
        assert_eq!(v.elements_data, 0x3000_0091);
        assert_eq!(v.task_templ, 124);

        // E o mesmo mecanismo com outros dados devolve outros números — sem isto, o teste
        // acima passaria só por o padrão do 1.5.3 coincidir.
        let outro =
            VersaoDoCliente::resolver(GameVersion::V1_5_3, Some(0x3000_0007), Some(55), |_| None)
                .unwrap();
        assert_eq!(outro.elements_data, 0x3000_0007, "o `elements.data` do realm foi ignorado");
        assert_eq!(outro.task_templ, 55, "o `tasks.data` do realm foi ignorado");
    }

    #[test]
    fn o_ambiente_ganha_dos_dados_do_realm() {
        // A saída de emergência: pasta de dados de um build, cliente de outro.
        let v = VersaoDoCliente::resolver(
            GameVersion::V1_5_3,
            Some(0x3000_0091),
            Some(124),
            |k| match k {
                VAR_ELEMENTS => Some("3000007f".into()),
                VAR_TASK => Some("121".into()),
                _ => None,
            },
        )
        .unwrap();
        assert_eq!(v.elements_data, 0x3000_007F);
        assert_eq!(v.task_templ, 121);
    }

    #[test]
    fn um_ambiente_igual_ao_padrao_ainda_e_uma_decisao() {
        // Se "definido com o valor do padrão" fosse confundido com "não definido", este
        // realm acabaria usando o 0x30000091 do `elements.data` — o contrário do que o
        // administrador escreveu. O bug não apareceria em nenhum outro teste.
        let padrao = VersaoDoCliente::padrao(GameVersion::V1_5_3);
        let v = VersaoDoCliente::resolver(
            GameVersion::V1_5_3,
            Some(0x3000_0007),
            Some(55),
            |k| (k == VAR_ELEMENTS).then(|| format!("{:x}", padrao.elements_data)),
        )
        .unwrap();
        assert_eq!(v.elements_data, padrao.elements_data);
        assert_eq!(v.task_templ, 55, "o `tasks.data` do realm devia ter sido mantido");
    }

    #[test]
    fn sem_dados_e_sem_ambiente_sobra_o_padrao_da_versao() {
        let v = VersaoDoCliente::resolver(GameVersion::V1_2_6, None, None, |_| None).unwrap();
        assert_eq!(v, VersaoDoCliente::padrao(GameVersion::V1_2_6));
    }

    #[test]
    fn sem_variaveis_o_ambiente_devolve_o_padrao_da_versao() {
        let v = VersaoDoCliente::do_ambiente_com(GameVersion::V1_5_3, |_| None).unwrap();
        assert_eq!(v, VersaoDoCliente::padrao(GameVersion::V1_5_3));
    }

    #[test]
    fn o_126_nao_manda_edition() {
        assert!(!GameVersion::V1_2_6.challenge_has_edition());
        assert!(GameVersion::V1_5_3.challenge_has_edition());
    }
}
