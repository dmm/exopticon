'use strict';

import React from 'react';
import CameraChannel from '../camera_channel';
import CameraView from './camera_view';
import CameraPlayer from '../camera_player';

import './../../css/components/camera_panel.css';

class CameraPanel extends React.Component {
  constructor(props) {
    super(props);

    this.cameraElements = new Map();

    var channel = props.socket.channel('camera:stream');
    channel.join();

    this.state = {
      cameras: props.initialCameras,
      channel: channel,
      viewColumns: 0
    };
  }

  componentDidMount() {

  }

  componentWillUnmount() {
    this.state.channel.leave();
  }

  render() {
    var cameraPanelClass = 'camera-panel';

    if (this.state.viewColumns !== 0) {
      cameraPanelClass += `panel-col-${this.state.viewColumns.toString()}`;
    }
    this.cameraElements.clear();
    const cameras = [];
    const cameraChannel = new CameraChannel(this.state.channel);
    this.state.cameras.forEach((cam) => {
      let player = new CameraPlayer(cam, cameraChannel);
      cameras.push(
        <div key={cam.id} className="wrapper">
          <div className="camera-width"></div>
          <div className="content">
            <CameraView camera={cam}
                        cameraPlayer={player}

                        ref={
                          (el) => {
                            this.cameraElements.set(cam.id, el);
                          }
              }/>
          </div>
        </div>
      );
    });
    return (
      <div className={cameraPanelClass}>
        {cameras}
      </div>
    );
  }
}

export default CameraPanel;
