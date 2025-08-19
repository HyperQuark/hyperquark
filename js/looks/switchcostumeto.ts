import { renderer, costumes, target_skins } from '../shared';

export function switchcostumeto(costume_num: number, target_index: number) {
    const costume = costumes()[target_index][costume_num];
    renderer().getSkin(target_skins()[target_index][0]).setSVG(costume.data);
}