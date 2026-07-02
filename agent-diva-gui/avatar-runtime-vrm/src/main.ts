import { mountDemo } from './demo/app'
import './demo/styles.css'

const app = document.querySelector<HTMLElement>('#app')

if (!app) {
  throw new Error('Missing #app mount point')
}

void mountDemo(app)
