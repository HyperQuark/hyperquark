import { renderer, target_skins } from '../shared';

export function pointindirection(target_index: number, direction: number) {
    renderer().getDrawable(target_skins()[target_index][1]).updateDirection(direction);
}
