import { suffix } from "./suffix.js";

export interface Named {
  name(): string;
}

export type Identifier = string;

export enum Mode {
  Loud,
  Quiet,
}

export class Greeter implements Named {
  constructor(private readonly prefix: string) {}

  name(): string {
    return this.prefix;
  }

  greet(person: Identifier): string {
    return formatName(`${this.name()}, ${person}${suffix}`);
  }
}

export function formatName(value: string): string {
  return value.trim();
}
