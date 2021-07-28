import VMWorker from "./worker.js?worker";

export class VM {
  constructor({ memory }) {
    this.memory = memory;
  }
  init () {
    this.worker = new VMWorker();
    this.worker.onerror = e => console.error(e.message);
    this.worker.onmessage = ({data}) => console.log(data);
    this.worker.postMessage({ msg: "loadWasm", memory: this.memory });
  }
}
