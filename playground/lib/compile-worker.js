import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";

console.log("web worker initialised");

postMessage("ready");

addEventListener("message", ({ data }) => {
  switch (data.stage) {
    case "compile": {
      console.log("compile message received");
      let wasmProject = sb3_to_wasm(data.proj, WasmFlags.from_js(data.flags));
      postMessage(
        {
          wasm_bytes: wasmProject.wasm_bytes,
          strings: wasmProject.strings,
          target_names: wasmProject.target_names,
        },
        [wasmProject.wasm_bytes.buffer],
      );
      break;
    }
    case "optimise": {
      console.log("optimise message received!");
      console.log(data);
      import("binaryen").then((imports) => {
        const binaryen = imports.default;
        const binaryenModule = binaryen.readBinary(data.wasmBytes);
        console.log(binaryenModule.emitBinary().length);
        binaryenModule.setFeatures(binaryen.Features.All);
        binaryen.setOptimizeLevel(2);
        binaryen.setShrinkLevel(0);
        binaryenModule.runPasses(["generate-global-effects"]);
        console.log(binaryenModule.emitBinary().length);
        binaryenModule.optimize();
        console.log(binaryenModule.emitBinary().length);
        binaryenModule.optimize();
        const wasmBytes = binaryenModule.emitBinary();
        console.log(wasmBytes.length);
        postMessage(
          {
            wasmBytes,
          },
          [wasmBytes.buffer],
        );
        console.log("finished optimising");
      });
    }
  }
});
