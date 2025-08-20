// Copyright 2018-2025 the Deno authors. MIT license.
use deno_core::{op2, OpState};
use deno_error::{builtin_classes::GENERIC_ERROR, JsErrorBox, JsErrorClass};
use rustyline::{
    config::Configurer, error::ReadlineError, Cmd, Editor, KeyCode, KeyEvent, Modifiers,
};

trait TtyPlatform {
    type FileDescriptor;

    fn console_size(fd: Self::FileDescriptor) -> Result<ConsoleSize, std::io::Error>;
    fn set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), TtyError>;
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::WindowsTtyPlatform as Tty;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::UnixTtyPlatform as Tty;

deno_core::extension!(
    deno_tty,
    ops = [op_set_raw, op_console_size, op_read_line_prompt],
);

deno_error::js_error_wrapper!(ReadlineError, JsReadlineError, |err| {
    match err {
        #[cfg(unix)]
        ReadlineError::Errno(e) => deno_process::JsNixError(*e).get_class(),

        ReadlineError::Io(e) => e.get_class(),
        _ => GENERIC_ERROR.into(),
    }
});

/// The size of the console, in columns and rows.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ConsoleSize {
    pub cols: u32,
    pub rows: u32,
}

/// Errors that can occur when working with the TTY.
#[derive(Debug, thiserror::Error, deno_error::JsError)]
pub enum TtyError {
    #[class(inherit)]
    #[error(transparent)]
    Resource(
        #[from]
        #[inherit]
        deno_core::error::ResourceError,
    ),

    #[class(inherit)]
    #[error("{0}")]
    Io(
        #[from]
        #[inherit]
        std::io::Error,
    ),

    #[cfg(unix)]
    #[class(inherit)]
    #[error(transparent)]
    Nix(#[inherit] JsNixError),

    #[class(inherit)]
    #[error(transparent)]
    Other(#[inherit] JsErrorBox),
}

#[op2(fast)]
fn op_set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), TtyError> {
    Tty::set_raw(state, rid, is_raw, cbreak)
}

#[op2(fast)]
fn op_console_size(state: &mut OpState, #[buffer] result: &mut [u32]) -> Result<(), TtyError> {
    fn check_console_size(
        state: &mut OpState,
        result: &mut [u32],
        rid: u32,
    ) -> Result<(), TtyError> {
        let fd = state.resource_table.get_fd(rid)?;
        let size = Tty::console_size(fd)?;
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

#[op2]
#[string]
pub fn op_read_line_prompt(
    #[string] prompt_text: &str,
    #[string] default_value: &str,
) -> Result<Option<String>, JsReadlineError> {
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
        Err(err) => Err(JsReadlineError(err)),
    }
}
