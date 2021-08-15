import { isPenDown, penDown, penUp } from "./vm/as/blocks/pen";

let running: i32 = 0;

export function start(): void {
  running = 1;
}

export function stop(): void {
  running = 0;
}

export function e(): i32 {
  return 56;
}
