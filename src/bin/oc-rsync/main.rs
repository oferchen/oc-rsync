// src/bin/oc-rsync/main.rs
mod stdio;

use oc_rsync_cli::options::OutBuf;
use oc_rsync_cli::{cli_command, exit_code_from_engine_error};
use protocol::ExitCode;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if oc_rsync_cli::print_version_if_requested(args.iter().cloned()) {
        return;
    }
    let mut cmd = cli_command();
    let matches = cmd
        .try_get_matches_from_mut(&args)
        .unwrap_or_else(|e| oc_rsync_cli::handle_clap_error(&cmd, e));
    if matches.get_flag("dump-help-body") {
        print!("{}", oc_rsync_cli::dump_help_body(&cmd));
        return;
    }
    if let Some(mode) = matches.get_one::<OutBuf>("outbuf") {
        if let Err(err) = stdio::set_std_buffering(*mode) {
            eprintln!("failed to set stdio buffers: {err}");
            std::process::exit(u8::from(ExitCode::FileIo) as i32);
        }
    } else if matches.get_flag("daemon") {
        if let Err(err) = stdio::set_std_buffering(OutBuf::L) {
            eprintln!("failed to set stdio buffers: {err}");
            std::process::exit(u8::from(ExitCode::FileIo) as i32);
        }
    }
    if let Err(e) = oc_rsync_cli::run(&matches) {
        eprintln!("{e}");
        let code = exit_code_from_engine_error(&e);
        std::process::exit(u8::from(code) as i32);
    }
}
