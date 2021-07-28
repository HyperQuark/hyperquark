export function e (): i32 {
  store<i32>(0, 1);
  for (let i: u32 = 1; i < 262145; i++) {
    store<i32>(i, 255);
  }
  return 6;
}