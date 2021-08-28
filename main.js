/* global crossOriginIsolated wasm */
/*
import "./style.css";

import { VM } from "./vm/js";
import { Renderer } from "./render";
import { instantiate } from "./vm/as/vm.ts?exportTable";

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
        document.body.textContent = (
          "Please reload your page - if you see this message after reloading then something's gone wrong and you'll need to do some stuff to try and fix it that I can't be bothered to explain. Have a nice day :D"
        );
      }
      main();
    },
    function(err) {
      // registration failed :(
      console.log("ServiceWorker registration failed: ", err);
      document.body.textContent = (
        "Something's gone terribly wrong. If you'd like. open up your browser's dev tools and try to find the problem. Of course, the problem could be that you might just be using an old, unupported browser."
      );
    }
  );
} else document.body.textContent = "You're using an unsupported browser :(";
*/
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
  const wasmHeader = [0, 97, 115, 109, 1, 0, 0, 0];
  const encodeSignedLeb128FromInt32 = value => {
    value |= 0;
    const result = [];
    while (true) {
      const byte = value & 0x7f;
      value >>= 7;
      if (
        (value === 0 && (byte & 0x40) === 0) ||
        (value === -1 && (byte & 0x40) !== 0)
      ) {
        result.push(byte);
        return result;
      }
      result.push(byte | 0x80);
    }
  };
  const createSection = (type, content) => [
    type,
    encodeSignedLeb128FromInt32(content.length),
    ...content
  ];
  const typeSection = types => createSection(0x01, types);
  const types = {
    i32: 0x7f,
    i64: 0x7e,
    f32: 0x7d,
    f64: 0x7c
  };
  const funcType = (paramTypes = [], returnTypes = []) => {
    let e = [
    0x60,
    paramTypes.length,
    ...paramTypes,
      returnTypes.length,
      ...returnTypes
    ];
    console.log(paramTypes, returnTypes, e)
    return e;
  }
  // we shouldn't needimports, here just in case
  // const importSection = imports => createSection(2, imports);
  const createWasmModule = ({ types }) => {
    //return new Uint8Array(
     return wasmHeader.concat(
        typeSection(types.map(t => funcType(t.params, t.returns)))
      )
    //);
  };
  console.log(
    createWasmModule({
      types: [
        {
          params: [types.i32, types.i32],
          returns: []
        }
      ]
    })
  );
}
main();