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

    const ref = this.channel.on(`jpg${cameraId}`, (data) => {
      this.channel.push('ack', { ts: data.ts });
      callback(data);
    });

    this.watchedCameras.set(cameraId, ref);

    this.subscribe([cameraId]);
  }

  leave(cameraId) {
    const ref = this.watchedCameras.get(cameraId);
    if (ref !== undefined) {
      this.channel.off(`jpg${cameraId}`, ref);
    }
    this.unsubscribe([cameraId]);
    this.watchedCameras.delete(cameraId);
  }
}

export default CameraChannel;
