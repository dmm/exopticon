'use strict';

import SuperImage from './super_image';

import socket from './socket.js';
import renderFrame from './render_frame.js';

class CameraPlayer {
  constructor(camera, channel) {
    this.camera = camera;
    Object.assign(this, camera);
    this.relativeMoveUrl = `/v1/cameras/${camera.id}/relativeMove`;
    this.status = 'paused';
    this.channel = channel;
    this.isDrawing = false;
    this.drawImg = null;
    this.img = null;
    this.subTimer = null;
    this.statusCallback = () => {};

    // Bind functions so they can be used as callbacks
    // and use 'this'.
    this.checkFrame = this.checkFrame.bind(this);
    this.renderFrame = this.renderFrame.bind(this);
    this.left = this.left.bind(this);
    this.right = this.right.bind(this);
    this.up = this.up.bind(this);
    this.down = this.down.bind(this);
  }

  checkFrame() {
    if (this.isDrawing && this.drawImg.complete) {
      this.isDrawing = false;
      this.drawImg = null;
    } else {
      window.requestAnimationFrame(this.checkFrame);
    }
  }

  renderFrame(img, imageArrayBuffer, callback) {
    if (this.isDrawing === false) {
      this.isDrawing = true;
      this.drawImg = img;
      var blob  = new Blob([imageArrayBuffer],{type: "image/jpeg"});

      img.onload = function (e) {
        window.URL.revokeObjectURL(img.src);
        img = null;
        if (callback) callback();
      };

      img.onerror = img.onabort = function () {
        console.log('error loading image!');
        img = null;
        if (callback) callback();
      };
      img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    } else {
      console.log('Skipping render ' + this.camera.id.toString());
    }
  }

  bufferToBlob(imageArrayBuffer) {
    var blob = new Blob([imageArrayBuffer],{type: "image/jpeg"});
    return window.URL.createObjectURL(blob);
  }

  setStatus(newStatus) {
    const oldStatus = this.status;
    this.status = newStatus;
    if (this.oldStatus !== newStatus) {
      this.statusCallback(newStatus);
    }
  }

  playRealtime(img) {
    this.setStatus('loading');
    this.img = new SuperImage(img);

    this.channel.join(this.camera.id, (data) => {
      if (this.status !== 'paused' && this.img !== null) {
        this.setStatus('playing');
        this.img.renderIfReady(this.bufferToBlob(data.frameJpeg));
      }
    });
  }

  play(timeUtc) {
  }

  stop() {
    this.channel.leave(this.camera.id);
    this.setStatus('paused');
    this.img = null;
  }

  hasPtz() {
    if (this.camera.ptzType === null) {
      return false;
    } else {
      return true;
    }
  }

  relativeMove(x, y, callback) {
    var myInit = { method: 'POST' };
    console.log('relative move!');
    fetch(this.relativeMoveUrl, {
      method: 'post',
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({x: x, y: y})
    }).then(function(response) {
      console.log('Move complete');
      if (callback) callback(response);
    });
  }

  left() {
    this.relativeMove('-0.05', '0.0');
  }

  right() {
    this.relativeMove('0.05', '0.0');
  }

  up() {
    console.log('up1');
    this.relativeMove('0.0', '0.1');
  }

  down() {
    this.relativeMove('0.0', '-0.1');
  }
}

export default CameraPlayer;
