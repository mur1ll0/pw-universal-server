import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

// Tema do painel (B218): o mesmo `pw-painel-tema` do navegador.
try {
  const tema = localStorage.getItem("pw-painel-tema");
  if (tema) document.documentElement.dataset.tema = tema;
} catch { /* sem armazenamento: segue o sistema */ }

mount(App, { target: document.getElementById("app")! });
