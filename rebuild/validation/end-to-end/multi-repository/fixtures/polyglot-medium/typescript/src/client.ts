export interface Reply {
  message: string;
}

export function renderReply(reply: Reply): string {
  return `<p>${reply.message}</p>`;
}
