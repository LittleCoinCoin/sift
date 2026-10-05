// Test-only build of the update-UX harness (e2e.html). It is never part of the
// production bundle: `vite.config.ts` has no e2e entry, and the aliases below
// exist only here. The real store and Toast are bundled unchanged; only the
// Tauri plugin modules they import are replaced by scripted fakes.
import { fileURLToPath } from "node:url";
import { defineConfig, mergeConfig } from "vite";
import base from "./vite.config";

const fake = (name: string) =>
  fileURLToPath(new URL(`./src/lib/e2e/fakes/${name}.ts`, import.meta.url));

export default mergeConfig(
  base,
  defineConfig({
    resolve: {
      alias: [
        { find: /^@tauri-apps\/plugin-updater$/, replacement: fake("updater") },
        { find: /^@tauri-apps\/plugin-process$/, replacement: fake("process") },
        { find: /^@tauri-apps\/api\/event$/, replacement: fake("event") },
      ],
    },
    build: {
      outDir: "dist-e2e",
      rollupOptions: { input: fileURLToPath(new URL("./e2e.html", import.meta.url)) },
    },
  }),
);
