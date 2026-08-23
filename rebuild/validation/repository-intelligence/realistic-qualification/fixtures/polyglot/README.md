# Greeting relay

The Java gateway accepts `POST /v1/greet` using the shared
`greeting.request.v1` JSON contract. It places a request document in the inbox;
the Python worker formats that document, and the TypeScript client consumes the
same endpoint and schema identifier. `system.json` is the deployment-neutral
component boundary manifest.
