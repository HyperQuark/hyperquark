import { getSettings } from './settings.js';
import { imports } from './imports.js';
import { useDebugModeStore } from '../stores/debug.js';
import { setup as sharedSetup } from '../../js/shared.ts'
await import('../assets/renderer.js');

const debugModeStore = useDebugModeStore();

function createSkin(renderer, type, layer, ...params) {
  let drawableId = renderer.createDrawable(layer.toString());
  const realType = {
    pen: 'Pen',
    text: 'Text',
    svg: 'SVG'
  }[type.toLowerCase()];
  let skin = renderer[`create${realType}Skin`](...params);
  renderer.updateDrawableSkinId(drawableId, skin);
  return [skin, drawableId];
}

const spriteInfoLen = 80;
let _setup = false;

function setup(renderer, project_json, assets, target_names) {
  if (_setup) return;
  _setup = true;
  renderer.getDrawable = id => renderer._allDrawables[id];
  renderer.getSkin = id => renderer._allSkins[id];
  renderer.createSkin = (type, layer, ...params) => createSkin(renderer, type, layer, ...params);

  const costumes = project_json.targets.map(
    (target, index) => target.costumes.map(
      ({ md5ext }) => assets[md5ext]
    )
  );

  const costumeNameMap = project_json.targets.map(
    target => Object.fromEntries(target.costumes.map(
      ({ name }, index) => [name, index]
    ))
  );

  // @ts-ignore
  window.renderer = renderer;
  renderer.setLayerGroupOrdering(["background", "video", "pen", "sprite"]);
  //window.open(URL.createObjectURL(new Blob([wasm_bytes], { type: "octet/stream" })));
  const pen_skin = createSkin(renderer, "pen", "pen")[0];

  const target_skins = project_json.targets.map((target, index) => {
    const realCostume = target.costumes[target.currentCostume];
    const costume = costumes[index][target.currentCostume];
    const [skin, drawableId] = createSkin(renderer, costume[0], 'sprite', costume[1], [realCostume.rotationCenterX, realCostume.rotationCenterY]);
    const drawable = renderer.getDrawable(drawableId);
    if (!target.is_stage) {
      drawable.updateVisible(target.visible);
      drawable.updatePosition([target.x, target.y]);
      drawable.updateDirection(target.rotation);
      drawable.updateScale([target.size, target.size]);
    }
    return [skin, drawableId];
  });
  console.log(target_skins)

  sharedSetup(target_names, renderer);
}

// @ts-ignore
export default async (
  { framerate = 30, turbo, renderer, wasm_bytes, target_names, string_consts, project_json, assets } = {
    framerate: 30, turbo: false,
  }
) => {
  if (debugModeStore.debug) window.open(URL.createObjectURL(new Blob([wasm_bytes], { type: 'application/wasm' })));
  const framerate_wait = Math.round(1000 / framerate);
  let assert;
  let exit;
  let browser = false;
  let output_div;
  let text_div;

  setup(renderer, project_json, assets, target_names);

  console.log('green flag setup complete')

  let strings_tbl;

  let updatePenColor;
  let start_time = 0;
  let sprite_info_offset = 0;

  const settings = getSettings();
  const builtins = [...(settings['js-string-builtins'] ? ['js-string'] : [])]

  try {
    if (!WebAssembly.validate(wasm_bytes, {
      builtins
    })) {
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
      requestAnimationFrame(resolve);
    });
  }
  WebAssembly.instantiate(wasm_bytes, imports, {
    builtins
  })
    .then(async ({ instance }) => {
      const {
        flag_clicked,
        tick,
        memory,
        strings,
        step_funcs,
        vars_num,
        threads_count,
        requests_refresh,
        upc,
        threads,
        noop,
        unreachable_dbg
      } = instance.exports;
      strings.grow(Object.entries(string_consts).length);
      for (const [i, str] of Object.entries(string_consts || {})) {
        // @ts-ignore
        strings.set(i, str);
      }
      updatePenColor = (i) => null;//upc(i - 1);
      strings_tbl = strings;
      window.memory = memory;
      window.flag_clicked = flag_clicked;
      window.tick = tick;
      window.stop = () => {
        if (typeof threads == "undefined") {
          let memArr = new Uint32Array(memory.buffer);
          for (let i = 0; i < threads_count.value; i++) {
            memArr[i] = 0;
          }
        } else {
        for (let i = 0; i < threads.length; i++) {
          threads.set(i, noop);
        }
      }
        threads_count.value = 0;
      };
      // @ts-ignore
      //sprite_info_offset = vars_num.value * 16 + thn_offset + 4;
      const dv = new DataView(memory.buffer);
      /*for (let i = 0; i < target_names.length - 1; i++) {
        dv.setFloat32(
          sprite_info_offset + i * spriteInfoLen + 16,
          66.66,
          true
        );
        dv.setFloat32(sprite_info_offset + i * spriteInfoLen + 20, 100, true);
        dv.setFloat32(sprite_info_offset + i * spriteInfoLen + 24, 100, true);
        dv.setFloat32(sprite_info_offset + i * spriteInfoLen + 28, 0, true);
        dv.setFloat32(sprite_info_offset + i * spriteInfoLen + 40, 1, true);
        dv.setFloat32(sprite_info_offset + i * spriteInfoLen + 44, 1, true);
        dv.setFloat64(sprite_info_offset + i * spriteInfoLen + 48, 1, true);
      }*/
      try {
        // expose the module to devtools
        unreachable_dbg();
      } catch (error) {
        console.info('synthetic error to expose wasm modulee to devtools:', error)
      }
      flag_clicked();
      start_time = Date.now();
      console.log("green_flag()");
      $outertickloop: while (true) {
        renderer.draw();
        const thisTickStartTime = Date.now();
        // @ts-ignore
        $innertickloop: do {//for (const _ of [1]) {
          // @ts-ignore
          tick();
          // @ts-ignore
          if (threads_count.value === 0) {
            break $outertickloop;
          }
        } while (
          (Date.now() - thisTickStartTime) < (framerate_wait * 0.8) &&
          (!turbo && requests_refresh.value === 0)
        )
        // @ts-ignore
        requests_refresh.value = 0;
        if (framerate_wait > 0) {
          await sleep(
            Math.max(0, framerate_wait - (Date.now() - thisTickStartTime))
          );
        } else {
          await waitAnimationFrame();
        }
      }
    })
    .catch((e) => {
      throw new Error("error when instantiating module:\n" + e.stack);
      /*exit(1);*/
    });
};
