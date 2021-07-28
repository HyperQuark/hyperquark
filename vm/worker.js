import { instantiate } from "./vm.ts?enable=threads&importMemory&noExportMemory&sharedMemory&initialMemory=11&maximumMemory=100";

let wasm;

self.addEventListener("message", async ({ data: { msg, memory }}) => {
  postMessage(5);
  switch (msg) {
    case "loadWasm":
      wasm = await instantiate({
        env: {
          memory
        }
      });
      postMessage(wasm.exports.e());
      break;
    case "compile":
      wasm.exports.compile();
      break;
    case "start":
      wasm.exports.start();
      break;
    case "stop":
      wasm.exports.stop()
      break;
  };
});