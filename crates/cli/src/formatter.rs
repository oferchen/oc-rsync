use clap::Command;

const HELP_TEMPLATE: &str =
    "{before-help}{about-with-newline}Usage: {usage}\n\nOptions\n{all-args}{after-help}";

pub fn apply(cmd: Command) -> Command {
    cmd.term_width(80).help_template(HELP_TEMPLATE)
}
