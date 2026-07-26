use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidationIssue {
    path: String,
    category: &'static str,
    line: Option<usize>,
    message: String,
}

impl ValidationIssue {
    pub(crate) fn new(
        path: impl Into<String>,
        category: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::at_line(path, category, None, message)
    }

    pub(crate) fn at_line(
        path: impl Into<String>,
        category: &'static str,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            category,
            line,
            message: message.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn category(&self) -> &'static str {
        self.category
    }

    pub fn line(&self) -> Option<usize> {
        self.line
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Ord for ValidationIssue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.path, self.line, self.category, &self.message).cmp(&(
            &other.path,
            other.line,
            other.category,
            &other.message,
        ))
    }
}

impl PartialOrd for ValidationIssue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(
                formatter,
                "{}:{line}: {}: {}",
                self.path, self.category, self.message
            ),
            None => write!(
                formatter,
                "{}: {}: {}",
                self.path, self.category, self.message
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    pub(crate) issues: Vec<ValidationIssue>,
}

impl CheckReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }
}
