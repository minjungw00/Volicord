import { renderReply } from "../src/client.js";

export function testRender(): void {
  if (renderReply({message: "Hello"}) !== "<p>Hello</p>") throw new Error("render failed");
}
