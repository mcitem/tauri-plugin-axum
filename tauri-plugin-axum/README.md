# tauri-plugin-axum

[![Crates.io Version](https://img.shields.io/crates/v/tauri-plugin-axum)](https://crates.io/crates/tauri-plugin-axum)
[![NPM Version](https://img.shields.io/npm/v/@mcitem/tauri-plugin-axum)](https://www.npmjs.com/package/@mcitem/tauri-plugin-axum)

A Tauri plugin that integrates the [Axum](https://github.com/tokio-rs/axum) web framework directly into your Tauri application.
It provides a convenient way to expose APIs via custom protocols or through an HTTP-like interface inside the Tauri WebView.

---

## ✨ Features

- **Custom protocol registration**

  ```rust,no_run
  // Important: if you use a custom protocol, register it *before* the setup hook.
  // https://docs.rs/tauri/2.8.2/tauri/plugin/struct.Builder.html#known-limitations
  tauri::Builder::default().plugin(tauri_plugin_axum::init(Router::new()))
  ```

Once registered, you can access your Axum routes via:

- macOS, iOS, and Linux: `axum://localhost/<path>`
- Windows and Android: `http://axum.localhost/<path>` (default)

> ⚠️ Note: Custom protocols currently do **not** support streaming.

- **Partial stream body support**

  Supports streaming responses using either the provided fetch API or an Axios adapter:

  ```typescript
  import { fetch } from "@mcitem/tauri-plugin-axum/fetch";
  import { Adapter } from "@mcitem/tauri-plugin-axum/axios";
  ```

---

## 📦 Installation

Rust crate:

```bash
cargo add tauri-plugin-axum
```

npm package:

```bash
pnpm i @mcitem/tauri-plugin-axum
```

Add required capability in `src-tauri/capabilities/default.json`:

```jsonc
{
  // ...
  "permissions": ["axum:default"]
  // ...
}
```

---

## 🚀 Usage Example

### Rust

```rust,no_run
// URI scheme protocols are registered when the WebView is created.
// If the plugin is registered after a WebView has been created,
// the protocol will not be available.
//
// macOS, iOS, Linux: axum://localhost/<path>
// Windows, Android: http://axum.localhost/<path> (default)

tauri::Builder::default().plugin(tauri_plugin_axum::init(Router::new()));

window.fetch("http://axum.localhost/");
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

// Initialize router asynchronously
tauri::Builder::default()
    .setup(|app| {
        let path = app.path().app_config_dir()?;
        app.handle().plugin(tauri_plugin_axum::block_init(async {
            println!("Application config path: {:?}", path);
            router::router()
        }))?;
        Ok(())
    });
```

### TypeScript

```typescript
// Using fetch
import { fetch } from "@mcitem/tauri-plugin-axum/fetch";

fetch("/", { method: "GET" })
  .then((res) => res.text())
  .then((res) => console.log(res));

// Using Axios adapter
import axios from "axios";
import { Adapter } from "@mcitem/tauri-plugin-axum/axios";

const instance = axios.create({ adapter: Adapter });
```

---

## 📚 Example Project

- [Example App](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app)

  - [main.tsx](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app/src/main.tsx)
  - [lib.rs](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app/src-tauri/src/lib.rs)

### Run Example

```sh
git clone https://github.com/mcitem/tauri-plugin-axum
cd tauri-plugin-axum
pnpm install
pnpm build
pnpm --filter tauri-app install
pnpm --filter tauri-app tauri dev
```
