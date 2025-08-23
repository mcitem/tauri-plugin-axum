# tauri-plugin-axum

[![Crates.io Version](https://img.shields.io/crates/v/tauri-plugin-axum)](https://crates.io/crates/tauri-plugin-axum)
[![NPM Version](https://img.shields.io/npm/v/@mcitem/tauri-plugin-axum)](https://www.npmjs.com/package/@mcitem/tauri-plugin-axum)

# Feature

- [Axios](https://github.com/axios/axios) Adapter

# Install

```bash
cargo add tauri-plugin-axum
```

```bash
pnpm i @mcitem/tauri-plugin-axum
```

```json
// src-tauri/capabilities/default.json
{
  // ..
  "permissions": ["axum:default"]
  // ..
}
```

# Example

```rust,no_run
// https://docs.rs/tauri/2.8.2/tauri/plugin/struct.Builder.html#known-limitations
// Known limitations
// URI scheme protocols are registered when the webview is created.
// Due to this limitation, if the plugin is registered after a webview has been created,
// protocol won’t be available.
// macOS, iOS and Linux: axum://localhost/<path>
// Windows and Android: http://axum.localhost/<path> by default
// if use protocol, register it before setup hook
tauri::Builder::default().plugin(tauri_plugin_axum::init(Router::new()))

window.fetch("http://axum.localhost/")
```

```rust,no_run
#[derive(Serialize, Deserialize)]
struct Greet {
    axum: String,
    tauri: String,
}

async fn post_handle(Json(j): Json<Greet>) -> Json<Greet> {
    Json(Greet {
        axum: format!("axum, {}!", j.axum),
        tauri: format!("tauri, {}!", j.tauri),
    })
}

// async init router
tauri::Builder::default()
    .setup(|app| {
        let path = app.path().app_config_dir()?;
        app.handle().plugin(tauri_plugin_axum::block_init(async {
            println!("do something with path: {:?}", path);
            router::router()
        }))?;
        Ok(())
    })
```

```typescript
import { axum } from "@mcitem/tauri-plugin-axum";
axum.get<string>("/").then((response) => {
  console.log(response.body); // "Hello, World!"
});
axum.post<{ axum: string; tauri: string }>("/post", {
  axum: "hello axum",
  tauri: "hello tauri",
});

import { fetch } from "@mcitem/tauri-plugin-axum/fetch";
fetch("/", { method: "GET" })
  .then((res) => res.text())
  .then((res) => {
    console.log(res);
  });

import axios from "axios";
import { Adapter } from "@mcitem/tauri-plugin-axum/axios";
const instance = axios.create({
  adapter: Adapter,
});
```

- [Example](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app)

  [main.tsx](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app/src/main.tsx)

  [lib.rs](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app/src-tauri/src/lib.rs)

## Run Example

```sh
git clone https://github.com/mcitem/tauri-plugin-axum
cd tauri-plugin-axum
pnpm install
pnpm build
pnpm --filter tauri-app install
pnpm --filter tauri-app tauri dev
```
