import { invoke } from "@tauri-apps/api/core";

const ERROR_REQUEST_CANCELLED = "Request cancelled";

export { ERROR_REQUEST_CANCELLED };

export async function fetch(
  input: URL | Request | string,
  init?: RequestInit,
): Promise<Response> {
  // Optimistically check for abort signal and avoid doing any work
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

  // append new headers created by the browser `Request` implementation,
  // if not already declared by the caller of this function
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

      // we need to ensure we have all header values as strings
      typeof val === "string" ? val : (val as any).toString(),
    ],
  );

  // Optimistically check for abort signal and avoid doing any work on the Rust side
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

  const abort = () => invoke("plugin:axum|fetch_cancel", { txid });

  // Optimistically check for abort signal
  // and avoid doing any work after doing intial work on the Rust side
  if (signal?.aborted) {
    // we don't care about the result of this proimse
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

  const dropBody = () => invoke("plugin:axum|fetch_cancel_body", { bodyid });

  const readChunk = async (
    controller: ReadableStreamDefaultController<Uint8Array>,
  ) => {
    let data: ArrayBuffer;
    try {
      data = await invoke("plugin:axum|fetch_read_body", {
        bodyid,
      });
    } catch (e) {
      // close the stream if an error occurs
      // and drop the body on Rust side
      controller.error(e);
      void dropBody();
      return;
    }

    const dataUint8 = new Uint8Array(data);
    const lastByte = dataUint8[dataUint8.byteLength - 1];
    const actualData = dataUint8.slice(0, dataUint8.byteLength - 1);

    // close when the signal to close (last byte is 1) is sent from the IPC.
    if (lastByte === 1) {
      controller.close();
      return;
    }

    controller.enqueue(actualData);
  };

  // No body for 101, 103, 204, 205 and 304
  const body = [101, 103, 204, 205, 304].includes(status)
    ? null
    : new ReadableStream<Uint8Array>({
        start: (controller) => {
          signal?.addEventListener("abort", () => {
            controller.error(ERROR_REQUEST_CANCELLED);
            void dropBody();
          });
        },
        pull: (controller) => readChunk(controller),
        cancel: () => {
          // Ensure body resources are released on stream cancellation
          void dropBody();
        },
      });

  const res = new Response(body, { status, statusText });

  // `Response.url` cannot be set via the constructor, so we define it manually
  Object.defineProperty(res, "url", { value: req.url, writable: false });

  // Expose `set-cookie` via `response.headers` (and `getSetCookie()` where
  // supported). This is not Fetch-spec compliant for network responses in
  // browsers, where `set-cookie` is treated as a forbidden response
  // header and is generally not readable from JavaScript.
  Object.defineProperty(res, "headers", {
    value: new Headers(responseHeaders),
    writable: false,
  });

  // Patch clone() per-instance so cloning preserves the overridden properties
  const originalClone = res.clone.bind(res);
  Object.defineProperty(res, "clone", {
    value: () => {
      const cloned = originalClone();
      Object.defineProperty(cloned, "url", { value: req.url, writable: false });
      Object.defineProperty(cloned, "headers", {
        value: new Headers(responseHeaders),
        writable: false,
      });
      return cloned;
    },
  });

  return res;
}
