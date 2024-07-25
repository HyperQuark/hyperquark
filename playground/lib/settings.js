const defaultSettings = {
    'js-string-builtins': true,
}

export const settingsInfo = {
    'js-string-builtins': {
        name: 'Use the JS String Builtins proposal (where possible)',
        description: 'May achieve a slight performance gain in some string operations',
        type: 'switch'
    }
}

export function getSettings(){
    let store = localStorage["settings"];
    try {
        return { ...defaultSettings, ...JSON.parse(store) };
    } catch {
        return defaultSettings;
    }
}

export function saveSettings(settings){
    console.log(settings)
    localStorage['settings'] = JSON.stringify(settings);
}