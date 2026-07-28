import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";

console.log("web worker initialised");

postMessage("ready");

addEventListener("message", ({ data }) => {
  switch (data.stage) {
    case "compile": {
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
      import("binaryen")
        .then((imports) => {
          const binaryen = imports.default;
          const binaryenModule = binaryen.readBinary(data.wasmBytes);
          console.log(binaryenModule.emitBinary().length);
          binaryenModule.setFeatures(
            // We can't set to Features.All because that enables custom descriptors which causes
            // some passes to emit exact types, which are probably not actually available in the browser.
            // TODO: base this on actual available features.
            binaryen.Features.BulkMemory |
              binaryen.Features.ExtendedConst |
              binaryen.Features.GC |
              binaryen.Features.Multivalue |
              binaryen.Features.MutableGlobals |
              binaryen.Features.ReferenceTypes |
              binaryen.Features.SignExt |
              binaryen.Features.NontrappingFPToInt |
              binaryen.Features.Strings |
              binaryen.Features.TailCall,
          );
          binaryen.setOptimizeLevel(3);
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
        })
        .catch((e) => {
          throw e;
        });
    }
  }
});
