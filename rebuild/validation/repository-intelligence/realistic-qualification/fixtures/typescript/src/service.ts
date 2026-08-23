import type { GreetingPort } from "./api.js";
import { normalize } from "./names.js";

export class GreetingService implements GreetingPort {
  render(name: string): string;
  render(name: string, repeat: number): string;
  render(name: string, repeat = 1): string {
    return normalize(name).repeat(repeat);
  }
}
