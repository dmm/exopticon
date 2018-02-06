'use strict';

import SuperImage from './super_image';

import socket from './socket.js';

class CameraPlayer {
  constructor(camera, channel) {
    this.camera = camera;
    Object.assign(this, camera);
    this.relativeMoveUrl = `/v1/cameras/${camera.id}/relativeMove`;
    this.status = 'paused';
    this.channel = channel;
    this.img = null;
    this.statusCallback = () => {};

    // Bind functions so they can be used as callbacks
    // and use 'this'.
    this.left = this.left.bind(this);
    this.right = this.right.bind(this);
    this.up = this.up.bind(this);
    this.down = this.down.bind(this);
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
        this.img.renderArrayIfReady(data.frameJpeg);
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
