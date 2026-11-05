import { mount } from "svelte";
import "./app.css";
import Panel from "./Panel.svelte";

const target = document.getElementById("panel");
if (!target) {
  throw new Error("elemen #panel tidak ditemukan di panel.html");
}

export default mount(Panel, { target });
