'use strict';

import React from 'react';
import ReactDOM from 'react-dom';
import CameraOverlay from './components/camera_overlay';
import CameraPlayer from './camera_player';
import CameraView from './components/camera_view';
import CameraPanel from './components/camera_panel';

var camera = function(id, name) {
    this.id = id;
    this.name = name;

    return this;
};

class CameraManager {
  constructor(socket) {
    this.socket = socket;
    this.visibleCameras = new Map();
    this.checkingVisibility = false;
    this.lastScrollPosition = 0;

    this.panel =
      React.createElement(CameraPanel,
                          {
                            socket: this.socket,
                            initialCameras: new Map()
                          });

    this.panelComponent = ReactDOM.render(this.panel, document.getElementById('allCameras'));
  }

  updateCameras(allCameras) {
    this.panelComponent.setState({cameras: allCameras});
  }
}

export default CameraManager;
