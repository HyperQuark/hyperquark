import { renderer, costumes, target_skins } from "../shared";

export function setvisible(visible: boolean, target_index: number) {
  renderer()
    .getDrawable(target_skins()[target_index][1])
    .updateVisible(visible);
}
