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

import CameraManager from '../camera_manager.js';
import socket from '../socket.js';

import MainView from './main';

/**
 * PageIndexView class
 * implements index page
 */
export default class View extends MainView {
  /**
   * fetches cameras and update camera panel
   */
  updateCameras() {
    fetch('/v1/cameras', {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((response) => {
      return response.json();
    }).then((cameras) => {
      window.cameraManager.updateCameras(cameras);
    });
  }

  /**
   * view entry point
   */
  mount() {
    super.mount();

    console.log('Mounting page index view.');
    window.cameraManager = new CameraManager(socket);
    this.updateCameras();

    document.addEventListener('keyup', (event) => {
      const keyName = event.key;
      console.log(keyName);
      if (keyName === 'ArrowRight') {
        window.cameraManager.shiftFullscreen(1);
      } else if (keyName === 'ArrowLeft') {
        window.cameraManager.shiftFullscreen(-1);
      }
    }, false);

    let cameraPanelWidth = window.localStorage.getItem('camera-panel-width');

    if (!cameraPanelWidth) {
      cameraPanelWidth = '0';
    }

    window.cameraManager.setColumnCount(parseInt(cameraPanelWidth, 10));

    let columnSelector = document.querySelector('#panel-width-select');
    columnSelector.value = cameraPanelWidth;

    columnSelector.onchange = function() {
      window.cameraManager.setColumnCount(parseInt(this.value, 10));
      window.localStorage.setItem('camera-panel-width', this.value);
    };

    let useFs = window.localStorage.getItem('use-fs-api');
    if (!useFs) {
      useFs = '1';
      window.localStorage.setItem('use-fs-api', '1');
    }
    let fsCheck = document.querySelector('#use-fs-checkbox');
    if (useFs == '1') {
      fsCheck.checked = true;
    } else {
      fsCheck.checked = false;
    }
    fsCheck.addEventListener('click', function(e) {
      if (e.target.checked === true) {
        window.localStorage.setItem('use-fs-api', '1');
      } else {
        window.localStorage.setItem('use-fs-api', '0');
      }
    });
  }
}
