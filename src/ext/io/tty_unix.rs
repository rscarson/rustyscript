// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Used with expressed written permission from the Deno project

use std::io::Error;

use deno_core::error::AnyError;
use deno_core::op2;
use deno_core::OpState;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::Cmd;
use rustyline::Editor;
use rustyline::KeyCode;
use rustyline::KeyEvent;
use rustyline::Modifiers;

use deno_core::ResourceId;
use nix::sys::termios;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default, Clone)]
struct TtyModeStore(std::rc::Rc<RefCell<HashMap<ResourceId, termios::Termios>>>);

impl TtyModeStore {
    pub fn get(&self, id: ResourceId) -> Option<termios::Termios> {
        self.0.borrow().get(&id).map(ToOwned::to_owned)
    }

    pub fn take(&self, id: ResourceId) -> Option<termios::Termios> {
        self.0.borrow_mut().remove(&id)
    }

    pub fn set(&self, id: ResourceId, mode: termios::Termios) {
        self.0.borrow_mut().insert(id, mode);
    }
}

deno_core::extension!(
    deno_tty,
    ops = [op_set_raw, op_console_size, op_read_line_prompt],
    state = |state| {
        state.put(TtyModeStore::default());
    },
);

#[op2(fast)]
fn op_set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), AnyError> {
    let handle_or_fd = state.resource_table.get_fd(rid)?;
    fn prepare_stdio() {
        // SAFETY: Save current state of stdio and restore it when we exit.
        unsafe {
            use libc::atexit;
            use libc::tcgetattr;
            use libc::tcsetattr;
            use libc::termios;
            use once_cell::sync::OnceCell;

            // Only save original state once.
            static ORIG_TERMIOS: OnceCell<Option<termios>> = OnceCell::new();
            ORIG_TERMIOS.get_or_init(|| {
                let mut termios = std::mem::zeroed::<termios>();
                if tcgetattr(libc::STDIN_FILENO, &mut termios) == 0 {
                    extern "C" fn reset_stdio() {
                        // SAFETY: Reset the stdio state.
                        unsafe {
                            tcsetattr(libc::STDIN_FILENO, 0, &ORIG_TERMIOS.get().unwrap().unwrap())
                        };
                    }

                    atexit(reset_stdio);
                    return Some(termios);
                }

                None
            });
        }
    }

    prepare_stdio();
    let tty_mode_store = state.borrow::<TtyModeStore>().clone();
    let previous_mode = tty_mode_store.get(rid);

    let raw_fd = handle_or_fd;

    fn wrap_fd<'a>(
        r: &'a deno_core::ResourceTable,
        fd: std::os::fd::RawFd,
    ) -> std::os::fd::BorrowedFd<'a> {
        match fd {
            -1 => Err(anyhow!("bad file descriptor")),
            _ => unsafe { std::os::fd::BorrowedFd::borrow_raw(fd) },
        }
    }
    let fd = wrap_fd(&state.resource_table, raw_fd)?;

    if is_raw {
        let mut raw = match previous_mode {
            Some(mode) => mode,
            None => {
                // Save original mode.
                let original_mode = termios::tcgetattr(fd)?;
                tty_mode_store.set(rid, original_mode.clone());
                original_mode
            }
        };

        raw.input_flags &= !(termios::InputFlags::BRKINT
            | termios::InputFlags::ICRNL
            | termios::InputFlags::INPCK
            | termios::InputFlags::ISTRIP
            | termios::InputFlags::IXON);

        raw.control_flags |= termios::ControlFlags::CS8;

        raw.local_flags &= !(termios::LocalFlags::ECHO
            | termios::LocalFlags::ICANON
            | termios::LocalFlags::IEXTEN);
        if !cbreak {
            raw.local_flags &= !(termios::LocalFlags::ISIG);
        }
        raw.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 1;
        raw.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 0;
        termios::tcsetattr(fd, termios::SetArg::TCSADRAIN, &raw)?;
    } else {
        // Try restore saved mode.
        if let Some(mode) = tty_mode_store.take(rid) {
            termios::tcsetattr(fd, termios::SetArg::TCSADRAIN, &mode)?;
        }
    }

    Ok(())
}

#[op2(fast)]
fn op_console_size(state: &mut OpState, #[buffer] result: &mut [u32]) -> Result<(), AnyError> {
    fn check_console_size(
        state: &mut OpState,
        result: &mut [u32],
        rid: u32,
    ) -> Result<(), AnyError> {
        let fd = state.resource_table.get_fd(rid)?;
        let size = console_size_from_fd(fd)?;
        result[0] = size.cols;
        result[1] = size.rows;
        Ok(())
    }

    let mut last_result = Ok(());
    // Since stdio might be piped we try to get the size of the console for all
    // of them and return the first one that succeeds.
    for rid in [0, 1, 2] {
        last_result = check_console_size(state, result, rid);
        if last_result.is_ok() {
            return last_result;
        }
    }

    last_result
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConsoleSize {
    pub cols: u32,
    pub rows: u32,
}

fn console_size_from_fd(fd: std::os::unix::prelude::RawFd) -> Result<ConsoleSize, std::io::Error> {
    // SAFETY: libc calls
    unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        if libc::ioctl(fd, libc::TIOCGWINSZ, &mut size as *mut _) != 0 {
            return Err(Error::last_os_error());
        }
        Ok(ConsoleSize {
            cols: size.ws_col as u32,
            rows: size.ws_row as u32,
        })
    }
}

#[op2]
#[string]
pub fn op_read_line_prompt(
    #[string] prompt_text: &str,
    #[string] default_value: &str,
) -> Result<Option<String>, AnyError> {
    let mut editor =
        Editor::<(), rustyline::history::DefaultHistory>::new().expect("Failed to create editor.");

    editor.set_keyseq_timeout(Some(1));
    editor.bind_sequence(KeyEvent(KeyCode::Esc, Modifiers::empty()), Cmd::Interrupt);

    let read_result = editor.readline_with_initial(prompt_text, (default_value, ""));
    match read_result {
        Ok(line) => Ok(Some(line)),
        Err(ReadlineError::Interrupted) => {
            // SAFETY: Disable raw mode and raise SIGINT.
            unsafe {
                libc::raise(libc::SIGINT);
            }
            Ok(None)
        }
        Err(ReadlineError::Eof) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
