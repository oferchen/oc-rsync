use crate::rule::RuleFlags;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PerDir {
    pub(crate) file: String,
    pub(crate) anchored: bool,
    pub(crate) root_only: bool,
    pub(crate) inherit: bool,
    pub(crate) cvs: bool,
    pub(crate) word_split: bool,
    pub(crate) sign: Option<char>,
    pub(crate) flags: RuleFlags,
}
