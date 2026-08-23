export { Parser } from "./core/parse.js";
export * from "./other/parse.js";

export async function loadFormatter(name) {
  return import(`./formatters/${name}.js`);
}
