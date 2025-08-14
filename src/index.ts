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

export class AxumClient {
  private textdecoder = new TextDecoder();
  private bodyParser: (body: any) => any = (body) => {
    if (Array.isArray(body)) {
      const text = this.textdecoder.decode(new Uint8Array(body));
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
