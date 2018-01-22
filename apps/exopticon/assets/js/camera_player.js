'use strict';

import socket from './socket.js';
import renderFrame from './render_frame.js';

class CameraPlayer {
  constructor(camera, channel) {
    this.camera = camera;
    Object.assign(this, camera);
    this.relativeMoveUrl = `/v1/cameras/${camera.id}/relativeMove`;
    this.playing = false;
    this.status = 'paused';
    this.channel = channel;
    this.isDrawing = false;
    this.drawImg = null;
    this.img = null;
    this.subTimer = null;
    this.statusCallback = () => {};

    this.checkFrame = this.checkFrame.bind(this);
    this.renderFrame = this.renderFrame.bind(this);

    this.left = this.left.bind(this);
    this.right = this.right.bind(this);
    this.up = this.up.bind(this);
    this.down = this.down.bind(this);


    this.channel.on('jpg' + this.camera.id.toString(), (data) => {
      this.channel.push("ack", { ts: data.ts });
      if (this.playing && this.img !== null) {
        this.status = 'playing';
        this.renderFrame(this.img, data.frameJpeg);
      }
    });

    this.channel.on('subscribe', () => {
      console.log('got subscribe!');
      if (this.playing === true) {
        console.log('rejoining!');
        this.playRealtime(this.img, this.startCb);
      }
    });
  }

  checkFrame() {
    if (this.isDrawing && this.drawImg.complete) {
      this.isDrawing = false;
      this.drawImg = null;
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
      img.src = window.URL.createObjectURL(blob);
      window.requestAnimationFrame(this.checkFrame);
    } else {
      console.log('Skipping render ' + this.camera.id.toString());
    }
  }

  setStatus(newStatus) {
    this.status = newStatus;
    this.statusCallback(newStatus);
  }

  playRealtime(img) {
    this.channel.push('watch' + this.camera.id.toString(), "");
    this.subTimer = setInterval(() => {
      if (this.playing === true) {
        this.channel.push('watch' + this.camera.id.toString(), "");
      }
    }, 5000);
    this.playing = true;
    this.setStatus('loading');
    this.img = img;

    let cb = () => {
      this.setStatus('playing');
      img.removeEventListener('load', cb);
    };
    img.addEventListener('load', cb);
  }

  play(timeUtc) {
  }

  stop() {
    console.log('stopping...' + this.camera.id.toString());
    clearInterval(this.subTimer);
    this.channel.push('close' + this.camera.id.toString(), "");
    this.playing = false;
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
