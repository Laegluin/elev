#[cfg(not(windows))]
compile_error!("this crate only supports windows");

extern crate elev;

use std::env;
use std::io;
use std::path::Path;
use std::process;

fn main() {
    let command_line: Vec<_> = env::args_os().skip(1).collect();
    let program = command_line[0].clone();

    match elev::start_elevated(command_line) {
        Ok(exit_code) => process::exit(exit_code),
        Err(why) => {
            if why.kind() == io::ErrorKind::NotFound {
                let _ = elev::print_err(format!("cannot find '{}'", Path::new(&program).display()));
            } else {
                let _ = elev::print_err(why);
            }

            process::exit(i32::min_value())
        }
    }
}
