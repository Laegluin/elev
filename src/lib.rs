extern crate winapi;

use std::env;
use std::ffi::OsString;
use std::io;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::process::Command;
use std::ptr;
use winapi::shared::minwindef::FALSE;
use winapi::shared::ntdef::NULL;
use winapi::um::combaseapi::CoInitializeEx;
use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use winapi::um::processthreadsapi::GetExitCodeProcess;
use winapi::um::shellapi::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW,
};
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::{INFINITE, WAIT_FAILED};
use winapi::um::wincon::{AttachConsole, FreeConsole, ATTACH_PARENT_PROCESS};
use winapi::um::winuser::SW_HIDE;

pub fn start_runner() -> Result<i32, io::Error> {
    // TODO: don't use current_exe to make this safe
    let elev_runner = env::current_exe()?.join("../elev-runner.exe");

    let verb = into_wide_str("runas");
    let program = into_wide_str(elev_runner);
    let args = encode_windows_args(env::args_os().skip(1).collect());

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
        // TODO: CoUninitialize
        CoInitializeEx(NULL, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);

        if ShellExecuteExW(&mut info) == FALSE {
            return Err(io::Error::last_os_error());
        }

        if info.hProcess.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "the process handle is null".to_owned(),
            ));
        }

        if WaitForSingleObject(info.hProcess, INFINITE) == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }

        let mut exit_code = 0;
        if GetExitCodeProcess(info.hProcess, &mut exit_code) == FALSE {
            return Err(io::Error::last_os_error());
        }

        Ok(exit_code as i32)
    }
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
        // remove the null terminator
        let mut arg = into_wide_str(arg);
        arg.pop();

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

pub fn start_elevated() -> Result<i32, io::Error> {
    unsafe {
        if FreeConsole() == FALSE {
            return Err(io::Error::last_os_error());
        }

        if AttachConsole(ATTACH_PARENT_PROCESS) == FALSE {
            return Err(io::Error::last_os_error());
        }
    }

    let args: Vec<_> = env::args().skip(1).collect();
    let status = Command::new(&args[0]).args(&args[1..]).status()?;

    // on windows, code() is always Some
    Ok(status.code().unwrap())
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