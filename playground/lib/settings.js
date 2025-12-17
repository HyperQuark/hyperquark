import * as hyperquarkExports from '../../js/no-compiler/hyperquark.js';
import { WasmFlags, WasmFeature, all_wasm_features, wasm_feature_detect_name } from '../../js/no-compiler/hyperquark.js';
import * as WasmFeatureDetect from 'wasm-feature-detect';
export { WasmFlags };


console.log(WasmFeature);
if (typeof window !== "undefined") window.hyperquarkExports = hyperquarkExports;

export const supportedWasmFeatures = await getSupportedWasmFeatures();
export const defaultSettings = new WasmFlags(Array.from(supportedWasmFeatures, (feat) => WasmFeature[feat]));
const defaultSettingsObj = defaultSettings.to_js();

if (typeof window !== "undefined") window.defaultSettings = defaultSettings;

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
        let flag_info = WasmFlags.flag_info(key);
        return [key, {
            flag_info,
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

/**
 * @returns {Set<string>}
 */
export async function getSupportedWasmFeatures() {
    const featureSet = new Set();
    for (const feature of all_wasm_features()) {
        if (await WasmFeatureDetect[wasm_feature_detect_name(feature)]()) {
            featureSet.add(WasmFeature[feature]);
        }
    }
    return featureSet;
}

console.log(await getSupportedWasmFeatures())

/**
 * @returns {Set<string>}
 */
export function getUsedWasmFeatures() {
    const settings = getSettings().to_js();
    console.log(settings)
    const featureSet = new Set();
    for (const [id, info] of Object.entries(settingsInfo)) {
        const theseFeatures = info.flag_info.wasm_features(settings[id])?.map?.((num) => WasmFeature[num]);
        for (const feature of theseFeatures || []) {
            featureSet.add(feature);
        }
    }
    return featureSet;
}