# volicord-privacy

Project-scoped privacy and optional background semantic-provider boundary.

The crate persists inspectable opt-in, revoke, transmission, retention, and
managed-deletion observations separately from Canonical Context and Session
Candidates. Provider dispatch consumes a request-time authorization token and
revalidates the current Project policy, leaving an exact dispatch boundary for
later Guarded-effect coordination. The durable request record is directly
addressable by Project and provider-request identity so a Guarded operation can
report its privacy manifest and provider outcome without persisting ephemeral
filtered source bodies.
