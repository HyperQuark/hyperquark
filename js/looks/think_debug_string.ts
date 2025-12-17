import { target_names } from "../shared";

export function think_debug_string(data: string, target_idx: number): void {
  console.log("%s thinks: %s", target_names()[target_idx], data);
}
