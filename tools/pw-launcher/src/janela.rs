//! Janela do launcher em Win32 puro (B275). Substitui a de Tauri/WebView2, que exigia Windows 10:
//! aqui só se usa user32/gdi32/comctl32/shell32/ole32 em funções que existem desde o Windows 7
//! (a escolha de pasta é a `SHBrowseForFolderW`). Mesmo comportamento da antiga `ui/index.html`:
//! versão escolhida, situação da pasta, botões conforme a situação, barra de progresso e campo do
//! servidor de atualização.
//!
//! Tudo o que fala com o servidor roda numa thread à parte e devolve o resultado à janela com
//! `PostMessageW` (um `Box` no `LPARAM`); o estado da janela vive só na thread dela.

use crate::acoes::{self, Estado, Inicio, Situacao};
use crate::local;

#[path = "opcoes.rs"]
mod opcoes;
use pw_atualizador::cliente::Progresso;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const TITULO_JANELA: &str = "Perfect World — pw-universal";

const ID_VERSAO: i32 = 100;
const ID_BOTAO0: i32 = 110;
const ID_SALVAR: i32 = 120;
const ID_OPCOES: i32 = 130;
const N_BOTOES: usize = 5;

const WM_INICIO: u32 = WM_APP + 1;
const WM_SITUACAO: u32 = WM_APP + 2;
const WM_PROGRESSO: u32 = WM_APP + 3;
const WM_FIM: u32 = WM_APP + 4;

// Estilos de STATIC (winuser.h); a windows-sys 0.52 os põe em outro módulo.
const SS_NOPREFIX: u32 = 0x0080;
const SS_ETCHEDHORZ: u32 = 0x0010;
const SS_EDITCONTROL: u32 = 0x2000;
// winuser.h: DM_GETDEFID = WM_USER + 0, DC_HASDEFID = 0x534B.
const DM_GETDEFID: u32 = WM_USER;
const DC_HASDEFID: isize = 0x534B;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Acao {
    Jogar,
    JogarAssim,
    Atualizar,
    Reparar,
    AbrirPasta,
    TrocarPasta,
    Continuar,
    OutraPasta,
    Baixar,
    JaTenho,
    TentarDeNovo,
}

impl Acao {
    fn rotulo(self) -> &'static str {
        match self {
            Acao::Jogar => "Jogar",
            Acao::JogarAssim => "Jogar assim mesmo",
            Acao::Atualizar => "Atualizar",
            Acao::Reparar => "Verificar arquivos",
            Acao::AbrirPasta => "Abrir pasta",
            Acao::TrocarPasta => "Trocar pasta",
            Acao::Continuar => "Continuar download",
            Acao::OutraPasta => "Escolher outra pasta",
            Acao::Baixar => "Baixar o jogo",
            Acao::JaTenho => "Já tenho instalado",
            Acao::TentarDeNovo => "Tentar de novo",
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Controles {
    janela: HWND,
    versao: HWND,
    titulo: HWND,
    detalhe: HWND,
    botoes: [HWND; N_BOTOES],
    barra: HWND,
    etapa: HWND,
    mensagem: HWND,
    servidor: HWND,
    salvar: HWND,
    opcoes: HWND,
    fonte: HFONT,
    dpi: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cor {
    Normal,
    Ok,
    Erro,
}

#[derive(Default)]
struct EstadoJanela {
    versoes: Vec<String>,
    versao: String,
    ocupado: bool,
    acoes: [Option<Acao>; N_BOTOES],
    /// Número da última consulta de situação: respostas de consultas velhas são ignoradas.
    consulta: u64,
}

thread_local! {
    static CTL: Cell<Controles> = Cell::new(Controles::default());
    static EST: RefCell<EstadoJanela> = RefCell::new(EstadoJanela::default());
    static COR: Cell<Cor> = const { Cell::new(Cor::Normal) };
}

fn ctl() -> Controles {
    CTL.with(Cell::get)
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Tamanho de projeto (96 dpi) na escala da tela.
fn s(v: i32) -> i32 {
    v * ctl().dpi / 96
}

fn texto(h: HWND, t: &str) {
    unsafe { SetWindowTextW(h, w(t).as_ptr()) };
}

fn ler_texto(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h);
        let mut buf = vec![0u16; n as usize + 1];
        let lidos = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..lidos.max(0) as usize])
    }
}

fn mostrar_controle(h: HWND, sim: bool) {
    unsafe { ShowWindow(h, if sim { SW_SHOW } else { SW_HIDE }) };
}

fn caixa_de_erro(msg: &str) {
    unsafe { MessageBoxW(ctl().janela, w(msg).as_ptr(), w(TITULO_JANELA).as_ptr(), MB_OK | MB_ICONERROR) };
}

/// Manda `valor` para a janela; se ela já fechou, o valor é descartado.
fn postar<T>(janela: HWND, msg: u32, valor: T) {
    let p = Box::into_raw(Box::new(valor));
    if unsafe { PostMessageW(janela, msg, 0, p as LPARAM) } == 0 {
        drop(unsafe { Box::from_raw(p) });
    }
}

unsafe fn receber<T>(lparam: LPARAM) -> T {
    *Box::from_raw(lparam as *mut T)
}

// A windows-sys 0.52 condiciona estas ao feature `Win32_UI_Shell_Common`, que ela não publica;
// declaradas aqui como no shlobj_core.h (shell32.dll, presentes desde o Windows Vista).
#[repr(C)]
struct BROWSEINFOW {
    hwnd_owner: HWND,
    pidl_root: *const core::ffi::c_void,
    psz_display_name: *mut u16,
    lpsz_title: PCWSTR,
    ul_flags: u32,
    lpfn: Option<unsafe extern "system" fn(HWND, u32, LPARAM, LPARAM) -> i32>,
    l_param: LPARAM,
    i_image: i32,
}
const BIF_RETURNONLYFSDIRS: u32 = 0x0001;
const BIF_EDITBOX: u32 = 0x0010;
const BIF_NEWDIALOGSTYLE: u32 = 0x0040;
#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderW(lpbi: *const BROWSEINFOW) -> *mut core::ffi::c_void;
    fn SHGetPathFromIDListW(pidl: *const core::ffi::c_void, pszpath: *mut u16) -> BOOL;
    fn ILFree(pidl: *const core::ffi::c_void);
}

// ---------------------------------------------------------------- telas

fn mensagem(t: &str, cor: Cor) {
    COR.with(|c| c.set(cor));
    let h = ctl().mensagem;
    texto(h, t);
    unsafe { InvalidateRect(h, std::ptr::null(), 1) };
}

/// Põe os botões de ação, lado a lado, cada um do tamanho do seu texto.
fn definir_botoes(lista: &[Acao]) {
    let c = ctl();
    EST.with(|e| {
        let mut e = e.borrow_mut();
        e.acoes = [None; N_BOTOES];
        for (i, a) in lista.iter().take(N_BOTOES).enumerate() {
            e.acoes[i] = Some(*a);
        }
    });
    let mut x = s(16);
    unsafe {
        let hdc = GetDC(c.janela);
        let antiga = SelectObject(hdc, c.fonte);
        for (i, &h) in c.botoes.iter().enumerate() {
            let Some(a) = lista.get(i) else {
                ShowWindow(h, SW_HIDE);
                continue;
            };
            let t: Vec<u16> = a.rotulo().encode_utf16().collect();
            let mut tam = SIZE { cx: 0, cy: 0 };
            GetTextExtentPoint32W(hdc, t.as_ptr(), t.len() as i32, &mut tam);
            let largura = (tam.cx + s(28)).max(if i == 0 { s(140) } else { s(90) });
            texto(h, a.rotulo());
            MoveWindow(h, x, s(136), largura, s(30), 1);
            ShowWindow(h, SW_SHOW);
            x += largura + s(8);
        }
        SelectObject(hdc, antiga);
        ReleaseDC(c.janela, hdc);
    }
}

fn travar(sim: bool) {
    let c = ctl();
    EST.with(|e| e.borrow_mut().ocupado = sim);
    unsafe {
        for h in c.botoes.iter().chain([c.versao, c.servidor, c.salvar, c.opcoes].iter()) {
            EnableWindow(*h, (!sim) as BOOL);
        }
        SendMessageW(c.barra, PBM_SETPOS, 0, 0);
    }
    texto(c.etapa, "");
    mostrar_controle(c.barra, sim);
    mostrar_controle(c.etapa, sim);
}

fn ocupado() -> bool {
    EST.with(|e| e.borrow().ocupado)
}

fn versao_atual() -> String {
    EST.with(|e| e.borrow().versao.clone())
}

/// Ao abrir e ao trocar de servidor: busca as versões que o servidor oferece.
fn iniciar() {
    let c = ctl();
    texto(c.titulo, "Conectando ao servidor…");
    texto(c.detalhe, "");
    definir_botoes(&[]);
    let janela = c.janela;
    std::thread::spawn(move || postar(janela, WM_INICIO, acoes::iniciar()));
}

fn ao_iniciar(i: Inicio) {
    let c = ctl();
    texto(c.servidor, &i.servidor);
    unsafe { SendMessageW(c.versao, CB_RESETCONTENT, 0, 0) };
    if let Some(e) = i.erro_servidor {
        EST.with(|s| s.borrow_mut().versao.clear());
        texto(c.titulo, "Não foi possível falar com o servidor");
        texto(c.detalhe, &e);
        definir_botoes(&[Acao::TentarDeNovo]);
        return;
    }
    if i.versoes.is_empty() {
        texto(c.titulo, "O servidor não tem nenhuma versão publicada.");
        return;
    }
    let escolhida = if i.versoes.contains(&i.versao) { i.versao.clone() } else { i.versoes[0].clone() };
    for v in &i.versoes {
        unsafe { SendMessageW(c.versao, CB_ADDSTRING, 0, w(v).as_ptr() as LPARAM) };
    }
    let indice = i.versoes.iter().position(|v| *v == escolhida).unwrap_or(0);
    unsafe { SendMessageW(c.versao, CB_SETCURSEL, indice, 0) };
    EST.with(|s| {
        let mut s = s.borrow_mut();
        s.versoes = i.versoes;
        s.versao = escolhida;
    });
    mostrar();
}

/// Consulta a situação da versão escolhida (na thread de trabalho) e redesenha ao voltar.
fn mostrar() {
    let versao = versao_atual();
    if versao.is_empty() {
        return;
    }
    let c = ctl();
    texto(c.titulo, &format!("Perfect World {versao}: consultando…"));
    texto(c.detalhe, "");
    definir_botoes(&[]);
    unsafe { EnableWindow(c.opcoes, 0) };
    let numero = EST.with(|e| {
        let mut e = e.borrow_mut();
        e.consulta += 1;
        e.consulta
    });
    let janela = c.janela;
    std::thread::spawn(move || postar(janela, WM_SITUACAO, (numero, acoes::situacao(&versao))));
}

fn ao_situacao(numero: u64, r: Result<Situacao, String>) {
    if EST.with(|e| e.borrow().consulta) != numero {
        return;
    }
    let c = ctl();
    let versao = versao_atual();
    let s = match r {
        Ok(s) => s,
        Err(e) => {
            texto(c.titulo, "Erro");
            texto(c.detalhe, &e);
            definir_botoes(&[]);
            return;
        }
    };
    unsafe { EnableWindow(c.opcoes, (s.estado == Estado::Instalado && !ocupado()) as BOOL) };
    let sem_servidor = s.erro_servidor.as_ref().map(|e| format!(" (servidor fora do ar: {e})")).unwrap_or_default();
    let pasta = s.pasta.clone().unwrap_or_default();
    match s.estado {
        Estado::Instalado => {
            let atualizar = s.remota.is_some() && s.remota != s.numero;
            if atualizar {
                texto(c.titulo, &format!("Atualização disponível: versão {}", s.remota.unwrap_or(0)));
            } else {
                texto(c.titulo, "Pronto para jogar");
            }
            let numero = s.numero.map(|n| n.to_string()).unwrap_or_else(|| "?".into());
            texto(c.detalhe, &format!("{pasta} — versão {numero}{sem_servidor}"));
            if atualizar {
                definir_botoes(&[Acao::Atualizar, Acao::JogarAssim, Acao::Reparar, Acao::AbrirPasta, Acao::TrocarPasta]);
            } else {
                definir_botoes(&[Acao::Jogar, Acao::Reparar, Acao::AbrirPasta, Acao::TrocarPasta]);
            }
        }
        Estado::Interrompida => {
            texto(c.titulo, "Download interrompido");
            texto(c.detalhe, &format!("{pasta} — continua de onde parou{sem_servidor}"));
            definir_botoes(&[Acao::Continuar, Acao::OutraPasta]);
        }
        Estado::SemPasta | Estado::PastaSumiu | Estado::SemCliente => {
            let motivo = match s.estado {
                Estado::PastaSumiu => format!("A pasta {pasta} não existe mais."),
                Estado::SemCliente => format!("A pasta {pasta} não tem este jogo."),
                _ => "Primeira vez com esta versão neste computador.".to_string(),
            };
            texto(c.titulo, &format!("Perfect World {versao} ainda não está instalado"));
            texto(c.detalhe, &format!("{motivo}{sem_servidor}"));
            definir_botoes(&[Acao::Baixar, Acao::JaTenho]);
        }
    }
}

/// Roda uma operação demorada fora da thread da janela, com a barra de progresso
/// (no máximo ~5 atualizações por segundo, como no launcher antigo).
fn tarefa(f: impl FnOnce(&mut dyn FnMut(Progresso)) -> Result<String, String> + Send + 'static) {
    if ocupado() {
        return;
    }
    travar(true);
    mensagem("", Cor::Normal);
    let janela = ctl().janela;
    std::thread::spawn(move || {
        let mut ultimo = Instant::now() - Duration::from_secs(1);
        let mut ao_progredir = |p: Progresso| {
            if ultimo.elapsed() >= Duration::from_millis(200) || p.feito == p.total {
                ultimo = Instant::now();
                postar(janela, WM_PROGRESSO, p);
            }
        };
        let r = f(&mut ao_progredir);
        postar(janela, WM_FIM, r);
    });
}

fn ao_progresso(p: Progresso) {
    let c = ctl();
    let pct = if p.total > 0 { (100.0 * p.feito as f64 / p.total as f64).min(100.0) } else { 0.0 };
    unsafe { SendMessageW(c.barra, PBM_SETPOS, (pct * 10.0) as usize, 0) };
    let mb = |n: u64| format!("{:.0} MB", n as f64 / 1048576.0);
    let linha = if p.total == 0 {
        p.etapa.clone()
    } else {
        let contagem = if p.etapa.starts_with("verificando") || p.etapa.starts_with("reparando") {
            format!("{} / {} arquivos", p.feito, p.total)
        } else {
            format!("{} / {}", mb(p.feito), mb(p.total))
        };
        format!("{} — {contagem} ({pct:.0}%)", p.etapa)
    };
    texto(c.etapa, &linha);
}

fn ao_fim(r: Result<String, String>) {
    travar(false);
    match r {
        Ok(m) => mensagem(&m, Cor::Ok),
        Err(e) => mensagem(&e, Cor::Erro),
    }
    mostrar();
}

/// Janela de pastas do Windows (a clássica, que existe no Windows 7).
fn escolher_pasta(titulo: &str) -> Option<PathBuf> {
    let t = w(titulo);
    let mut nome = [0u16; 260];
    let info = BROWSEINFOW {
        hwnd_owner: ctl().janela,
        pidl_root: std::ptr::null(),
        psz_display_name: nome.as_mut_ptr(),
        lpsz_title: t.as_ptr(),
        ul_flags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX,
        lpfn: None,
        l_param: 0,
        i_image: 0,
    };
    unsafe {
        let pidl = SHBrowseForFolderW(&info);
        if pidl.is_null() {
            return None;
        }
        let mut caminho = [0u16; 260];
        let ok = SHGetPathFromIDListW(pidl, caminho.as_mut_ptr());
        ILFree(pidl);
        if ok == 0 {
            return None;
        }
        let n = caminho.iter().position(|&c| c == 0).unwrap_or(caminho.len());
        Some(PathBuf::from(String::from_utf16_lossy(&caminho[..n])))
    }
}

fn executar_acao(a: Acao) {
    if ocupado() {
        return;
    }
    let versao = versao_atual();
    match a {
        Acao::Jogar | Acao::JogarAssim => match acoes::jogar(&versao) {
            Ok(()) => mensagem("Jogo aberto. Bom jogo!", Cor::Ok),
            Err(e) => mensagem(&e, Cor::Erro),
        },
        Acao::Atualizar | Acao::Continuar => tarefa(move |p| acoes::continuar(&versao, p)),
        Acao::Reparar => tarefa(move |p| acoes::reparar(&versao, p)),
        Acao::AbrirPasta => {
            if let Err(e) = acoes::abrir_no_explorer(&versao) {
                mensagem(&e, Cor::Erro);
            }
        }
        Acao::TrocarPasta | Acao::JaTenho => {
            if let Some(p) = escolher_pasta(&format!("Pasta onde o Perfect World {versao} já está (a pasta element)")) {
                tarefa(move |pr| acoes::usar_existente(&versao, &p, pr));
            }
        }
        Acao::OutraPasta | Acao::Baixar => {
            if let Some(p) = escolher_pasta(&format!("Onde instalar o Perfect World {versao}?")) {
                tarefa(move |pr| acoes::baixar(&versao, Path::new(&p), pr));
            }
        }
        Acao::TentarDeNovo => iniciar(),
    }
}

fn ao_trocar_versao() {
    let c = ctl();
    let i = unsafe { SendMessageW(c.versao, CB_GETCURSEL, 0, 0) };
    let nova = EST.with(|e| e.borrow().versoes.get(i as usize).cloned());
    if let Some(v) = nova {
        EST.with(|e| e.borrow_mut().versao = v);
        mensagem("", Cor::Normal);
        mostrar();
    }
}

fn ao_salvar_servidor() {
    match acoes::trocar_servidor(&ler_texto(ctl().servidor)) {
        Ok(()) => {
            mensagem("", Cor::Normal);
            iniciar();
        }
        Err(e) => mensagem(&e, Cor::Erro),
    }
}

// ---------------------------------------------------------------- Win32

unsafe extern "system" fn procedimento(janela: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let codigo = ((wparam >> 16) & 0xFFFF) as u32;
            if id == ID_VERSAO && codigo == CBN_SELCHANGE {
                ao_trocar_versao();
            } else if id == ID_SALVAR && codigo == BN_CLICKED {
                ao_salvar_servidor();
            } else if id == ID_OPCOES && codigo == BN_CLICKED {
                opcoes::abrir();
            } else if (ID_BOTAO0..ID_BOTAO0 + N_BOTOES as i32).contains(&id) && codigo == BN_CLICKED {
                let a = EST.with(|e| e.borrow().acoes[(id - ID_BOTAO0) as usize]);
                if let Some(a) = a {
                    executar_acao(a);
                }
            }
            0
        }
        // Enter aperta o botão principal (o primeiro), como num diálogo.
        DM_GETDEFID => {
            let tem = EST.with(|e| e.borrow().acoes[0].is_some());
            if tem && !ocupado() {
                (DC_HASDEFID << 16) | ID_BOTAO0 as isize
            } else {
                0
            }
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkColor(hdc, GetSysColor(COLOR_BTNFACE));
            if lparam == ctl().mensagem {
                let cor = match COR.with(Cell::get) {
                    Cor::Normal => GetSysColor(COLOR_WINDOWTEXT),
                    Cor::Ok => 0x0000_7A1E,   // verde (COLORREF é 0x00BBGGRR)
                    Cor::Erro => 0x0000_00C0, // vermelho
                };
                SetTextColor(hdc, cor);
            } else if lparam == ctl().titulo {
                SetTextColor(hdc, 0x0060_3010); // azul-escuro
            } else {
                SetTextColor(hdc, GetSysColor(COLOR_WINDOWTEXT));
            }
            GetSysColorBrush(COLOR_BTNFACE)
        }
        WM_INICIO => {
            ao_iniciar(receber(lparam));
            0
        }
        WM_SITUACAO => {
            let (n, r): (u64, Result<Situacao, String>) = receber(lparam);
            ao_situacao(n, r);
            0
        }
        WM_PROGRESSO => {
            ao_progresso(receber(lparam));
            0
        }
        WM_FIM => {
            ao_fim(receber(lparam));
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(janela, msg, wparam, lparam),
    }
}

fn fonte(pontos: i32, negrito: bool, dpi: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -(pontos * dpi / 72),
            0,
            0,
            0,
            if negrito { 700 } else { 400 },
            0,
            0,
            0,
            1, // DEFAULT_CHARSET
            0,
            0,
            5, // CLEARTYPE_QUALITY
            0,
            w("Segoe UI").as_ptr(),
        )
    }
}

struct Criador {
    janela: HWND,
    instancia: HINSTANCE,
}

impl Criador {
    #[allow(clippy::too_many_arguments)]
    fn criar(&self, classe: PCWSTR, t: &str, estilo: u32, ex: u32, r: (i32, i32, i32, i32), id: i32, f: HFONT) -> HWND {
        unsafe {
            let h = CreateWindowExW(
                ex,
                classe,
                w(t).as_ptr(),
                WS_CHILD | WS_VISIBLE | estilo,
                s(r.0),
                s(r.1),
                s(r.2),
                s(r.3),
                self.janela,
                id as HMENU,
                self.instancia,
                std::ptr::null(),
            );
            SendMessageW(h, WM_SETFONT, f as WPARAM, 1);
            h
        }
    }
}

pub fn executar() {
    std::panic::set_hook(Box::new(|info| caixa_de_erro(&format!("Erro interno do launcher:\n{info}"))));
    unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let iccx = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_PROGRESS_CLASS | ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&iccx);

        let tela = GetDC(0);
        let dpi = GetDeviceCaps(tela, LOGPIXELSY as i32).max(96);
        ReleaseDC(0, tela);
        let normal = fonte(9, false, dpi);
        let titulo_f = fonte(12, true, dpi);
        let cabecalho_f = fonte(16, true, dpi);
        CTL.with(|c| c.set(Controles { dpi, fonte: normal, ..Controles::default() }));

        let instancia = GetModuleHandleW(std::ptr::null());
        let classe = w("PwUniversalLauncher");
        let icone = LoadIconW(instancia, 1 as PCWSTR); // recurso 1 (launcher.rc)
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(procedimento),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instancia,
            hIcon: icone,
            hCursor: LoadCursorW(0, IDC_ARROW),
            hbrBackground: (COLOR_BTNFACE + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: classe.as_ptr(),
            hIconSm: icone,
        };
        let classe_opcoes = w(opcoes::CLASSE);
        let wc_opcoes = WNDCLASSEXW { lpfnWndProc: Some(opcoes::procedimento), lpszClassName: classe_opcoes.as_ptr(), ..wc };
        if RegisterClassExW(&wc) == 0 || RegisterClassExW(&wc_opcoes) == 0 {
            caixa_de_erro("Não foi possível registrar a janela do launcher.");
            return;
        }

        let estilo = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut r = RECT { left: 0, top: 0, right: s(700), bottom: s(344) };
        AdjustWindowRect(&mut r, estilo, 0);
        let (larg, alt) = (r.right - r.left, r.bottom - r.top);
        let x = (GetSystemMetrics(SM_CXSCREEN) - larg).max(0) / 2;
        let y = (GetSystemMetrics(SM_CYSCREEN) - alt).max(0) / 2;
        let janela = CreateWindowExW(
            0,
            classe.as_ptr(),
            w(TITULO_JANELA).as_ptr(),
            estilo,
            x,
            y,
            larg,
            alt,
            0,
            0,
            instancia,
            std::ptr::null(),
        );
        if janela == 0 {
            caixa_de_erro("Não foi possível abrir a janela do launcher.");
            return;
        }

        let cr = Criador { janela, instancia };
        let st = w("STATIC");
        let bt = w("BUTTON");
        let ed = w("EDIT");
        let estatico = |t: &str, f: HFONT, r: (i32, i32, i32, i32), extra: u32| {
            cr.criar(st.as_ptr(), t, SS_NOPREFIX | extra, 0, r, -1, f)
        };

        estatico("Perfect World", cabecalho_f, (16, 14, 300, 34), 0);
        let opcoes = cr.criar(bt.as_ptr(), "Opções do jogo…", WS_TABSTOP | WS_DISABLED, 0, (318, 17, 150, 28), ID_OPCOES, normal);
        estatico("Versão:", normal, (484, 22, 56, 20), 0);
        let versao = cr.criar(
            WC_COMBOBOXW,
            "",
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
            0,
            (544, 18, 140, 200),
            ID_VERSAO,
            normal,
        );
        let titulo = estatico("Conectando ao servidor…", titulo_f, (16, 64, 668, 26), 0);
        let detalhe = estatico("", normal, (16, 92, 668, 38), SS_EDITCONTROL);
        let mut botoes = [0; N_BOTOES];
        for (i, b) in botoes.iter_mut().enumerate() {
            let tipo = if i == 0 { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON } as u32;
            *b = cr.criar(bt.as_ptr(), "", tipo | WS_TABSTOP, 0, (16, 136, 100, 30), ID_BOTAO0 + i as i32, normal);
            ShowWindow(*b, SW_HIDE);
        }
        let barra = cr.criar(PROGRESS_CLASSW, "", 0, 0, (16, 182, 668, 16), -1, normal);
        SendMessageW(barra, PBM_SETRANGE32, 0, 1000);
        let etapa = estatico("", normal, (16, 202, 668, 20), SS_EDITCONTROL);
        let mensagem = estatico("", normal, (16, 228, 668, 52), SS_EDITCONTROL);
        estatico("", normal, (16, 290, 668, 2), SS_ETCHEDHORZ);
        estatico("Servidor de atualização:", normal, (16, 307, 168, 20), 0);
        let servidor = cr.criar(
            ed.as_ptr(),
            "",
            ES_AUTOHSCROLL as u32 | WS_TABSTOP,
            WS_EX_CLIENTEDGE,
            (188, 303, 334, 24),
            -1,
            normal,
        );
        let salvar = cr.criar(bt.as_ptr(), "Salvar e reconectar", WS_TABSTOP, 0, (530, 302, 154, 26), ID_SALVAR, normal);

        CTL.with(|c| {
            c.set(Controles {
                janela,
                versao,
                titulo,
                detalhe,
                botoes,
                barra,
                etapa,
                mensagem,
                servidor,
                salvar,
                opcoes,
                fonte: normal,
                dpi,
            })
        });
        travar(false);
        ShowWindow(janela, SW_SHOW);
        UpdateWindow(janela);
        iniciar();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            // Tab/Enter/Esc valem na janela de cima (a principal ou a de opções).
            let raiz = GetAncestor(msg.hwnd, GA_ROOT);
            if IsDialogMessageW(if raiz != 0 { raiz } else { janela }, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
