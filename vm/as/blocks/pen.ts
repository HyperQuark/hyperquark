let penDown: i32 = 0;

export function isPenDown(): i32 {
  return penDown;
}

export function penDown(): void {
  penDown = 1;
}
export function penUp(): void {
  penDown = 0;
}
