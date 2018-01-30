'use strict';

class CameraChannel {
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
        this.channel.push('ack', { ts: payload.ts });
      }
      return payload;
    };
  }

  watchedCameraIds() {
    return Array.from(this.watchedCameras.keys());
  }

  close() {
    this.unsubscribe(this.watchedCameraIds());
  }

  subscribe(cameras) {
   cameras.forEach((cameraId) => {
      this.channel.push(`watch${cameraId.toString()}`, '');
    });
  }

  unsubscribe(cameras) {
    cameras.forEach((cameraId) => {
      this.channel.push(`close${cameraId.toString()}`, '');
    });
  }

  join(cameraId, callback) {
    this.leave(cameraId);

    this.channel.off(`jpg${cameraId}`, ref);

    const ref = this.channel.on(`jpg${cameraId}`, callback);
    this.watchedCameras.set(cameraId, ref);

    this.subscribe([cameraId]);
  }

  leave(cameraId) {
    const ref = this.watchedCameras.get(cameraId);
    if (false && ref !== undefined) { // ref doesn't work yet :(
      this.channel.off(`jpg${cameraId}`, ref);
    }
    this.unsubscribe([cameraId]);
    this.watchedCameras.delete(cameraId);
  }
}

export default CameraChannel;
