export class Renderer {
  constructor({ canvas, memory }) {
    this.canvas = canvas;
    this.memory = memory;
  }
  start() {
    console.log(79);
    this.stopped = false;
    let frame = () => {
      if (this.stopped) return;
      let wasmByteMemoryArray = new Uint8Array(this.memory.buffer);
      console.log(wasmByteMemoryArray.slice(0, 200));
      if (wasmByteMemoryArray.slice(0, 1)) {
        console.log(84);
        let context = this.canvas.getContext("2d");
        console.log(99)
        context.clearRect(0, 0, 480, 360);
        console.log("aaaaaaa")
        const canvasImageData = context.createImageData(480, 360);
        console.log("eeeeeee");
        const imageDataArray = wasmByteMemoryArray.slice(1, 480 * 360 * 4 + 1);
        
        canvasImageData.data.set(imageDataArray);
        context.putImageData(canvasImageData, 20, 0);
      }
    };
   requestAnimationFrame(frame);
  }
  stop() {
    this.stopped = true;
  }
}
