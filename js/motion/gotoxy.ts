import { renderer, target_skins } from "../shared";

export function gotoxy(x: number, y: number, target_index: number) {
  renderer()
    .getDrawable(target_skins()[target_index][1])
    .updatePosition([x, y]);
}
