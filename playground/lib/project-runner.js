import { getSettings } from "./settings.js";
import { imports as baseImports } from "./imports.js";
import {
  setup as sharedSetup,
  is_setup,
  renderer as get_renderer,
} from "../../js/shared.ts";
import { WasmStringType } from "../../js/no-compiler/hyperquark.js";

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

async function setup(makeRenderer, project_json, assets, target_names) {
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
      "sprite",
      costume.data,
      [realCostume.rotationCenterX, realCostume.rotationCenterY],
    );

    const drawable = renderer.getDrawable(drawableId);
    if (!target.is_stage) {
      drawable.updateVisible(!!target.visible);
      drawable.updatePosition([target.x, target.y]);
      drawable.updateDirection(target.direction);
      drawable.updateScale([target.size, target.size]);
    }
    return [skin, drawableId];
  });
  console.log(target_skins);

  sharedSetup(target_names, renderer, pen_skin, target_skins, costumes);
}

// @ts-ignore
export async function instantiateProject({
  framerate = 30,
  turbo,
  wasm_bytes,
  target_names,
  strings,
  project_json,
  assets,
  makeRenderer,
  isDebug = () => false,
  timeout,
  onTimeout = () => null,
  importOverrides,
}) {
  if (isDebug() && typeof window === "object")
    window.open(
      URL.createObjectURL(new Blob([wasm_bytes], { type: "application/wasm" })),
    );

  await setup(makeRenderer, project_json, assets, target_names);

  const renderer = get_renderer();

  console.log("project setup complete");

  const framerate_wait = Math.round(1000 / framerate);

  const settings = getSettings();
  const builtins = [
    ...(WasmStringType[settings.string_type] === "JsStringBuiltins"
      ? ["js-string"]
      : []),
  ];

  const imports = Object.assign(baseImports, {
    "": Object.fromEntries(strings.map((string) => [string, string])),
  });

  console.log(importOverrides);

  for (const [module, obj] of Object.entries(importOverrides ?? {})) {
    for (const [name, val] of Object.entries(importOverrides[module])) {
      imports[module][name] = val;
    }
  }

  try {
    if (
      !WebAssembly.validate(wasm_bytes, {
        builtins,
      })
    ) {
      throw Error();
    }
  } catch {
    try {
      new WebAssembly.Module(wasm_bytes);
      throw new Error("invalid WASM module");
    } catch (e) {
      throw new Error("invalid WASM module: " + e.message);
    }
  }
  function sleep(ms) {
    return new Promise((resolve) => {
      setTimeout(resolve, ms);
    });
  }
  function waitAnimationFrame() {
    return new Promise((resolve) => {
      if (typeof requestAnimationFrame === "function")
        requestAnimationFrame(resolve);
      else setTimeout(resolve, 1);
    });
  }
  let { instance } = await WebAssembly.instantiate(wasm_bytes, imports, {
    builtins,
    importedStringConstants: "",
  });
  const {
    flag_clicked,
    tick,
    memory,
    threads_count,
    requests_refresh,
    threads,
    unreachable_dbg,
  } = instance.exports;
  if (typeof window === "object") {
    window.memory = memory;
    window.flag_clicked = flag_clicked;
    window.tick = tick;
  }

  try {
    // expose the module to devtools
    unreachable_dbg();
  } catch (error) {
    console.info("synthetic error to expose wasm module to devtools:", error);
  }

  let running = false;

  const run = async () => {
    console.log("running");
    if (running) return;

    running = true;

    renderer.draw();

    let startTime = Date.now();
    $outertickloop: while (running) {
      if (timeout && Date.now() - startTime > timeout) {
        return onTimeout();
      }
      let thisTickStartTime = Date.now();
      do {
        tick();
        if (threads_count.value === 0) {
          break $outertickloop;
        }
      } while (
        Date.now() - thisTickStartTime < framerate_wait * 0.8 &&
        !turbo &&
        requests_refresh.value === 0
      );
      requests_refresh.value = 0;
      renderer.draw();
      if (framerate_wait > 0) {
        await sleep(
          Math.max(0, framerate_wait - (Date.now() - thisTickStartTime)),
        );
      } else {
        await waitAnimationFrame();
      }
    }
    renderer.draw();
    running = false;
    console.log("project stopped (or maybe paused)");
  };

  return {
    greenFlag: () => {
      console.log("green flag clicked");
      flag_clicked();
      run();
    },
    flag_clicked,
    stop: () => {
      console.log("stopping");
      threads_count.value = 0;
      running = false;
      for (let i = 0; i < threads.length; i++) {
        threads.set(i, null);
      }
    },
    pause: () => {
      running = false;
    },
    run,
  };
}
