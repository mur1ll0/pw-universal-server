//! B247: controladores e `idCtrl` das áreas de um `npcgen.data` (diagnóstico).
use pw_data_loader::npcgen_editavel::{ler_campos, NpcgenEditavel, CAMPOS_DA_AREA, CAMPOS_DO_CONTROLADOR, CAMPOS_DO_OBJETO};
fn main() {
    let p = std::env::args().nth(1).expect("caminho do npcgen.data");
    let n = NpcgenEditavel::ler(&std::fs::read(p).unwrap()).unwrap();
    println!("versão {}", n.versao);
    for c in &n.controladores {
        let m = ler_campos(c, CAMPOS_DO_CONTROLADOR, n.versao);
        println!("ctrl id {} iControllerID {} ativo {}", m["id"], m["iControllerID"], m["bActived"]);
    }
    let mut refs: Vec<i64> = n.areas.iter().filter_map(|a| ler_campos(&a.bruto, CAMPOS_DA_AREA, n.versao).get("idCtrl").and_then(|x| x.as_i64())).filter(|&c| c != 0).collect();
    refs.sort();
    refs.dedup();
    println!("idCtrl das áreas: {refs:?}");
    let o: Vec<_> = n.objetos.iter().filter_map(|o| ler_campos(o, CAMPOS_DO_OBJETO, n.versao).get("idController").cloned()).collect();
    println!("idController dos objetos (10 primeiros): {:?}", &o[..o.len().min(10)]);
}
