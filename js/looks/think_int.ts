import { update_bubble } from "../shared";

export function think_int(data: number, target_idx: number): void {
  update_bubble(target_idx, "think", data.toString());
}
