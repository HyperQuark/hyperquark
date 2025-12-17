import { update_bubble } from "../shared";

export function say_string(data: string, target_idx: number): void {
  update_bubble(target_idx, "say", data);
}
