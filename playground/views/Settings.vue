<template>
    <header>
        <h1>Compiler settings</h1>
    </header>
    <main>
        <div v-for="id in Object.keys(settings)" class="setting-box">
            <div class="description">
                <h3 class="green" >{{ settingsInfo[id].name }}</h3>
                <p>{{ settingsInfo[id].description }}</p>
            </div>
            <div>
                <input type="checkbox" v-model="settings[id]" v-if="settingsInfo[id].type === 'switch'">
            </div>
        </div>
    </main>
</template>

<script setup>
    import { reactive, watch } from 'vue';
    import { getSettings, saveSettings, settingsInfo } from '../lib/settings.js';

    const settings = reactive(getSettings());

    watch(settings, saveSettings);
</script>

<style>
    div.setting-box {
        display: flex;
        flex-direction: row;
        align-items: center;
        width: max-content;
        & div {
            padding: 1em
        }
    }
</style>