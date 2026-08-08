export interface Reply {
  message: string;
}

export function readReply(reply: Reply): string {
  return reply.message;
}
