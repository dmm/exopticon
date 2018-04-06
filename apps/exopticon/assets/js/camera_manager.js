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
   */
  constructor(socket, columns = 0) {
    this.socket = socket;
    this.visibleCameras = new Map();
    this.checkingVisibility = false;
    this.lastScrollPosition = 0;

    this.panel =
      React.createElement(CameraPanel,
                          {
                            socket: this.socket,
                            initialCameras: new Map(),
                            initialColumns: columns
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
}

export default CameraManager;
