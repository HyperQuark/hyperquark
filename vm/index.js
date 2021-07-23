import { initialise as initialiseWasm } from "./vm.ts?importMemory&maximumMemory=100&initialMemory=12&sharedMemory&noExportMemory&enable=threads";

export class VM {
  memory;
  worker;
  mod;
  constructor({ memory }) {
    this.memory = memory;
    this.worker = URL.createObjectURL(new Blob([this.workerFn.toString()]), {
      type: "application/javascript; charset=utf-8"
    });
    //this.worker.postMessage({ msg: "loadWasm", initialise: initialiseWasm });
  }
  workerFn() {
    let wasm;
    onMessage = async ({ data }) => {
      const { msg, initialise } = data;
      switch (msg) {
        case "loadWasm":
          wasm = await initialise({ env: {
            abort: () => console.log("Abort!")
          }});
          break;
        case "start":
          wasm.exports.start();
          break;
        case "stop":
          wasm.exports.stop();
          break;
      }
    };
  }
}
