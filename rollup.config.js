import { join } from "node:path";
import { cwd } from "node:process";
import alias from "@rollup/plugin-alias";
import copy from "rollup-plugin-copy";
import typescript from "@rollup/plugin-typescript";
import { defineConfig } from "rollup";

export default defineConfig({
  treeshake: {
    moduleSideEffects: false,
  },
  input: ["src/index.ts", "src/fetch.ts", "src/axios.js"],
  output: [
    {
      entryFileNames: "[name].js",
      format: "esm",
      dir: "dist",
    },
    {
      entryFileNames: "[name].cjs",
      format: "cjs",
      dir: "dist",
    },
  ],
  plugins: [
    copy({
      targets: [{ src: "src/axios.d.ts", dest: "dist" }],
    }),
    alias({
      entries: [
        {
          find: "axios/lib",
          replacement: join(cwd(), "./node_modules/axios/lib"),
        },
      ],
    }),
    typescript({
      declaration: true,
      declarationDir: "dist",
    }),
  ],
  external: [/^@tauri-apps\/api/, /^axios/],
});
