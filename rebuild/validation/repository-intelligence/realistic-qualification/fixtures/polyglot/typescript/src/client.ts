export interface GreetingRequest {
  name: string;
}

export const endpoint = "/v1/greet";
export const schema = "greeting.request.v1";

export async function send(request: GreetingRequest): Promise<Response> {
  return fetch(endpoint, {
    method: "POST",
    headers: { "x-schema": schema },
    body: JSON.stringify(request),
  });
}
