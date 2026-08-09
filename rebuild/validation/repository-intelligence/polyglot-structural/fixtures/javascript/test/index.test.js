import { Greeter } from "../src/index.js";

export function testGreeting() {
  return new Greeter("hello").greet("Ada");
}
