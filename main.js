import "./style.css";

import { instantiate, text } from "./test.ts?exportTable&exportRuntime&explicitStart";
import { Asdom } from "asdom/glue/index.js";

if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js').then(function(registration) {
      // Registration was successful
      console.log('ServiceWorker registration successful with scope: ', registration.scope);
    }, function(err) {
      // registration failed :(
      console.log('ServiceWorker registration failed: ', err);
    });
}

const asdom = new Asdom();

instantiate({ env: { abort: () => console.log("Abort!") }, ...asdom.wasmImports }).then(
  ( instance ) => {
    asdom.wasmExports = instance.exports
    console.log(instance.exports.table);
    instance.exports._start();
  }
);
/*
import wasmUrl from "asc:./test.as";

WebAssembly.instantiateStreaming(fetch(wasmUrl), {}).then(({ instance }) =>
  console.log(instance.exports.add(40, 2))
);*/
