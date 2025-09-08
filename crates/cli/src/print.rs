// crates/cli/src/print.rs
use crate::branding;
use crate::formatter::render_help;
use crate::options::ClientOpts;
use crate::validate::exit_code_from_error_kind;
use engine::Stats;
use logging::{human_bytes, progress_formatter, rate_formatter, InfoFlag};
use protocol::ExitCode;

pub fn handle_clap_error(cmd: &clap::Command, e: clap::Error) -> ! {
    use clap::error::ErrorKind;
    let kind = e.kind();
    let code = exit_code_from_error_kind(kind);
    if kind == ErrorKind::DisplayHelp {
        println!("{}", render_help(cmd));
    } else {
        let mut msg = e.to_string();
        if matches!(kind, ErrorKind::ValueValidation | ErrorKind::InvalidValue) {
            let first = msg.lines().next().unwrap_or("");
            if first.contains("--block-size") || first.contains("'-B") {
                let val = first.split('\'').nth(1).unwrap_or("");
                msg = format!("--block-size={val} is invalid");
            } else if let Some(rest) = first.strip_prefix("error: invalid value '") {
                if let Some((val, rest)) = rest.split_once('\'') {
                    if let Some(rest) = rest.strip_prefix(" for '") {
                        if let Some((opt, _)) = rest.split_once('\'') {
                            let opt_name = opt.split_whitespace().next().unwrap_or("");
                            let kind = if first.contains("invalid digit") {
                                "invalid numeric value"
                            } else {
                                "invalid value"
                            };
                            msg = format!("{opt_name}={val}: {kind}");
                        }
                    }
                }
            }
        } else if kind == ErrorKind::UnknownArgument {
            let first = msg.lines().next().unwrap_or("");
            let arg = first.split('\'').nth(1).unwrap_or("");
            msg = format!("{arg}: unknown option");
        } else if let Some(stripped) = msg.strip_prefix("error: ") {
            msg = stripped.to_string();
        }
        msg = msg.trim_end().to_string();
        let desc = match code {
            ExitCode::Unsupported => "requested action not supported",
            _ => "syntax or usage error",
        };
        let code_num = u8::from(code);
        let prog = branding::program_name();
        let mut lines = msg.lines();
        if let Some(first) = lines.next() {
            eprintln!("{prog}: {first}");
            for line in lines {
                eprintln!("{line}");
            }
        }
        let version = option_env!("UPSTREAM_VERSION").unwrap_or("3.4.1");
        let line_no = option_env!("MAIN_C_SYNTAX_LINE").unwrap_or("1836");
        eprintln!("{prog} error: {desc} (code {code_num}) at main.c({line_no}) [client={version}]",);
    }
    std::process::exit(u8::from(code) as i32);
}

pub(crate) fn print_stats(stats: &Stats, opts: &ClientOpts) {
    let fmt_count = |n: u64| {
        if opts.human_readable {
            n.to_string()
        } else {
            progress_formatter(n, false)
        }
    };
    let fmt_bytes = |n: u64| {
        if opts.human_readable {
            human_bytes(n)
        } else {
            format!("{} bytes", progress_formatter(n, false))
        }
    };

    println!("Number of files: {}", fmt_count(stats.files_total as u64));
    println!(
        "Number of created files: {}",
        fmt_count((stats.files_created - stats.dirs_created) as u64)
    );
    println!(
        "Number of deleted files: {}",
        fmt_count(stats.files_deleted as u64)
    );
    println!(
        "Number of regular files transferred: {}",
        fmt_count(stats.files_transferred as u64)
    );
    println!("Total file size: {}", fmt_bytes(stats.total_file_size));
    println!(
        "Total transferred file size: {}",
        fmt_bytes(stats.bytes_transferred)
    );
    println!("Literal data: {}", fmt_bytes(stats.literal_data));
    println!("Matched data: {}", fmt_bytes(stats.matched_data));
    println!("File list size: {}", fmt_count(stats.file_list_size));
    println!(
        "File list generation time: {:.3} seconds",
        stats.file_list_gen_time.as_secs_f64()
    );
    println!(
        "File list transfer time: {:.3} seconds",
        stats.file_list_transfer_time.as_secs_f64()
    );
    println!(
        "Total bytes sent: {}",
        progress_formatter(stats.bytes_sent, opts.human_readable)
    );
    println!(
        "Total bytes received: {}",
        progress_formatter(stats.bytes_received, opts.human_readable)
    );
    let elapsed = stats.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        let total = stats.bytes_sent + stats.bytes_received;
        rate_formatter(total as f64 / elapsed)
    } else {
        rate_formatter(0.0)
    };
    println!(
        "\nsent {} bytes  received {} bytes  {}",
        progress_formatter(stats.bytes_sent, opts.human_readable),
        progress_formatter(stats.bytes_received, opts.human_readable),
        rate
    );
    if stats.bytes_transferred > 0 {
        let speedup = stats.total_file_size as f64 / stats.bytes_transferred as f64;
        println!(
            "total size is {}  speedup is {:.2}",
            progress_formatter(stats.total_file_size, opts.human_readable),
            speedup
        );
    } else {
        println!(
            "total size is {}  speedup is 0.00",
            progress_formatter(stats.total_file_size, opts.human_readable)
        );
    }
    tracing::info!(
        target: InfoFlag::Stats.target(),
        files_transferred = stats.files_transferred,
        files_deleted = stats.files_deleted,
        bytes = stats.bytes_transferred
    );
}
