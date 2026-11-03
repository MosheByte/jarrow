/** The low-resolution layer: an RGBA buffer, one entry per grid cell, fully opaque or empty. */

import type { Rgb } from './palette.ts';

export interface PixelImage {
  readonly width: number;
  readonly height: number;
  readonly data: Uint8ClampedArray;
}

export class PixelBuffer {
  readonly width: number;
  readonly height: number;
  readonly data: Uint8ClampedArray;

  constructor(image: PixelImage) {
    this.width = image.width;
    this.height = image.height;
    this.data = image.data;
  }

  static create(width: number, height: number): PixelBuffer {
    return new PixelBuffer({ width, height, data: new Uint8ClampedArray(width * height * 4) });
  }

  clear(): void {
    this.data.fill(0);
  }

  /** Paints one cell; cells outside the buffer are ignored. */
  set(x: number, y: number, c: Rgb): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return;
    const i = (y * this.width + x) * 4;
    this.data[i] = c[0];
    this.data[i + 1] = c[1];
    this.data[i + 2] = c[2];
    this.data[i + 3] = 255;
  }

  /** Paints cells x0 .. x1 - 1 of one row. */
  span(x0: number, x1: number, y: number, c: Rgb): void {
    if (y < 0 || y >= this.height) return;
    const a = Math.max(0, x0);
    const b = Math.min(this.width, x1);
    for (let x = a; x < b; x++) {
      const i = (y * this.width + x) * 4;
      this.data[i] = c[0];
      this.data[i + 1] = c[1];
      this.data[i + 2] = c[2];
      this.data[i + 3] = 255;
    }
  }

  /** Paints a column from row y0 to the bottom of the buffer. */
  column(x: number, y0: number, c: Rgb): void {
    if (x < 0 || x >= this.width) return;
    for (let y = Math.max(0, y0); y < this.height; y++) {
      const i = (y * this.width + x) * 4;
      this.data[i] = c[0];
      this.data[i + 1] = c[1];
      this.data[i + 2] = c[2];
      this.data[i + 3] = 255;
    }
  }

  /** Empties one cell. */
  erase(x: number, y: number): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return;
    this.data[(y * this.width + x) * 4 + 3] = 0;
  }

  isSet(x: number, y: number): boolean {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return false;
    return this.data[(y * this.width + x) * 4 + 3]! > 0;
  }

  /**
   * Stamps a sprite given as rows of characters; each character maps to a colour in `inks`,
   * and characters without an ink (such as '.') are left alone.
   */
  sprite(x: number, y: number, rows: readonly string[], inks: Readonly<Record<string, Rgb>>): void {
    for (let r = 0; r < rows.length; r++) {
      const line = rows[r]!;
      for (let c = 0; c < line.length; c++) {
        const ink = inks[line[c]!];
        if (ink) this.set(x + c, y + r, ink);
      }
    }
  }
}
