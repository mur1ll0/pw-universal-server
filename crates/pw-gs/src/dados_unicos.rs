//! Dados únicos do servidor — o `UniqueDataClient` do original (`uniquedataclient.h/.cpp`).
//!
//! São pares `chave → valor` do servidor inteiro, que a política do `aipolicy.data` lê
//! (`expr_history_value`, `aitrigger.cpp:160-165`) e muda (`op_revise_history` soma,
//! `op_set_history` define — `aitrigger.cpp:907-915`, `_ModifyDataNoLock`,
//! `uniquedataclient.cpp:400-460`). Chave nunca escrita vale 0 (`UDOctets val(0)`).
//!
//! **Não é igual:** no original o valor fica no banco do `gdeliveryd` e vale para todos os
//! processos do servidor, com aviso ao cliente nas chaves registradas com `noticeclient`. Aqui
//! fica na memória do processo do GS (todos os mapas dele) e volta a 0 ao reiniciar — `falta` a
//! gravação no banco e o aviso ao cliente.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn tabela() -> &'static Mutex<HashMap<i32, i32>> {
    static T: OnceLock<Mutex<HashMap<i32, i32>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `GetData(key, val)`.
pub fn ler(chave: i32) -> i32 {
    tabela().lock().map(|t| t.get(&chave).copied().unwrap_or(0)).unwrap_or(0)
}

/// `ModifyData(key, val, setflag)`: `definir` troca o valor; senão soma ao que há.
pub fn modificar(chave: i32, valor: i32, definir: bool) -> bool {
    let Ok(mut t) = tabela().lock() else { return false };
    let v = t.entry(chave).or_insert(0);
    *v = if definir { valor } else { v.wrapping_add(valor) };
    true
}

#[cfg(test)]
mod testes {
    #[test]
    fn soma_e_define() {
        let k = 0x7AB0_0001;
        assert_eq!(super::ler(k), 0);
        assert!(super::modificar(k, 5, false));
        assert!(super::modificar(k, 3, false));
        assert_eq!(super::ler(k), 8);
        assert!(super::modificar(k, 2, true));
        assert_eq!(super::ler(k), 2);
    }
}
