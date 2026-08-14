# Request flow

1. `Gateway.route` validates the incoming name.
2. The deployment passes JSON to the Python formatter process.
3. The TypeScript client reads the resulting `message` field.

The processes communicate through a deployment-owned JSON boundary. Source
analysis must not represent this documented relationship as a direct call.
