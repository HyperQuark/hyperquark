export class Renderer {
  canvas;
  memory;
  stopped = true;
  constructor({ canvas, memory }) {
    this.canvas = canvas;
    this.memory = memory;
  }
  start() {
    this.stopped = false;
    let frame = () => {
      if (this.stopped) return;
      let wasmByteMemoryArray = new Uint8Array(this.memory.buffer);
      if (wasmByteMemoryArray[0]) {
        let context = this.canvas.value.getContext("2d");
        context.clearRect(0, 0, 480, 360);
        const canvasImageData = context.createImageData(480, 360);
        const imageDataArray = wasmByteMemoryArray.slice(1, 480 * 360 * 4 + 1);
        canvasImageData.data.set(imageDataArray);
        context.putImageData(canvasImageData, 20, 0);
      }
      requestAnimationFrame(frame);
    };
  }
  stop() {
    this.stopped = true;
  }
}
