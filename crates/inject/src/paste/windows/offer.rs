//! O texto no clipboard por delayed rendering, numa janela message-only própria.
//!
//! A oferta registra `CF_UNICODETEXT` sem dado; o Windows manda `WM_RENDERFORMAT` à janela
//! quando um app pede o texto, e só então ele é escrito. Essa mensagem é a prova de que alguém
//! leu. Junto vão os formatos que tiram o conteúdo do histórico do Win+V, da nuvem e dos
//! monitores de clipboard ("Cloud clipboard and clipboard history formats", Microsoft Learn).
//! A janela e a thread dela vivem o tempo de uma [`DelayedOffer`].

use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, SetLastError, HGLOBAL, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardOwner, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, HWND_MESSAGE, MSG, WM_CLOSE, WM_DESTROY, WM_RENDERALLFORMATS,
    WM_RENDERFORMAT, WNDCLASSW,
};

use crate::InjectError;

/// `CF_UNICODETEXT`, de `Win32_System_Ole`, que não vale a feature só pela constante.
const CF_UNICODETEXT: u32 = 13;
/// Formatos que tiram o conteúdo do histórico do Win+V, da nuvem e dos monitores de clipboard,
/// cada um gravado com um `DWORD 0`.
const EXCLUSION_FORMATS: [&str; 3] = [
    "ExcludeClipboardContentFromMonitorProcessing",
    "CanIncludeInClipboardHistory",
    "CanUploadToCloudClipboard",
];
/// Quanto a thread da janela tem para abrir o clipboard e registrar a oferta.
const SETUP_TIMEOUT: Duration = Duration::from_secs(2);

/// O que a thread da janela conta para quem cola.
enum Event {
    /// A oferta está no clipboard: a janela (como `isize`, para cruzar threads) e o número de
    /// sequência depois dela; ou o motivo de não estar.
    Ready(Result<(isize, u32), InjectError>),
    /// Um app leu o texto; o número de sequência depois da entrega.
    Rendered(u32),
}

/// Um texto no clipboard, entregue só quando um app o lê. Ao sair de escopo, a janela fecha; se
/// ela ainda é dona do clipboard com o texto não entregue, o texto é entregue antes, para o
/// clipboard não ficar com um formato vazio.
pub(super) struct DelayedOffer {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
    events: Receiver<Event>,
    /// O número de sequência que o clipboard tem enquanto ninguém mais escreve nele.
    sequence: u32,
    read: bool,
}

impl DelayedOffer {
    /// Esvazia o clipboard e põe `text` nele, fora do histórico e da nuvem.
    pub(super) fn start(text: &str) -> Result<Self, InjectError> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let (sender, events) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("fala-clipboard-offer".to_owned())
            .spawn(move || owner_thread(wide, sender))
            .map_err(|err| InjectError::Clipboard(err.to_string()))?;
        match events.recv_timeout(SETUP_TIMEOUT) {
            Ok(Event::Ready(Ok((hwnd, sequence)))) => Ok(Self {
                hwnd,
                thread: Some(thread),
                events,
                sequence,
                read: false,
            }),
            Ok(Event::Ready(Err(err))) => {
                let _ = thread.join();
                Err(err)
            }
            // A thread não respondeu a tempo: fica solta, sem janela conhecida para fechar.
            Ok(Event::Rendered(_)) | Err(_) => Err(InjectError::Clipboard(
                "a janela do clipboard não respondeu".to_owned(),
            )),
        }
    }

    /// Espera até `timeout` que um app leia o texto; `true` se leu (agora ou antes).
    pub(super) fn wait_read(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.read {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.events.recv_timeout(left) {
                Ok(event) => self.note(event),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        self.read
    }

    /// `true` se ninguém escreveu no clipboard desde a oferta (ou desde a entrega do texto).
    pub(super) fn unchanged(&mut self) -> bool {
        // Uma leitura que chegou depois do prazo também muda o número de sequência.
        while let Ok(event) = self.events.try_recv() {
            self.note(event);
        }
        // SAFETY: sem pré-condições.
        unsafe { GetClipboardSequenceNumber() == self.sequence }
    }

    fn note(&mut self, event: Event) {
        if let Event::Rendered(sequence) = event {
            self.sequence = sequence;
            self.read = true;
        }
    }
}

impl Drop for DelayedOffer {
    fn drop(&mut self) {
        // SAFETY: `hwnd` é a janela da thread da oferta, que só sai do laço de mensagens depois
        // de destruí-la; se ela já não existe, o `PostMessageW` só falha.
        if unsafe { PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0) } == 0 {
            // Sem o WM_CLOSE a thread não sai do laço; fica solta em vez de travar a cola.
            let err = os_error("PostMessageW");
            log::warn!("a janela do clipboard não recebeu WM_CLOSE: {err}");
            return;
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::warn!("a thread da janela do clipboard terminou em pânico");
        }
    }
}

fn os_error(call: &'static str) -> InjectError {
    // SAFETY: sem pré-condições; lê o último erro desta thread.
    let code = unsafe { GetLastError() };
    InjectError::Os { call, code }
}

/// O texto em oferta, visto pelo procedimento da janela, que roda nesta mesma thread.
struct OwnerState {
    text: Vec<u16>,
    events: Sender<Event>,
    rendered: bool,
}

thread_local! {
    static OWNER: RefCell<Option<OwnerState>> = const { RefCell::new(None) };
}

/// Cria a janela, põe a oferta no clipboard e atende mensagens até `WM_CLOSE`.
fn owner_thread(text: Vec<u16>, events: Sender<Event>) {
    OWNER.with_borrow_mut(|owner| {
        *owner = Some(OwnerState {
            text,
            events: events.clone(),
            rendered: false,
        });
    });
    let ready = create_window().and_then(|hwnd| match write_offer(hwnd) {
        Ok(sequence) => Ok((hwnd, sequence)),
        Err(err) => {
            // SAFETY: a janela é desta thread e ainda não foi destruída.
            unsafe { DestroyWindow(hwnd) };
            Err(err)
        }
    });
    let failed = ready.is_err();
    let _ = events.send(Event::Ready(
        ready.map(|(hwnd, sequence)| (hwnd as isize, sequence)),
    ));
    if !failed {
        let mut msg = MSG::default();
        // SAFETY: `msg` vive durante o laço; o filtro nulo atende todas as janelas da thread.
        while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
            // SAFETY: `msg` veio do `GetMessageW` acima.
            unsafe { DispatchMessageW(&msg) };
        }
    }
    OWNER.with_borrow_mut(|owner| *owner = None);
}

fn create_window() -> Result<HWND, InjectError> {
    let class: Vec<u16> = "FalaClipboardOffer\0".encode_utf16().collect();
    // SAFETY: `class` termina em 0 e vive até o fim da função; o Windows copia o nome da classe
    // no registro. Registrar de novo a mesma classe falha sem efeito, e por isso o retorno é
    // ignorado: o `CreateWindowExW` abaixo é que diz se ela existe.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let wndclass = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..WNDCLASSW::default()
        };
        RegisterClassW(&wndclass);
        CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err(os_error("CreateWindowExW"));
    }
    Ok(hwnd)
}

/// Esvazia o clipboard, registra o texto sem dado e grava os formatos de exclusão. Devolve o
/// número de sequência depois de fechar o clipboard.
fn write_offer(hwnd: HWND) -> Result<u32, InjectError> {
    open_clipboard(hwnd)?;
    let written = (|| {
        // SAFETY: o clipboard está aberto por esta thread (acima) até o `CloseClipboard` abaixo.
        unsafe {
            if EmptyClipboard() == 0 {
                return Err(os_error("EmptyClipboard"));
            }
            // Com dado nulo, o retorno é nulo mesmo no sucesso; quem diz se falhou é o erro.
            SetLastError(0);
            if SetClipboardData(CF_UNICODETEXT, std::ptr::null_mut()).is_null()
                && GetLastError() != 0
            {
                return Err(os_error("SetClipboardData"));
            }
            for name in EXCLUSION_FORMATS {
                let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let format = RegisterClipboardFormatW(name.as_ptr());
                if format == 0 {
                    return Err(os_error("RegisterClipboardFormatW"));
                }
                set_clipboard_bytes(format, &0u32.to_ne_bytes())?;
            }
        }
        Ok(())
    })();
    // SAFETY: fecha o clipboard que `open_clipboard` abriu.
    unsafe { CloseClipboard() };
    written?;
    // SAFETY: sem pré-condições.
    Ok(unsafe { GetClipboardSequenceNumber() })
}

/// Abre o clipboard para `hwnd`, tentando por até ~100 ms se outro app o tem aberto.
fn open_clipboard(hwnd: HWND) -> Result<(), InjectError> {
    for _ in 0..10 {
        // SAFETY: `hwnd` é uma janela desta thread.
        if unsafe { OpenClipboard(hwnd) } != 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(os_error("OpenClipboard"))
}

/// Grava `bytes` no formato `format` do clipboard, que precisa estar aberto por esta thread.
fn set_clipboard_bytes(format: u32, bytes: &[u8]) -> Result<(), InjectError> {
    // SAFETY: o bloco tem `bytes.len()` bytes e é copiado enquanto travado; se o
    // `SetClipboardData` aceita, o sistema passa a ser dono dele, e se recusa, ele é liberado.
    unsafe {
        let global: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if global.is_null() {
            return Err(os_error("GlobalAlloc"));
        }
        let target = GlobalLock(global).cast::<u8>();
        if target.is_null() {
            let err = os_error("GlobalLock");
            GlobalFree(global);
            return Err(err);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        GlobalUnlock(global);
        if SetClipboardData(format, global).is_null() {
            let err = os_error("SetClipboardData");
            GlobalFree(global);
            return Err(err);
        }
    }
    Ok(())
}

/// Entrega o texto em oferta ao clipboard aberto e avisa quem cola. O texto nunca vai ao log.
fn render() {
    OWNER.with_borrow_mut(|owner| {
        let Some(owner) = owner.as_mut() else {
            return;
        };
        let bytes: Vec<u8> = owner.text.iter().flat_map(|u| u.to_ne_bytes()).collect();
        match set_clipboard_bytes(CF_UNICODETEXT, &bytes) {
            Ok(()) => {
                owner.rendered = true;
                // SAFETY: sem pré-condições.
                let sequence = unsafe { GetClipboardSequenceNumber() };
                let _ = owner.events.send(Event::Rendered(sequence));
            }
            Err(err) => log::warn!("o texto oferecido não foi entregue ao clipboard: {err}"),
        }
    });
}

fn rendered() -> bool {
    OWNER.with_borrow(|owner| owner.as_ref().is_some_and(|owner| owner.rendered))
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        // Um app pediu o texto: o clipboard já está aberto por ele.
        WM_RENDERFORMAT if wparam == CF_UNICODETEXT as usize => {
            render();
            0
        }
        // A janela vai ser destruída ainda dona de um formato não entregue (a restauração
        // falhou): entrega o texto para o clipboard não ficar com um formato vazio.
        WM_RENDERALLFORMATS => {
            // SAFETY: `hwnd` é esta janela; o clipboard só é usado se abrir.
            unsafe {
                if !rendered() && OpenClipboard(hwnd) != 0 {
                    if GetClipboardOwner() == hwnd {
                        render();
                    }
                    CloseClipboard();
                }
            }
            0
        }
        WM_DESTROY => {
            // SAFETY: sem pré-condições; encerra o laço de mensagens desta thread.
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: os argumentos vieram do Windows para esta janela.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Estes testes usam o clipboard real do Windows (sem janela visível) e o sobrescrevem, por isso
/// ficam fora do `cargo test` comum: `cargo test -p fala-inject offer -- --ignored`.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable;

    use super::*;

    /// O clipboard é um só: um teste por vez.
    static CLIPBOARD: Mutex<()> = Mutex::new(());

    fn read_text() -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    #[test]
    #[ignore = "usa o clipboard do sistema"]
    fn offer_is_rendered_when_read() {
        let _lock = CLIPBOARD.lock().expect("lock do clipboard");
        let mut offer = DelayedOffer::start("FALA-OFERTA-lida").expect("oferta");
        assert!(
            !offer.wait_read(Duration::from_millis(200)),
            "lida sem leitor"
        );
        assert_eq!(read_text().as_deref(), Some("FALA-OFERTA-lida"));
        assert!(
            offer.wait_read(Duration::from_secs(1)),
            "a leitura não chegou"
        );
        assert!(offer.unchanged(), "a entrega contou como escrita alheia");
    }

    #[test]
    #[ignore = "usa o clipboard do sistema"]
    fn foreign_write_is_detected() {
        let _lock = CLIPBOARD.lock().expect("lock do clipboard");
        let mut offer = DelayedOffer::start("FALA-OFERTA-trocada").expect("oferta");
        assert!(offer.unchanged());
        arboard::Clipboard::new()
            .expect("arboard")
            .set_text("FALA-OUTRA-COPIA")
            .expect("cópia alheia");
        assert!(!offer.unchanged(), "a cópia alheia passou despercebida");
        drop(offer);
        assert_eq!(read_text().as_deref(), Some("FALA-OUTRA-COPIA"));
    }

    #[test]
    #[ignore = "usa o clipboard do sistema"]
    fn offer_carries_exclusion_formats() {
        let _lock = CLIPBOARD.lock().expect("lock do clipboard");
        let _offer = DelayedOffer::start("FALA-OFERTA-excluida").expect("oferta");
        for name in EXCLUSION_FORMATS {
            let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `wide` termina em 0; consultar formato não precisa abrir o clipboard.
            let available = unsafe {
                let format = RegisterClipboardFormatW(wide.as_ptr());
                IsClipboardFormatAvailable(format)
            };
            assert_ne!(available, 0, "{name} ausente");
        }
    }

    #[test]
    #[ignore = "usa o clipboard do sistema"]
    fn unread_offer_is_rendered_on_drop() {
        let _lock = CLIPBOARD.lock().expect("lock do clipboard");
        let mut offer = DelayedOffer::start("FALA-OFERTA-final").expect("oferta");
        assert!(!offer.wait_read(Duration::from_millis(100)));
        drop(offer);
        assert_eq!(read_text().as_deref(), Some("FALA-OFERTA-final"));
    }
}
