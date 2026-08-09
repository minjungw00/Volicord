import { Greeter } from "../src/index.js";

export function testGreeting(): string {
  return new Greeter("hello").greet("Ada");
}
