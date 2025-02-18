import { createRouter, createWebHistory, createWebHashHistory } from 'vue-router';
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
  history: createWebHashHistory(import.meta.env.BASE_URL),
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
      path: '/projects/test',
      name: 'testProjectPlayer',
      component: view('TestProject'),
    },
    {
      path: '/about',
      name: 'about',
      component: view('AboutView'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: view('Settings'),
    }
  ]
})

export default router;
