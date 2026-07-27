use super::*;
use crate::methods::close_readiness::test_support;

#[test]
fn sensitive_control_requires_approval_and_a_ticket_backed_basis() {
    let mut facts = test_support::facts();
    assert!(!sensitive_approval_required(&facts).expect("valid light control"));
    assert!(!sensitive_action_basis_missing(&facts).expect("valid light control"));

    facts.task.effective_control_level = "sensitive".to_owned();
    assert!(sensitive_approval_required(&facts).expect("valid sensitive control"));
    assert!(sensitive_action_basis_missing(&facts).expect("valid sensitive control"));
}
