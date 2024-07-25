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

// @ts-ignore
export default async (
  { framerate = 30, renderer, wasm_bytes, target_names, string_consts, project_json, assets } = {
    framerate: 30,
  }
) => {
    if (window.debug) window.open(URL.createObjectURL(new Blob([wasm_bytes], { type: 'application/wasm' })));
    const framerate_wait = Math.round(1000 / framerate);
    let assert;
    let exit;
    let browser = false;
    let output_div;
    let text_div;
    
    renderer.getDrawable = id => renderer._allDrawables[id];
    renderer.getSkin = id => renderer._allSkins[id];
    
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
    if (typeof require === "undefined") {
      browser = true;
      output_div = document.querySelector("div#hq-output");
      text_div = (txt) =>
        Object.assign(document.createElement("div"), { textContent: txt });
      assert = (bool) => {
        if (!bool) {
          throw new AssertionError("Assertion failed");
        }
      };
      exit = (_) => null;
    } else {
      exit = process.exit;
      assert = require("node:assert") /*.strict*/;
    }
    let last_output;
    let strings_tbl;
    const target_bubbles = target_names.map(_ => null);
    
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
    
    const wasm_val_to_js = (type, value_i64) => {
      return type === 0
        ? new Float64Array(new BigInt64Array([value_i64]).buffer)[0]
        : type === 1
        ? Boolean(value_i64)
        : type === 2
        ? strings_tbl.get(Number(value_i64))
        : null;
    };
    const wasm_output = (...args) => {
      const val = wasm_val_to_js(...args);
      if (!browser) {
        console.log("output: \x1b[34m%s\x1b[0m", val);
      } else {
        output_div.appendChild(text_div("output: " + String(val)));
      }
      last_output = val;
    };
    const assert_output = (...args) => {
      /*assert.equal(last_output, wasm_val_to_js(...args));*/
      const val = wasm_val_to_js(...args);
      if (!browser) {
        console.log("assert: \x1b[34m%s\x1b[0m", val);
      } else {
        output_div.appendChild(text_div("assert: " + String(val)));
      }
    };
    const targetOutput = (targetIndex, verb, text) => {
      console.log("a", targetIndex, verb, text);
      let targetName = target_names[targetIndex];
      if (!browser) {
        console.log(
          `\x1b[1;32m${targetName} ${verb}:\x1b[0m \x1b[35m${text}\x1b[0m`
        );
      } else {
        if (target_bubbles[targetIndex] === null) {
          target_bubbles[targetIndex] = createSkin(
            renderer,
            "text",
            "sprite",
            verb,
            text,
            false
          );
        } else {
          renderer.updateTextSkin(target_bubbles[targetIndex][0], verb, text, false);
        }
      }
    };
    let updatePenColor;
    let start_time = 0;
    let sprite_info_offset = 0;
    const importObject = {
      dbg: {
        log: wasm_output,
        assert: assert_output,
        logi32(i32) {
          console.log("logi32: %d", i32);
          //console.log("logi32: \x1b[33m%d\x1b[0m", i32);
          return i32;
        },
      },
      runtime: {
        looks_say: (ty, val, targetIndex) => {
          targetOutput(targetIndex, "say", wasm_val_to_js(ty, val).toString());
        },
        looks_think: (ty, val, targetIndex) => {
          targetOutput(targetIndex, "think", wasm_val_to_js(ty, val).toString());
        },
        /*operator_equals: (ty1, val1, ty2, val2) => {
          if (ty1 === ty2 && val1 === val2) return true;
          let j1 = wasm_val_to_js(ty1, val1);
          let j2 = wasm_val_to_js(ty2, val2);
          if (typeof j1 === "string") j1 = j1.toLowerCase();
          if (typeof j2 === "string") j2 = j2.toLowerCase();
          return j1 == j2;
        },*/
        operator_random: (lower, upper) =>
          Math.random() * (upper - lower) + lower,
        operator_letterof: (idx, str) =>
          str.toString()[idx - 1] ?? "",
        operator_contains: (str1, str2) =>
          str1
            .toString()
            .toLowerCase()
            .includes(str2.toString().toLowerCase()),
        mathop_sin: (n) =>
          parseFloat(Math.sin((Math.PI * n) / 180).toFixed(10)),
        mathop_cos: (n) =>
          parseFloat(Math.cos((Math.PI * n) / 180).toFixed(10)),
        mathop_tan: (n) => {
          /* https://github.com/scratchfoundation/scratch-vm/blob/f1f10e0aa856fef6596a622af72b49e2f491f937/src/util/math-util.js#L53-65 */
          n = n % 360;
          switch (n) {
            case -270:
            case 90:
              return Infinity;
            case -90:
            case 270:
              return -Infinity;
            default:
              return parseFloat(Math.tan((Math.PI * n) / 180).toFixed(10));
          }
        },
        mathop_asin: (n) => (Math.asin(n) * 180) / Math.PI,
        mathop_acos: (n) => (Math.acos(n) * 180) / Math.PI,
        mathop_atan: (n) => (Math.atan(n) * 180) / Math.PI,
        mathop_ln: (n) => Math.log(n),
        mathop_log: (n) => Math.log(n) / Math.LN10,
        mathop_pow_e: (n) => Math.exp(n),
        mathop_pow10: (n) => Math.pow(10, n),
        sensing_timer: () => (Date.now() - start_time) / 1000,
        sensing_resettimer: () => (start_time = Date.now()),
        pen_clear: () => renderer.penClear(pen_skin),
        pen_down: (radius, x, y, r, g, b, a) =>
          renderer.penPoint(
            pen_skin,
            {
              diameter: radius, // awkward variable naming moment
              color4f: [r, g, b, a],
            },
            x,
            y
          ),
        pen_lineto: (radius, x1, y1, x2, y2, r, g, b, a) => renderer.penLine(
          pen_skin,
          {
            diameter: radius,
            color4f: [r,g,b,a],
          },
          x1, y1, x2, y2,
        ),
        pen_up: () => null,
        pen_setcolor: () => null,
        pen_changecolorparam: () => null,
        pen_setcolorparam: (param, val, i) => {
          switch (param) {
            case "color":
              new DataView(memory.buffer).setFloat32(
                sprite_info_offset + (i - 1) * spriteInfoLen + 16,
                val,
                true
              );
              break;
            case "saturation":
              new DataView(memory.buffer).setFloat32(
                sprite_info_offset + (i - 1) * spriteInfoLen + 20,
                val,
                true
              );
              break;
            case "brightness":
              new DataView(memory.buffer).setFloat32(
                sprite_info_offset + (i - 1) * spriteInfoLen + 24,
                val,
                true
              );
              break;
            case "transparency":
              new DataView(memory.buffer).setFloat32(
                sprite_info_offset + (i - 1) * spriteInfoLen + 28,
                val,
                true
              );
              break;
            default:
              console.warn(`can\'t update invalid color param ${param}`);
          }
          updatePenColor(i);
        },
        pen_changesize: () => null,
        pen_changehue: () => null,
        pen_sethue: () => null,
        emit_sprite_pos_change: (i) => {
          const x = new Float64Array(memory.buffer)[(sprite_info_offset + (i - 1) * spriteInfoLen) / 8];
          const y = new Float64Array(memory.buffer)[(sprite_info_offset + (i - 1) * spriteInfoLen + 8) / 8];
          renderer.getDrawable(target_skins[i][1]).updatePosition([x, y]);
        },
        emit_sprite_x_change: (i) => null,
        emit_sprite_y_change: (i) => null,
        emit_sprite_size_change: (i) => {
          const size = new Float64Array(memory.buffer)[(sprite_info_offset + (i - 1) * spriteInfoLen + 64) / 8];
          renderer.getDrawable(target_skins[i][1]).updateScale([size, size]);
        },
        emit_sprite_costume_change: (i) => {
          const costumeNum = new Int32Array(memory.buffer)[(sprite_info_offset + (i - 1) * spriteInfoLen + 60) / 4];
          const costume = costumes[i][costumeNum];
          renderer.getSkin(target_skins[i][0]).setSVG(costume[1]);
        },
        emit_sprite_rotation_change: (i) => {
          const rot = new Float64Array(memory.buffer)[(sprite_info_offset + (i - 1) * spriteInfoLen + 72) / 8]
          renderer.getDrawable(target_skins[i][1]).updateDirection(rot);
        },
        emit_sprite_visibility_change: (i) => {
          renderer.getDrawable(target_skins[i][1]).updateVisible(!!new Uint8Array(memory.buffer)[sprite_info_offset + (i - 1) * spriteInfoLen + 57]);
        },
      },
      cast: {
        stringtofloat: parseFloat,
        stringtobool: Boolean,
        floattostring: (i) => i.toString(),
      },
      // for if the string builtina proposal isn't available
      "wasm:js-string": {
        equals(a, b) {
          return a === b;
        },
        length(a) {
          return a.length;
        },
        concat(a, b){
          return a.concat(b);
        }
      }
    };
    try {
      assert(WebAssembly.validate(wasm_bytes, {
        builtins: ['js-string']
      }));
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
    WebAssembly.instantiate(wasm_bytes, importObject, {
      builtins: ['js-string']
    })
      .then(async ({ instance }) => {
        const {
          green_flag,
          tick,
          memory,
          strings,
          step_funcs,
          vars_num,
          rr_offset,
          thn_offset,
          upc,
        } = instance.exports;
        for (const [i, str] of Object.entries(string_consts)) {
          // @ts-ignore
          strings.set(i, str);
        }
        updatePenColor = (i) => upc(i - 1);
        strings_tbl = strings;
        window.memory = memory;
        window.stop = () => {
          for (let i = 0; i < new Uint32Array(memory.buffer)[thn_offset.value / 4]; i++) {
            new Uint32Array(memory.buffer)[(i + sprite_info_offset + spriteInfoLen * (target_names.length - 1)) / 4] = 0;
          }
        };
        // @ts-ignore
        sprite_info_offset = vars_num.value * 16 + thn_offset + 4;
        const dv = new DataView(memory.buffer);
        for (let i = 0; i < target_names.length - 1; i++) {
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
        }
        green_flag();
        start_time = Date.now();
        console.log("green_flag()");
        $outertickloop: while (true) {
          renderer.draw();
          const thisTickStartTime = Date.now();
          // @ts-ignore
          $innertickloop: while (
            Date.now() - thisTickStartTime < 23 &&
            new Uint8Array(memory.buffer)[rr_offset.value] === 0
          ){//for (const _ of [1]) {
            // @ts-ignore
            tick();
            // @ts-ignore
            if (new Uint32Array(memory.buffer)[thn_offset.value / 4] === 0) {
              break $outertickloop;
            }
          }
          // @ts-ignore
          new Uint8Array(memory.buffer)[rr_offset.value] = 0;
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
