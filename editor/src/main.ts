import init from "./wasm/vectro_wasm.js";
import { mount } from "svelte";
import "./app.css";

// The Rust core must load before the editor reads its first document.
await init();
const { default: App } = await import("./App.svelte");
mount(App, { target: document.getElementById("app")! });
