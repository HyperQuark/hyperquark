import "./style.css";

import { instantiate } from "./test.ts";

instantiate({
  env: {
    abort: () => console.log("Abort!")
  }
}).then(instance => {
  document.querySelector("#app").innerHTML = `
  <h1>Hello Vite!</h1>
  <a href="https://vitejs.dev/guide/features.html" target="_blank">Documentation</a>
  ${instance.add(5, 7)}
`;
});
