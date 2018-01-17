'use strict';

import socket from './socket.js';
import renderFrame from './render_frame.js';

class CameraPlayer {
  constructor(camera, socket) {
    this.camera = camera;
    this.socket = socket;
    this.relativeMoveUrl = `/v1/cameras/${camera.id}/relativeMove`;
    this.playing = false;
    this.channel = null;
    this.isDrawing = false;
    this.drawImg = null;

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
      this.channel.push("ack", "");
    }

    window.requestAnimationFrame(this.checkFrame);
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

      window.requestAnimationFrame(this.checkFrame);

      img.src = window.URL.createObjectURL(blob);
    }
  }

  playRealtime(img) {
    if (this.channel !== null) {
      this.stop();
    }
    this.channel = this.socket.channel('camera:' + this.camera.id.toString());
    this.channel.on('jpg', (data) => {
      if (this.playing) {
        this.renderFrame(img, data.frameJpeg);
      }

//      this.channel.push("ack", "");
    });
    this.channel.join();
    this.playing = true;
  }

  play(timeUtc) {
  }

  stop() {
    if (this.channel) {
      this.channel.leave();
    }
    this.channel = null;
    this.playing = false;
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
