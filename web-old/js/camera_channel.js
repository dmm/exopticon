"use strict";

import msgpack from "./msgpack";

/**
 * @class
 */
class CameraChannel {
  /**
   * @param {WebSocketClient} client
   */
  constructor(client) {
    this.client = client;
    this.prefix = "camera";

    this.subscriptions = new Map();

    this.subscribe = this.subscribe.bind(this);
    this.unsubscribe = this.unsubscribe.bind(this);
    this.join = this.join.bind(this);
    this.leave = this.leave.bind(this);

    this.client.onopen = () => {
      this.subscriptions.forEach((s, cameraId) => {
        this.subscribe([parseInt(cameraId)], s.resolution);
      });
    };

    this.client.onmessage = event => {
      let msg = this.decodeMessage(event.data);

      if (msg === undefined) return;

      const key = msg.camera_id.toString() + msg.resolution.type;
      if (this.subscriptions.has(key)) {
        this.subscriptions.get(key).callback(msg);
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
   * closes all watched cameras
   */
  close() {
    this.subscriptions.forEach(s => {
      this.leave(s.cameraId, s.resolution);
    });
  }

  /**
   * @param {Array} cameras - Array of camera ids to watch
   * @private
   */
  subscribe(cameras, resolution = "SD") {
    if (cameras.length == 0) return;
    this.client.send(
      JSON.stringify({
        command: "subscribe",
        resolution: { type: resolution },
        cameraIds: cameras
      })
    );
  }
  /**
   * @param {Array} cameras - Array of cameras to stop watching
   * @private
   */
  unsubscribe(cameras, resolution) {
    this.client.send(
      JSON.stringify({
        command: "unsubscribe",
        resolution: { type: resolution },
        cameraIds: cameras
      })
    );
  }

  /**
   * @param {number} cameraId
   * @param {string} resolution
   * @param {Function} callback - function to be called when frame is
   *                   received for given camera
   */
  join(cameraId, resolution, callback) {
    if (this.subscriptions.has(cameraId.toString() + resolution)) {
      this.leave(cameraId, resolution);
    }

    this.subscriptions.set(cameraId.toString() + resolution, {
      callback: callback,
      resolution: resolution
    });

    this.subscribe([cameraId], resolution);
  }

  /**
   * @param {number} cameraId - stop watching given camera id
   */
  leave(cameraId, resolution) {
    if (this.subscriptions.has(cameraId.toString() + resolution)) {
      this.unsubscribe([cameraId], resolution);
      this.subscriptions.delete(cameraId.toString() + resolution);
    }
  }

  /**
   * @param {number} cameraId - id of camera to change
   * @param {string} resolution - "sd" or "hd" specifying desired resolution
   *
   */
  setResolution(cameraId, resolution) {
    return;
    if (resolution === "hd") {
      this.channel.push(`hdon${cameraId.toString()}`, "");
    } else if (resolution === "sd") {
      this.channel.push(`hdoff${cameraId.toString()}`, "");
    }
  }
}

export default CameraChannel;
