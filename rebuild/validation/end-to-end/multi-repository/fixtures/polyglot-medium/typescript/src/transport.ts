import type { Reply } from "./client.js";

export function decodeReply(value: string): Reply {
  return JSON.parse(value) as Reply;
}
