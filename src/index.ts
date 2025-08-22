import { invoke, InvokeArgs } from "@tauri-apps/api/core";

export type Method =
  | "OPTIONS"
  | "GET"
  | "POST"
  | "PUT"
  | "DELETE"
  | "HEAD"
  | "TRACE"
  | "CONNECT"
  | "PATCH";

export interface AxumResponse<T = any> {
  status: number;
  headers: Record<string, string>;
  body: T;
}

export interface AxumClientOptions {
  bodyParser?: (body: any) => any;
  defaultHeaders?: Record<string, string>;
}

export const textdecoder = new TextDecoder();
export const textencoder = new TextEncoder();

export class AxumClient {
  private bodyParser: (body: any) => any = (body) => {
    if (Array.isArray(body)) {
      const text = textdecoder.decode(new Uint8Array(body));
      try {
        return JSON.parse(text);
      } catch {
        return text;
      }
    }
    return body;
  };
  private defaultHeaders: Record<string, string>;

  constructor(options: AxumClientOptions = {}) {
    if (options.bodyParser) {
      this.bodyParser = options.bodyParser;
    }
    this.defaultHeaders = options.defaultHeaders || {};
  }

  setBodyParser(fn: (body: any) => any) {
    this.bodyParser = fn;
  }

  setDefaultHeaders(headers: Record<string, string>) {
    this.defaultHeaders = headers;
  }

  async request<T>(
    method: Method,
    uri: string,
    args: InvokeArgs = {},
    bodyParser?: (body: any) => any
  ): Promise<AxumResponse<T>> {
    const res = await invoke<AxumResponse<any>>("plugin:axum|call", args, {
      headers: {
        ...this.defaultHeaders,
        "x-method": method,
        "x-uri": uri,
      },
    });

    if (bodyParser) {
      res.body = bodyParser(res.body);
    } else {
      res.body = this.bodyParser(res.body);
    }

    return res as AxumResponse<T>;
  }

  get<T>(uri: string, args: InvokeArgs = {}, bodyParser?: (body: any) => any) {
    return this.request<T>("GET", uri, args, bodyParser);
  }

  post<T>(uri: string, args: InvokeArgs = {}, bodyParser?: (body: any) => any) {
    return this.request<T>("POST", uri, args, bodyParser);
  }
}

export const axum = new AxumClient({});

/// If you don't care about statuscode and headers and only use JSON for communication in the axum body,
/// you can simply use the call_json method for requests.
export const call_json = async <T>(method: Method, uri: string, body: any) => {
  return JSON.parse(
    textdecoder.decode(
      new Uint8Array(
        await invoke<number[]>(
          "plugin:axum|call_json",
          textencoder.encode(JSON.stringify(body)),
          {
            headers: {
              "x-method": method,
              "x-uri": uri,
            },
          }
        )
      )
    )
  ) as T;
};
