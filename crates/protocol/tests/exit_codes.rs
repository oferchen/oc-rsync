use protocol::ExitCode;
use std::convert::TryFrom;

#[test]
fn exit_code_roundtrip() {
    let codes = [
        (0u8, ExitCode::Ok),
        (1, ExitCode::SyntaxOrUsage),
        (2, ExitCode::Protocol),
        (23, ExitCode::Partial),
        (127, ExitCode::CmdNotFound),
    ];
    for (num, code) in codes {
        assert_eq!(ExitCode::try_from(num).unwrap(), code);
        let back: u8 = code.into();
        assert_eq!(back, num);
    }
}

#[test]
fn unknown_exit_code_errors() {
    assert!(ExitCode::try_from(99u8).is_err());
}
