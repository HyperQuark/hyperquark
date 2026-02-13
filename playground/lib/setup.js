import { setup as sharedSetup, is_setup } from "../../js/shared.ts";

function createSkin(renderer, type, layer, ...params) {
  let drawableId = renderer.createDrawable(layer.toString());
  const realType = {
    pen: "Pen",
    text: "Text",
    svg: "SVG",
  }[type.toLowerCase()];
  let skin = renderer[`create${realType}Skin`](...params);
  renderer.updateDrawableSkinId(drawableId, skin);
  return [skin, drawableId];
}

export async function setup(
  makeRenderer,
  project_json,
  assets,
  target_names,
  queue_question,
  update_var_val,
  update_var_visible,
) {
  if (is_setup()) return;

  let renderer = await makeRenderer();

  renderer.getDrawable = (id) => renderer._allDrawables[id];
  renderer.getSkin = (id) => renderer._allSkins[id];
  renderer.createSkin = (type, layer, ...params) =>
    createSkin(renderer, type, layer, ...params);

  const costumes = project_json.targets.map((target, index) =>
    target.costumes.map(({ md5ext }) => assets[md5ext]),
  );

  if (typeof window === "object") {
    window.renderer = renderer;
  }
  renderer.setLayerGroupOrdering(["background", "video", "pen", "sprite"]);
  //window.open(URL.createObjectURL(new Blob([wasm_bytes], { type: "octet/stream" })));
  const pen_skin = renderer.createSkin("pen", "pen")[0];

  const target_skins = project_json.targets.map((target, index) => {
    const realCostume = target.costumes[target.currentCostume];
    const costume = costumes[index][target.currentCostume];
    if (costume.dataFormat.toLowerCase() !== "svg") {
      throw new Error("todo: non-svg costumes");
    }

    const [skin, drawableId] = renderer.createSkin(
      costume.dataFormat,
      target.isStage ? "background" : "sprite",
      costume.data,
      [realCostume.rotationCenterX, realCostume.rotationCenterY],
    );

    const drawable = renderer.getDrawable(drawableId);
    if (!target.isStage) {
      drawable.updateVisible(!!target.visible);
      drawable.updatePosition([target.x, target.y]);
      drawable.updateDirection(target.direction);
      drawable.updateScale([target.size, target.size]);
    } else {
      drawable.updateVisible(true);
      drawable.updatePosition([0, 0]);
      drawable.updateDirection(90);
      drawable.updateScale([100, 100]);
    }
    return [skin, drawableId];
  });
  console.log(target_skins);

  const stageIndex = project_json.targets.findIndex((target) => target.isStage);

  sharedSetup(
    target_names,
    renderer,
    pen_skin,
    target_skins,
    costumes,
    queue_question,
    stageIndex,
    update_var_val,
    update_var_visible,
  );
}
