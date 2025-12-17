export function dayssince2000(): number {
  // https://github.com/scratchfoundation/scratch-vm/blob/f10ab17bf351939153d9d0a17c577b5ba7b3c908/src/blocks/scratch3_sensing.js#L252
  const msPerDay = 24 * 60 * 60 * 1000;
  const start = new Date(2000, 0, 1); // Months are 0-indexed.
  const today = new Date();
  const dstAdjust = today.getTimezoneOffset() - start.getTimezoneOffset();
  let mSecsSinceStart = today.valueOf() - start.valueOf();
  mSecsSinceStart += (today.getTimezoneOffset() - dstAdjust) * 60 * 1000;
  return mSecsSinceStart / msPerDay;
}
