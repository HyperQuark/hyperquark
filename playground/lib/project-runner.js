import { getSettings } from "./settings.js";
import { imports as baseImports } from "./imports.js";
import { renderer as get_renderer } from "../../js/shared.ts";
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

  constructor({
    sensing_answer,
    mark_question_resolved,
    renderer,
    tick,
    timeout,
    framerate_wait,
    requests_refresh,
    turbo,
    threads_count,
    flag_clicked,
    threads,
    sensing_timer,
  }) {
    super();

    this.#sensing_answer = sensing_answer;
    this.#mark_question_resolved_func = mark_question_resolved;
    this.#renderer = renderer;
    this.#tick = tick;
    this.#timeout = timeout;
    this.#framerate_wait = framerate_wait;
    this.#requests_refresh = requests_refresh;
    this.turbo = turbo;
    this.#sensing_timer = sensing_timer;
    this.#threads_count = threads_count;
    this.flag_clicked = flag_clicked;
    this.#threads = threads;
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
      sensing_answer,
      mark_question_resolved: mark_waiting_flag,
      renderer,
      tick,
      timeout,
      framerate_wait,
      requests_refresh,
      turbo,
      sensing_timer,
      threads_count,
      flag_clicked,
      threads,
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
}
