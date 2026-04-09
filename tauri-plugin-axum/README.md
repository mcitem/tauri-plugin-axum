# tauri-plugin-axum

[![Crates.io Version](https://img.shields.io/crates/v/tauri-plugin-axum)](https://crates.io/crates/tauri-plugin-axum)
[![NPM Version](https://img.shields.io/npm/v/tauri-plugin-axum-api)](https://www.npmjs.com/package/tauri-plugin-axum-api)

Call your Axum router directly — just like a local database, not a remote server.

---

## Features

- **Custom protocol registration**

  ```rust,no_run
  // Important: if you use a custom protocol, register it *before* the setup hook.
  // https://docs.rs/tauri/2.8.2/tauri/plugin/struct.Builder.html#known-limitations
  tauri::Builder::default().plugin(tauri_plugin_axum::init(Router::new()))
  ```

Once registered, you can access your Axum routes via:

- macOS, iOS, and Linux: `axum://localhost/<path>`
- Windows and Android: `http://axum.localhost/<path>` (default)

> Note: Custom protocols currently do **not** support streaming. ([#1404](https://github.com/tauri-apps/wry/issues/1404))

- **stream body support**

  Supports streaming responses using either the provided fetch API or an Axios adapter:

  ```typescript
  import { fetch } from "tauri-plugin-axum-api/fetch";
  import { Adapter } from "tauri-plugin-axum-api/axios";
  ```

---

## Installation

Automatic

```bash
pnpm tauri add axum
```

or Manual

Rust crate:

```bash
cargo add tauri-plugin-axum
```

npm package:

```bash
pnpm i tauri-plugin-axum-api
```

Add required capability in `src-tauri/capabilities/default.json`:

```jsonc
{
  // ...
  "permissions": ["axum:default"],
  // ...
}
```

---

## Usage Example

```rust,no_run
// URI scheme protocols are registered when the WebView is created.
// If the plugin is registered after a WebView has been created,
// the protocol will not be available.
//
// macOS, iOS, Linux: axum://localhost/<path>
// Windows, Android: http://axum.localhost/<path> (default)

tauri::Builder::default().plugin(tauri_plugin_axum::init(axum::Router::new()));

```

```typescript
window.fetch("http://axum.localhost/");

import { fetch } from "tauri-plugin-axum-api/fetch";

fetch("/", { method: "GET" })
  .then((res) => res.text())
  .then((res) => console.log(res));

// Using Axios adapter
import axios from "axios";
import { Adapter } from "tauri-plugin-axum-api/axios";

const instance = axios.create({ adapter: Adapter });
```

---

## Example Project

- [Example App](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app)
  - [App.tsx](https://github.com/mcitem/tauri-plugin-axum/blob/master/example/tauri-app/src/App.tsx)
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
