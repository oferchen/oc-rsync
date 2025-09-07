// crates/filters/tests/rooted_and_parents.rs
use filters::rooted_and_parents;

#[test]
fn dir_prefix_double_star_and_class() {
    let (rooted, parents) = rooted_and_parents("dir*/**/keep[0-9].txt");
    assert_eq!(rooted, "dir*/**/keep[0-9].txt");
    assert_eq!(parents, vec!["dir*/".to_string()]);
}

#[test]
fn leading_double_star_with_question_mark() {
    let (rooted, parents) = rooted_and_parents("**/file?.log");
    assert_eq!(rooted, "**/file?.log");
    assert!(parents.is_empty());
}

#[test]
fn trailing_double_star() {
    let (rooted, parents) = rooted_and_parents("dir/sub/**");
    assert_eq!(rooted, "dir/sub/**");
    assert_eq!(parents, vec!["dir/".to_string(), "dir/sub/".to_string()]);
}
