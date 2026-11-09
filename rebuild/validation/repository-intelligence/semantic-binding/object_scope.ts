// Independent scope oracle: object keys and named expressions are not outer bindings.
function orphan() {
  const object = { run() {} };
  run();
}
class Outer {
  run() { return "outer"; }
  make() {
    return { run() { return "object"; }, nested() { return this.run(); } };
  }
}
function real(value: number) { return "outer function"; }
function expression() {
  return function real(depth: number) { if (depth === 0) return real(1); return "inner function"; };
}
class Named { static value() { return "outer class"; } }
function classExpression() {
  return class Named { static classNested() { return Named.value(); } static value() { return "inner class"; } };
}
let absent = false;
try { orphan(); } catch (error) { absent = error instanceof ReferenceError; }
if (!absent) throw new Error("an object method became a lexical binding");
if (new Outer().make().nested() !== "object") throw new Error("wrong this owner");
const inner = expression();
if (inner(0) !== "inner function") throw new Error("named expression lost its own binding");
if (classExpression().classNested() !== "inner class") throw new Error("named class expression lost its own binding");
