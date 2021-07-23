/* global crossOriginIsolated */

import "./style.css";

import {
  instantiate,
  text
} from "./test.ts?exportTable&exportRuntime&explicitStart";
import { Asdom } from "asdom/glue/index.js";
import vm from "."
import eruda from "eruda";
eruda.init();

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js").then(
    async registration => {
      // Registration was successful
      console.log(
        "ServiceWorker registration successful with scope: ",
        registration.scope
      );
      if (crossOriginIsolated) {
        console.log("Phew everything's working (I think)");
      } else {
        console.log("COOP+COEP failed :C");
        document.write(
          "Please reload your page - if you see this message after reloading then something's gone wrong and you'll need to do some stuff to try and fix it that I can't be bothered to explain. Have a nice day :D"
        );
      }
      main();
    },
    function(err) {
      // registration failed :(
      console.log("ServiceWorker registration failed: ", err);
      document.write(
        "Something's gone terribly wrong. If you'd like. open up your browser's dev tools and try to find the problem. You might just be using an old, unupported browser."
      );
    }
  );
}

async function main() {
  const asdom = new Asdom();

  let instance = await instantiate({
    env: { abort: () => console.log("Abort!") },
    ...asdom.wasmImports
  });
  asdom.wasmExports = instance.exports;

  const memory = new WebAssembly.Memory({
    shared: true,
    initial: 11,
    maximum: 100
  });
  
  let vmWorker = URL.createObjectURL(new Blob([vm.toString()]), {
			type: "application/javascript; charset=utf-8",
		});
}
