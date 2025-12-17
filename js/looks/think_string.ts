import { update_bubble } from "../shared";

export function think_string(data: string, target_idx: number): void {
  update_bubble(target_idx, "think", data);
}
