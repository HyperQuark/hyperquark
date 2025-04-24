import * as hyperquarkExports from '../../js/no-compiler/hyperquark.js';
import { WasmFlags, WasmStringType } from '../../js/no-compiler/hyperquark.js';
export { WasmFlags };

const defaultSettings = new WasmFlags();
const defaultSettingsObj = defaultSettings.to_js();

window.defaultSettings = defaultSettings;
window.hyperquarkExports = hyperquarkExports;

function settingsInfoFromType(type) {
    if (type === "boolean") {
        return {
            type: "checkbox"
        }
    } else if (type in hyperquarkExports) {
        return {
            type: "radio",
            options: Object.keys(hyperquarkExports[type]).filter(key => typeof key === 'string' && !/\d+/.test(key)),
            enum_obj: hyperquarkExports[type],
        }
    } else {
        return null;
    }
}

export const settingsInfo = Object.fromEntries(Object.entries(Object.getOwnPropertyDescriptors(WasmFlags.prototype))
    .filter(([_, descriptor]) => typeof descriptor.get === 'function')
    .map(([key, _]) => key)
    .map(key => {
        let flag_info = WasmFlags.flag_info(key).to_js()
        return [key, {
            ...flag_info,
            ...settingsInfoFromType(flag_info.ty)
        }]
    }));

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