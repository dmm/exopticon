/*
 * This file is part of Exopticon (https://github.com/dmm/exopticon).
 * Copyright (c) 2018 David Matthew Mattli
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

let globalIsDrawing = false;

/**
 * SuperImage class - Implements an image that drops render requests
 * as needed
 */
class SuperImage {
  /**
   * @param {Image} img - image to wrap
   */
  constructor(img) {
    this.img = img;
    this.isDrawing = false;

    this.checkFrame = this.checkFrame.bind(this);
  }

  /**
   * Checks to see if image is finished drawing until it is.
   * @private
   */
  checkFrame() {
    if (this.img.complete) {
      //      this.isDrawing = false;
      globalIsDrawing = false;
      window.URL.revokeObjectURL(this.img.src);
      if (this.callback) this.callback();
    } else {
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  /**
   * renders the given source if SuperImage is ready.
   * @param {string} src - src to render
   * @param {function} callback - called if image is rendered
   */
  renderIfReady(src, callback) {
    this.callback = callback;

    //    if (this.isDrawing === false) {
    if (globalIsDrawing === false) {
      //      this.isDrawing = true;
      globalIsDrawing = true;
      this.img.src = src;
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  /**
   * renders specified arrayBuffer as image/jpeg if ready
   * @param {ArrayBuffer} arrayBuffer - image/jpeg to render
   * @param {function} callback - called if image renders
   */
  renderArrayIfReady(arrayBuffer, callback) {
    this.callback = callback;

    if (globalIsDrawing === false) {
      globalIsDrawing = true;
      let blob = new Blob([arrayBuffer], {type: 'image/jpeg'});
      this.img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  /**
   * renders specified blob as image/jpeg if ready
   * @param {Blob} blob - image blob to render
   * @param {function} callback - called if image renders
   */
  renderBlobIfReady(blob, callback) {
    this.callback = callback;

    if (globalIsDrawing === false) {
      globalIsDrawing = true;
      this.img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    }
  }
}

export default SuperImage;
