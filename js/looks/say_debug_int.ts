import { target_names } from "../shared";

export function say_debug_int(data: number, target_idx: number): void {
  console.log('%s says: %d', target_names()[target_idx], data);
}