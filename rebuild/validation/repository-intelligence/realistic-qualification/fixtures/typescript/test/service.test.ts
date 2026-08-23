import { GreetingService } from "../src/index.js";

export function testGreeting(): boolean {
  return new GreetingService().render(" Ada ") === "Ada";
}
