//! Modelo de tipos do **segundo** modelo de fio: os subcomandos do `GamedataSend`.
//!
//! Os protocolos GNET (ver `ty.rs`) são big-endian e usam `CompactUINT` para prefixar
//! tamanhos. Os comandos do mundo 3D não têm nada disso. Em
//! `CElementClient/Network/EC_GPDataType.h`, tudo entre o `#pragma pack(1)` da linha
//! 522 e o `#pragma pack()` da 6.189 é copiado para o payload por `memcpy` cru:
//!
//! * **sem alinhamento** — `pack(1)`, de modo que o deslocamento de um campo é a soma
//!   dos tamanhos dos anteriores, sem preenchimento;
//! * **little-endian** — é a memória do processo i386 do cliente indo direto para o
//!   fio, sem conversão de ordem de bytes;
//! * **sem prefixo de tamanho** — listas trazem um `count` explícito declarado como
//!   campo, e o restante do buffer é lido elemento a elemento.
//!
//! São dois modelos incompatíveis convivendo na mesma conexão, e é por isso que este
//! módulo existe separado do `ty.rs` em vez de reaproveitá-lo.
//!
//! Os tamanhos abaixo são os do alvo original — Win32/i386, 32 bits — e não os do host
//! onde o `pw-rpcgen` roda. `size_t` tem 4 bytes, não 8, e `long` tem 4, não 8.

use crate::json::Json;
use crate::ty::Prim;

/// Tamanho de `A3DVECTOR3`: três `float` consecutivos, sem preenchimento.
pub const VEC3_BYTES: usize = 12;

/// Tipo de um campo de struct empacotada.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GdTy {
    /// Escalar copiado cru, em little-endian.
    Prim(Prim),
    /// `A3DVECTOR3` — três `float`. Aparece em 25 campos (posição, destino, direção).
    Vec3,
    /// Referência a outra struct empacotada, pelo nome qualificado (`S2C::info_npc`).
    Struct(String),
    /// O `BYTE placeholder` que marca o início de uma lista de tamanho variável.
    /// Não é um campo de dados: é o endereço de onde os elementos começam.
    Placeholder,
    /// Tipo que **não** é serializável por `memcpy` (`abase::vector<T>`, `char*`) ou
    /// que o parser não soube resolver. Fica no IR com o texto C++ original para que a
    /// divergência apareça na revisão em vez de virar um campo silenciosamente errado.
    Unresolved(String),
}

impl GdTy {
    /// Tamanho em bytes no alvo i386, ou `None` se o tipo não for resolvível.
    ///
    /// `Struct` também devolve `None`: o tamanho de uma struct aninhada só é conhecido
    /// depois que todas as declarações foram lidas, então quem resolve isso é o
    /// `gd_ir`, que tem a tabela completa em mãos.
    pub fn inline_bytes(&self) -> Option<usize> {
        match self {
            GdTy::Prim(p) => Some(p.wire_size()),
            GdTy::Vec3 => Some(VEC3_BYTES),
            GdTy::Placeholder => Some(1),
            GdTy::Struct(_) | GdTy::Unresolved(_) => None,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        matches!(self, GdTy::Unresolved(_))
    }

    pub fn to_json(&self) -> Json {
        match self {
            GdTy::Prim(p) => Json::object([
                ("kind", Json::str("prim")),
                ("prim", Json::str(p.as_str())),
                ("bytes", Json::Int(p.wire_size() as i64)),
            ]),
            GdTy::Vec3 => Json::object([
                ("kind", Json::str("vec3")),
                ("prim", Json::str("f32")),
                ("count", Json::Int(3)),
                ("bytes", Json::Int(VEC3_BYTES as i64)),
            ]),
            GdTy::Struct(name) => Json::object([
                ("kind", Json::str("struct")),
                ("name", Json::str(name.clone())),
            ]),
            GdTy::Placeholder => Json::object([("kind", Json::str("placeholder"))]),
            GdTy::Unresolved(raw) => Json::object([
                ("kind", Json::str("unresolved")),
                ("cxx", Json::str(raw.clone())),
            ]),
        }
    }
}

/// Traduz um tipo C++ escrito no cabeçalho para o modelo empacotado.
///
/// Recebe o tipo já normalizado (espaços colapsados, sem `const`). Nomes desconhecidos
/// que pareçam identificadores simples viram `Struct`, e a existência da struct é
/// conferida depois, quando a tabela completa está montada.
pub fn translate(cxx: &str) -> GdTy {
    let t = normalize(cxx);
    match t.as_str() {
        // 1 byte
        "char" | "signed char" | "int8_t" => GdTy::Prim(Prim::I8),
        "unsigned char" | "byte" | "BYTE" | "unsigned __int8" | "uint8_t" => GdTy::Prim(Prim::U8),
        "bool" => GdTy::Prim(Prim::Bool),

        // 2 bytes
        "short" | "short int" | "signed short" | "__int16" | "int16_t" => GdTy::Prim(Prim::I16),
        "unsigned short" | "unsigned short int" | "WORD" | "uint16_t" => GdTy::Prim(Prim::U16),

        // 4 bytes — i386: `long` e `size_t` têm 32 bits.
        "int" | "signed int" | "long" | "long int" | "__int32" | "LONG" | "int32_t" => {
            GdTy::Prim(Prim::I32)
        }
        "unsigned" | "unsigned int" | "unsigned long" | "unsigned long int" | "DWORD"
        | "size_t" | "UINT" | "ULONG" | "uint32_t" => GdTy::Prim(Prim::U32),

        // 8 bytes
        "__int64" | "long long" | "signed __int64" | "int64_t" => GdTy::Prim(Prim::I64),
        "unsigned __int64" | "unsigned long long" | "ULONGLONG" | "uint64_t" => {
            GdTy::Prim(Prim::U64)
        }

        // ponto flutuante
        "float" => GdTy::Prim(Prim::F32),
        "double" => GdTy::Prim(Prim::F64),

        // vetor 3D do motor gráfico
        "A3DVECTOR3" | "A3DVECTOR" => GdTy::Vec3,

        other => {
            // `abase::vector<T>` tem ponteiros internos e `char*` é um endereço do
            // processo do cliente. Nenhum dos dois sobrevive a um `memcpy` para o fio,
            // então o IR marca ambos em vez de fingir um tamanho.
            if other.contains('*') || other.contains('<') {
                return GdTy::Unresolved(cxx.trim().to_string());
            }
            // Caminho com escopo (`info_pet::skills`, `externa::building_data`): é uma
            // struct aninhada, que o parser nomeia assim. Sem isto, toda struct
            // declarada dentro de outra viraria um campo sem tamanho.
            if is_scoped_identifier(other) {
                GdTy::Struct(other.to_string())
            } else {
                GdTy::Unresolved(cxx.trim().to_string())
            }
        }
    }
}

/// Colapsa espaços e remove qualificadores que não mudam o formato no fio.
fn normalize(cxx: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for word in cxx.split_whitespace() {
        match word {
            "const" | "volatile" | "struct" | "class" | "mutable" => continue,
            w => parts.push(w),
        }
    }
    let joined = parts.join(" ");
    // `int *` e `int*` são o mesmo tipo; normalizar para detectar ponteiros adiante.
    joined.replace(" *", "*").replace("* ", "*")
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Um ou mais identificadores separados por `::`, sem parâmetros de template.
fn is_scoped_identifier(s: &str) -> bool {
    !s.is_empty() && s.split("::").all(is_identifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_t_e_long_tem_quatro_bytes_no_alvo_i386() {
        // O host onde o pw-rpcgen roda é 64 bits; o cliente original não é. Se este
        // teste passar a falhar, todos os deslocamentos de campo saem errados.
        assert_eq!(translate("size_t").inline_bytes(), Some(4));
        assert_eq!(translate("unsigned long").inline_bytes(), Some(4));
        assert_eq!(translate("long").inline_bytes(), Some(4));
        assert_eq!(translate("__int64").inline_bytes(), Some(8));
    }

    #[test]
    fn traduz_os_tipos_do_inventario() {
        assert_eq!(translate("int"), GdTy::Prim(Prim::I32));
        assert_eq!(translate("unsigned char"), GdTy::Prim(Prim::U8));
        assert_eq!(translate("unsigned short"), GdTy::Prim(Prim::U16));
        assert_eq!(translate("char"), GdTy::Prim(Prim::I8));
        assert_eq!(translate("unsigned int"), GdTy::Prim(Prim::U32));
        assert_eq!(translate("short"), GdTy::Prim(Prim::I16));
        assert_eq!(translate("float"), GdTy::Prim(Prim::F32));
        assert_eq!(translate("bool"), GdTy::Prim(Prim::Bool));
        assert_eq!(translate("BYTE"), GdTy::Prim(Prim::U8));
        assert_eq!(translate("byte"), GdTy::Prim(Prim::U8));
        assert_eq!(translate("DWORD"), GdTy::Prim(Prim::U32));
    }

    #[test]
    fn tipos_do_stdint_sao_reconhecidos() {
        // O `protocol.h` do servidor usa `int64_t`/`uint64_t` em máscaras de equipamento
        // e em valores de dinheiro. Sem isto eles caíam como "struct desconhecida", e as
        // structs que os contêm ficavam sem tamanho — foi o compilador de 32 bits que
        // apontou a falta.
        assert_eq!(translate("int64_t").inline_bytes(), Some(8));
        assert_eq!(translate("uint64_t").inline_bytes(), Some(8));
        assert_eq!(translate("int32_t").inline_bytes(), Some(4));
        assert_eq!(translate("uint16_t").inline_bytes(), Some(2));
        assert_eq!(translate("uint8_t").inline_bytes(), Some(1));
    }

    #[test]
    fn a3dvector3_sao_tres_floats_sem_preenchimento() {
        assert_eq!(translate("A3DVECTOR3"), GdTy::Vec3);
        assert_eq!(translate("A3DVECTOR3").inline_bytes(), Some(VEC3_BYTES));
    }

    #[test]
    fn tipos_nao_memcpy_ficam_marcados() {
        assert!(translate("abase::vector<int>").is_unresolved());
        assert!(translate("abase::vector<building_data>").is_unresolved());
        assert!(translate("char*").is_unresolved());
        assert!(translate("char *").is_unresolved());
    }

    #[test]
    fn nome_desconhecido_simples_vira_referencia_de_struct() {
        assert_eq!(translate("info_matter"), GdTy::Struct("info_matter".into()));
        assert_eq!(translate("player_info_2"), GdTy::Struct("player_info_2".into()));
        assert_eq!(translate("const info_npc"), GdTy::Struct("info_npc".into()));
    }

    #[test]
    fn caminho_com_escopo_e_struct_aninhada_e_nao_tipo_desconhecido() {
        // O parser nomeia structs aninhadas assim. Rejeitar `::` de forma geral fazia
        // toda struct declarada dentro de outra perder o tamanho.
        assert_eq!(
            translate("info_pet::skills"),
            GdTy::Struct("info_pet::skills".into())
        );
        // Mas um template continua fora: `abase::vector<T>` não é copiável por memcpy.
        assert!(translate("abase::vector<IconState>").is_unresolved());
    }

    #[test]
    fn placeholder_ocupa_um_byte_mas_nao_e_dado() {
        assert_eq!(GdTy::Placeholder.inline_bytes(), Some(1));
    }
}
