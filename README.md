# elev

`elev` allows you to spawn windows processes as administrator, right from your terminal. Unlike
for example powershell's `Start-Process -Verb runAs`, the spawned process will use the current
console.

Usage is similar to linux' `sudo`:

```powershell
elev choco install sqlite3
```

will execute `choco install sqlite3` as administrator.

## Installation

Simply move `elev.exe` and `elev-run.exe` to a place you like and add the directory to your PATH.
`elev-run.exe` does not have to be in your PATH, but it has to be placed next to `elev.exe`.

## How it works

`elev` simply uses `ShellExecuteEx` to launch `elev-run.exe`, with the verb `runas`. This always
creates a new console for the process. The console is hidden by starting the process with the `SW_HIDE` flag.
`elev-run` then uses `FreeConsole` followed by `AttachConsole` to set the correct console before starting
the actual program.

`elev` locates `elev-run` by searching next to its binary (after resolving symlinks). To avoid
executing the wrong binary (and causing privilege escalation for the wrong program!), the resolved
`elev-run` binary is hashed and compared to a hash specified at compile time.

## Building

`elev` requires Rust 1.30 or higher and the Visual C++ Build Tools (if you are using Rust's
pc-windows-msvc toolchain, you already have them).

If you want to do a release build, simple run `scripts/build.ps1`.

For a manual build, build `elev-run` first: `cargo build --bin elev-run --feature require-elevation`.
Next, set the `ELEV_RUN_SHA256` environment variable to the hex string (lowercase) of the sha256 hash of the
`elev-run.exe` binary you just built. Now you can build `elev`: `cargo build --bin elev`.

## Contributing

If you have any ideas or improvements, simply open an issue or submit a PR.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## License

Licensed under either of

- Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
