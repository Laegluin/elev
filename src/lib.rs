#![deny(bare_trait_objects)]

#[cfg(not(windows))]
compile_error!("this crate only supports windows");

use scopeguard::defer;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs;
use std::io::{self, Write};
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use winapi::shared::minwindef::FALSE;
use winapi::shared::ntdef::NULL;
use winapi::shared::winerror::{E_INVALIDARG, E_OUTOFMEMORY, E_UNEXPECTED};
use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::shellapi::{
    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    ShellExecuteExW,
};
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::{INFINITE, WAIT_FAILED};
use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};
use winapi::um::winuser::SW_HIDE;

macro_rules! try_win32 {
    ($call: expr_2021, $($errs: expr_2021),+) => {
        let result = $call;

        #[allow(unused_parens)]
        let is_err = {
            $((result == $errs))||+
        };

        if is_err {
            return Err(io::Error::last_os_error());
        }
    };
}

pub fn start_runner(command_line: impl IntoIterator<Item = OsString>) -> Result<i32, io::Error> {
    let verb = into_wide_str("runas");
    let program = into_wide_str(find_runner()?);
    let args = encode_windows_args(command_line.into_iter().collect());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        hwnd: ptr::null_mut(),
        lpVerb: verb.as_ptr(),
        lpFile: program.as_ptr(),
        lpParameters: args.as_ptr(),
        lpDirectory: ptr::null(),
        nShow: SW_HIDE,
        hInstApp: ptr::null_mut(),
        lpIDList: ptr::null_mut(),
        lpClass: ptr::null_mut(),
        hkeyClass: ptr::null_mut(),
        dwHotKey: 0,
        hMonitor: ptr::null_mut(),
        hProcess: ptr::null_mut(),
    };

    unsafe {
        try_win32!(
            CoInitializeEx(NULL, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE),
            E_INVALIDARG,
            E_OUTOFMEMORY,
            E_UNEXPECTED
        );

        defer!(CoUninitialize());

        try_win32!(ShellExecuteExW(&mut info), FALSE);

        if info.hProcess.is_null() {
            return io_err("the process handle is null");
        }

        try_win32!(WaitForSingleObject(info.hProcess, INFINITE), WAIT_FAILED);

        let mut exit_code = 0;
        try_win32!(GetExitCodeProcess(info.hProcess, &mut exit_code), FALSE);

        Ok(exit_code as i32)
    }
}

fn find_runner() -> Result<PathBuf, io::Error> {
    // resolve all symlinks to elev, then look for elev-run next to it
    let mut elev_run = env::current_exe()?;

    while elev_run.metadata()?.file_type().is_symlink() {
        elev_run = fs::read_link(elev_run)?
    }

    elev_run.pop();
    elev_run.push("elev-run.exe");

    if !elev_run.is_file() {
        return io_err(format!(
            "cannot find elev-run (tried '{}')",
            elev_run.display()
        ));
    }

    // compare the hash of elev-run with the one specified at build time
    // this prevents security issues with hardlinks of elev, which would allow
    // replacing elev-run without modifying elev itself
    let expected_hash = match option_env!("ELEV_RUN_SHA256") {
        Some(hash) => hash,
        None => {
            return io_err(
                "elev was compiled without specifying the hash of elev-run, running it is not safe",
            );
        }
    };

    let actual_hash = hex::encode(Sha256::digest(fs::read(&elev_run)?));

    if expected_hash == actual_hash {
        Ok(elev_run)
    } else {
        io_err(format!(
            "'{}' is not the correct version of elev-run",
            elev_run.display()
        ))
    }
}

fn io_err<T>(msg: impl Into<String>) -> Result<T, io::Error> {
    Err(io::Error::new(io::ErrorKind::Other, msg.into()))
}

/// Converts `string` to a null-terminated wstr.
fn into_wide_str(string: impl Into<OsString>) -> Vec<u16> {
    let mut string = string.into();
    string.push("\0");
    string.encode_wide().collect()
}

/// Escapes arguments as described in https://msdn.microsoft.com/en-us/library/ms880421 and
/// converts them to a null-terminated wstr.
fn encode_windows_args(args: Vec<OsString>) -> Vec<u16> {
    const SPACE: u16 = 0x20;
    const TAB: u16 = 0x09;
    const QUOTE: u16 = 0x22;
    const BACKSLASH: u16 = 0x5c;

    let mut escaped = Vec::new();

    for arg in args {
        let arg: Vec<_> = arg.encode_wide().collect();

        let start_idx = escaped.len();
        let mut has_spaces = false;

        for (i, c) in arg.iter().enumerate() {
            match *c {
                QUOTE => escaped.extend_from_slice(&[BACKSLASH, QUOTE]),
                // double backslash if it's followed by a quote or it's the last char and
                // the arg contains whitespace (because that means it will be followed by a quote
                // to escape the whitespace)
                BACKSLASH
                    if arg.get(i + 1) == Some(&QUOTE) || arg.get(i + 1) == None && has_spaces =>
                {
                    escaped.extend_from_slice(&[BACKSLASH, BACKSLASH])
                }
                whitespace if whitespace == SPACE || whitespace == TAB => {
                    has_spaces = true;
                    escaped.push(whitespace)
                }
                other => escaped.push(other),
            }
        }

        if has_spaces {
            escaped.insert(start_idx, QUOTE);
            escaped.push(QUOTE);
        }

        escaped.push(SPACE);
    }

    // remove the trailing space
    escaped.pop();

    // terminate the string
    escaped.push(0);
    escaped
}

pub fn start_elevated(command_line: impl IntoIterator<Item = OsString>) -> Result<i32, io::Error> {
    unsafe {
        try_win32!(FreeConsole(), FALSE);
        try_win32!(AttachConsole(ATTACH_PARENT_PROCESS), FALSE);
    }

    let args: Vec<_> = command_line.into_iter().collect();
    let status = Command::new(&args[0]).args(&args[1..]).status()?;

    // on windows, code() is always Some
    Ok(status.code().unwrap())
}

pub fn print_err(err: impl Display) -> Result<(), io::Error> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut err_spec = ColorSpec::new();
    err_spec.set_bold(true).set_fg(Some(Color::Red));

    stderr.set_color(&err_spec)?;
    write!(&mut stderr, "error: ")?;
    stderr.set_color(&ColorSpec::new())?;
    writeln!(&mut stderr, "{}", err)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn escape_args() {
        let args = vec![
            OsString::from("a b c"),
            OsString::from("d"),
            OsString::from("e"),
        ];

        let escaped = into_wide_str(r#""a b c" d e"#);
        assert_eq!(escaped, encode_windows_args(args));

        let args = vec![
            OsString::from(r#"ab"c"#),
            OsString::from(r#"\"#),
            OsString::from(r#"d"#),
        ];

        let escaped = into_wide_str(r#"ab\"c \ d"#);
        assert_eq!(escaped, encode_windows_args(args));

        let args = vec![
            OsString::from(r#"a\\\b"#),
            OsString::from(r#"de fg"#),
            OsString::from(r#"h"#),
        ];

        let escaped = into_wide_str(r#"a\\\b "de fg" h"#);
        assert_eq!(escaped, encode_windows_args(args));

        let args = vec![
            OsString::from(r#"a\"b"#),
            OsString::from(r#"c"#),
            OsString::from(r#"d"#),
        ];

        let escaped = into_wide_str(r#"a\\\"b c d"#);
        assert_eq!(escaped, encode_windows_args(args));

        let args = vec![
            OsString::from(r#"a\\b c"#),
            OsString::from(r#"d"#),
            OsString::from(r#"e"#),
        ];

        let escaped = into_wide_str(r#""a\\b c" d e"#);
        assert_eq!(escaped, encode_windows_args(args));
    }
}
