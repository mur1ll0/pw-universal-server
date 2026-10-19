//! Embute o ícone e o manifesto (`launcher.rc`): controles comuns v6, compatibilidade declarada
//! do Windows 7 ao 11 e consciência de DPI (a janela escala pelo `LOGPIXELSY`).
fn main() {
    println!("cargo:rerun-if-changed=launcher.rc");
    println!("cargo:rerun-if-changed=launcher.manifest");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    embed_resource::compile("launcher.rc", embed_resource::NONE);
}
