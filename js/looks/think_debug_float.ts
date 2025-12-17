import { target_names } from "../shared";

export function think_debug_float(data: number, target_idx: number): void {
  console.log("%s thinks: %d", target_names()[target_idx], data);
}
