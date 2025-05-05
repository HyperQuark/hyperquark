<template>
    <header>
        <h1>Compiler settings</h1>
    </header>
    <main>
        <button @click="resetSettings()">Reset all to default</button>
        <h4>Currently used WASM features:</h4>
        <ul>
            <li v-for="feature in wasmFeatures">{{ feature }}</li>
            <li v-if="wasmFeatures.size === 0">None</li>
        </ul>
        <br>
        <hr>
        <div v-for="id in Object.keys(settings)" :key="id">
            <h3 class="green" :title="id" :id="'setting-'.concat(id)">
                <RouterLink :to="{ hash: '#setting-'.concat(id) }">
                    {{ settingsInfo[id].flag_info.name }}
                </RouterLink>
            </h3>
            <div class="setting-box">
                <p v-html="settingsInfo[id].flag_info.description"></p>
                <div>
                    <input type="checkbox" v-model="settings[id]" v-if="settingsInfo[id].type === 'checkbox'">
                    <span v-if="settingsInfo[id].type === 'radio'" v-for="option in settingsInfo[id].options">
                        <input type="radio" v-model="settings[id]" :value="option" /> {{ option }}<br />
                    </span>
                </div>
            </div>
        </div>
    </main>
</template>

<script setup>
import { reactive, watch, nextTick, onMounted, ref } from 'vue';
import { RouterLink, useRoute } from 'vue-router';
import { getSettings, saveSettings, settingsInfo, WasmFlags, getUsedWasmFeatures, defaultSettings } from '../lib/settings.js';

console.log(settingsInfo)
console.log(defaultSettings.to_js())

const settings = reactive(getSettings().to_js());
const route = useRoute();
console.log(getUsedWasmFeatures())
const wasmFeatures = ref(getUsedWasmFeatures());
console.log(wasmFeatures)

console.log(settings);

function scrollToAnchor() {
    if (!!route.hash) {
        document.getElementById(route.hash.substring(1))?.scrollIntoView?.({ behavior: 'smooth' });
    }
}

watch(() => route.hash, () => {
    nextTick().then(scrollToAnchor);
});

onMounted(scrollToAnchor);

watch(settings, (value, oldValue) => {
    console.log(settings);
    saveSettings(WasmFlags.from_js(settings));
});

watch(settings, () => {
    wasmFeatures.value = getUsedWasmFeatures();
    console.log(wasmFeatures);
}, {
    immediate: true
});

function resetSettings() {
    Object.assign(settings, defaultSettings.to_js());
}
</script>

<style sccoped>
div.setting-box {
    display: flex;
    flex-direction: row;
    align-items: center;
    width: max-content;
    margin-top: 0;

    & div {
        padding: 0 1em;
    }

    & div,
    & p {
        margin-top: 0.5rem;
    }
}

div.description {
    border-right: 1px solid var(--color-border);
}

h3 {
    text-decoration: underline;
}

h3::before {
    content: "#";
    margin-right: 2px;
    opacity: 0;
    transition: var(--transition-time);
}

h3:hover a {
    background-color: initial;
}

h3:hover::before {
    opacity: 1;
}

hr {
    border: 1px solid var(--color-border);
}

ul {
    list-style: none;
    padding: inherit;
}
</style>