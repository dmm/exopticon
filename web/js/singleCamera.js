/*
 * This file is part of Exopticon (https://github.com/dmm/exopticon).
 * Copyright (c) 2018 David Matthew Mattli
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

import CameraManager from './camera_manager';
import socket from './socket';

/**
 * @param {number} cameraId - id of camera to fetch
 */
function updateCameras(cameraId) {
  let request = new XMLHttpRequest();
  request.open('GET', 'v1/cameras', true);

  request.onload = function() {
    if (this.status >= 200 && this.status < 400) {
      // Success!
      let cameras = JSON.parse(this.response);
      cameras.forEach(function(c) {
        if (c.id === cameraId) {
          window.cameraManager.updateCameras(cameras);
        }
      });
    } else {
      console.log('reached server but something went wrong');
    }
  };

  request.onerror = function() {
    console.log('There was a connection error of some sort...');
  };

  request.send();
}

window.onload = function() {
  let cam = document.getElementById('singleCamera');
  if (cam) {
    window.cameraManager = new CameraManager(socket);
    updateCameras(cam.dataset.id);
    setInterval(updateCameras, 5000);
  }
};
