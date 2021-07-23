import {initialise} from "./vm.ts?importMemory"

export class vm {
  memory;
  worker;
  constructor ({ memory }) {
    this.worker = URL.createObjectURL(new Blob([this.workerFn.toString()]), {
    type: "application/javascript; charset=utf-8"
  });
  }
  workerFn () {
    onMessage = ({ data }) => {
      switch (data.msg) {
        case "loadWasm":
          
          break;
        case "start":
          break;
        case "stop":
          break;
      }
    }
  }
}