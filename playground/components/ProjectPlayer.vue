<template>
  <div v-show="loaded">
    <h1>{{ props.title || "untitled" }}</h1>
    <span>by {{ props.author || "unknown" }}</span>
    <div v-if="!!id">
      <a :href="`https://scratch.mit.edu/projects/${props.id}/`"
        >View on Scratch</a
      >
      | <a :href="`https://turbowarp.org/${props.id}/`">View on TurboWarp</a>
    </div>
    <details v-if="error">
      <summary>
        {{ errorMode }} was emitted whilst {{ errorStage }} project.
      </summary>
      <span v-html="error"></span>
    </details>
    <div>
      <button @click="greenFlag">green flag</button>
      <button @click="stop">stop</button>
      <input type="checkbox" id="turbo" :value="turbo" />
      <label for="turbo">turbo mode</label>
    </div>
    <div id="stage-container">
      <canvas width="480" height="360" ref="canvas"></canvas>
      <div
        v-for="[id, { visible, x, y, name, sprite, value }] in Object.entries(
          monitors,
        )"
        :key="id"
        class="variable-monitor"
        v-show="visible"
        :style="{
          left: x.toString() + 'px',
          top: y.toString() + 'px',
        }"
      >
        <span>
          <span v-if="!!sprite">{{ sprite }}: </span>{{ name }}
        </span>
        <span class="variable-value">{{ value }}</span>
      </div>
      <div v-show="queued_questions.length > 0" id="question-div">
        <div v-if="!!queued_questions[0]?.[0]?.length">
          {{ queued_questions[0]?.[0] }}
        </div>
        <form @submit.prevent="submitQuestion">
          <input
            type="text"
            name="answer"
            v-model="question_response"
            ref="questionInput"
            autocomplete="off"
          />
          <button type="submit">✓</button>
        </form>
      </div>
    </div>
    <div class="instructions">
      <div v-if="props.instructions">
        <h2>Instructions</h2>
        {{ props.instructions }}
      </div>
      <div v-if="props.description">
        <h2>Notes and credits</h2>
        {{ props.description }}
      </div>
    </div>
  </div>
  <Loading v-if="!loaded">{{ loadingMsg }}</Loading>
</template>

<script setup>
import Loading from "./Loading.vue";
import { ProjectRunner } from "../lib/project-runner.js";
import { ref, onMounted, reactive, watch, onBeforeUnmount } from "vue";
import { getSettings } from "../lib/settings.js";
import { useDebugModeStore } from "../stores/debug.js";
import { unsetup } from "../../js/shared.js";

const debugModeStore = useDebugModeStore();

const props = defineProps([
  "json",
  "title",
  "author",
  "assets",
  "zip",
  "instructions",
  "description",
  "id",
]);

let error = ref(null);
let errorStage = ref("loading");
let errorMode = ref("An error");
let turbo = ref(false);
let canvas = ref(null);
let loadingMsg = ref("compiling project");
let loaded = ref(false);
let questionInput = ref(null);
let monitors = ref({});

let greenFlag = () => null;
let stop = () => null;
let success = true;

const declareError = (e, terminate, mode, stage, extra) => {
  console.error(e);
  errorMode.value = mode;
  errorStage.value = stage;
  error.value = e.toString();
  if (e.stack) {
    error.value += "<br>" + e.stack;
  }
  if (extra) {
    error.value += extra;
  }
  if (terminate) {
    success = false;
    loaded.value = true;
  }
};

const queued_questions = reactive([]);
let question_response = ref("");
let mark_question_resolved = () => {};

let setAnswer = () => {};

function submitQuestion() {
  setAnswer(question_response.value);
  question_response.value = "";
  const [_, struct] = queued_questions.shift();
  mark_question_resolved(struct);
}

watch(queued_questions, () => {
  if (queued_questions.length > 0) {
    questionInput.value.focus();
  }
});

function queue_question(question, struct) {
  queued_questions.push([question, struct]);
}

let mouseMove, mouseDown, mouseUp, keyDown, keyUp, runner, compileWorker;

onBeforeUnmount(() => {
  document.removeEventListener("mousemove", mouseMove);
  canvas.value.removeEventListener("mousedown", mouseDown);
  canvas.value.removeEventListener("mouseup", mouseUp);
  document.removeEventListener("keydown", keyDown);
  document.removeEventListener("keyup", keyUp);
  runner?.stop?.();
  unsetup();
  compileWorker?.terminate?.();
});

onMounted(async () => {
  const load_asset = async (md5ext) => {
    try {
      if (props.zip) {
        console.log(props.zip);
        const file = props.zip.file(md5ext) ?? props.zip.files[md5ext];
        const data = await file.async("text"); //.then(console.log);
        //console.log(file, data);
        return data;
      }
      return await (
        await fetch(
          `https://assets.scratch.mit.edu/internalapi/asset/${md5ext}/get/`,
        )
      ).text();
    } catch (e) {
      error.value = `failed to load asset ${md5ext}\n${e.stack}`;
    }
  };

  let wasmBytes;
  let assets = null;
  let wasmProject;

  console.log(props);

  try {
    // imports can take a long time, so need to wait for worker to tell us that it's initialised
    await new Promise((resolve) => {
      compileWorker = new Worker(
        new URL("../lib/compile-worker.js", import.meta.url),
        { type: "module" },
      );
      compileWorker.onmessage = resolve;
    });
    wasmProject = await new Promise((resolve, reject) => {
      compileWorker.onmessage = ({ data }) => resolve(data);
      compileWorker.onerror = (e) => {
        reject(e.message);
      };
      compileWorker.postMessage({
        stage: "compile",
        proj: JSON.stringify(props.json),
        flags: getSettings().to_js(),
      });
      console.log("compile message posted!");
    });

    console.log(
      wasmProject.target_names,
      wasmProject.strings,
      wasmProject.wasm_bytes,
    );

    // if ((!wasmProject instanceof FinishedWasm)) {
    //   throw new Error("unknown error occurred when compiling project");
    // }

    wasmBytes = wasmProject.wasm_bytes;
  } catch (e) {
    declareError(e, true, "An error", "compiling");
  }

  if (!success) return;

  if (getSettings().to_js().wasm_opt == "On") {
    try {
      loadingMsg.value = "optimising project";
      wasmBytes = await new Promise((resolve, reject) => {
        compileWorker.onmessage = ({ data }) => resolve(data.wasmBytes);
        compileWorker.onerror = (e) => {
          reject(e.message);
        };
        compileWorker.postMessage({
          stage: "optimise",
          wasmBytes: wasmBytes
        }, [wasmBytes.buffer]);
        console.log("optimise message posted!");
      });
    } catch (e) {
      declareError(
        e,
        false,
        "A warning",
        "optimising",
        "<br>See browser console for more info.\
        <brThis might not be an unrecoverable error; the project may play \
        as normal (possibly with worse-than-expected performance).",
      );
    }
  } else {
    console.log("skipping wasm-opt due to user settings");
  }

  try {
    loadingMsg.value = "loading assets";
    const assetsResult = await Promise.all(
      props.json.targets.map(
        (target) =>
          new Promise((r1) =>
            Promise.all(
              target.costumes.map(
                ({ md5ext, dataFormat }) =>
                  new Promise((r2) =>
                    load_asset(md5ext).then((data) =>
                      r2([md5ext, { dataFormat, data }]),
                    ),
                  ),
              ),
            ).then(r1),
          ),
      ),
    );
    assets = Object.fromEntries(assetsResult.flat());
  } catch (e) {
    declareError(e, false, "A warning", "loading assets for");
  }

  try {
    loadingMsg.value = "instantiating project";
    runner = new ProjectRunner();
    await runner.init({
      framerate: 30,
      turbo: turbo.value,
      wasm_bytes: wasmBytes,
      strings: wasmProject.strings,
      target_names: wasmProject.target_names,
      project_json: props.json,
      assets,
      makeRenderer: async () => {
        const scratch_render =
          await import("scratch-render/dist/web/scratch-render.js");
        const RenderWebGL = scratch_render.default;
        return new RenderWebGL(canvas.value);
      },
      isDebug: () => debugModeStore.debug,
    });

    loaded.value = true;

    runner.addEventListener("stopped", () => queued_questions.splice(0));
    runner.addEventListener(
      "queueQuestion",
      ({ detail: { question, struct } }) => queue_question(question, struct),
    );
    runner.addEventListener(
      "updateVariableVal",
      ({ detail: { id, value } }) => {
        monitors.value[id].value = value;
      },
    );
    runner.addEventListener(
      "updateVariableVisibility",
      ({ detail: { id, visible } }) => {
        monitors.value[id].visible = visible;
      },
    );

    const onMouseMove = (e, isDown) => {
      const rect = canvas.value.getBoundingClientRect();
      runner.onMouseMove({
        clientX: e.clientX,
        clientY: e.clientY,
        rect,
        isDown,
      });
    };

    mouseMove = (e) => {
      onMouseMove(e);
    };
    document.addEventListener("mousemove", mouseMove);
    mouseDown = (e) => {
      onMouseMove(e, true);
      e.preventDefault();
    };
    canvas.value.addEventListener("mousedown", mouseDown);
    mouseUp = (e) => {
      onMouseMove(e, false);
      e.preventDefault();
    };
    canvas.value.addEventListener("mouseup", mouseUp);
    keyDown = (e) => {
      runner.onKeyPressChange({
        key: e.key,
        pressed: true,
      });
    };
    document.addEventListener("keydown", keyDown);
    keyUp = (e) => {
      runner.onKeyPressChange({
        key: e.key,
        pressed: false,
      });
    };
    document.addEventListener("keyup", keyUp);

    greenFlag = runner.greenFlag.bind(runner);
    stop = runner.stop.bind(runner);
    setAnswer = runner.setAnswer.bind(runner);
    mark_question_resolved = runner.mark_question_resolved.bind(runner);
    monitors.value = runner.monitors;
    console.log(monitors.value);
  } catch (e) {
    declareError(e, true, "An error", "instantiating");
  }
});
</script>

<style scoped>
canvas {
  border: 1px solid black;
  background: white;
  width: 100%;
  height: 100%;
}

div#stage-container {
  float: left;
  margin-right: 1em;
  margin-bottom: 1.5em;
  width: 480px;
  height: 360px;
  position: relative;

  & > div#question-div {
    width: calc(100% - 1em);
    position: absolute;
    bottom: 0;
    margin: 0.5em;
    padding: 0.5em;
    background: var(--color-background-soft);
    border-radius: 5px;
    box-sizing: border-box;

    & > div {
      padding: 0;
      margin-top: 0;
      line-height: 1em;
      margin-bottom: 0.4em;
    }

    & > form {
      display: flex;

      & > input {
        flex-grow: 100;
        border-radius: 5px;
      }
    }
  }
}

div.instructions {
  border-radius: 1em;
  border: 2px solid var(--color-border);
  padding: 1em;
  margin: 1em;
  width: fit-content;
  max-width: calc((100vw - 1rem) * 0.95);
  float: none;
  overflow: auto;
  min-width: 0;

  & > h2 {
    font-weight: bold;
  }

  white-space: pre-wrap;
}

div.variable-monitor {
  position: absolute;
  background-color: var(--color-background-soft);
  border-radius: 5px;
  padding: 0 0.4em 0.2em 0.4em;

  & span {
    padding: 0;
    margin: 0;
    vertical-align: middle;
  }

  & > span.variable-value {
    background-color: hsl(39.3, 100%, 37%);
    color: var(--color-background);
    border-radius: 5px;
    padding: 0 0.3em;
    margin-left: 0.3em;
  }
}
</style>
