'use strict';

import PropTypes from 'prop-types';
import React from 'react';

import CameraChannel from '../camera_channel';
import CameraPlayer from '../camera_player';

import CameraView from './camera_view';


import './../../css/components/camera_panel.css';

/**
 * CameraPanel: implements a panel of camera views
 * @class
 */
class CameraPanel extends React.Component {
  /**
   * CameraPanel constructor
   * @param {object} props
   */
  constructor(props) {
    super(props);

    this.cameraElements = new Map();

    let channel = props.socket.channel('camera:stream');
    channel.join();

    this.state = {
      cameras: props.initialCameras,
      channel: channel,
      viewColumns: 0,
    };
  }

  /**
   * closes the channel when component unmounts
   * @private
   */
  componentWillUnmount() {
    this.state.channel.leave();
  }

  /**
   * renders the component
   * @return {Object} react component
   */
  render() {
    let cameraPanelClass = 'camera-panel';

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

CameraPanel.propTypes = {
  socket: PropTypes.object.isRequired,
  initialCameras: PropTypes.object,
};

export default CameraPanel;
