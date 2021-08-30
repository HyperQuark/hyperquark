/* global crossOriginIsolated wasm */

import "./style.css";

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js").then(
    async (registration: ServiceWorkerRegistration): Promise<void> => {
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
    (err: Error): void => {
      // registration failed :(
      console.log("ServiceWorker registration failed: ", err);
      document.body.textContent =
        "Something's gone terribly wrong. If you'd like. open up your browser's dev tools and try to find the problem. Of course, the problem could be that you might just be using an old, unupported browser.";
    }
  );
} else document.body.textContent = "You're using an unsupported browser :(";

function main(): void {
  /* const memory = new WebAssembly.Memory({
    shared: true,
    initial: 11,
    maximum: 100
  });*/

  document.getElementById("app")!.innerHTML = `
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
  const encodeSignedLeb128FromInt32 = (value: number): Array<number> => {
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
  class Vector<T> extends Array<any> {
    constructor (array: Array<T>) {
      array ||= [];
      super([array.length, ...array])
    }
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
  enum possibleTypes {
    i32 = 0x7f,
    i64 = 0x7e,
    f32 = 0x7d,
    f64 = 0x7c
  }
  class funcType extends Array<number> {
    constructor (paramTypes: Array<number>, returnTypes: Array<number>) {
      let e: Array<number> = [
        0x60,
        ...new Vector<number>(paramTypes),
        ...new Vector<number>(returnTypes)
      ];
      //console.log(paramTypes, returnTypes, e)
      super([...e]);
    }
  };
  // we shouldn't needimports, here just in case
  // const importSection = imports => createSection(2, imports);
  interface wasmType { 
    params: Array<possibleTypes>,
    returns: Array<possibleTypes>
  }
  class WasmUint8Array extends Uint8Array {
    constructor ({ types }: { types: Array<wasmType> }) {
      let a: Vector<number> = (new Vector<number[]>(types.map(t => new funcType(t.params, t.returns)))).flat(2) as Vector<number>;
      console.log(a);
      super([...wasmHeader, ...typeSection(a)])
    }
  };
  let wasm: WasmUint8Array = new WasmUint8Array({
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
