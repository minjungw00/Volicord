use std::ffi::OsString;

const UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExplicitGitMetadata {
    pub(crate) commit: String,
    pub(crate) dirty: bool,
}

pub(crate) fn parse_explicit_git_metadata(
    commit: Option<OsString>,
    dirty: Option<OsString>,
) -> Result<Option<ExplicitGitMetadata>, String> {
    match (commit, dirty) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(
            "VOLICORD_BUILD_GIT_COMMIT and VOLICORD_BUILD_GIT_DIRTY must be set together"
                .to_owned(),
        ),
        (Some(commit), Some(dirty)) => {
            let commit = commit.into_string().map_err(|_| {
                "VOLICORD_BUILD_GIT_COMMIT must be a Unicode 40- or 64-digit hexadecimal SHA"
                    .to_owned()
            })?;
            let commit = normalized_git_commit(&commit).ok_or_else(|| {
                "VOLICORD_BUILD_GIT_COMMIT must be a 40- or 64-digit hexadecimal SHA".to_owned()
            })?;
            let dirty = dirty
                .into_string()
                .map_err(|_| "VOLICORD_BUILD_GIT_DIRTY must be exactly true or false".to_owned())?;
            let dirty = normalized_dirty_state(&dirty).ok_or_else(|| {
                "VOLICORD_BUILD_GIT_DIRTY must be exactly true or false".to_owned()
            })?;
            Ok(Some(ExplicitGitMetadata { commit, dirty }))
        }
    }
}

pub(crate) fn parse_explicit_profile(value: Option<OsString>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| "VOLICORD_BUILD_PROFILE must be a Unicode Cargo profile name".to_owned())?;
    if value == UNKNOWN
        || value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
    {
        return Err(
            "VOLICORD_BUILD_PROFILE must contain 1-64 lowercase ASCII letters, digits, '_' or '-'"
                .to_owned(),
        );
    }
    Ok(Some(value))
}

pub(crate) fn normalized_git_commit(value: &str) -> Option<String> {
    let value = value.to_ascii_lowercase();
    matches!(value.len(), 40 | 64)
        .then_some(value)
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn normalized_dirty_state(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}
