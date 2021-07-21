import "./style.css";

import { instantiate } from "./test.as";

instantiate({
  env: {
    abort: () => console.log("Abort!")
  }
}).then(({ instance }) => {
  document.querySelector("#app").innerHTML = `
  <h1>Hello Vite!</h1>
  <a href="https://vitejs.dev/guide/features.html" target="_blank">Documentation</a>
  ${instance.exports.add(5, 7)}
`;
  console.log(instance.exports.table);
});
/*
import wasmUrl from "asc:./test.as";

WebAssembly.instantiateStreaming(fetch(wasmUrl), {}).then(({ instance }) =>
  console.log(instance.exports.add(40, 2))
);*/