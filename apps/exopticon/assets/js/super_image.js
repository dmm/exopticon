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

let drawCount = 0;
const drawMax = 2;

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
   * Check if ready for draw
   * @private
   */
  drawReady() {
    if (drawCount < drawMax) {
      return true;
    }
    return false;
  }

  /**
   * Checks to see if image is finished drawing until it is.
   * @private
   */
  checkFrame() {
    if (this.img.complete) {
      //      this.isDrawing = false;
      //      globalIsDrawing = false;
      drawCount--;
      this.isDrawing = false;
      window.URL.revokeObjectURL(this.img.src);
      if (this.callback) this.callback();
    } else {
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

    if (this.drawReady()) {
      drawCount++;
      this.isDrawing = true;
      let blob = new Blob([arrayBuffer], {type: 'image/jpeg'});
      this.img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    } else {
      console.log('skipping draw!');
    }
  }
}

export default SuperImage;
