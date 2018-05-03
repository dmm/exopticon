'use strict';

import React from 'react';
import ReactDOM from 'react-dom';

import CameraPanel from './components/camera_panel';

/**
 * Supervisor class slowly being replaced with react components
 * @Class
 */
class CameraManager {
  /**
   * @param {Pheonix.socket} socket - phoenix socket to use as camera transport
   * @param {Number} columns - number of columns in camera panel
   */
  constructor(socket, columns = 0, showDisabled = false) {
    this.socket = socket;
    this.visibleCameras = new Map();
    this.checkingVisibility = false;
    this.lastScrollPosition = 0;

    this.panel =
      React.createElement(CameraPanel,
                          {
                            socket: this.socket,
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
    this.component.setState({cameras: allCameras});
  }

  /**
   * @param {Number} columnCount - number of columns wide the view should be
   */
  setColumnCount(columnCount) {
    this.component.setColumnCount(columnCount);
  }

  shiftFullscreen(amount) {
    this.component.shiftFullscreen(amount);
  }
}

export default CameraManager;
