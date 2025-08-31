// bin/oc-rsync/src/main.rs
use engine::Result;
use logging::LogFormat;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut verbose = 0u8;
    let mut info = false;
    let mut debug = false;
    let mut log_format = LogFormat::Text;
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--info" || arg.starts_with("--info=") {
            info = true;
        } else if arg == "--debug" || arg.starts_with("--debug=") {
            debug = true;
        } else if arg == "--log-format" {
            if let Some(next) = iter.peek() {
                if next.as_str() == "json" {
                    log_format = LogFormat::Json;
                }
            }
        } else if let Some(f) = arg.strip_prefix("--log-format=") {
            if f == "json" {
                log_format = LogFormat::Json;
            }
        } else if arg == "--verbose" {
            verbose += 1;
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            for ch in arg.chars().skip(1) {
                if ch == 'v' {
                    verbose += 1;
                }
            }
        }
    }
    logging::init(log_format, verbose, info, debug);
    oc_rsync_cli::run()
}
