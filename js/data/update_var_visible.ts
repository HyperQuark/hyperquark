import { update_var_visible as update_var_visibility } from "../shared.ts";

export function update_var_visible(id: string, visible: boolean) {
  update_var_visibility(id, visible);
}
