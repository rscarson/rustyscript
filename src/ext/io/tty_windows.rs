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

use winapi::shared::minwindef::DWORD;
use winapi::um::wincon;

deno_core::extension!(
    deno_tty,
    ops = [op_set_raw, op_console_size, op_read_line_prompt],
);

// ref: <https://learn.microsoft.com/en-us/windows/console/setconsolemode>
const COOKED_MODE: DWORD =
    // enable line-by-line input (returns input only after CR is read)
    wincon::ENABLE_LINE_INPUT
  // enables real-time character echo to console display (requires ENABLE_LINE_INPUT)
  | wincon::ENABLE_ECHO_INPUT
  // system handles CTRL-C (with ENABLE_LINE_INPUT, also handles BS, CR, and LF) and other control keys (when using `ReadFile` or `ReadConsole`)
  | wincon::ENABLE_PROCESSED_INPUT;

fn mode_raw_input_on(original_mode: DWORD) -> DWORD {
    original_mode & !COOKED_MODE | wincon::ENABLE_VIRTUAL_TERMINAL_INPUT
}

fn mode_raw_input_off(original_mode: DWORD) -> DWORD {
    original_mode & !wincon::ENABLE_VIRTUAL_TERMINAL_INPUT | COOKED_MODE
}

#[op2(fast)]
fn op_set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), AnyError> {
    let handle_or_fd = state.resource_table.get_fd(rid)?;

    // From https://github.com/kkawakam/rustyline/blob/master/src/tty/windows.rs
    // and https://github.com/kkawakam/rustyline/blob/master/src/tty/unix.rs
    // and https://github.com/crossterm-rs/crossterm/blob/e35d4d2c1cc4c919e36d242e014af75f6127ab50/src/terminal/sys/windows.rs
    // Copyright (c) 2015 Katsu Kawakami & Rustyline authors. MIT license.
    // Copyright (c) 2019 Timon. MIT license.
    use winapi::shared::minwindef::FALSE;
    use winapi::um::consoleapi;

    let handle = handle_or_fd;

    if cbreak {
        return Err(deno_core::error::not_supported());
    }

    let mut original_mode: DWORD = 0;
    // SAFETY: winapi call
    if unsafe { consoleapi::GetConsoleMode(handle, &mut original_mode) } == FALSE {
        return Err(Error::last_os_error().into());
    }

    let new_mode = if is_raw {
        mode_raw_input_on(original_mode)
    } else {
        mode_raw_input_off(original_mode)
    };

    // SAFETY: winapi call
    if unsafe { consoleapi::SetConsoleMode(handle, new_mode) } == FALSE {
        return Err(Error::last_os_error().into());
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

fn console_size_from_fd(
    handle: std::os::windows::io::RawHandle,
) -> Result<ConsoleSize, std::io::Error> {
    // SAFETY: winapi calls
    unsafe {
        let mut bufinfo: winapi::um::wincon::CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();

        if winapi::um::wincon::GetConsoleScreenBufferInfo(handle, &mut bufinfo) == 0 {
            return Err(Error::last_os_error());
        }
        Ok(ConsoleSize {
            cols: bufinfo.dwSize.X as u32,
            rows: bufinfo.dwSize.Y as u32,
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

    editor.set_keyseq_timeout(1);
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
