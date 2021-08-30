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
        "Something's gone terribly wrong. If you'd like. open up your browser's dev tools and try to find the problem. Of course, the problem could be that you might just be using an old, unupported browser.";
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

  const wasmHeader = [0, 97, 115, 109, 1, 0, 0, 0] as const;
  const encodeSignedLeb128FromInt32 = value => {
    value |= 0;
    const result: Array<number> = [];
    while (true) {
      const byte: number = value & 0x7f;
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
  class Vector extends Array {
    constructor (array) {
      array ||= [];
      super([array.length, ...array]);
    }
   /* *[Symbol.iterator] () {
      this.forEach(n => yield n);
    }*/
  }
  const createSection = (type, content) => {
    let e = [
      type,
      //encodeSignedLeb128FromInt32(content.length),
      content.length, // we're going to assume that each section isn't more than 127 bits long, plus I don't really understand when this leb128 thing is meant to be used, nor have I seen ang examples of it being used in wasm... we'll just assume it works
      ...content
    ];
    //console.log(type, content, e);
    return e;
  };
  const typeSection = types => createSection(0x01, types);
  const types = {
    i32: 0x7f,
    i64: 0x7e,
    f32: 0x7d,
    f64: 0x7c
  };
  class funcType extends Array {
    constructor (paramTypes, returnTypes) {
      console.log(paramTypes, returnTypes)
      super([
        0x60,
        ...new Vector(paramTypes),
        ...new Vector(returnTypes)
      ]);
      //console.log(paramTypes, returnTypes, e)
    }
  };
  class WasmUint8Array extends Uint8Array {
    constructor ({ types }) {
      let a = (new Vector(types.map(t => {
        let e = new funcType(t.params, t.returns)
        console.log("hmm", e);
        return e
      }))).flat(2);
      console.log(a);
      super([...wasmHeader, ...typeSection(a)])
    }
  };
  let wasm = new WasmUint8Array({
    types: [
      {
        params: [types.i32, types.i32],
        returns: []
      }
    ]
  });
  console.log(wasm);
  let mod = new WebAssembly.Module(new Uint8Array(wasm));
  console.log(mod);
}
