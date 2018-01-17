'use strict';

import React from 'react';
import CameraView from './camera_view';
import CameraPlayer from '../camera_player';

class CameraPanel extends React.Component {
  constructor(props) {
    super(props);
    this.state = { cameras: props.initialCameras };
    this.updateCameras = this.updateCameras.bind(this);
    this.updateCameras();
    this.cameraElements = new Map();
  }

  updateCameras() {
    fetch('/v1/cameras', {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json'
      }
    }).then((response) => {
      return response.json();
    }).then((cameras) => {
      this.setState({cameras: cameras});
    });
  }

  render() {
    this.cameraElements.clear();
    const cameras =[];
    this.state.cameras.forEach((cam) => {
      let player = new CameraPlayer(cam, this.props.socket);
      cameras.push(
        <CameraView camera={cam}
                    cameraPlayer={player}
                    key={cam.id}
                    ref={
                      (el) => {
                        this.cameraElements.set(cam.id, el);
                      }
          }/>
      );
    });
    return (
      <div className="camera-panel">
        {cameras}
      </div>
    );
  }
}

export default CameraPanel;
