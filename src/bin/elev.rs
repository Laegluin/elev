#[cfg(not(windows))]
compile_error!("this crate only supports windows");

use clap::{Arg, ArgAction, Command, crate_authors, crate_version};
use std::env;
use std::ffi::OsString;
use std::process;

fn main() {
    let app = Command::new("elev")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Runs a command as administrator.")
        .arg(
            Arg::new("command-line")
                .help("The command and its arguments")
                .required(true)
                .action(ArgAction::Append),
        );

    let args: Vec<_> = env::args_os().collect();

    if args.len() == 1 || is_help_or_version(args.get(1)) {
        app.get_matches_from(args.clone());
    }

    match elev::start_runner(args.into_iter().skip(1)) {
        Ok(exit_code) => process::exit(exit_code),
        Err(why) => {
            let _ = elev::print_err(why);
            process::exit(i32::min_value())
        }
    }
}

fn is_help_or_version(arg: Option<&OsString>) -> bool {
    arg.and_then(|arg| arg.to_str())
        .map(|arg| match arg {
            "-V" | "--version" | "-h" | "--help" => true,
            _ => false,
        })
        .unwrap_or(false)
}
