declare namespace Protocol {
  interface Envelope<T> { payload: T }
  interface Envelope<T> { traceId?: string }
  type Handler<T extends { id: string }> = (value: Readonly<T>) => Promise<T>;
}
