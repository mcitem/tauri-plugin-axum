import { axum, AxumResponse } from "@mcitem/tauri-plugin-axum";
import { invoke } from "@tauri-apps/api/core";

window.addEventListener("DOMContentLoaded", () => {
  const getButton = document.querySelector<HTMLButtonElement>("#get")!;
  const postButton = document.querySelector<HTMLButtonElement>("#post")!;
  const customButton = document.querySelector<HTMLButtonElement>("#custom")!;
  getButton.addEventListener("click", () => {
    axum.get<string>("/").then((response) => {
      console.log(response);
      document.getElementById("greet-msg")!.innerText = response.body;
    });
  });

  postButton.addEventListener("click", () => {
    axum
      .post<{ axum: string; tauri: string }>("/post", {
        axum: "hello axum",
        tauri: "hello tauri",
      })
      .then((response) => {
        console.log(response);
        document.getElementById("greet-msg")!.innerText = JSON.stringify(
          response.body,
          null,
          2
        );
      });
  });

  customButton.addEventListener("click", () => {
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
        response.body = new TextDecoder().decode(new Uint8Array(response.body));
        document.getElementById("greet-msg")!.innerText = JSON.stringify(
          response,
          null,
          2
        );
      })
      .catch((error) => {
        console.error(error);
      });
  });
});
