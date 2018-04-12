'use strict';

import SuperImage from './super_image';

/**
 * CameraPlayer controls a camera stream
 * @class
 */
class CameraPlayer {
  /**
   * @param {Exopticon.Camera} camera - Camera object to play
   * @param {Exopticon.CameraChannel} channel - channel to play over
   */
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
  /**
   * Change player status, firing callbacks if different
   * @param {string} newStatus - status to change to
   * @private
   */
  setStatus(newStatus) {
    const oldStatus = this.status;
    this.status = newStatus;
    if (oldStatus !== newStatus) {
      this.statusCallback(newStatus);
    }
  }

  /**
   * begin realtime playback of camera to given Image object
   * @param {Image} img - image object to stream video to
   * @param {Function} cb - function to call when playback starts
   */
  playRealtime(img, cb = ()=>{}) {
    this.setStatus('loading');
    this.img = new SuperImage(img);

    this.channel.join(this.camera.id, (data) => {
      if (this.status !== 'paused' && this.img !== null) {
        this.setStatus('playing');
        this.img.renderArrayIfReady(data.frameJpeg);
        cb();
      }
    });
  }

  /**
   * stop all playback of given camera
   */
  stop() {
    this.channel.leave(this.camera.id);
    this.setStatus('paused');
    this.img = null;
  }

  /**
   * @return {boolean} true if camera report ptz capability
   */
  hasPtz() {
    if (this.camera.ptzType === null) {
      return false;
    } else {
      return true;
    }
  }

  /**
   * request relative movement from camera
   * @param {number} x - number between -1 and 1 specifying amount to
   *                     move horizontally
   * @param {number} y - number between -1 and 1 specifying amount to
   *                     move vertically
   * @param {Function} callback - movement complete callback
   */
  relativeMove(x, y, callback) {
    fetch(this.relativeMoveUrl,
          {
            method: 'post',
            credentials: 'same-origin',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({x: x, y: y}),
    }).then(function(response) {
      if (callback) callback(response);
    });
  }

  /**
   * move camera left
   */
  left() {
    this.relativeMove('-0.05', '0.0');
  }

  /**
   * move camera right
   */
  right() {
    this.relativeMove('0.05', '0.0');
  }

  /**
   * move camera up
   */
  up() {
    this.relativeMove('0.0', '0.1');
  }

  /**
   * move camera down
   */
  down() {
    this.relativeMove('0.0', '-0.1');
  }
}

export default CameraPlayer;
