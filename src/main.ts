import { mount } from "svelte";
import App from "./App.svelte";

if (import.meta.env.VITE_SIFT_E2E === '1') void import('./lib/e2e/updater-driver');

const app = mount(App, { target: document.getElementById("app")! });

export default app;
