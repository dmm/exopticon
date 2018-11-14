'use strict';

import React from 'react';
import ReactDOM from 'react-dom';

import CameraChannel from './camera_channel';
import CameraPanel from './components/camera_panel';

/**
 * Supervisor class slowly being replaced with react components
 * @Class
 */
class CameraManager {
  /**
   * @param {Pheonix.socket} socket - phoenix socket to use as camera transport
   * @param {Number} columns - number of columns in camera panel
   * @param {Boolean} showDisabled - show disabled camera flag
   */
  constructor(socket, columns = 0, showDisabled = false) {
    this.socket = socket;
    this.channel = new CameraChannel(this.socket);
    this.visibleCameras = new Map();
    this.checkingVisibility = false;
    this.lastScrollPosition = 0;

    this.panel =
      React.createElement(CameraPanel,
                          {
                            channel: this.channel,
                            initialCameras: new Map(),
                            initialColumns: columns,
                            showDisabled: showDisabled,
                          });

    this.component = ReactDOM.render(this.panel,
                                     document.getElementById('allCameras'));
  }
  /**
   * @param {Array} allCameras - array of exopticon camera objects to
   * be used by camera manager
   */
  updateCameras(allCameras) {
    this.component.setCameras(allCameras);
  }

  /**
   * @param {Number} columnCount - number of columns wide the view should be
   */
  setColumnCount(columnCount) {
    this.component.setColumnCount(columnCount);
  }

  /**
   * rotate fullscreen index by argument
   * @param {Number} amount - number to shift index by
   * @private
   */
  shiftFullscreen(amount) {
    this.component.shiftFullscreen(amount);
  }
}

export default CameraManager;
