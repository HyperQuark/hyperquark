let isPenDownVar: i32 = 0;

export function isPenDown(): i32 {
  return isPenDownVar;
}

export function penDown(): void {
  isPenDownVar = 1;
}
export function penUp(): void {
  isPenDownVar = 0;
}

