#[cfg(not(windows))]
compile_error!("this crate only supports windows");

extern crate elev;

use std::process;

fn main() {
    match elev::start_elevated() {
        Ok(exit_code) => process::exit(exit_code),
        Err(why) => {
            eprintln!("{}", why);
            process::exit(i32::min_value())
        }
    }
}
