function createSkin(renderer, type, layer, ...params) {
    let drawable = renderer.createDrawable(layer.toString());
    let skin = renderer[`create${type}Skin`](...params);
    renderer.updateDrawableSkinId(drawable, skin);
    return skin;
}

// from https://github.com/scratchfoundation/scratch-vm/blob/352b20b5c3f67e44179a0f8043f260b92dd7c392/src/util/color.js#L103
function hsvToRgb (hsv) {
        let h = hsv.h % 360;
        if (h < 0) h += 360;
        const s = Math.max(0, Math.min(hsv.s, 1));
        const v = Math.max(0, Math.min(hsv.v, 1));

        const i = Math.floor(h / 60);
        const f = (h / 60) - i;
        const p = v * (1 - s);
        const q = v * (1 - (s * f));
        const t = v * (1 - (s * (1 - f)));

        let r;
        let g;
        let b;

        switch (i) {
        default:
        case 0:
            r = v;
            g = t;
            b = p;
            break;
        case 1:
            r = q;
            g = v;
            b = p;
            break;
        case 2:
            r = p;
            g = v;
            b = t;
            break;
        case 3:
            r = p;
            g = q;
            b = v;
            break;
        case 4:
            r = t;
            g = p;
            b = v;
            break;
        case 5:
            r = v;
            g = p;
            b = q;
            break;
        }

        return {
            r: Math.floor(r * 255),
            g: Math.floor(g * 255),
            b: Math.floor(b * 255)
        };
    }
// @ts-ignore
export default ({ framerate=30, renderer, wasm_bytes, target_names, string_consts } = { framerate: 30 }) => new Promise((resolve, reject) => {
    const framerate_wait = Math.round(1000 / framerate);
    let assert;
    let exit;
    let browser = false;
    let output_div;
    let text_div;
    // @ts-ignore
    window.renderer=renderer;
    renderer.setLayerGroupOrdering(['background', 'video', 'pen', 'sprite']);
    //window.open(URL.createObjectURL(new Blob([wasm_bytes], { type: "octet/stream" })));
    let sprite_info = Array.from({ length: target_names.length }, _ => ({
      x: 0,
      y: 0,
      pen: {
        color: 66.66,
        saturation: 100,
        brightness: 100,
        transparency: 0,
        color4f: [0, 0, 1, 1],
        size: 1,
      }
    }));
    const pen_skin = createSkin(renderer, 'Pen', 'pen');
    if (typeof require === 'undefined') {
      browser = true;
      output_div = document.querySelector('div#hq-output');
      text_div = txt => Object.assign(document.createElement('div'), { textContent: txt });
      assert = (bool) => {
        if (!bool) {
          throw new AssertionError('Assertion failed');
        }
      }
      exit = _ => null;
    } else {
      exit = process.exit;
      assert = require('node:assert')/*.strict*/;
    }
    let last_output;
    let strings_tbl;
    const renderBubble = createSkin(renderer, 'Text', 'sprite', 'say', '', false);
    const wasm_val_to_js = (type, value_i64) => {
        return type === 0 ? new Float64Array(new BigInt64Array([value_i64]).buffer)[0] : (type === 1 ? Boolean(value_i64) : (type === 2 ? strings_tbl.get(Number(value_i64)) : null));
    };
    const wasm_output = (...args) => {
        const val = wasm_val_to_js(...args);
        if (!browser) {
          console.log('output: \x1b[34m%s\x1b[0m', val);
        } else {
          output_div.appendChild(text_div('output: ' + String(val)));
        }
        last_output = val;
    };
    const assert_output = (...args) => {
        /*assert.equal(last_output, wasm_val_to_js(...args));*/
        const val = wasm_val_to_js(...args);
        if (!browser) {
          console.log('assert: \x1b[34m%s\x1b[0m', val);
        } else {
          output_div.appendChild(text_div('assert: ' + String(val)));
        }
    }
    const targetOutput = (targetIndex, verb, text) => {
      console.log('a',targetIndex,verb,text)
        let targetName = target_names[targetIndex];
        if (!browser) {
          console.log(`\x1b[1;32m${targetName} ${verb}:\x1b[0m \x1b[35m${text}\x1b[0m`);
        } else {
          //output_div.appendChild(text_div(`${targetName} ${verb}: ${text}`));
          renderer.updateTextSkin(renderBubble, verb, text, false)
        }
    };
    function updatePenColor(i) {
        const rgb = hsvToRgb({
            h: sprite_info[i].pen.color * 360 / 100,
            s: sprite_info[i].pen.saturation / 100,
            v: sprite_info[i].pen.brightness / 100
        });
        sprite_info[i].pen.color4f[0] = rgb.r / 255.0;
        sprite_info[i].pen.color4f[1] = rgb.g / 255.0;
        sprite_info[i].pen.color4f[2] = rgb.b / 255.0;
        sprite_info[i].pen.color4f[3] = 1 - sprite_info[i].pen.transparency / 100;
    }
    let start_time = 0;
    let sprite_info_offset = 0;
    const importObject = {
        dbg: {
            log: wasm_output,
            assert: assert_output,
            logi32 (i32) {
                console.log('logi32: \x1b[33m%d\x1b[0m', i32);
                return i32;
            },
        },
        runtime: {
            looks_say: (ty, val, targetIndex) => {console.log('a');targetOutput(targetIndex, 'say', wasm_val_to_js(ty, val))},
            looks_think: (ty, val, targetIndex) => {console.log('b');targetOutput(targetIndex, 'think', wasm_val_to_js(ty, val))},
            operator_equals: (ty1, val1, ty2, val2) => {
                if (ty1 === ty2 && val1 === val2) return true;
                let j1 = wasm_val_to_js(ty1, val1);
                let j2 = wasm_val_to_js(ty2, val2);
                if (typeof j1 === 'string') j1 = j1.toLowerCase();
                if (typeof j2 === 'string') j2 = j2.toLowerCase();
                return j1 == j2;
            },
            operator_random: (lower, upper) => Math.random() * (upper - lower) + lower,
            operator_join: (ty1, val1, ty2, val2) => wasm_val_to_js(ty1, val1).toString() + wasm_val_to_js(ty2, val2).toString(),
            operator_letterof: (idx, ty, val) => wasm_val_to_js(ty, val).toString()[idx - 1] ?? '',
            operator_length: (ty, val) => wasm_val_to_js(ty, val).toString().length,
            operator_contains: (ty1, val1, ty2, val2) => wasm_val_to_js(ty1, val1).toString().toLowerCase().includes(wasm_val_to_js(ty2, val2).toString().toLowerCase()),
            mathop_sin: (n) => parseFloat(Math.sin((Math.PI * n) / 180).toFixed(10)),
            mathop_cos: (n) => parseFloat(Math.cos((Math.PI * n) / 180).toFixed(10)),
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
            sensing_resettimer: () => start_time = Date.now(),
            pen_clear: () => renderer.penClear(pen_skin),
            pen_down: (i) => renderer.penPoint(pen_skin, { diameter: sprite_info[i].pen.size * 2, color4f: sprite_info[i].pen.color4f }, 0, 0),
            pen_up: () => null,
            pen_setcolor: () => null,
            pen_changecolorparam: () => null,
            pen_setcolorparam: (param, val, i) => {
              switch (param) {
                case 'color':
                  sprite_info[i].pen.color = val;
                  break;
                case 'saturation':
                  sprite_info[i].pen.saturation = val;
                  break;
                case 'brightness':
                  sprite_info[i].pen.brightness = val;
                  break;
                case 'transparency':
                  sprite_info[i].pen.transparency = val;
                  break;
                default:
                  console.warn(`can\'t update invalid color param ${param}`)
              }
              updatePenColor(i);
            },
            pen_changesize: () => null,
            pen_setsize: (s, i) => sprite_info[i].pen.size = s,
            pen_changehue: () => null,
            pen_sethue: () => null,
        },
        cast: {
          stringtofloat: parseFloat,
          stringtobool: Boolean,
          floattostring: Number.prototype.toString,
        },
    };
    //const buf = new Uint8Array(wasm_bytes);
    try {
        assert(WebAssembly.validate(wasm_bytes));
    } catch {
        try {
            new WebAssembly.Module(wasm_bytes);
            return reject('invalid WASM module');
        } catch (e) {
            return reject('invalid WASM module: ' + e.message);
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
    WebAssembly.instantiate(wasm_bytes, importObject).then(async ({ instance }) => {
        const { green_flag, tick, memory, strings, step_funcs, vars_num, rr_offset, thn_offset, upc } = instance.exports;
        for (const [i, str] of Object.entries(string_consts)) {
          // @ts-ignore
          strings.set(i, str);
        }
        updatePenColor = upc;
        strings_tbl = strings;
        window.memory=memory;
        // @ts-ignore
        sprite_info_offset = vars_num.value * 12 + thn_offset + 4;
        /*resolve({ strings, green_flag, step_funcs, tick, memory })*/;
        // @ts-ignore
        green_flag();
        start_time = Date.now();
        console.log('green_flag()')
        $outertickloop: while (true) {
           // console.log(new Uint32Array(memory.buffer)[thn_offset.value / 4])
            renderer.draw();
            // console.log('outer')
            const thisTickStartTime = Date.now();
            // @ts-ignore
            $innertickloop: while (Date.now() - thisTickStartTime < 23 && new Uint8Array(memory.buffer)[rr_offset.value] === 0) {
                //console.log('inner')
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
                await sleep(Math.max(0, framerate_wait - (Date.now() - thisTickStartTime)));
            } else {
                await waitAnimationFrame();
            }
        }
    }).catch((e) => {
        reject('error when instantiating module:\\n' + e.stack);
        /*exit(1);*/
    });
});