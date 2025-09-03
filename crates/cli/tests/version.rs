// crates/cli/tests/version.rs
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
    expected.extend(
        include_str!("fixtures/oc-rsync-version-tail.txt")
            .lines()
            .map(|l| l.to_string()),
    );
    assert_eq!(version::render_version_lines(), expected);
}

#[test]
fn banner_matches_rsync() {
    let upstream: Vec<_> = include_str!("fixtures/rsync-version.txt")
        .lines()
        .skip(3)
        .take_while(|l| !l.is_empty())
        .collect();
    let ours = version::render_version_lines();
    assert_eq!(&ours[5..5 + upstream.len()], upstream);
}

#[test]
fn banner_renders_correctly() {
    let expected = format!("{}\n", version::render_version_lines().join("\n"));
    assert_eq!(version::version_banner(), expected);
}
