import { createRouter, createWebHistory } from 'vue-router';
import { h, ref } from 'vue';
import Loading from '../components/Loading.vue';

let componentCache = Object.setPrototypeOf({}, null);

const view = (name) => ({
  setup() {
    let component = componentCache[name];
    const loading = ref(!Boolean(component));
    if (loading.value) {
      import(`../views/${name}.vue`).then((c) => {
        loading.value = false;
        component = c.default;
        componentCache[name] = component;
      });
    }
    return () => loading.value ? h(Loading) : h(component);
  }
});

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: view('HomeView'),
    },
    {
      path: '/projects/:id(\\d+)',
      name: 'projectIdPlayer',
      component: view('ProjectIdView'),
      props: true,
    },
    {
      path: '/projects/file',
      name: 'projectFilePlayer',
      component: view('ProjectFileView'),
    },
    {
      path: '/about',
      name: 'about',
      component: view('AboutView'),
    }
  ]
})

export default router;
