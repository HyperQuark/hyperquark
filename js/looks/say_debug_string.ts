import { target_names } from "../shared";

export function say_debug_string(data: string, target_idx: number): void {
  console.log('%s says: %s', target_names()[target_idx], data);
}