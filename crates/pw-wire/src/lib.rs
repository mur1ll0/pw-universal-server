//! Os **dois** formatos de fio do Perfect World, um por módulo.
//!
//! Uma única conexão de jogo carrega dois formatos incompatíveis, e confundi-los é a
//! forma mais fácil de escrever um servidor que conecta mas não funciona:
//!
//! | | [`gnet`] | [`gamedata`] |
//! | :--- | :--- | :--- |
//! | Onde | protocolos GNET entre cliente e daemons | subcomandos do `GamedataSend`, o mundo 3D |
//! | Ordem de bytes | **big-endian** | **little-endian** |
//! | Tamanhos | `CompactUINT` antes de `Octets`, strings e contêineres | nenhum prefixo; a contagem é um campo explícito |
//! | Alinhamento | não se aplica (tudo é escrito campo a campo) | `#pragma pack(1)`: deslocamento = soma dos anteriores |
//! | Origem | `marshal_i386.h` / `byteorder_i386.h` do servidor | `memcpy` cru da memória i386 do cliente |
//!
//! O `gamedata` é literalmente a memória do processo do cliente indo para o fio, e é
//! por isso que ele é little-endian e sem preenchimento. O `gnet` é serialização de
//! verdade, escrita campo a campo em ordem de rede.
//!
//! Este crate não conhece protocolo nenhum: ele só sabe ler e escrever os dois
//! formatos. Os esquemas ficam em `specs/protocol/gnet_153.json` e
//! `specs/protocol/gamedata_153.json`, extraídos dos fontes C++ originais pelo
//! `pw-rpcgen`, e o `pw-protocol` é escrito à mão sobre eles usando este crate.
//!
//! Os testes em `tests/` fazem a ponte: eles leem esses dois IRs e conferem, contra as
//! **1.190 structs reais** do jogo, que os deslocamentos que este crate produz são os
//! mesmos que o compilador C++ de 32 bits produziu para os cabeçalhos originais.

pub mod error;
pub mod gamedata;
pub mod gnet;

pub use error::{WireError, WireResult};
