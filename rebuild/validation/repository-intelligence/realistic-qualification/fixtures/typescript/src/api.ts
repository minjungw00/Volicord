export interface GreetingPort {
  render(name: string): string;
}

export type Identifier = string & { readonly brand: unique symbol };
