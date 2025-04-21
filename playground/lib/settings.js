import { WasmFlags, WasmStringType } from '../../js/no-compiler/hyperquark.js';
export { WasmFlags };

const defaultSettings = new WasmFlags();
const defaultSettingsObj = defaultSettings.to_js();

window.defaultSettings = defaultSettings;

// TODO: can this be automated somehow?
const settings_type = {
    string_type: WasmStringType,
    wasm_opt: "boolean",
}

const settings_descriptions = {
    string_type: "How strings should be represented internally.",
    wasm_opt: "Should we try to optimise generated WASM modules using wasm-opt?"
}

function settingsInfoFromType(type) {
    if (typeof type === "object") {
        return {
            type: "radio",
            options: Object.keys(type).filter(key => typeof key === 'string' && !/\d+/.test(key)),
            enum_obj: type
        }
    } else if (type === "boolean") {
        return {
            type: "checkbox"
        }
    } else {
        return null;
    }
}

export const settingsInfo = Object.fromEntries(Object.entries(Object.getOwnPropertyDescriptors(WasmFlags.prototype))
    .filter(([_, descriptor]) => typeof descriptor.get === 'function')
    .map(([key, _]) => key)
    .map(key => [key, { ...settingsInfoFromType(settings_type[key]), description: settings_descriptions[key] }]));

/**
 * @returns {WasmFlags}
 */
export function getSettings() {
    let store = localStorage["settings"];
    try {
        return WasmFlags.from_js({ ...defaultSettingsObj, ...JSON.parse(store) });
    } catch {
        return defaultSettings;
    }
}

/**
 * @param {WasmFlags} settings 
 */
export function saveSettings(settings) {
    console.log(settings.to_js())
    localStorage['settings'] = JSON.stringify(settings.to_js());
}