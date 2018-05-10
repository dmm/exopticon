'use strict';
/**
 * @class
 */
class CameraChannel {
  /**
   * @param {Pheonix.Channel} channel
   */
  constructor(channel) {
    this.channel = channel;
    this.prefix = 'camera';

    this.watchedCameras = new Map();

    this.subscribe = this.subscribe.bind(this);
    this.unsubscribe = this.unsubscribe.bind(this);

    this.channel.on('subscribe', () => {
      this.subscribe(this.watchedCameraIds());
    });
    this.channel.onMessage = (event, payload, ref) => {
      if (event.match(/jpg([0-9]+)/)) {
        this.channel.push('ack', {ts: payload.ts});
      }
      return payload;
    };
  }
  /**
   * @return {Array} returns ids of cameras being watched as array
   */
  watchedCameraIds() {
    return Array.from(this.watchedCameras.keys());
  }

  /**
   * closes all watched cameras
   */
  close() {
    this.unsubscribe(this.watchedCameraIds());
  }

  /**
   * @param {Array} cameras - Array of camera ids to watch
   * @private
   */
  subscribe(cameras) {
   cameras.forEach((cameraId) => {
      this.channel.push(`watch${cameraId.toString()}`, '');
    });
  }
  /**
   * @param {Array} cameras - Array of cameras to stop watching
   * @private
   */
  unsubscribe(cameras) {
    cameras.forEach((cameraId) => {
      this.channel.push(`close${cameraId.toString()}`, '');
    });
  }

  /**
   * @param {number} cameraId
   * @param {Function} callback - function to be called when frame is
   *                   received for given camera
   */
  join(cameraId, callback) {
    this.leave(cameraId);

    this.channel.off(`jpg${cameraId}`, ref);

    const ref = this.channel.on(`jpg${cameraId}`, callback);
    this.watchedCameras.set(cameraId, ref);

    this.subscribe([cameraId]);
  }

  /**
   * @param {number} cameraId - stop watching given camera id
   */
  leave(cameraId) {
    const ref = this.watchedCameras.get(cameraId);
    if (false && ref !== undefined) { // ref doesn't work yet :(
      this.channel.off(`jpg${cameraId}`, ref);
    }
    this.unsubscribe([cameraId]);
    this.watchedCameras.delete(cameraId);
  }

  /**
   * @param {number} cameraId - id of camera to change
   * @param {string} resolution - "sd" or "hd" specifying desired resolution
   *
   */
  setResolution(cameraId, resolution) {
    if (resolution === 'hd') {
      this.channel.push(`hdon${cameraId.toString()}`, '');
    } else if (resolution === 'sd') {
      this.channel.push(`hdoff${cameraId.toString()}`, '');
    }
  }
}

export default CameraChannel;
