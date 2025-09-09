// crates/logging/tests/subscriber_builder.rs

use logging::{DebugFlag, InfoFlag, LogFormat, StderrMode, SubscriberConfig};
use std::path::PathBuf;

#[test]
fn builder_sets_fields() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Json)
        .verbose(2)
        .info([InfoFlag::Backup])
        .debug([DebugFlag::Acl])
        .quiet(true)
        .stderr(StderrMode::All)
        .log_file(Some((PathBuf::from("log"), Some("json".into()))))
        .syslog(true)
        .journald(true)
        .colored(false)
        .timestamps(true)
        .build();

    assert_eq!(cfg.format, LogFormat::Json);
    assert_eq!(cfg.verbose, 2);
    assert_eq!(cfg.info, vec![InfoFlag::Backup]);
    assert_eq!(cfg.debug, vec![DebugFlag::Acl]);
    assert!(cfg.quiet);
    assert_eq!(cfg.stderr, StderrMode::All);
    assert!(cfg.log_file.is_some());
    assert!(cfg.syslog);
    assert!(cfg.journald);
    assert!(!cfg.colored);
    assert!(cfg.timestamps);
}
