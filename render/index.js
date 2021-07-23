export const createRenderer = ({ canvas, memory }) => () => {
  let context = canvas.value.getContext("2d");
  context.clearRect(0, 0, 480, 360);
  let wasmByteMemoryArray = new Uint8Array(memory.buffer);
  const canvasImageData = context.createImageData(480, 360);
  const imageDataArray = wasmByteMemoryArray.slice(
    0,
    480 * 360 * 4
  );
  canvasImageData.data.set(imageDataArray);
  context.putImageData(canvasImageData, 20, 0);
};
