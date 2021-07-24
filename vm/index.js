//import { binary } from "./vm.ts?importMemory&maximumMemory=100&initialMemory=12&noExportMemory&sharedMemory&enable=threads";

export class VM {
  memory;
  worker;
  mod;
  constructor({ memory }) {
    this.memory = memory;
    this.worker = new Worker(URL.createObjectURL(new Blob([this.workerFn.toString().slice(12, -1)]), {
      type: "application/javascript; charset=utf-8"
    }));
  //  console.log(this.workerFn.toString().slice(12, -1));
    this.worker.onerror = e => console.error(e.message);
    this.worker.onmessage = console.log;
    this.worker.p
  }
  workerFn() {
    let wasm;
    onMessage = async ({ data }) => {
      postMessage("aa");
      const { msg, /*initialise, */memory } = data;
      switch (msg) {
        case "loadWasm":
          let { initialise } = await import("./vm.ts?importMemort*maximumMemory=100&initialMemory=12&noExportMemory&sharedMemory&enable=threads");
          wasm = await initialise({ env: {
            abort: () => console.log("Abort!"),
            memory
          }});
          postMessage("ee");
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
