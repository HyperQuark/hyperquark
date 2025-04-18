import { update_bubble } from "../shared";

export function say_int(data: number, target_idx: number): void {
  update_bubble(target_idx, "say", data.toString());
}