pub mod filters {
    use globset::{Glob, GlobMatcher};
    use std::path::Path;

    /// A single include or exclude rule compiled into a glob matcher.
    #[derive(Clone)]
    pub enum Rule {
        Include(GlobMatcher),
        Exclude(GlobMatcher),
    }

    impl Rule {
        fn matches<P: AsRef<Path>>(&self, path: P) -> bool {
            match self {
                Rule::Include(m) | Rule::Exclude(m) => m.is_match(path),
            }
        }

        fn is_include(&self) -> bool {
            matches!(self, Rule::Include(_))
        }
    }

    /// Matcher evaluates rules sequentially against paths.
    #[derive(Clone, Default)]
    pub struct Matcher {
        rules: Vec<Rule>,
    }

    impl Matcher {
        pub fn new(rules: Vec<Rule>) -> Self {
            Self { rules }
        }

        /// Determine if the provided path is included by the rules.
        pub fn is_included<P: AsRef<Path>>(&self, path: P) -> bool {
            let path = path.as_ref();
            for rule in &self.rules {
                if rule.matches(path) {
                    return rule.is_include();
                }
            }
            true
        }

        /// Merge additional rules, as when reading a per-directory `.rsync-filter`.
        pub fn merge(&mut self, more: Vec<Rule>) {
            self.rules.extend(more);
        }
    }

    #[derive(Debug)]
    pub enum ParseError {
        InvalidRule(String),
        Glob(globset::Error),
    }

    impl From<globset::Error> for ParseError {
        fn from(e: globset::Error) -> Self {
            Self::Glob(e)
        }
    }

    enum RuleKind {
        Include,
        Exclude,
    }

    /// Parse filter rules from input.
    pub fn parse(input: &str) -> Result<Vec<Rule>, ParseError> {
        let mut rules = Vec::new();

        for raw_line in input.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (kind, pattern) = match line.chars().next() {
                Some('+') => (RuleKind::Include, line[1..].trim_start()),
                Some('-') => (RuleKind::Exclude, line[1..].trim_start()),
                _ => return Err(ParseError::InvalidRule(raw_line.to_string())),
            };

            if pattern.is_empty() {
                return Err(ParseError::InvalidRule(raw_line.to_string()));
            }

            let matcher = Glob::new(pattern)?.compile_matcher();

            match kind {
                RuleKind::Include => rules.push(Rule::Include(matcher)),
                RuleKind::Exclude => rules.push(Rule::Exclude(matcher)),
            }
        }

        Ok(rules)
    }
}
