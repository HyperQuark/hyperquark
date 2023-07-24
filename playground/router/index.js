import { createRouter, createWebHistory } from 'vue-router';

const view = (name) => () => import(`../views/${name}.vue`);

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
