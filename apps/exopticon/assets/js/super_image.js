'use strict';

class SuperImage {
  constructor(img) {
    this.img = img;
    this.isDrawing = false;

    this.checkFrame = this.checkFrame.bind(this);
  }

  checkFrame() {
    if (this.img.complete) {
      this.isDrawing = false;
      window.URL.revokeObjectURL(this.img.src);
      if (this.callback) this.callback();
    } else {
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  renderIfReady(src, callback) {
    this.callback = callback;

    if (this.isDrawing === false) {
      this.isDrawing = true;
      this.img.src = src;
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  renderArrayIfReady(arrayBuffer, callback) {
    this.callback = callback;

    if (this.isDrawing === false) {
      this.isDrawing = true;
      var blob = new Blob([arrayBuffer],{type: "image/jpeg"});
      this.img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    }
  }
}

export default SuperImage;
