import {
  AVATAR_EVENT_NAMES,
  type AvatarAnimationState,
  type AvatarCharacterSpec,
  type AvatarEventName,
  type AvatarMood,
  type AvatarMotionEntry,
  type AvatarMotionState,
  type AvatarSpeechState,
  type AvatarTransform,
} from '../protocol'
import { createVrmRuntime } from '../runtime'

const MOODS: AvatarMood[] = ['neutral', 'happy', 'sad', 'angry', 'surprised']
const DEFAULT_MODEL_SOURCE = '/vrm/Alice.vrm'

declare global {
  interface Window {
    __demoRuntime?: Awaited<ReturnType<typeof createVrmRuntime>>
  }
}

function formatPayload(payload: unknown): string {
  if (payload === undefined) {
    return 'undefined'
  }
  return JSON.stringify(payload, null, 2)
}

function createEventLogger(output: HTMLElement) {
  return (name: AvatarEventName, payload: unknown) => {
    const timestamp = new Date().toLocaleTimeString()
    const item = document.createElement('div')
    item.className = 'event-log__item'
    item.textContent = `[${timestamp}] ${name} ${formatPayload(payload)}`
    output.prepend(item)

    const items = output.querySelectorAll('.event-log__item')
    if (items.length > 40) {
      items[items.length - 1]?.remove()
    }
  }
}

function bindTransformInputs(
  runtime: Awaited<ReturnType<typeof createVrmRuntime>>,
  inputs: Record<keyof AvatarTransform, HTMLInputElement>,
  textOutput: HTMLElement,
): void {
  const syncText = (transform: AvatarTransform) => {
    textOutput.textContent = JSON.stringify(transform, null, 2)
  }

  const syncInputs = (transform: AvatarTransform) => {
    inputs.scale.value = String(transform.scale)
    inputs.offsetX.value = String(transform.offsetX)
    inputs.offsetY.value = String(transform.offsetY)
    inputs.rotationAzimuth.value = String(transform.rotationAzimuth)
    inputs.rotationPolar.value = String(transform.rotationPolar)
    syncText(transform)
  }

  syncInputs(runtime.getTransform())
  runtime.bridge.on(AVATAR_EVENT_NAMES.transformChange, syncInputs)

  for (const [key, input] of Object.entries(inputs) as [keyof AvatarTransform, HTMLInputElement][]) {
    input.addEventListener('change', () => {
      const value = Number(input.value)
      void runtime.setTransform({ [key]: Number.isFinite(value) ? value : 0 })
    })
  }
}

function bindAnimationControls(
  runtime: Awaited<ReturnType<typeof createVrmRuntime>>,
  root: HTMLElement,
): void {
  const idleToggle = root.querySelector<HTMLInputElement>('#animation-idle')
  const breathToggle = root.querySelector<HTMLInputElement>('#animation-breath')
  const blinkToggle = root.querySelector<HTMLInputElement>('#animation-blink')
  const stateOutput = root.querySelector<HTMLElement>('#animation-state-output')

  if (!idleToggle || !breathToggle || !blinkToggle || !stateOutput) {
    throw new Error('Demo mount failed: missing animation controls')
  }

  const syncView = (state: AvatarAnimationState) => {
    idleToggle.checked = state.idleEnabled
    breathToggle.checked = state.breathEnabled
    blinkToggle.checked = state.blinkEnabled
    stateOutput.textContent = JSON.stringify(state, null, 2)
  }

  const applyState = () => {
    void runtime.setAnimationState({
      idleEnabled: idleToggle.checked,
      breathEnabled: breathToggle.checked,
      blinkEnabled: blinkToggle.checked,
    })
  }

  syncView(runtime.getAnimationState())
  runtime.bridge.on(AVATAR_EVENT_NAMES.animationStateChange, syncView)
  idleToggle.addEventListener('change', applyState)
  breathToggle.addEventListener('change', applyState)
  blinkToggle.addEventListener('change', applyState)
}

function createMotionButton(label: string, variant: 'idle' | 'oneshot'): HTMLButtonElement {
  const button = document.createElement('button')
  button.className = `motion-button motion-button--${variant}`
  button.textContent = label
  return button
}

function bindMotionControls(
  runtime: Awaited<ReturnType<typeof createVrmRuntime>>,
  root: HTMLElement,
): void {
  const idleToggle = root.querySelector<HTMLInputElement>('#motion-idle-enabled')
  const stateOutput = root.querySelector<HTMLElement>('#motion-state-output')
  const idleList = root.querySelector<HTMLElement>('#motion-idle-list')
  const oneShotList = root.querySelector<HTMLElement>('#motion-oneshot-list')
  const stopButton = root.querySelector<HTMLButtonElement>('#motion-stop')
  const nextIdleButton = root.querySelector<HTMLButtonElement>('#motion-next-idle')

  if (!idleToggle || !stateOutput || !idleList || !oneShotList || !stopButton || !nextIdleButton) {
    throw new Error('Demo mount failed: missing motion controls')
  }

  let catalog = runtime.getMotionCatalog()
  let currentState = runtime.getMotionState()

  const syncState = (state: AvatarMotionState) => {
    currentState = state
    idleToggle.checked = state.idleEnabled
    stateOutput.textContent = JSON.stringify(state, null, 2)
    renderCatalog()
  }

  const restartIdleLoop = () => {
    Promise.resolve(runtime.setMotionState({ idleEnabled: false })).then(() => {
      void runtime.setMotionState({ idleEnabled: true })
    })
  }

  const renderCatalog = () => {
    const renderSection = (
      container: HTMLElement,
      motions: AvatarMotionEntry[],
      kind: 'idle' | 'oneshot',
    ) => {
      container.innerHTML = ''
      for (const motion of motions) {
        const row = document.createElement('div')
        row.className = 'motion-item'
        if (motion.id === currentState.activeMotionId) {
          row.dataset.active = 'true'
        }

        const label = document.createElement('div')
        label.className = 'motion-item__meta'
        label.innerHTML = `<strong>${motion.name}</strong><span>${motion.id}</span>`

        const button = createMotionButton(
          kind === 'idle' ? 'Switch Idle' : 'Play One-Shot',
          kind,
        )

        if (kind === 'idle') {
          button.disabled = !currentState.idleEnabled || currentState.oneShotPlaying
          button.addEventListener('click', restartIdleLoop)
        } else {
          button.disabled = currentState.oneShotPlaying
          button.addEventListener('click', () => {
            void runtime.playMotion(motion.id)
          })
        }

        row.append(label, button)
        container.appendChild(row)
      }
    }

    renderSection(
      idleList,
      catalog.filter((motion) => motion.kind === 'idle'),
      'idle',
    )
    renderSection(
      oneShotList,
      catalog.filter((motion) => motion.kind === 'oneshot'),
      'oneshot',
    )
  }

  syncState(currentState)
  renderCatalog()

  idleToggle.addEventListener('change', () => {
    void runtime.setMotionState({ idleEnabled: idleToggle.checked })
  })
  stopButton.addEventListener('click', () => {
    runtime.stopMotion()
  })
  nextIdleButton.addEventListener('click', restartIdleLoop)

  runtime.bridge.on(AVATAR_EVENT_NAMES.motionCatalogChange, (nextCatalog) => {
    catalog = nextCatalog
    renderCatalog()
  })
  runtime.bridge.on(AVATAR_EVENT_NAMES.motionStateChange, syncState)
}

export async function mountDemo(root: HTMLElement): Promise<void> {
  root.innerHTML = `
    <div class="app-shell">
      <section class="hero">
        <div class="hero__copy">
          <p class="eyebrow">Avatar Runtime VRM</p>
          <h1>VRMA idle and one-shot runtime demo</h1>
          <p class="lede">Phase 3 validation for built-in motion catalog, idle fallback, one-shot playback, and runtime state/event flow.</p>
        </div>
      </section>
      <section class="layout">
        <div class="viewer-card">
          <div id="viewer" class="viewer">
            <div id="viewer-status" class="viewer-status">Initializing runtime...</div>
          </div>
        </div>
        <div class="panel-stack">
          <section class="panel">
            <h2>Character</h2>
            <label class="field">
              <span>VRM source</span>
              <input id="model-source" type="text" value="${DEFAULT_MODEL_SOURCE}" />
            </label>
            <button id="load-vrm" class="action">Load VRM</button>
          </section>
          <section class="panel">
            <h2>Motion Catalog</h2>
            <div class="inline-row">
              <label class="toggle">
                <input id="motion-idle-enabled" type="checkbox" />
                <span>VRMA Idle Enabled</span>
              </label>
              <button id="motion-next-idle" class="action action--ghost">Switch Idle</button>
              <button id="motion-stop" class="action action--ghost">Stop One-Shot</button>
            </div>
            <div class="motion-groups">
              <div class="motion-group">
                <h3>Idle Motions</h3>
                <div id="motion-idle-list" class="motion-list"></div>
              </div>
              <div class="motion-group">
                <h3>One-Shot Motions</h3>
                <div id="motion-oneshot-list" class="motion-list"></div>
              </div>
            </div>
            <pre id="motion-state-output" class="code-block"></pre>
          </section>
          <section class="panel">
            <h2>Mood</h2>
            <div id="mood-group" class="chip-row"></div>
          </section>
          <section class="panel">
            <h2>Speech</h2>
            <div class="inline-row">
              <label class="toggle">
                <input id="speech-toggle" type="checkbox" />
                <span>Speaking</span>
              </label>
              <select id="viseme-select">
                <option value="">Auto viseme</option>
                <option value="aa">aa</option>
                <option value="ih">ih</option>
                <option value="ou">ou</option>
                <option value="ee">ee</option>
                <option value="oh">oh</option>
              </select>
            </div>
          </section>
          <section class="panel">
            <h2>Base Animation</h2>
            <p class="panel-note">Procedural idle becomes the fallback layer when VRMA idle is unavailable or disabled. Blink and breathing stay active independently.</p>
            <div class="field-grid field-grid--single">
              <label class="toggle">
                <input id="animation-idle" type="checkbox" />
                <span>Procedural Idle Fallback</span>
              </label>
              <label class="toggle">
                <input id="animation-breath" type="checkbox" />
                <span>Breathing</span>
              </label>
              <label class="toggle">
                <input id="animation-blink" type="checkbox" />
                <span>Blink</span>
              </label>
            </div>
            <pre id="animation-state-output" class="code-block code-block--compact"></pre>
          </section>
          <section class="panel">
            <h2>Transform</h2>
            <div class="field-grid">
              <div class="field field--scale">
                <span>Zoom <strong id="scale-display">100%</strong></span>
                <input id="scale-slider" type="range" min="0.75" max="1.6" step="0.01" value="1" />
                <button id="reset-scale" class="action action--ghost action--sm">Reset (100%)</button>
              </div>
              <label class="field"><span>Offset X</span><input id="offsetX" type="number" step="0.01" /></label>
              <label class="field"><span>Offset Y</span><input id="offsetY" type="number" step="0.01" /></label>
              <label class="field"><span>Azimuth</span><input id="rotationAzimuth" type="number" step="0.01" /></label>
              <label class="field"><span>Polar</span><input id="rotationPolar" type="number" step="0.01" /></label>
            </div>
            <pre id="transform-output" class="code-block"></pre>
          </section>
          <section class="panel">
            <h2>Runtime</h2>
            <div class="inline-row">
              <button id="pause-runtime" class="action action--ghost">Pause</button>
              <button id="resume-runtime" class="action action--ghost">Resume</button>
              <button id="resize-runtime" class="action action--ghost">Resize</button>
            </div>
            <div class="inline-row">
              <label class="toggle">
                <input id="shadow-toggle" type="checkbox" />
                <span>Shadow</span>
              </label>
            </div>
          </section>
          <section class="panel">
            <h2>Events</h2>
            <div id="event-log" class="event-log"></div>
          </section>
        </div>
      </section>
    </div>
  `

  const viewer = root.querySelector<HTMLElement>('#viewer')
  const eventLog = root.querySelector<HTMLElement>('#event-log')
  if (!viewer || !eventLog) {
    throw new Error('Demo mount failed: missing viewer or event log container')
  }

  const runtime = await createVrmRuntime(viewer, {
    mode: 'embedded',
    transparent: true,
    allowInteraction: true,
    maxFps: 60,
  })
  window.__demoRuntime = runtime

  const viewerStatus = root.querySelector<HTMLElement>('#viewer-status')
  if (!viewerStatus) {
    throw new Error('Demo mount failed: missing viewer status container')
  }

  const setViewerStatus = (message: string, tone: 'info' | 'success' | 'error' = 'info') => {
    viewerStatus.textContent = message
    viewerStatus.dataset.tone = tone
  }
  setViewerStatus('Runtime ready. Loading default VRM...', 'info')

  const logEvent = createEventLogger(eventLog)
  for (const eventName of Object.values(AVATAR_EVENT_NAMES)) {
    runtime.bridge.on(eventName, (payload: unknown) => {
      logEvent(eventName, payload)
    })
  }

  runtime.bridge.on(AVATAR_EVENT_NAMES.loadStart, () => {
    setViewerStatus('Loading VRM model and built-in VRMA motions...', 'info')
  })
  runtime.bridge.on(AVATAR_EVENT_NAMES.loadSuccess, () => {
    setViewerStatus('VRM and motion catalog loaded. Drag to rotate.', 'success')
  })
  runtime.bridge.on(AVATAR_EVENT_NAMES.loadError, ({ error }) => {
    setViewerStatus(`Load failed: ${error.message}`, 'error')
  })
  runtime.bridge.on(AVATAR_EVENT_NAMES.runtimeError, (error) => {
    setViewerStatus(`Runtime error: ${error.message}`, 'error')
  })
  runtime.bridge.on(AVATAR_EVENT_NAMES.motionError, (error) => {
    setViewerStatus(`Motion error: ${error.message}`, 'error')
  })

  const loadButton = root.querySelector<HTMLButtonElement>('#load-vrm')
  const modelSourceInput = root.querySelector<HTMLInputElement>('#model-source')
  if (!loadButton || !modelSourceInput) {
    throw new Error('Demo mount failed: missing load controls')
  }

  const loadCurrentModel = () => {
    const spec: AvatarCharacterSpec = {
      id: `demo-${Date.now()}`,
      kind: 'vrm',
      modelSource: modelSourceInput.value.trim(),
      displayName: 'Demo VRM',
      initialMood: 'neutral',
    }
    void runtime.loadCharacter(spec)
  }

  loadButton.addEventListener('click', loadCurrentModel)

  const moodGroup = root.querySelector<HTMLElement>('#mood-group')
  if (!moodGroup) {
    throw new Error('Demo mount failed: missing mood controls')
  }

  for (const mood of MOODS) {
    const button = document.createElement('button')
    button.className = 'chip'
    button.textContent = mood
    button.addEventListener('click', () => {
      void runtime.setMood(mood)
    })
    moodGroup.appendChild(button)
  }

  const speechToggle = root.querySelector<HTMLInputElement>('#speech-toggle')
  const visemeSelect = root.querySelector<HTMLSelectElement>('#viseme-select')
  if (!speechToggle || !visemeSelect) {
    throw new Error('Demo mount failed: missing speech controls')
  }

  const applySpeechState = () => {
    const state: AvatarSpeechState = {
      speaking: speechToggle.checked,
      viseme: visemeSelect.value || null,
      intensity: 0.85,
    }
    void runtime.setSpeechState(state)
  }

  speechToggle.addEventListener('change', applySpeechState)
  visemeSelect.addEventListener('change', applySpeechState)

  const scaleSlider = root.querySelector<HTMLInputElement>('#scale-slider')
  const offsetXInput = root.querySelector<HTMLInputElement>('#offsetX')
  const offsetYInput = root.querySelector<HTMLInputElement>('#offsetY')
  const rotationAzimuthInput = root.querySelector<HTMLInputElement>('#rotationAzimuth')
  const rotationPolarInput = root.querySelector<HTMLInputElement>('#rotationPolar')
  const transformOutput = root.querySelector<HTMLElement>('#transform-output')
  const scaleDisplay = root.querySelector<HTMLElement>('#scale-display')
  const resetScaleButton = root.querySelector<HTMLButtonElement>('#reset-scale')

  if (
    !scaleSlider ||
    !offsetXInput ||
    !offsetYInput ||
    !rotationAzimuthInput ||
    !rotationPolarInput ||
    !transformOutput ||
    !scaleDisplay ||
    !resetScaleButton
  ) {
    throw new Error('Demo mount failed: missing transform controls')
  }

  const updateScaleDisplay = (scale: number) => {
    const pct = Math.round(scale * 100)
    scaleDisplay.textContent = `${pct}%`
  }

  bindTransformInputs(
    runtime,
    {
      scale: scaleSlider,
      offsetX: offsetXInput,
      offsetY: offsetYInput,
      rotationAzimuth: rotationAzimuthInput,
      rotationPolar: rotationPolarInput,
    },
    transformOutput,
  )

  // Slider gets live 'input' events for real-time zoom feedback
  scaleSlider.addEventListener('input', () => {
    const value = Number(scaleSlider.value)
    if (Number.isFinite(value)) {
      void runtime.setTransform({ scale: value })
    }
  })

  // Display sync: keep percentage and slider in sync with transform state
  const onTransformChange = (transform: AvatarTransform) => {
    updateScaleDisplay(transform.scale)
  }
  onTransformChange(runtime.getTransform())
  runtime.bridge.on(AVATAR_EVENT_NAMES.transformChange, onTransformChange)

  // Reset scale button
  resetScaleButton.addEventListener('click', () => {
    void runtime.setTransform({ scale: 1 })
  })
  bindAnimationControls(runtime, root)
  bindMotionControls(runtime, root)

  root.querySelector<HTMLButtonElement>('#pause-runtime')?.addEventListener('click', () => {
    void runtime.pause()
  })
  root.querySelector<HTMLButtonElement>('#resume-runtime')?.addEventListener('click', () => {
    void runtime.resume()
  })
  root.querySelector<HTMLButtonElement>('#resize-runtime')?.addEventListener('click', () => {
    runtime.resize({
      width: viewer.clientWidth,
      height: viewer.clientHeight,
    })
  })

  const shadowToggle = root.querySelector<HTMLInputElement>('#shadow-toggle')
  shadowToggle?.addEventListener('change', () => {
    runtime.setShadowEnabled(shadowToggle.checked)
  })

  const resizeObserver = new ResizeObserver(() => {
    runtime.resize({
      width: viewer.clientWidth,
      height: viewer.clientHeight,
    })
  })
  resizeObserver.observe(viewer)

  loadCurrentModel()
}
