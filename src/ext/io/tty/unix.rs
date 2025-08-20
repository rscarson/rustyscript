// Copyright 2018-2025 the Deno authors. MIT license.
use deno_core::{parking_lot::Mutex, OpState};
use deno_error::JsErrorBox;
use libc::{atexit, tcgetattr, tcsetattr, termios as libc_termios};
use nix::sys::termios;
use once_cell::sync::OnceCell;

use super::{ConsoleSize, TtyError, TtyPlatform};

// From https://github.com/kkawakam/rustyline/blob/master/src/tty/windows.rs
// and https://github.com/kkawakam/rustyline/blob/master/src/tty/unix.rs
// and https://github.com/crossterm-rs/crossterm/blob/e35d4d2c1cc4c919e36d242e014af75f6127ab50/src/terminal/sys/windows.rs
// Copyright (c) 2015 Katsu Kawakami & Rustyline authors. MIT license.
// Copyright (c) 2019 Timon. MIT license.
fn prepare_stdio() {
    // SAFETY: Save current state of stdio and restore it when we exit.
    unsafe {
        // Only save original state once.
        static ORIG_TERMIOS: OnceCell<Option<libc_termios>> = OnceCell::new();
        ORIG_TERMIOS.get_or_init(|| {
            let mut termios = std::mem::zeroed::<libc_termios>();
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

pub struct UnixTtyPlatform;
impl TtyPlatform for UnixTtyPlatform {
    type FileDescriptor = std::os::unix::prelude::RawFd;

    fn console_size(fd: Self::FileDescriptor) -> Result<ConsoleSize, std::io::Error> {
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

    fn set_raw(state: &mut OpState, rid: u32, is_raw: bool, cbreak: bool) -> Result<(), TtyError> {
        let handle_or_fd = state.resource_table.get_fd(rid)?;

        prepare_stdio();
        let tty_mode_store = state.borrow::<TtyModeStore>().clone();
        let previous_mode = tty_mode_store.get(rid);

        // SAFETY: Nix crate requires value to implement the AsFd trait
        let raw_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(handle_or_fd) };

        if is_raw {
            let mut raw = match previous_mode {
                Some(mode) => mode,
                None => {
                    // Save original mode.
                    let original_mode =
                        termios::tcgetattr(raw_fd).map_err(|e| TtyError::Nix(JsNixError(e)))?;
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
            termios::tcsetattr(raw_fd, termios::SetArg::TCSADRAIN, &raw)
                .map_err(|e| TtyError::Nix(JsNixError(e)))?;
        } else {
            // Try restore saved mode.
            if let Some(mode) = tty_mode_store.take(rid) {
                termios::tcsetattr(raw_fd, termios::SetArg::TCSADRAIN, &mode)
                    .map_err(|e| TtyError::Nix(JsNixError(e)))?;
            }
        }

        Ok(())
    }
}
