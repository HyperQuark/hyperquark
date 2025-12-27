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

import { describe, test } from "vitest";

import { imports as baseImports } from "../../js/imports.ts";
import { unpackProject } from "../../playground/lib/project-loader.js";
import { instantiateProject } from "../../playground/lib/project-runner.js";
import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";
import { WasmStringType } from "../../js/no-compiler/hyperquark";
import { defaultSettings } from "../../playground/lib/settings.js";

/**
 * @fileoverview Transform each sb2 in test/fixtures/execute into a test.
 *
 * Test execution of a group of scratch blocks by SAYing if a test did "pass",
 * or did "fail". Two keywords can be set at the beginning of a SAY messaage
 * to indicate a test primitive.
 *
 * - "pass MESSAGE" will report a passing test with MESSAGE.
 * - "fail MESSAGE" will report a failing test with MESSAGE.
 *
 * A good strategy to follow is to SAY "pass" or "fail" depending on expected
 * scratch results in conditions, event scripts, or what is best for testing
 * the target block or group of blocks.
 */

const executeDir = path.resolve(__dirname, "../fixtures/execute");

// Find files which end in ".sb", ".sb2", or ".sb3"
const fileFilter = /\.sb[23]?$/i;

const makeTestDrawable = () => ({
  updateVisible() {},
  updatePosition() {},
  updateDirection() {},
  updateScale() {},
});

const makeTestRenderer = () =>
  new Proxy(
    {
      draw() {},
      updateTextSkin() {},
      setLayerGroupOrdering() {},
      getDrawable: () => makeTestDrawable(),
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
        if (p === "getDrawable") return true;
        return Reflect.set(t, p, v);
      },
    },
  );

describe("Integration tests", () => {
  const files = fs
    .readdirSync(executeDir)
    .filter((uri) => fileFilter.test(uri))
    // ignore tests that crash the runner, or that test custom reporters
    .filter(
      (uri) =>
        ![
          "tw-comparison-matrix-inline.sb3",
          "tw-comparison-matrix-runtime.sb3",
          "tw-unsafe-equals.sb3",
          "tw-custom-report-repeat.sb3",
          "tw-procedure-return-non-existant.sb3",
          "tw-procedure-return-recursion.sb3",
          "tw-procedure-return-simple.sb3",
          "tw-procedure-return-stops-scripts.sb3",
          "tw-procedure-return-warp.sb3",
          "tw-repeat-procedure-reporter-infinite-analyzer-loop.sb3",
        ].includes(uri),
    );
  for (const uri of files) {
    test.sequential(`${uri} (default flags)`, async () => {
      let plannedCount = 0;
      let testCount = 0;
      let didEnd = false;
      const testResults = { passes: [], failures: [] };
      const reporters = {
        comment(message) {
          console.log(`[${uri}]`, message);
        },
        pass(reason) {
          testCount++;
          testResults.passes.push(reason);
          console.log(`[${uri}] pass:`, reason);
        },
        fail(reason) {
          testCount++;
          testResults.failures.push(reason);
          console.log(`[${uri}] fail:`, reason);
        },
        plan(count) {
          plannedCount = Number(count);
          console.log(`[${uri}] planned ${plannedCount} tests`);
        },
        end() {
          didEnd = true;
          console.log(`[${uri}] test ended`);
        },
      };

      const reportVmResult = (text) => {
        const command = text.split(/\s+/, 1)[0].toLowerCase();
        if (reporters[command]) {
          return reporters[command](text.substring(command.length).trim());
        }

        // Default to a comment with the full text if we didn't match
        // any command prefix
        return reporters.comment(text);
      };

      const projectBuffer = Buffer.from(
        fs.readFileSync(path.join(executeDir, uri)),
      );

      const [project_json, _] = await unpackProject(projectBuffer);
      // console.log(JSON.stringify(project_json, null, 2));
      console.log("loaded project");
      await new Promise((resolve) => setTimeout(resolve, 10));
      const project_wasm = sb3_to_wasm(
        JSON.stringify(project_json, null, 2),
        WasmFlags.from_js(defaultSettings.to_js()),
      );
      await new Promise((resolve) => setTimeout(resolve, 10));

      console.log("compiled project");
      await new Promise((resolve) => setTimeout(resolve, 10));

      // todo: run wasm-opt if specified in flags?

      // Run the project and once all threads are complete check the results.
      const runner = await instantiateProject({
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
        onTimeout: () => {
          throw new Error(`Timeout waiting for threads to complete: ${uri}`);
        },
        importOverrides: {
          looks: {
            say_string: (string) => reportVmResult(string),
          },
        },
        makeRenderer: makeTestRenderer,
      });

      runner.flag_clicked();

      await runner.run();

      // Verify test end was called
      if (!didEnd) {
        throw new Error(`Test did not call "end"`);
      }

      // If a plan was specified, verify we ran the planned number of tests
      if (plannedCount > 0 && testCount !== plannedCount) {
        throw new Error(`Expected ${plannedCount} tests, but ran ${testCount}`);
      }

      // All failures should be reported
      if (testResults.failures.length > 0) {
        throw new Error(`Test failures: ${testResults.failures.join(", ")}`);
      }
    });
  }
});
