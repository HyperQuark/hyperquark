import { initialise as initialiseWasm } from "./vm.ts?importMemory&maximumMemory=100&initialMemory=12&sharedMemory&noExportMemory";

export class vm {
  constructor({ memory }) {
    
    this.worker = URL.createObjectURL(new Blob([this.workerFn.toString()]), {
      type: "application/javascript; charset=utf-8"
    });
  }
  workerFn() {
    onMessage = ({ data }) => {
      switch (data.msg) {
        case "loadWasm":
          break;
        case "start":
          break;
        case "stop":
          break;
      }
    };
  }
  memory;
  worker;
  mod;
}
