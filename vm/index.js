import VMWorker from "./worker.js?worker";

export class VM {
  constructor({ memory }) {
    this.memory = memory;
  }
  init () {
    return new Promise(r => {
      this.worker = new VMWorker();
      this.worker.onerror = e => console.error(e.message);
      this.worker.onmessage = ({data}) => {
        console.log(data);
        data === 6 && r();
      };
      this.worker.postMessage({ msg: "loadWasm", memory: this.memory });
    });
  }
}
