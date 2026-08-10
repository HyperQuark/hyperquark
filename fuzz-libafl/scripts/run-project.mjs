import { makeTestRunner } from "../../test/integration/test-run-project.mjs";
import { defaultSettings } from "../../playground/lib/settings.js";

export async function run(project_json, opts = {}) {
  let settings = Object.assign(defaultSettings.to_js(), opts);

  const runner = await makeTestRunner(project_json, {}, settings);

  runner.addEventListener("timeout", () => {
    throw new Error(`Timeout waiting for threads to complete: ${uri}`);
  });

  runner.flag_clicked();

  await runner.run();
}
