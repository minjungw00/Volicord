pub(crate) const DIAGNOSTIC_SUMMARY_GUARANTEE: &str =
    "Local diagnostic observation; not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.";

pub(crate) const USER_CHANNEL_SUMMARY_GUARANTEE: &str =
    "Local User Channel view; listing does not resolve a user action or prove close readiness.";

pub(crate) fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_state_text_distinguishes_empty_and_nonempty_collections() {
        assert_eq!(count_state_text("pending", 0), "none");
        assert_eq!(count_state_text("pending", 2), "pending (2)");
    }
}
