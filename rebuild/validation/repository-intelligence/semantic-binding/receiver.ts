// Independent runtime oracle: demo uses the host console, never the local log.
let localCalls = 0;
function log(value: string): void {
  localCalls += value.length;
}
function demo(): void {
  console.log("host");
}
function local(): void {
  log("local");
}
demo();
if (localCalls !== 0) throw new Error("console.log called the local declaration");
local();
if (Number(localCalls) !== 5) throw new Error("local call lost its binding");
