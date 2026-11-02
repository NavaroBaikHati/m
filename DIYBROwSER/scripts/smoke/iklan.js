// Skrip "iklan" untuk skenario uji content blocker.
// Kalau request ini diblokir, kotak merah di bawah tidak pernah muncul.
const kotak = document.createElement("div");
kotak.id = "kotak-iklan";
kotak.textContent = "IKLAN";
kotak.style.cssText = [
  "position:fixed",
  "left:40px",
  "top:40px",
  "width:120px",
  "height:80px",
  "background:#ff0000",
  "color:#ffffff",
  "font:700 16px system-ui,sans-serif",
  "display:grid",
  "place-items:center",
  "z-index:9999",
].join(";");
document.body.appendChild(kotak);
