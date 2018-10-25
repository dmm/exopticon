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
import verge from 'verge';

import CameraChannel from '../camera_channel';

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

    this.setCameras = this.setCameras.bind(this);
    this.shiftFullscreen = this.shiftFullscreen.bind(this);
    this.setFullscreenIndex = this.setFullscreenIndex.bind(this);
    this.cameraRequestFullscreen = this.cameraRequestFullscreen.bind(this);
    this.panelActive = true;
    this.fullscreenId = -1;

    /* Bind event handlers */
    this.handleScroll = this.handleScroll.bind(this);
    this.handleResize = this.handleResize.bind(this);
    this.visibilityChange = this.visibilityChange.bind(this);
    this.visibilityCheck = this.visibilityCheck.bind(this);
    this.updateActive = this.updateActive.bind(this);

    let enabledCameras = this.filterCameras(props.initialCameras);

    this.state = {
      cameras: enabledCameras,
      viewColumns: props.initialColumns,
      fullscreenIndex: -1,
    };
  }

  /** *
   * Event callbacks
   */

  /**
   * scroll event handler, debounces by setting a timer and then calls
   * visibilityCheck
   * @private
   */
  handleScroll() {
    window.clearTimeout(this.isScrolling);
    this.isScrolling = window.setTimeout(this.visibilityCheck, 300);
  }

  /**
   * resize event handler. It debounces by setting a timer and then calls
   * visibilityCheck
   * @private
   */
  handleResize() {
    window.clearTimeout(this.isResizing);
    this.isResizing = window.setTimeout(this.visibilityCheck, 300);
  }

  /**
   * visibilitychange event handler
   * @private
   *
   */
  visibilityChange() {
    if (document['hidden']) {
      this.panelActive = false;
    } else {
      this.panelActive = true;
    }
    this.updateActive();
  }

  /**
   * visibilityCheck
   * @private
   */
  visibilityCheck() {
    this.updateActive();
  }

  /**
   * updateActive
   * @private
   */
  updateActive() {
    const active = this.panelActive;
    const fsi = this.fullscreenId;

    for (let c of this.cameraElements.values()) {
      const visible = verge.inY(c._container);
      if (visible && active && fsi === -1) {
        c.setResolution('sd');
        c.play();
      } else if (visible && active && fsi === c.props.camera.id) {
        c.setResolution('hd');
        c.play();
      } else {
        c.pause();
      }
    };
  }

  /**
   * @param {List} cameras - list of cameras to process
   * @return {List} - list of active cameras
   * @private
   */
  filterCameras(cameras) {
    let enabledCameras = [];

    cameras.forEach((c) => {
      if (c.mode === 'enabled' && !this.props.showDisabled) {
        enabledCameras.push(c);
      } else if (c.mode === 'disabled' && this.props.showDisabled) {
        enabledCameras.push(c);
      }
    });

    return enabledCameras;
  }
  /**
   * @param {List} cameras - set cameras panel's cameras to this list
   *
   */
  setCameras(cameras) {
    this.setState({cameras: this.filterCameras(cameras)});
  }

  /**
   * @param {Number} columnCount - sets number of camera columns
   */
  setColumnCount(columnCount) {
    this.setState({viewColumns: columnCount});
    this.visibilityCheck();
  }

  /**
   * react mount handler
   * @private
   */
  componentDidMount() {
    this.visibilityChange();
    window.addEventListener('scroll', this.handleScroll);
    window.addEventListener('resize', this.handleResize);
    window.addEventListener('visibilitychange', this.visibilityChange);
  }

  /**
   * closes the channel when component unmounts
   * @private
   */
  componentWillUnmount() {
    window.removeEventListener('scroll', this.handleScroll);
    window.removeEventListener('resize', this.handleResize);
    window.removeEventListener('visibilityChange', this.visibilityChange);
  }

  /**
   * shift fullscreen by amount
   * @param {Number} amount - amount to shift
   */
  shiftFullscreen(amount) {
    if (this.state.fullscreenIndex === -1) {
      return;
    }
    const newIndex = (this.state.fullscreenIndex
                      + amount
                      + this.state.cameras.length)
          % this.state.cameras.length;
    this.setFullscreenIndex(newIndex);
  }

  /**
   * sets fullscreen index
   * @param {Number} i - new fullscreen index
   */
  setFullscreenIndex(i) {
    const newIndex = this.state.fullscreenIndex === i ? -1 : i;

    this.setState({
      fullscreenIndex: newIndex,
    });

    this.fullscreenId = newIndex == -1 ? -1 :this.state.cameras[newIndex].id;

    this.updateActive();
    this.handleResize();
    return;

    if (newIndex === -1) {
      for (let c of this.cameraElements.values()) {
        c.setResolution('sd');
      }
      fscreen.exitFullscreen();
    } else {
      const camera = this.state.cameras[i];
      let cameraComponent = this.cameraElements.get(camera.id);

      for (let c of this.cameraElements.values()) {
        if (c !== null && c !== cameraComponent) {
          c.setResolution('sd');
          c.pause();
        }
      }
      cameraComponent.setResolution('hd');
      cameraComponent.play();
    }
  }

  /**
   * attempts to make element fullscreen
   * @param {Object} elem - new fullscreen element
   * @private
   */
  cameraRequestFullscreen(elem) {
    screen.lockOrientationUniversal = screen.lockOrientation
      || screen.mozLockOrientation
      || screen.msLockOrientation;
    if (fscreen.fullscreenElement === elem) {
      fscreen.exitFullscreen();
    } else {
      // fullscreen not enabled, request it
      fscreen.requestFullscreen(elem);
      screen.lockOrientationUniversal('landscape-primary');
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

    this.state.cameras.forEach((cam, i) => {
      let fsClass = '';
      if (this.state.fullscreenIndex !== -1
          && this.state.fullscreenIndex !== i) {
        fsClass += 'background ';
      }
      fsClass += this.state.fullscreenIndex === i
        ? 'wrapper fullscreen' : 'wrapper';
      cameras.push(
        <div key={cam.id} className={fsClass}>
          <div className="camera-width"></div>
          <div className="content">
          <CameraView camera={cam}
                      socket={this.props.socket}
                      fullscreenHandler={() => {
                        this.setFullscreenIndex(i);
              }}
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
