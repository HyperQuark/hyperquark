/* global crossOriginIsolated */

import "./style.css";

import { instantiate, text } from "./test.ts?exportTable&exportRuntime&explicitStart";
import { Asdom } from "asdom/glue/index.js";
import eruda from "eruda";
eruda.init();

if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js').then(function(registration) {
      // Registration was successful
      console.log('ServiceWorker registration successful with scope: ', registration.scope);
      e();
    }, function(err) {
      // registration failed :(
      console.log('ServiceWorker registration failed: ', err);
    });
}

if (crossOriginIsolated) {
  // Post SharedArrayBuffer
  alert("oui");
} else {
  // Do something else
  alert("nein");
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

var req = new XMLHttpRequest();
req.open('GET', document.location, false);
req.send(null);
var headers = req.getAllResponseHeaders().toLowerCase();
headers = headers.split(/\n|\r|\r\n/g).reduce(function(a, b) {
    if (b.length) {
        var [ key, value ] = b.split(': ');
        a[key] = value;
    }
    return a;
}, {});

const e = () => console.log(headers);