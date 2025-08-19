import { fetch, ERROR_REQUEST_CANCELLED } from "./fetch";
// import buildFullPath from "axios/lib/core/buildFullPath";
// import buildURL from "axios/lib/helpers/buildURL";
import buildFullPath from "axios/unsafe/core/buildFullPath";
import buildURL from "axios/unsafe/helpers/buildURL";
import { AxiosError } from "axios";

const headersToObject = (headers) => {
  try {
    if (!headers) return {};
    if (typeof headers.toJSON === "function") return headers.toJSON();
    if (headers instanceof Headers) {
      return Object.fromEntries(headers.entries());
    }
    return { ...headers };
  } catch {
    return {};
  }
};

const transformBody = (readable, type) => {
  switch ((type || "json").toLowerCase()) {
    case "stream":
      return Promise.resolve(readable);
    case "arraybuffer":
      return new Response(readable).arrayBuffer();
    case "blob":
      return new Response(readable).blob();
    case "formdata":
      return new Response(readable).formData();
    case "json":
      return new Response(readable).json();
    case "text":
      return new Response(readable).text();
    default:
      return Promise.reject(
        new Error("Response type unsupported: " + String(type))
      );
  }
};

const inferResponseType = (headers) => {
  const h = headersToObject(headers);
  const ct = (h["content-type"] || h["Content-Type"] || "")
    .toString()
    .toLowerCase();
  if (ct.includes("application/json")) return "json";
  if (ct.startsWith("text/")) return "text";
  if (ct.includes("multipart/form-data")) return "formdata";
  if (ct.includes("application/octet-stream")) return "arraybuffer";
  if (
    ct.includes("application/blob") ||
    ct.includes("image/") ||
    ct.includes("video/")
  )
    return "blob";
  return "text";
};

/**
 * @type {import("axios").AxiosAdapter}
 */
function Adapter(config) {
  return new Promise((resolve, reject) => {
    const url = buildURL(
      buildFullPath(config.baseURL, config.url),
      config.params,
      config.paramsSerializer
    );

    const baseHeaders = headersToObject(config.headers);

    let controller;
    let signal = config.signal;
    if (config.cancelToken && typeof AbortController !== "undefined") {
      controller = new AbortController();
      signal = signal || controller.signal;
      config.cancelToken.promise.then(() => controller?.abort());
    }

    /**
     * @type {number | undefined}
     */
    let timeoutId;
    if (typeof config.timeout === "number" && config.timeout > 0) {
      if (!controller && typeof AbortController !== "undefined") {
        controller = new AbortController();
        signal = signal || controller.signal;
      }
      timeoutId = setTimeout(() => controller?.abort(), config.timeout);
    }

    const requestInit = {
      method: (config.method || "get").toUpperCase(),
      headers: baseHeaders,
      body: config.data,
      signal,
    };

    fetch(url, requestInit)
      .then((response) => {
        if (timeoutId) clearTimeout(timeoutId);
        const responseType =
          config.responseType || inferResponseType(response.headers);

        const finalize = (data) => {
          resolve({
            data,
            status: response.status,
            statusText: response.statusText,
            headers: headersToObject(response.headers),
            config,
            request: undefined,
          });
        };

        if (response.body != null) {
          transformBody(response.body, responseType)
            .then(finalize)
            .catch((reason) => {
              reject(
                new AxiosError(
                  String(reason?.message || reason),
                  AxiosError.ERR_BAD_RESPONSE,
                  config,
                  { url, ...requestInit }
                )
              );
            });
        } else {
          finalize(null);
        }
      })
      .catch((err) => {
        if (timeoutId) clearTimeout(timeoutId);
        const message = String(err?.message || err);
        const code =
          message === ERROR_REQUEST_CANCELLED
            ? AxiosError.ERR_CANCELED
            : AxiosError.ERR_BAD_REQUEST;
        reject(new AxiosError(message, code, config));
      });
  });
}

export { Adapter };
