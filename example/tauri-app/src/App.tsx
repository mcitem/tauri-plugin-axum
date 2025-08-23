import "./App.css";

import axios from "axios";
import { Adapter } from "@mcitem/tauri-plugin-axum/axios";
import { fetch as AxumFetch } from "@mcitem/tauri-plugin-axum/fetch";
import { invoke } from "@tauri-apps/api/core";
import { axum, AxumResponse, call_json } from "@mcitem/tauri-plugin-axum";
import { useState } from "react";

const instance = axios.create({
  adapter: Adapter,
});

function App() {
  const [state, setState] = useState("");
  const [controller, setController] = useState<AbortController | undefined>();

  return (
    <main className="container">
      <div>
        <button
          onClick={() => {
            fetch("http://axum.localhost/")
              .then((res) => res.text())
              .then((res) => {
                setState(res);
              })
              .catch(() => {
                fetch("axum://localhost/")
                  .then((res) => res.text())
                  .then((res) => {
                    setState(res);
                  });
              });
          }}
        >
          register_uri_scheme_protocol
        </button>
      </div>
      <button
        onClick={() => [
          axum.get<string>("/").then((response) => {
            console.log(response);
            setState(response.body);
          }),
        ]}
      >
        Hello World!
      </button>
      <button
        onClick={() => {
          axum
            .post<{ axum: string; tauri: string }>("/post", {
              axum: "hello axum",
              tauri: "hello tauri",
            })
            .then((response) => {
              console.log(response);
              setState(JSON.stringify(response.body, null, 2));
            });
        }}
      >
        HTTP POST
      </button>
      <button
        onClick={() => {
          invoke<AxumResponse>(
            "custom_usage",
            {},
            {
              headers: {
                "x-uri": "/",
                "x-method": "GET",
              },
            }
          )
            .then((response) => {
              console.log(response);
              response.body = new TextDecoder().decode(
                new Uint8Array(response.body)
              );
              setState(JSON.stringify(response, null, 2));
            })
            .catch((error) => {
              console.error(error);
            });
        }}
      >
        Custom Usage
      </button>
      <button
        onClick={() => {
          instance.get("/").then((response) => {
            console.log(response);
            setState(JSON.stringify(response, null, 2));
          });
        }}
      >
        axios
      </button>
      <button
        onClick={() => {
          AxumFetch("/", { method: "GET" })
            .then((res) => res.text())
            .then((res) => {
              setState(res);
            });
        }}
      >
        fetch
      </button>
      <button
        onClick={() => {
          controller?.abort();
          const newController = new AbortController();
          setController(newController);

          AxumFetch("/stream-body", {
            method: "GET",
            signal: newController.signal,
          })
            .then((res) => {
              const reader = res.body!.getReader();
              const decoder = new TextDecoder();
              setState("");

              const readStream = (): Promise<void> => {
                return reader.read().then(({ value, done }) => {
                  if (done) return Promise.resolve();
                  const chunk = decoder.decode(value, { stream: true });
                  setState((prev) => prev + chunk);
                  return readStream();
                });
              };

              return readStream();
            })
            .catch((error) => {
              console.log("Stream fetch cancelled or failed:", error);
            });
        }}
      >
        stream fetch
      </button>
      <button
        onClick={() => {
          controller?.abort();
          const newController = new AbortController();
          setController(newController);

          instance
            .get<ReadableStream<Uint8Array>>("/stream-body", {
              responseType: "stream",
              signal: newController.signal,
            })
            .then((resp) => {
              const stream = resp.data;
              const reader = stream.getReader();
              const decoder = new TextDecoder();
              setState("");

              const readStream = (): Promise<void> => {
                return reader.read().then(({ value, done }) => {
                  console.log(value, typeof value, value instanceof Uint8Array);
                  if (done) return Promise.resolve();
                  const chunk = decoder.decode(value, { stream: true });
                  setState((prev) => prev + chunk);
                  return readStream();
                });
              };

              return readStream();
            })
            .catch((error) => {
              console.log("Stream axios cancelled or failed:", error);
            });
        }}
      >
        stream axios
      </button>
      <button
        onClick={() => {
          console.log("Abort controller:", controller);
          controller?.abort();
          setController(undefined);
        }}
      >
        stream abort
      </button>
      <button
        onClick={() => {
          call_json<{ axum: string; tauri: string }>("POST", "/post", {
            axum: "hello axum",
            tauri: "hello tauri",
          }).then((response) => {
            console.log(response);
            setState(JSON.stringify(response, null, 2));
          });
        }}
      >
        call just json body
      </button>

      <div style={{ marginTop: "5px" }}></div>
      <div>
        <div id="greet-msg">{state}</div>
      </div>
    </main>
  );
}

export default App;
