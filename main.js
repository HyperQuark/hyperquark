import "./style.css";

import { instantiate, text } from "./test.ts?exportTable&exportRuntime";
import { Asdom } from "asdom/glue/index.js";

instantiate({ env: { abort: () => console.log("Abort!") }, ...Asdom.wasmImports }).then(
  ({ instance }) => {
    Asdom.wasmExports = instance.exports
    console.log(instance.exports.table);
    instance.exports.start();
  }
);
/*
import wasmUrl from "asc:./test.as";

WebAssembly.instantiateStreaming(fetch(wasmUrl), {}).then(({ instance }) =>
  console.log(instance.exports.add(40, 2))
);*/
