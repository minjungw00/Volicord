import { Parser } from "../src/index.js";

export function testParser() {
  return new Parser().parse(" Ada ") === "Ada";
}
