import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("elemen #app tidak ditemukan di index.html");
}

export default mount(App, { target });
