// crates/cli/tests/version.rs
use assert_cmd::Command;
use oc_rsync_cli::version;
use protocol::SUPPORTED_PROTOCOLS;

#[test]
fn banner_is_static() {
    let mut expected = vec![
        format!(
            "oc-rsync {} (protocol {})",
            env!("CARGO_PKG_VERSION"),
            SUPPORTED_PROTOCOLS[0],
        ),
        {
            let proto = option_env!("UPSTREAM_PROTOCOLS")
                .unwrap_or("32,31,30,29")
                .split(',')
                .next()
                .unwrap_or("0");
            format!(
                "compatible with rsync {} (protocol {proto})",
                option_env!("UPSTREAM_VERSION").unwrap_or("unknown"),
            )
        },
        format!(
            "{} {}",
            option_env!("BUILD_REVISION").unwrap_or("unknown"),
            option_env!("OFFICIAL_BUILD").unwrap_or("unofficial"),
        ),
    ];
    let year = option_env!("CURRENT_YEAR").unwrap_or("2025");
    expected.push(format!("Copyright (C) 2024-{year} oc-rsync contributors."));
    let tail = include_str!("fixtures/oc-rsync-version-tail.txt");
    expected.extend(tail.lines().map(|l| l.to_string()));
    assert_eq!(version::render_version_lines(), expected);
}

#[test]
fn banner_renders_correctly() {
    let expected = format!("{}\n", version::render_version_lines().join("\n"));
    assert_eq!(version::version_banner(), expected);
}

#[test]
fn cli_version_uses_banner() {
    let expected = version::version_banner();
    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}
