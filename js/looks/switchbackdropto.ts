import { renderer, costumes, target_skins, stageIndex } from "../shared";

export function switchbackdropto(costume_num: number) {
  const costume = costumes()[stageIndex()][costume_num];
  if (typeof costume === "undefined") return;
  renderer().getSkin(target_skins()[stageIndex()][0]).setSVG(costume.data);
}
