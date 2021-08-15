/*
import { isPenDown, penDown, penUp } from "./vm/as/blocks/pen";

let running: i32 = 0;

export function start(): void {
  running = 1;
}

export function stop(): void {
  running = 0;
}
*/
function a (): i32 {
  return 72;
}

function f (): i32 {
  return 43;
}

export function e(): () => i32 {
  return a;
}

export function b (): () => i32 {
  return f;
}
