//! Janela "Opções do jogo" (B280): modo janela/tela cheia, resolução, widescreen, qualidade e
//! volumes da versão escolhida, gravados em `userdata\systemsettings.ini` (`local.rs`).
//! Submódulo de `janela.rs` (usa os ajudantes de lá).

use super::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;

pub const CLASSE: &str = "PwUniversalOpcoes";
const ID_MODO: i32 = 200;
const ID_RES: i32 = 201;
const ID_WIDE: i32 = 202;
const ID_QUAL: i32 = 203;
const ID_SOM: i32 = 204;
const ID_MUSICA: i32 = 205;
const ID_SALVAR_OP: i32 = 206;
const ID_CANCELAR: i32 = 207;
// commctrl.h: TBM_GETPOS = WM_USER.
const TBM_GETPOS_: u32 = WM_USER;
// winuser.h: IDOK/IDCANCEL, mandados pelo IsDialogMessage no Enter/Esc.
const IDOK_: i32 = 1;
const IDCANCEL_: i32 = 2;

/// Nomes do `Level` 0 a 4 (`Slider_Total` com 4 passos, `setting_system.dtf:33`).
const QUALIDADES: [&str; 5] = ["Mínima", "Baixa", "Média", "Alta", "Máxima"];

#[derive(Clone, Copy, Default)]
struct Ctl {
    janela: HWND,
    modo: HWND,
    res: HWND,
    wide: HWND,
    qual: HWND,
    som: HWND,
    musica: HWND,
    som_valor: HWND,
    musica_valor: HWND,
}

thread_local! {
    static OP: Cell<Ctl> = Cell::new(Ctl::default());
    static RESOLUCOES: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
    static VERSAO: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Resoluções da placa, como o diálogo do cliente as lista: só acima de 400x400
/// (`DlgSettingSystem.cpp:290-293`).
fn resolucoes_da_tela() -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    let mut i = 0u32;
    loop {
        let mut dm: DEVMODEW = unsafe { std::mem::zeroed() };
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if unsafe { EnumDisplaySettingsW(std::ptr::null(), i, &mut dm) } == 0 {
            break;
        }
        if dm.dmPelsWidth > 400 && dm.dmPelsHeight > 400 {
            v.push((dm.dmPelsWidth, dm.dmPelsHeight));
        }
        i += 1;
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn combo_adicionar(h: HWND, t: &str) {
    unsafe { SendMessageW(h, CB_ADDSTRING, 0, w(t).as_ptr() as LPARAM) };
}

fn combo_indice(h: HWND) -> usize {
    unsafe { SendMessageW(h, CB_GETCURSEL, 0, 0) }.max(0) as usize
}

fn barra_valor(h: HWND) -> u32 {
    unsafe { SendMessageW(h, TBM_GETPOS_, 0, 0) }.clamp(0, 100) as u32
}

fn resolucao_escolhida(c: &Ctl) -> (u32, u32) {
    RESOLUCOES.with(|r| r.borrow().get(combo_indice(c.res)).copied().unwrap_or((800, 600)))
}

/// Widescreen só a partir de 800x600 (`DlgSettingSystem.cpp:483-494`).
fn conferir_wide() {
    let c = OP.with(Cell::get);
    let (l, a) = resolucao_escolhida(&c);
    let pode = l >= 800 && a >= 600;
    unsafe {
        if !pode {
            SendMessageW(c.wide, BM_SETCHECK, 0, 0);
        }
        EnableWindow(c.wide, pode as BOOL);
    }
}

fn mostrar_volumes() {
    let c = OP.with(Cell::get);
    texto(c.som_valor, &barra_valor(c.som).to_string());
    texto(c.musica_valor, &barra_valor(c.musica).to_string());
}

fn fechar() {
    let c = OP.with(Cell::get);
    if c.janela == 0 {
        return;
    }
    OP.with(|o| o.set(Ctl::default()));
    unsafe {
        EnableWindow(ctl().janela, 1);
        DestroyWindow(c.janela);
        SetForegroundWindow(ctl().janela);
    }
}

fn salvar() {
    let c = OP.with(Cell::get);
    let (largura, altura) = resolucao_escolhida(&c);
    let o = local::Opcoes {
        largura,
        altura,
        tela_cheia: combo_indice(c.modo) == 1,
        widescreen: unsafe { SendMessageW(c.wide, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize,
        qualidade: combo_indice(c.qual).min(4) as u32,
        volume_som: barra_valor(c.som),
        volume_musica: barra_valor(c.musica),
    };
    let versao = VERSAO.with(|v| v.borrow().clone());
    match acoes::gravar_opcoes(&versao, &o) {
        Ok(()) => {
            fechar();
            let modo = if o.tela_cheia { "tela cheia" } else { "janela" };
            mensagem(
                &format!(
                    "Opções do {versao} salvas ({} x {}, {modo}, qualidade {}). Valem na próxima vez que o jogo abrir.",
                    o.largura,
                    o.altura,
                    QUALIDADES[o.qualidade as usize].to_lowercase()
                ),
                Cor::Ok,
            );
        }
        Err(e) => unsafe {
            MessageBoxW(c.janela, w(&e).as_ptr(), w(TITULO_JANELA).as_ptr(), MB_OK | MB_ICONERROR);
        },
    }
}

pub unsafe extern "system" fn procedimento(janela: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as i32;
            let codigo = ((wparam >> 16) & 0xFFFF) as u32;
            match id {
                ID_RES if codigo == CBN_SELCHANGE => conferir_wide(),
                ID_SALVAR_OP | IDOK_ => salvar(),
                ID_CANCELAR | IDCANCEL_ => fechar(),
                _ => {}
            }
            0
        }
        DM_GETDEFID => (DC_HASDEFID << 16) | ID_SALVAR_OP as isize,
        WM_HSCROLL => {
            mostrar_volumes();
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkColor(hdc, GetSysColor(COLOR_BTNFACE));
            SetTextColor(hdc, GetSysColor(COLOR_WINDOWTEXT));
            GetSysColorBrush(COLOR_BTNFACE)
        }
        WM_CLOSE => {
            fechar();
            0
        }
        _ => DefWindowProcW(janela, msg, wparam, lparam),
    }
}

/// Abre a janela para a versão escolhida, com os valores do `systemsettings.ini` dela.
pub fn abrir() {
    if ocupado() || OP.with(Cell::get).janela != 0 {
        return;
    }
    let versao = versao_atual();
    let o = match acoes::ler_opcoes(&versao) {
        Ok(o) => o,
        Err(e) => return mensagem(&e, Cor::Erro),
    };
    let mut resolucoes = resolucoes_da_tela();
    if !resolucoes.contains(&(o.largura, o.altura)) {
        resolucoes.push((o.largura, o.altura));
        resolucoes.sort_unstable();
    }
    let c = ctl();
    unsafe {
        let instancia = GetModuleHandleW(std::ptr::null());
        let estilo = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let mut r = RECT { left: 0, top: 0, right: s(400), bottom: s(318) };
        AdjustWindowRectEx(&mut r, estilo, 0, WS_EX_DLGMODALFRAME);
        let (larg, alt) = (r.right - r.left, r.bottom - r.top);
        let mut pai = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(c.janela, &mut pai);
        let x = pai.left + ((pai.right - pai.left) - larg) / 2;
        let y = pai.top + ((pai.bottom - pai.top) - alt) / 2;
        let janela = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w(CLASSE).as_ptr(),
            w(&format!("Opções do jogo — Perfect World {versao}")).as_ptr(),
            estilo,
            x,
            y,
            larg,
            alt,
            c.janela,
            0,
            instancia,
            std::ptr::null(),
        );
        if janela == 0 {
            return mensagem("Não foi possível abrir a janela de opções.", Cor::Erro);
        }
        let cr = Criador { janela, instancia };
        let st = w("STATIC");
        let bt = w("BUTTON");
        let f = c.fonte;
        let rotulo = |t: &str, y: i32| cr.criar(st.as_ptr(), t, SS_NOPREFIX, 0, (16, y, 110, 20), -1, f);
        let combo = |y: i32, id: i32| {
            cr.criar(WC_COMBOBOXW, "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 0, (130, y, 254, 260), id, f)
        };
        let barra = |y: i32, id: i32, v: u32| {
            let h = cr.criar(TRACKBAR_CLASSW, "", WS_TABSTOP | TBS_NOTICKS, 0, (124, y, 214, 28), id, f);
            SendMessageW(h, TBM_SETRANGE, 1, (100 << 16) as LPARAM);
            SendMessageW(h, TBM_SETPOS, 1, v as LPARAM);
            h
        };

        rotulo("Modo:", 20);
        let modo = combo(16, ID_MODO);
        combo_adicionar(modo, "Janela");
        combo_adicionar(modo, "Tela cheia");
        SendMessageW(modo, CB_SETCURSEL, o.tela_cheia as usize, 0);

        rotulo("Resolução:", 52);
        let res = combo(48, ID_RES);
        for (l, a) in &resolucoes {
            combo_adicionar(res, &format!("{l} x {a}"));
        }
        let atual = resolucoes.iter().position(|&x| x == (o.largura, o.altura)).unwrap_or(0);
        SendMessageW(res, CB_SETCURSEL, atual, 0);

        let wide =
            cr.criar(bt.as_ptr(), "Widescreen (16:9)", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, 0, (130, 80, 254, 22), ID_WIDE, f);
        SendMessageW(wide, BM_SETCHECK, o.widescreen as usize, 0);

        rotulo("Qualidade:", 116);
        let qual = combo(112, ID_QUAL);
        for q in QUALIDADES {
            combo_adicionar(qual, q);
        }
        SendMessageW(qual, CB_SETCURSEL, o.qualidade.min(4) as usize, 0);

        rotulo("Volume do som:", 154);
        let som = barra(148, ID_SOM, o.volume_som);
        let som_valor = cr.criar(st.as_ptr(), "", SS_NOPREFIX, 0, (344, 154, 40, 20), -1, f);
        rotulo("Volume da música:", 190);
        let musica = barra(184, ID_MUSICA, o.volume_musica);
        let musica_valor = cr.criar(st.as_ptr(), "", SS_NOPREFIX, 0, (344, 190, 40, 20), -1, f);

        cr.criar(
            st.as_ptr(),
            "Valem na próxima vez que o jogo abrir. As demais opções ficam no próprio jogo.",
            SS_NOPREFIX | SS_EDITCONTROL,
            0,
            (16, 226, 368, 36),
            -1,
            f,
        );
        cr.criar(bt.as_ptr(), "Salvar", BS_DEFPUSHBUTTON as u32 | WS_TABSTOP, 0, (204, 274, 88, 28), ID_SALVAR_OP, f);
        cr.criar(bt.as_ptr(), "Cancelar", WS_TABSTOP, 0, (296, 274, 88, 28), ID_CANCELAR, f);

        RESOLUCOES.with(|r| *r.borrow_mut() = resolucoes);
        VERSAO.with(|v| *v.borrow_mut() = versao);
        OP.with(|op| op.set(Ctl { janela, modo, res, wide, qual, som, musica, som_valor, musica_valor }));
        conferir_wide();
        mostrar_volumes();
        EnableWindow(c.janela, 0);
        ShowWindow(janela, SW_SHOW);
        SetFocus(modo);
    }
}
