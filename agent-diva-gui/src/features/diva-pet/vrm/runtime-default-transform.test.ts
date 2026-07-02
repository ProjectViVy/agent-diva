import { describe, expect, it } from 'vitest'
import * as THREE from 'three'
import {
  DESKTOP_PET_DEFAULT_TRANSFORM,
  DEFAULT_CAMERA_DISTANCE,
  DEFAULT_CAMERA_TARGET_X,
  DEFAULT_CAMERA_TARGET_Y,
  DEFAULT_TRANSFORM,
  getCameraPosition,
  getCameraTarget,
  getDefaultTransform,
} from '../../../../avatar-runtime-vrm/src/runtime/constants'
import { TransformController } from '../../../../avatar-runtime-vrm/src/runtime/transform-controller'

function expectClose(actual: number, expected: number): void {
  expect(actual).toBeCloseTo(expected, 6)
}

describe('avatar runtime default transform', () => {
  it('uses a front-facing level camera for desktop pet mode', () => {
    const transform = getDefaultTransform('desktop-pet')
    const target = getCameraTarget(transform)
    const position = getCameraPosition(transform)

    expect(transform).toEqual(DESKTOP_PET_DEFAULT_TRANSFORM)
    expectClose(target.x, DEFAULT_CAMERA_TARGET_X + DESKTOP_PET_DEFAULT_TRANSFORM.offsetX)
    expectClose(target.y, DEFAULT_CAMERA_TARGET_Y + DESKTOP_PET_DEFAULT_TRANSFORM.offsetY)
    expectClose(target.z, 0)
    expectClose(position.x, DEFAULT_CAMERA_TARGET_X + DESKTOP_PET_DEFAULT_TRANSFORM.offsetX)
    expectClose(position.y, DEFAULT_CAMERA_TARGET_Y + DESKTOP_PET_DEFAULT_TRANSFORM.offsetY)
    expectClose(position.z, DEFAULT_CAMERA_DISTANCE)
  })

  it('keeps the embedded runtime angled default view', () => {
    const transform = getDefaultTransform('embedded')
    const target = getCameraTarget(transform)
    const position = getCameraPosition(transform)

    expect(transform).toEqual(DEFAULT_TRANSFORM)
    expect(position.y).toBeGreaterThan(target.y)
    expect(position.z).toBeGreaterThan(0)
  })

  it('resets transform controller to the mode-specific default', () => {
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 1000)
    const controls = {
      target: new THREE.Vector3(),
      update: () => undefined,
      getAzimuthalAngle: () => 0,
      getPolarAngle: () => Math.PI / 2,
    }
    const controller = new TransformController(camera, controls, 'desktop-pet')

    controller.setTransform({ offsetX: 0.8, offsetY: 0.4, scale: 1.4, rotationPolar: 1.1 })
    const reset = controller.reset()

    expect(reset).toEqual(DESKTOP_PET_DEFAULT_TRANSFORM)
    expectClose(controls.target.x, DEFAULT_CAMERA_TARGET_X + DESKTOP_PET_DEFAULT_TRANSFORM.offsetX)
    expectClose(controls.target.y, DEFAULT_CAMERA_TARGET_Y + DESKTOP_PET_DEFAULT_TRANSFORM.offsetY)
    expectClose(camera.position.x, DEFAULT_CAMERA_TARGET_X + DESKTOP_PET_DEFAULT_TRANSFORM.offsetX)
    expectClose(camera.position.y, DEFAULT_CAMERA_TARGET_Y + DESKTOP_PET_DEFAULT_TRANSFORM.offsetY)
    expectClose(camera.position.z, DEFAULT_CAMERA_DISTANCE)
  })

  it('uses offset as camera framing without moving the model root', () => {
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 1000)
    const controls = {
      target: new THREE.Vector3(),
      update: () => undefined,
      getAzimuthalAngle: () => 0,
      getPolarAngle: () => Math.PI / 2,
    }
    const modelRoot = new THREE.Object3D()
    const controller = new TransformController(camera, controls, 'desktop-pet')

    controller.attachModel(modelRoot)
    controller.setTransform({ offsetX: -0.3, offsetY: 0.25 })

    expectClose(modelRoot.position.x, 0)
    expectClose(modelRoot.position.y, 0)
    expectClose(modelRoot.position.z, 0)
    expectClose(controls.target.x, DEFAULT_CAMERA_TARGET_X - 0.3)
    expectClose(controls.target.y, DEFAULT_CAMERA_TARGET_Y + 0.25)
  })
})
