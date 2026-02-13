import { getSettings } from "./settings.js";
import { imports as baseImports } from "./imports.js";
import {
  renderer as get_renderer,
  stageIndex,
  target_skins,
} from "../../js/shared.ts";
import { WasmStringType } from "../../js/no-compiler/hyperquark.js";
import { setup } from "./setup.js";

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

export class ProjectRunner extends EventTarget {
  #sensing_answer;
  #mark_question_resolved_func;
  #running = false;
  #renderer;
  #tick;
  #timeout;
  #framerate_wait;
  #requests_refresh;
  turbo;
  #sensing_timer;
  #threads_count;
  flag_clicked;
  #threads;
  #mouseX;
  #mouseY;
  #mouseDown;
  #triggerSpriteClicked;

  constructor({ renderer, timeout, framerate_wait, turbo, exports }) {
    super();

    this.#sensing_answer = exports.sensing_answer;
    this.#mark_question_resolved_func = exports.mark_waiting_flag;
    this.#renderer = renderer;
    this.#tick = exports.tick;
    this.#timeout = timeout;
    this.#framerate_wait = framerate_wait;
    this.#requests_refresh = exports.requests_refresh;
    this.turbo = turbo;
    this.#sensing_timer = exports.sensing_timer;
    this.#threads_count = exports.threads_count;
    this.flag_clicked = exports.flag_clicked;
    this.#threads = exports.threads;
    this.#mouseX = exports.mouseX ?? { value: 0 };
    this.#mouseY = exports.mouseY ?? { value: 0 };
    this.#mouseDown = exports.mouseDown ?? { value: false };
    this.#triggerSpriteClicked = exports.trigger_sprite_clicked;
  }

  static async init({
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
    importOverrides,
  }) {
    if (isDebug() && typeof window === "object")
      window.open(
        URL.createObjectURL(
          new Blob([wasm_bytes], { type: "application/wasm" }),
        ),
      );

    await setup(
      makeRenderer,
      project_json,
      assets,
      target_names,
      (question, struct) => {
        // runner doesn't exist yet, but it will by the time the function is called
        runner.dispatchEvent(
          new CustomEvent("queueQuestion", { detail: { question, struct } }),
        );
      },
    );

    const renderer = get_renderer();

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

    for (const [module, _obj] of Object.entries(importOverrides ?? {})) {
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
      sensing_timer,
      mark_waiting_flag,
      sensing_answer,
      mouseX,
      mouseY,
      mouseDown,
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

    const runner = new ProjectRunner({
      renderer,
      exports: instance.exports,
      timeout,
      framerate_wait,
      turbo,
    });

    return runner;
  }

  async run() {
    console.log("running");
    if (this.#running) return;

    this.#running = true;

    this.#renderer.draw();

    let startTime = Date.now();
    let previousTickStartTime = startTime;
    $outertickloop: while (this.#running) {
      if (this.#timeout && Date.now() - startTime > this.#timeout) {
        return this.dispatchEcent(new CustomEvent("timeout"));
      }
      let thisTickStartTime = Date.now();
      if (typeof this.#sensing_timer !== "undefined") {
        this.#sensing_timer.value +=
          (thisTickStartTime - previousTickStartTime) / 1000;
      }
      previousTickStartTime = thisTickStartTime;
      do {
        this.#tick();
        if (this.#threads_count.value === 0) {
          break $outertickloop;
        }
      } while (
        Date.now() - thisTickStartTime < this.#framerate_wait * 0.8 &&
        !this.turbo &&
        this.#requests_refresh.value === 0
      );
      this.#requests_refresh.value = 0;
      this.#renderer.draw();
      if (this.#framerate_wait > 0) {
        await sleep(
          Math.max(0, this.#framerate_wait - (Date.now() - thisTickStartTime)),
        );
      } else {
        await waitAnimationFrame();
      }
    }
    await waitAnimationFrame();
    this.#renderer.draw();
    this.#running = false;
    console.log("project stopped (or maybe paused)");
  }

  greenFlag() {
    console.log("green flag clicked");
    if (typeof this.#sensing_timer !== "undefined") {
      this.#sensing_timer.value = 0.0;
    }
    this.flag_clicked();
    this.run();
  }

  pause() {
    this.#running = false;
  }

  stop() {
    console.log("stopping");
    this.#threads_count.value = 0;
    this.#running = false;
    for (let i = 0; i < this.#threads.length; i++) {
      this.#threads.set(i, null);
    }
    this.dispatchEvent(new CustomEvent("stopped"));
  }

  mark_question_resolved(struct) {
    this.#mark_question_resolved_func(struct);
  }

  setAnswer(answerText) {
    this.#sensing_answer.value = answerText;
  }

  onMouseMove({ clientX, clientY, rect, isDown }) {
    const x = clamp((clientX - rect.left) / rect.width, 0, 1) * 480 - 240;
    const y = clamp((clientY - rect.top) / rect.height, 0, 1) * 360 - 180;
    this.#mouseX.value = x;
    this.#mouseY.value = y;

    if (typeof isDown !== "undefined") {
      const prevIsDown = this.#mouseDown.value;
      if (prevIsDown && !isDown) {
        // TODO: update 'this sprite clicked?' values to be not clicked
      }
      if (!prevIsDown && isDown) {
        const clickedTarget = this.#pickMouseOverTarget(
          clientX - rect.left,
          clientY - rect.top,
        );
        this.#triggerSpriteClicked?.(clickedTarget);
        if (!this.#running) this.run();
      }

      this.#mouseDown.value = isDown;
    }

    if (this.#mouseDown.value) {
      // TODO: update 'this sprite clicked?' values to be maybe clicked depending on mouse position
    }
  }

  #pickMouseOverTarget(x, y) {
    // adapted from https://github.com/scratchfoundation/scratch-vm/blob/8dbcc1f/src/io/mouse.js#L40
    // (licensed under BSD-3.0 - see https://raw.githubusercontent.com/scratchfoundation/scratch-vm/8dbcc1f/LICENSE)
    const drawableID = this.#renderer.pick(x, y);
    const targetSkins = target_skins();
    for (let i = 0; i < targetSkins.length; i++) {
      const thisDrawableID = targetSkins[i][1];
      if (thisDrawableID === drawableID) {
        return i;
      }
    }
    // Return the stage if no target was found
    return stageIndex();
  }
}

function clamp(val, min, max) {
  return Math.max(Math.min(val, max), min);
}
