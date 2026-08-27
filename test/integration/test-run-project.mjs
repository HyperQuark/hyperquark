/**
 *  This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * Adapted from https://github.com/TurboWarp/scratch-vm/blob/develop/test/integration/execute.js
 */

const fs = require("node:fs");
import path from "node:path";

import { imports as baseImports } from "../../js/imports.ts";
import { unpackProject } from "../../playground/lib/project-loader.js";
import { ProjectRunner } from "../../playground/lib/project-runner.js";
import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";
import { WasmStringType } from "../../js/no-compiler/hyperquark.js";
import { defaultSettings } from "../../playground/lib/settings.js";

const makeTestDrawable = () => ({
  updateVisible() {},
  updatePosition() {},
  updateDirection() {},
  updateScale() {},
});

const makeTestSkin = () => ({
  setSVG() {},
});

const makeTestRenderer = () =>
  new Proxy(
    {
      draw() {},
      updateTextSkin() {},
      setLayerGroupOrdering() {},
      getDrawable: () => makeTestDrawable(),
      getSkin: () => makeTestSkin(),
      penClear() {},
      penLine() {},
      penPoint() {},
      createDrawable: () => 0,
      createPenSkin: () => 0,
      createSVGSkin: () => 0,
      createTextSkin: () => 0,
      updateDrawableSkinId() {},
    },
    {
      set(t, p, v) {
        if (p === "getDrawable" || p === "getSkin") return true;
        return Reflect.set(t, p, v);
      },
    },
  );

export const makeTestRunner = async (project_json, importOverrides, settings) => {
  let project_wasm;

  project_wasm = sb3_to_wasm(
    JSON.stringify(project_json, null, 2),
    WasmFlags.from_js(settings ?? defaultSettings.to_js()),
  );

  // todo: run wasm-opt if specified in flags?

  // Run the project and once all threads are complete check the results.
  const runner = new ProjectRunner();
  await runner.init({
    wasm_bytes: project_wasm.wasm_bytes,
    target_names: project_wasm.target_names,
    project_json,
    strings: project_wasm.strings,
    settings: defaultSettings,
    timeout: 5000,
    assets: new Proxy(
      {},
      {
        get() {
          return {
            dataFormat: "svg",
            data: "",
          };
        },
      },
    ),
    importOverrides,
    makeRenderer: makeTestRenderer,
  });

  return runner;
};
