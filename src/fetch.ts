import { Channel, invoke } from "@tauri-apps/api/core";

const ERROR_REQUEST_CANCELLED = "Request cancelled";

export { ERROR_REQUEST_CANCELLED };

export async function fetch(
  input: URL | Request | string,
  init?: RequestInit
): Promise<Response> {
  const signal = init?.signal;

  if (signal?.aborted) {
    throw new Error(ERROR_REQUEST_CANCELLED);
  }

  const headers = init?.headers
    ? init.headers instanceof Headers
      ? init.headers
      : new Headers(init.headers)
    : new Headers();

  const req = new Request(input, init);
  const buffer = await req.arrayBuffer();
  const data =
    buffer.byteLength !== 0 ? Array.from(new Uint8Array(buffer)) : null;

  for (const [key, value] of req.headers) {
    if (!headers.get(key)) headers.set(key, value);
  }

  const headersArray =
    headers instanceof Headers
      ? Array.from(headers.entries())
      : Array.isArray(headers)
      ? (headers as Array<[string, string]>)
      : Object.entries(headers as Record<string, string>);

  const mappedHeaders: Array<[string, string]> = headersArray.map(
    ([name, val]) => [
      name,
      typeof val === "string" ? val : (val as any).toString(),
    ]
  );

  if (signal?.aborted) {
    throw new Error(ERROR_REQUEST_CANCELLED);
  }

  const { rid, txid, rxid } = await invoke<{
    rid: number;
    txid: number;
    rxid: number;
  }>("plugin:axum|fetch", {
    conf: {
      method: req.method,
      uri: req.url,
      headers: mappedHeaders,
      body: data,
    },
  });

  const abort = () =>
    invoke("plugin:axum|fetch_cancel", { txid }).catch(() => {});
  if (signal?.aborted) {
    void abort();
    throw new Error(ERROR_REQUEST_CANCELLED);
  }
  signal?.addEventListener("abort", () => void abort());

  const {
    status,
    statusText,
    headers: responseHeaders,
    bodyid,
  } = await invoke<{
    status: number;
    statusText: string;
    headers: Array<[string, string]>;
    bodyid: number;
  }>("plugin:axum|fetch_send", { rid, rxid, txid });

  // No body for 101, 103, 204, 205 and 304
  const body = [101, 103, 204, 205, 304].includes(status)
    ? null
    : new ReadableStream<Uint8Array>({
        start: (controller) => {
          const streamChannel = new Channel<ArrayBuffer | number[]>();
          streamChannel.onmessage = (res: ArrayBuffer | number[]) => {
            if (signal?.aborted) {
              controller.error(ERROR_REQUEST_CANCELLED);
              return;
            }

            const resUint8 = new Uint8Array(res as ArrayBuffer);
            const lastByte = resUint8[resUint8.byteLength - 1];
            const actualRes = resUint8.slice(0, resUint8.byteLength - 1);

            if (lastByte === 1 || lastByte === 2) {
              controller.close();
              return;
            }

            controller.enqueue(actualRes);
          };

          // run a non-blocking body stream fetch
          // Send both keys to be safe with snake/camel case in Rust side
          invoke("plugin:axum|fetch_read_body", {
            bodyid,
            rxid,
            txid,
            streamChannel,
          }).catch((e) => controller.error(e));
        },
      });

  const res = new Response(body, { status, statusText });

  Object.defineProperty(res, "url", { value: req.url });
  Object.defineProperty(res, "headers", {
    value: new Headers(responseHeaders),
  });

  return res;
}
