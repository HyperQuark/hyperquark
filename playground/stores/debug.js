import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useDebugModeStore = defineStore('debugMode', () => {
    const debug = ref(typeof new URLSearchParams(window.location.search).get('debug') === 'string');
    const toggleDebug = () => {
        debug.value = !debug.value;
        if (!erudaEnabled && debug.value) {
            eruda.init();
        }
    }
    let erudaEnabled = false;
    if (debug.value) {
        eruda.init();
        erudaEnabled = true;
    }
    return { debug, toggleDebug };
})
