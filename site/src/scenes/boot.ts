/**
 * Mounts scenes on the page. Each scene component's script calls mountAll with its own scene,
 * so a page ships only the generators it shows; this module (and the registry below) is shared.
 */

import { type SceneController, type SceneDef, type WindowLike, browserEnv, mountScene } from './runtime.ts';

const mounted = new Map<Element, SceneController>();

/** Mounts every not-yet-mounted `[data-scene=<name>]` root in the document. */
export function mountAll<S>(name: string, def: SceneDef<S>): SceneController[] {
  const env = browserEnv(window satisfies WindowLike);
  const controllers: SceneController[] = [];
  for (const root of document.querySelectorAll<HTMLElement>(`[data-scene="${name}"]`)) {
    if (mounted.has(root)) continue;
    const smooth = root.querySelector<HTMLCanvasElement>('canvas[data-layer="smooth"]');
    const pixels = root.querySelector<HTMLCanvasElement>('canvas[data-layer="pixels"]');
    if (!smooth || !pixels) continue;
    const controller = mountScene({ root, smooth, pixels }, def, env, { variant: root.dataset.variant });
    mounted.set(root, controller);
    controllers.push(controller);
  }
  return controllers;
}

/** The controller of a mounted scene root, if any. */
export function sceneFor(root: Element): SceneController | undefined {
  return mounted.get(root);
}

/**
 * On a drop in the board card: the sun in every sunrise on the page rises one step
 * (BUILD-BOOK 8.7). Returns how many scenes moved.
 */
export function riseSunrise(): number {
  let count = 0;
  for (const controller of mounted.values()) {
    if (controller.name === 'sunrise') {
      controller.rise();
      count++;
    }
  }
  return count;
}
