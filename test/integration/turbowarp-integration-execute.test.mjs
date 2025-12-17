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

import { imports } from "../../js/imports.ts";
import { unpackProject } from "../../playground/lib/project-loader.js";
import { sb3_to_wasm, WasmFlags } from "../../js/compiler/hyperquark.js";
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

// this is adapted from playground/lib/project-runner.js to work in node
// todo: adapt so we can just import it?
function runProject({ wasm_bytes, settings, reportVmResult, timeoutFailure }) {
  const framerate_wait = Math.round(1000 / 30);

  const builtins = [...(settings["js-string-builtins"] ? ["js-string"] : [])];

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
      setTimeout(resolve, 1);
    });
  }
  imports.looks.say_string = reportVmResult;
  return WebAssembly.instantiate(wasm_bytes, imports, {
    builtins,
    importedStringConstants: "",
  }).then(async ({ instance }) => {
    const {
      flag_clicked,
      tick,
      memory,
      threads_count,
      requests_refresh,
      threads,
      noop,
    } = instance.exports;
    flag_clicked();

    let start_time = Date.now();
    let thisTickStartTime;
    $outertickloop: while (true) {
      if (Date.now() - start_time > 5000) {
        return timeoutFailure();
      }
      thisTickStartTime = Date.now();
      // renderer.draw();
      $innertickloop: do {
        tick();
        if (threads_count.value === 0) {
          break $outertickloop;
        }
      } while (
        Date.now() - thisTickStartTime < framerate_wait * 0.8 &&
        requests_refresh.value === 0
      );
      requests_refresh.value = 0;
      if (framerate_wait > 0) {
        await sleep(
          Math.max(0, framerate_wait - (Date.now() - thisTickStartTime)),
        );
      } else {
        await waitAnimationFrame();
      }
    }
    return;
  });
}

const executeDir = path.resolve(__dirname, "../fixtures/execute");

// Find files which end in ".sb", ".sb2", or ".sb3"
const fileFilter = /\.sb[23]?$/i;

describe("Integration tests", () => {
  const files = fs
    .readdirSync(executeDir)
    .filter((uri) => fileFilter.test(uri))
    // ignore tests that crash the runner, for now
    .filter(
      (uri) =>
        !["tw-comparison-matrix-inline.sb3", "tw-unsafe-equals.sb3"].includes(
          uri,
        ),
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
      // console.log(JSON.stringify(project_json, null, 2))
      const project_wasm = sb3_to_wasm(
        JSON.stringify(project_json, null, 2),
        WasmFlags.from_js(defaultSettings.to_js()),
      );

      // todo: run wasm-opt if specified in flags?

      // Run the project and once all threads are complete check the results.
      await runProject({
        wasm_bytes: project_wasm.wasm_bytes,
        target_names: project_wasm.target_names,
        settings: defaultSettings,
        reportVmResult,
        timeoutFailure: () => {
          throw new Error(`Timeout waiting for threads to complete: ${uri}`);
        },
      });

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
