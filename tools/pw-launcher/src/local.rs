//! Arquivos do jogador que o launcher ajusta na pasta do cliente (B280).
//!
//! **Endereço do jogo.** No `serverlist.txt` publicado os servidores do pw-universal apontam para
//! `127.0.0.1`, que aqui quer dizer "a mesma máquina do servidor de atualização". O launcher troca
//! esse endereço pelo host do servidor de atualização configurado (e o host que ele mesmo tinha
//! posto antes, para quando o IP muda). Linhas com outro endereço (servidores de terceiros) ficam
//! como estão. Formato lido pelo cliente: UTF-16LE com BOM (o `a_fgetc` pula o BOM), uma linha por
//! servidor `nome<TAB>portas:endereço[:linha][<TAB>zona...]`; linha sem tabulação é grupo
//! (`EC_ServerList.cpp:75-160`). O `currentserver.ini` guarda `CurrentServerAddress=porta:ip`
//! do último servidor escolhido (`EC_ServerList.cpp:497-536`), trocado da mesma forma.
//!
//! **Opções de vídeo e som.** `userdata\systemsettings.ini`, lido por
//! `CECConfigs::LoadSystemSettings` (`EC_Configs.cpp:389-431`, padrões quando falta a chave) e
//! gravado pelo próprio cliente (`EC_Configs.cpp:480-577`, formato `Chave = valor`).

use std::path::Path;

pub const SERVERLIST: &str = "userdata\\server\\serverlist.txt";
pub const SERVIDOR_ATUAL: &str = "userdata\\currentserver.ini";
pub const OPCOES: &str = "userdata\\systemsettings.ini";

/// Host de uma URL do servidor de atualização (`http://192.168.1.13:8100/` → `192.168.1.13`).
pub fn host_da_url(url: &str) -> Option<String> {
    let resto = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let autoridade = resto.split(['/', '?', '#']).next()?;
    let autoridade = autoridade.rsplit_once('@').map_or(autoridade, |(_, h)| h);
    let host = if let Some(v6) = autoridade.strip_prefix('[') {
        v6.split(']').next()?
    } else {
        autoridade.split(':').next()?
    };
    let host = host.trim();
    if host.is_empty() {
        return None;
    }
    Some(if host.eq_ignore_ascii_case("localhost") { "127.0.0.1".into() } else { host.to_string() })
}

/// Completa o que o jogador digitou: sem esquema vira `http://` (`192.168.1.13:8100`).
pub fn normalizar_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    if u.contains("://") {
        u.to_string()
    } else {
        format!("http://{u}")
    }
}

/// Endereços que o launcher pode trocar: o publicado (`127.0.0.1`/`localhost`) e os que ele
/// mesmo pôs antes.
fn trocavel(endereco: &str, anteriores: &[String]) -> bool {
    let e = endereco.trim();
    e == "127.0.0.1" || e.eq_ignore_ascii_case("localhost") || anteriores.iter().any(|a| a.eq_ignore_ascii_case(e))
}

/// `portas:endereço[:linha]` com o endereço trocado, se for trocável.
fn trocar_no_campo(campo: &str, host: &str, anteriores: &[String]) -> Option<String> {
    let mut partes: Vec<&str> = campo.split(':').collect();
    if partes.len() < 2 || !trocavel(partes[1], anteriores) || partes[1] == host {
        return None;
    }
    partes[1] = host;
    Some(partes.join(":"))
}

/// Nova `serverlist.txt` (bytes UTF-16LE com BOM) com os endereços trocados; `None` se nada muda.
pub fn ajustar_serverlist(bytes: &[u8], host: &str, anteriores: &[String]) -> Option<Vec<u8>> {
    let corpo = bytes.strip_prefix(&[0xFF, 0xFE])?;
    if corpo.len() % 2 != 0 {
        return None;
    }
    let unidades: Vec<u16> = corpo.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let texto = String::from_utf16(&unidades).ok()?;
    let mut mudou = false;
    let linhas: Vec<String> = texto
        .split('\n')
        .map(|linha| {
            let (sem_cr, cr) = linha.strip_suffix('\r').map_or((linha, ""), |l| (l, "\r"));
            let mut campos: Vec<String> = sem_cr.split('\t').map(str::to_string).collect();
            if campos.len() >= 2 {
                if let Some(novo) = trocar_no_campo(&campos[1], host, anteriores) {
                    campos[1] = novo;
                    mudou = true;
                }
            }
            format!("{}{cr}", campos.join("\t"))
        })
        .collect();
    if !mudou {
        return None;
    }
    let mut saida = vec![0xFF, 0xFE];
    for u in linhas.join("\n").encode_utf16() {
        saida.extend_from_slice(&u.to_le_bytes());
    }
    Some(saida)
}

/// `currentserver.ini` (ANSI, `GetPrivateProfileString`) com o `CurrentServerAddress` trocado.
pub fn ajustar_servidor_atual(bytes: &[u8], host: &str, anteriores: &[String]) -> Option<Vec<u8>> {
    let texto = String::from_utf8_lossy(bytes);
    let mut mudou = false;
    let linhas: Vec<String> = texto
        .split('\n')
        .map(|linha| {
            let (sem_cr, cr) = linha.strip_suffix('\r').map_or((linha, ""), |l| (l, "\r"));
            if let Some((chave, valor)) = sem_cr.split_once('=') {
                if chave.trim().eq_ignore_ascii_case("CurrentServerAddress") {
                    if let Some(novo) = trocar_no_campo(valor.trim(), host, anteriores) {
                        mudou = true;
                        return format!("{chave}={novo}{cr}");
                    }
                }
            }
            linha.to_string()
        })
        .collect();
    mudou.then(|| linhas.join("\n").into_bytes())
}

/// Aplica o host do jogo nos dois arquivos da pasta. Devolve quantos arquivos mudaram.
pub fn aplicar_endereco(pasta: &Path, host: &str, anteriores: &[String]) -> Result<u32, String> {
    let mut n = 0;
    for (rel, ajustar) in [
        (SERVERLIST, ajustar_serverlist as fn(&[u8], &str, &[String]) -> Option<Vec<u8>>),
        (SERVIDOR_ATUAL, ajustar_servidor_atual),
    ] {
        let p = pasta.join(rel);
        let Ok(atual) = std::fs::read(&p) else { continue };
        if let Some(novo) = ajustar(&atual, host, anteriores) {
            std::fs::write(&p, novo).map_err(|e| format!("gravar {}: {e}", p.display()))?;
            n += 1;
        }
    }
    Ok(n)
}

// ---------------------------------------------------------------- systemsettings.ini

/// Opções que o launcher mostra. Padrões = os do cliente quando a chave falta
/// (`EC_Configs.cpp:403-431`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opcoes {
    pub largura: u32,
    pub altura: u32,
    pub tela_cheia: bool,
    pub widescreen: bool,
    /// `Level`, 0 a 4 (`Slider_Total` com 4 passos, `setting_system.dtf:33`).
    pub qualidade: u32,
    /// `SoundVol`, 0 a 100 (`Slider_Ambience`, `setting_system.dtf:43`).
    pub volume_som: u32,
    /// `MusicVol`, 0 a 100 (`Slider_Music`, `setting_system.dtf:42`).
    pub volume_musica: u32,
}

impl Default for Opcoes {
    fn default() -> Self {
        Opcoes {
            largura: 800,
            altura: 600,
            tela_cheia: true,
            widescreen: false,
            qualidade: 0,
            volume_som: 50,
            volume_musica: 50,
        }
    }
}

fn valor(texto: &str, secao: &str, chave: &str) -> Option<i64> {
    let mut dentro = false;
    for linha in texto.lines() {
        let l = linha.trim();
        if let Some(nome) = l.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            dentro = nome.trim().eq_ignore_ascii_case(secao);
        } else if dentro {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim().eq_ignore_ascii_case(chave) {
                    return v.trim().parse().ok();
                }
            }
        }
    }
    None
}

pub fn ler_opcoes(bytes: &[u8]) -> Opcoes {
    let t = String::from_utf8_lossy(bytes);
    let p = Opcoes::default();
    let n = |s: &str, k: &str, d: u32| valor(&t, s, k).map_or(d, |v| v.max(0) as u32);
    let b = |s: &str, k: &str, d: bool| valor(&t, s, k).map_or(d, |v| v != 0);
    Opcoes {
        largura: n("Video", "RenderWid", p.largura),
        altura: n("Video", "RenderHei", p.altura),
        tela_cheia: b("Video", "FullScreen", p.tela_cheia),
        widescreen: b("Video", "WideScreen", p.widescreen),
        qualidade: n("Video", "Level", p.qualidade).min(4),
        volume_som: n("Audio", "SoundVol", p.volume_som).min(100),
        volume_musica: n("Audio", "MusicVol", p.volume_musica).min(100),
    }
}

/// Chaves que o diálogo do cliente muda junto com o `Level` (`CDlgSettingSystem::OnCommandTotal`,
/// `DlgSettingSystem.cpp:326-440`), supondo a placa capaz de reflexo, sombra e brilho; o cliente
/// confere a capacidade ao aplicar.
fn chaves_da_qualidade(n: u32) -> Vec<(&'static str, i64)> {
    let n = n.min(4) as i64;
    let sim = |c: bool| c as i64;
    vec![
        ("Level", n),
        ("Sight", n),
        ("WaterEffect", match n { 0 => 0, 4 => 2, _ => 1 }),
        ("SimpleTerrain", sim(n == 0)),
        ("TreeDetail", n),
        ("GrassDetail", n),
        ("CloudDetail", n),
        ("Shadow", sim(n > 0)),
        ("MipMapBias", sim(n > 0)),
        ("FullGlow", sim(n >= 2)),
        ("SpaceWarp", sim(n >= 2)),
        ("SunFlare", sim(n >= 2)),
        ("AdvancedWaterUD", sim(n >= 2)),
    ]
}

/// Troca (ou acrescenta) `chave = valor` na seção, mantendo o resto do arquivo.
fn definir(linhas: &mut Vec<String>, secao: &str, chave: &str, v: i64) {
    let nova = format!("{chave} = {v}");
    let mut inicio = None;
    let mut fim = linhas.len();
    for (i, linha) in linhas.iter().enumerate() {
        let l = linha.trim();
        if let Some(nome) = l.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            if inicio.is_some() {
                fim = i;
                break;
            }
            if nome.trim().eq_ignore_ascii_case(secao) {
                inicio = Some(i);
            }
        }
    }
    let Some(inicio) = inicio else {
        if linhas.last().is_some_and(|l| !l.trim().is_empty()) {
            linhas.push(String::new());
        }
        linhas.push(format!("[{secao}]"));
        linhas.push(nova);
        return;
    };
    for linha in &mut linhas[inicio + 1..fim] {
        if linha.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(chave)) {
            *linha = nova;
            return;
        }
    }
    // Depois da última linha não vazia da seção.
    let mut pos = fim;
    while pos > inicio + 1 && linhas[pos - 1].trim().is_empty() {
        pos -= 1;
    }
    linhas.insert(pos, nova);
}

/// O `systemsettings.ini` com as opções gravadas (cria do zero se `bytes` for vazio).
pub fn gravar_opcoes(bytes: &[u8], o: &Opcoes) -> Vec<u8> {
    let texto = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    let mut linhas: Vec<String> = texto.split('\n').map(str::to_string).collect();
    if linhas.last().is_some_and(String::is_empty) {
        linhas.pop();
    }
    // `WideScreen` só vale a partir de 800x600 (`DlgSettingSystem.cpp:483-494`).
    let wide = o.widescreen && o.largura >= 800 && o.altura >= 600;
    let video: Vec<(&str, i64)> = [
        ("RenderWid", o.largura as i64),
        ("RenderHei", o.altura as i64),
        ("FullScreen", o.tela_cheia as i64),
        ("WideScreen", wide as i64),
    ]
    .into_iter()
    .chain(chaves_da_qualidade(o.qualidade))
    .collect();
    for (k, v) in video {
        definir(&mut linhas, "Video", k, v);
    }
    definir(&mut linhas, "Audio", "SoundVol", o.volume_som.min(100) as i64);
    definir(&mut linhas, "Audio", "MusicVol", o.volume_musica.min(100) as i64);
    let mut s = linhas.join("\r\n");
    s.push_str("\r\n");
    s.into_bytes()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn utf16(s: &str) -> Vec<u8> {
        let mut v = vec![0xFF, 0xFE];
        for u in s.encode_utf16() {
            v.extend_from_slice(&u.to_le_bytes());
        }
        v
    }

    #[test]
    fn host_da_url_aceita_o_que_se_digita() {
        assert_eq!(host_da_url("http://192.168.1.13:8100").as_deref(), Some("192.168.1.13"));
        assert_eq!(host_da_url("192.168.1.13:8100/").as_deref(), Some("192.168.1.13"));
        assert_eq!(host_da_url("https://pw.exemplo.com/x").as_deref(), Some("pw.exemplo.com"));
        assert_eq!(host_da_url("http://localhost:8100").as_deref(), Some("127.0.0.1"));
        assert_eq!(normalizar_url(" 192.168.1.13:8100/ "), "http://192.168.1.13:8100");
    }

    #[test]
    fn serverlist_troca_so_os_nossos() {
        // As duas listas reais (1.5.5 BR com grupo, 1.2.6 com servidores de terceiros).
        let l155 = "pw-universal\r\nUniversal 1.5.5\t29003:127.0.0.1\t1\r\nUniversal 1.5.5BR\t29004:127.0.0.1\t2\r\n";
        let novo = ajustar_serverlist(&utf16(l155), "192.168.1.13", &[]).unwrap();
        assert_eq!(
            novo,
            utf16("pw-universal\r\nUniversal 1.5.5\t29003:192.168.1.13\t1\r\nUniversal 1.5.5BR\t29004:192.168.1.13\t2\r\n")
        );
        let l126 = "BATATA\t\t29000:192.168.1.200\r\nBatata_E\t29000:pwjurassic.servegame.com\r\nUniversal 1.2.6\t29000:127.0.0.1";
        let novo = ajustar_serverlist(&utf16(l126), "26.38.197.69", &[]).unwrap();
        assert_eq!(
            novo,
            utf16("BATATA\t\t29000:192.168.1.200\r\nBatata_E\t29000:pwjurassic.servegame.com\r\nUniversal 1.2.6\t29000:26.38.197.69")
        );
        // IP mudou: o anterior posto pelo launcher também é trocado; linha (`:1`) preservada.
        let antes = utf16("g\r\nU\t29000-29002:192.168.1.13:1\t1\r\n");
        let depois = ajustar_serverlist(&antes, "10.0.0.5", &["192.168.1.13".into()]).unwrap();
        assert_eq!(depois, utf16("g\r\nU\t29000-29002:10.0.0.5:1\t1\r\n"));
        // Já está certo: nada a gravar.
        assert!(ajustar_serverlist(&depois, "10.0.0.5", &["192.168.1.13".into()]).is_none());
    }

    #[test]
    fn servidor_atual_troca_o_endereco() {
        let ini = b"[Server]\r\nCurrentServer=Universal 1.5.5BR\r\nCurrentServerAddress=29004:127.0.0.1\r\nCurrentLine=0\r\n";
        let novo = ajustar_servidor_atual(ini, "192.168.1.13", &[]).unwrap();
        assert_eq!(
            String::from_utf8(novo).unwrap(),
            "[Server]\r\nCurrentServer=Universal 1.5.5BR\r\nCurrentServerAddress=29004:192.168.1.13\r\nCurrentLine=0\r\n"
        );
    }

    #[test]
    fn opcoes_ida_e_volta() {
        let ini = "[Info]\r\ncard = X\r\n\r\n[Video]\r\nLevel = 4\r\nRenderWid = 1916\r\nRenderHei = 1008\r\nFullScreen = 0\r\nWideScreen = 1\r\nGamma = 41\r\n\r\n[Audio]\r\nSoundVol = 38\r\nMusicVol = 2\r\n\r\n[UI]\r\nTheme = 0\r\n";
        let o = ler_opcoes(ini.as_bytes());
        assert_eq!(
            o,
            Opcoes { largura: 1916, altura: 1008, tela_cheia: false, widescreen: true, qualidade: 4, volume_som: 38, volume_musica: 2 }
        );
        let novo = Opcoes { largura: 1280, altura: 720, tela_cheia: true, qualidade: 1, volume_musica: 0, ..o };
        let gravado = gravar_opcoes(ini.as_bytes(), &novo);
        assert_eq!(ler_opcoes(&gravado), novo);
        let t = String::from_utf8(gravado).unwrap();
        assert!(t.contains("[Info]\r\ncard = X\r\n"));
        assert!(t.contains("Gamma = 41\r\n"));
        assert!(t.contains("[UI]\r\nTheme = 0\r\n"));
        assert!(t.contains("Sight = 1\r\n") && t.contains("FullGlow = 0\r\n") && t.contains("WaterEffect = 1\r\n"));
        // Sem arquivo: padrões do cliente e criação das seções.
        assert_eq!(ler_opcoes(b""), Opcoes::default());
        let criado = String::from_utf8(gravar_opcoes(b"", &Opcoes { widescreen: true, largura: 640, altura: 480, ..Opcoes::default() })).unwrap();
        assert!(criado.starts_with("[Video]\r\nRenderWid = 640\r\n"));
        assert!(criado.contains("WideScreen = 0\r\n"), "widescreen só a partir de 800x600");
        assert!(criado.contains("[Audio]\r\nSoundVol = 50\r\n"));
    }
}
