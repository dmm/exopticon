'use strict';

import msgpack from './msgpack';

/**
 * @class
 */
class CameraChannel {
  /**
   * @param {WebSocketClient} client
   */
  constructor(client) {
    this.client = client;
    this.prefix = 'camera';

    this.watchedCameras = new Map();

    this.subscribe = this.subscribe.bind(this);
    this.unsubscribe = this.unsubscribe.bind(this);
    this.join = this.join.bind(this);
    this.leave = this.leave.bind(this);

    this.client.onopen = () => {
      this.subscribe(this.watchedCameraIds());
    };

    this.client.onmessage = (event) => {
      let msg = this.decodeMessage(event.data);

      if (msg === undefined) return;

      if (this.watchedCameras.has(msg.camera_id)) {
        this.watchedCameras.get(msg.camera_id).callback(msg);
      }
    };
    /*
    this.channel.on('subscribe', () => {
      this.subscribe(this.watchedCameraIds());
    });
    this.channel.onMessage = (event, payload, ref) => {
      if (event.match(/jpg([0-9]+)/)) {
        this.channel.push('ack', {ts: payload.ts});
      }
      return payload;
    };
    */
  }

  /**
   * @param {ArrayBuffer} rawdata
   * @return {Object} decoded object
   */
  decodeMessage(rawdata) {
    if (!rawdata) {
      return undefined;
    }
    let binary = new Uint8Array(rawdata);
    let data;
    data = binary;

    let msg = msgpack.decode(data);
    return msg;
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
  subscribe(cameras, resolution) {
    if (cameras.length == 0) return;
    this.client.send(JSON.stringify({
      command: 'subscribe',
      resolution: {type: resolution},
      cameraIds: cameras,
    }));
  }
  /**
   * @param {Array} cameras - Array of cameras to stop watching
   * @private
   */
  unsubscribe(cameras) {
    this.client.send(JSON.stringify({
      command: 'unsubscribe',
      resolution: {type: 'SD'},
      cameraIds: cameras,
    }));
  }

  /**
   * @param {number} cameraId
   * @param {Function} callback - function to be called when frame is
   *                   received for given camera
   */
  join(cameraId, resolution, callback) {
    this.leave(cameraId);

    this.watchedCameras.set(cameraId, {
      callback: callback,
      resolution: resolution,
    });

    this.subscribe([cameraId], resolution);
  }

  /**
   * @param {number} cameraId - stop watching given camera id
   */
  leave(cameraId) {
    this.unsubscribe([cameraId]);
    this.watchedCameras.delete(cameraId);
  }

  /**
   * @param {number} cameraId - id of camera to change
   * @param {string} resolution - "sd" or "hd" specifying desired resolution
   *
   */
  setResolution(cameraId, resolution) {
    return;
    if (resolution === 'hd') {
      this.channel.push(`hdon${cameraId.toString()}`, '');
    } else if (resolution === 'sd') {
      this.channel.push(`hdoff${cameraId.toString()}`, '');
    }
  }
}

export default CameraChannel;
