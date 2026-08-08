import { suffix } from "./suffix.js";

export class Greeter {
  constructor(prefix) {
    this.prefix = prefix;
  }

  greet(person) {
    return formatName(`${this.prefix}, ${person}${suffix}`);
  }
}

export function formatName(value) {
  return value.trim();
}
