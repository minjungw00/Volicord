use super::*;

#[test]
fn close_state_selection_covers_every_current_intent() {
    assert_eq!(
        close_state_for_policy(CloseIntent::Check, true),
        CloseState::Ready
    );
    assert_eq!(
        close_state_for_policy(CloseIntent::Complete, true),
        CloseState::Closed
    );
    assert_eq!(
        close_state_for_policy(CloseIntent::Cancel, true),
        CloseState::Cancelled
    );
    assert_eq!(
        close_state_for_policy(CloseIntent::Supersede, true),
        CloseState::Superseded
    );
    for intent in [
        CloseIntent::Check,
        CloseIntent::Complete,
        CloseIntent::Cancel,
        CloseIntent::Supersede,
    ] {
        assert_eq!(close_state_for_policy(intent, false), CloseState::Blocked);
    }
}
