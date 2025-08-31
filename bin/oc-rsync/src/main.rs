// bin/oc-rsync/src/main.rs
use engine::Result;
use logging::LogFormat;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut verbose = 0u8;
    let mut info = false;
    let mut debug = false;
    for arg in &args {
        if arg == "--info" || arg.starts_with("--info=") {
            info = true;
        } else if arg == "--debug" || arg.starts_with("--debug=") {
            debug = true;
        } else if arg.starts_with('-') && !arg.starts_with("--") {
            for ch in arg.chars().skip(1) {
                if ch == 'v' {
                    verbose += 1;
                }
            }
        }
    }
    logging::init(LogFormat::Text, verbose, info, debug);
    oc_rsync_cli::run()
}
