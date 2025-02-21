<template>
    <header>
        <h1>Compiler settings</h1>
    </header>
    <main>
        <div v-for="id in Object.keys(settings)" class="setting-box">
            <div class="description">
                <h3 class="green">{{ id }}</h3>
                <p>{{ settingsInfo[id].description }}</p>
            </div>
            <div>
                <input type="checkbox" v-model="settings[id]" v-if="settingsInfo[id].type === 'checkbox'">
                <span v-if="settingsInfo[id].type === 'radio'" v-for="option in settingsInfo[id].options">
                    <input type="radio" v-model="settings[id]" :value="option" /> {{ option }}<br />
                </span>
            </div>
        </div>
    </main>
</template>

<script setup>
import { reactive, watch } from 'vue';
import { getSettings, saveSettings, settingsInfo, WasmFlags } from '../lib/settings.js';

console.log(settingsInfo)

const settings = reactive(getSettings().to_js());

console.log(settings)

watch(settings, () => {
    console.log(settings);
    saveSettings(WasmFlags.from_js(settings))
});
</script>

<style sccoped>
div.setting-box {
    display: flex;
    flex-direction: row;
    align-items: center;
    width: max-content;

    & div {
        padding: 1em
    }
}

h3 {
    text-decoration: underline;
}
</style>