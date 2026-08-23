function registered<T extends new (...args: never[]) => object>(value: T): T {
  return value;
}

@registered
export class Loader {
  async load(name: string): Promise<unknown> {
    return import(`./plugins/${name}.js`);
  }
}
