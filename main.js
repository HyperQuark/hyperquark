/* global crossOriginIsolated wasm */

import "./style.css";

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
        document.body.textContent =
          "Please reload your page - if you see this message after reloading then something's gone wrong and you'll need to do some stuff to try and fix it that I can't be bothered to explain. Have a nice day :D";
      }
      main();
    },
    err => {
      // registration failed :(
      console.log("ServiceWorker registration failed: ", err);
      document.body.textContent =
        "Something's gone terribly wrong. If you'd like. open up your browser's dev tools and try to find the problem. Of course, the problem could be that you might just be using an old, unupported browser, but in theory you should grt a shorter error for that.";
    }
  );
} else document.body.textContent = "You're using an unsupported browser :(";

function main() {
  /* const memory = new WebAssembly.Memory({
    shared: true,
    initial: 11,
    maximum: 100
  });*/

  document.getElementById("app").innerHTML = `
    <button id="start">green flag</button>
    <button id="stop">stop</button>
    <canvas id="stage"></canvas>
    <br>
    <br>
    Running: <span id="running">false</span>
  `;
  // let { exports } = await instantiate({});
  //console.log(wasm.exports.table.get(wasm.exports.e())());
  // window.wasm = exports;
  //console.log(wasm.e(), wasm.b());
  //console.log(wasm.table.get(wasm.e())(54455445));
  // console.log(wasm.table.get(wasm.b())());
  /* let vm = new VM({ memory });
  await vm.init();
  let renderer = new Renderer({ canvas: document.getElementById("stage"), memory });
  renderer.start();
*/
  //}

  class WasmUint8Array extends Uint8Array {
    constructor({ types }) {
      let typeSec = new Vector(
        ...types.map(t => new FuncType(t.params, t.returns))
      ).flat(1);
      super([...WasmUint8Array.WasmHeader, ...new TypeSection(typeSec)]);
    }
    static MagicNumber = [0, 97, 115, 109];
    static Version = [1, 0, 0, 0];
    static WasmHeader = [
      ...WasmUint8Array.MagicNumber,
      ...WasmUint8Array.Version
    ];
  }
  let wasm = new WasmUint8Array({
    types: [
      {
        params: [NumTypes.i32, NumTypes.i32],
        returns: []
      }
    ]
  });
  console.log(wasm);
  let mod = new WebAssembly.Module(new Uint8Array(wasm));
  console.log(mod);
}
