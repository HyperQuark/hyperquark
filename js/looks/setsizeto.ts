import { renderer, costumes, target_skins } from '../shared';

export function setsizeto(size: number, target_index: number) {
    renderer().getDrawable(target_skins()[target_index][1]).updateScale([size, size]);
}
