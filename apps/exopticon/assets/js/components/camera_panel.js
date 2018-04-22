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

import fscreen from 'fscreen';
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
    this.cameraRequestFullscreen = this.cameraRequestFullscreen.bind(this);

    let channel = props.socket.channel('camera:stream');
    channel.join();

    this.state = {
      cameras: props.initialCameras,
      channel: channel,
      viewColumns: props.initialColumns,
    };
  }

  /**
   * @param {Number} columnCount - sets number of camera columns
   */
  setColumnCount(columnCount) {
    this.setState({viewColumns: columnCount});
    this.cameraElements.forEach((c) => {
      c.visibilityCheck();
    });
  }

  /**
   * closes the channel when component unmounts
   * @private
   */
  componentWillUnmount() {
    this.state.channel.leave();
  }

  /**
   * attempts to make element fullscreen
   * @private
   */
  cameraRequestFullscreen(elem) {
    if (fscreen.fullscreenElement === null) {
      // fullscreen not enabled, request it
      fscreen.requestFullscreen(elem);
    } else {
      fscreen.exitFullscreen();
    }

    this.cameraElements.forEach((c) => {
      c.visibilityCheck();
    });
  }

  /**
   * renders the component
   * @return {Object} react component
   */
  render() {
    let cameraPanelClass = 'camera-panel';

    if (this.state.viewColumns !== 0) {
      cameraPanelClass += ` panel-col-${this.state.viewColumns.toString()}`;
    }
    this.cameraElements.clear();
    const cameras = [];
    const cameraChannel = new CameraChannel(this.state.channel);
    this.state.cameras.forEach((cam) => {
      if (!this.props.showDisabled && cam.mode === 'disabled') {
        return;
      }
      let player = new CameraPlayer(cam, cameraChannel);
      cameras.push(
        <div key={cam.id} className="wrapper">
          <div className="camera-width"></div>
          <div className="content">
            <CameraView camera={cam}
                        cameraPlayer={player}
                        fullscreenHandler={this.cameraRequestFullscreen}
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
  initialColumns: PropTypes.number,
  showDisabled: PropTypes.bool,
};

CameraPanel.defaultProps = {
  showDisabled: false,
};

export default CameraPanel;
