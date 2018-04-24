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

import CameraOverlay from './camera_overlay';
import StatusOverlay from './status_overlay';

import '../../css/components/camera_view.css';

/**
 * CameraView class implements a components that shows video for a
 * single camera. It implements logic that pauses playback when the
 * component is not in the viewport.
 *
 */
class CameraView extends React.Component {
  /**
   * CameraView constructor
   * @param {Object} props - accepts cameraPlayer and camera props
   */
  constructor(props) {
    super(props);

    this.state = {
      status: 'loading',
    };
    this.props.cameraPlayer.statusCallback = (newStatus) => {
      this.setState({status: newStatus});
    };
    this.visibilityCheck = this.visibilityCheck.bind(this);
    this.handleScroll = this.handleScroll.bind(this);
    this.handleResize = this.handleResize.bind(this);
    this.handleFullscreen = this.handleFullscreen.bind(this);
    this.fullscreenCallback = this.fullscreenCallback.bind(this);

    this.isScrolling = true; // junk value to be replaced by timer
    this.isResizing = true;
    this.initialTimeout = true; // junk value to be replaced by timer
  }

  /**
   * callback that checks if view within viewport, if not it pauses playback
   */
  visibilityCheck() {
    const fullscreenNotMe = !(fscreen.fullscreenElement !== null
                              && fscreen.fullscreenElement !== this._container);
    const visible = verge.inY(this._container) && fullscreenNotMe;
    if (this._img && visible
        && this.state.status === 'paused') {
      this.setState({status: 'loading'});
      this.props.cameraPlayer.playRealtime(this._img, () => {
        this.setState({status: 'playing'});
      });
    } else if (this._img && !visible && this.state.status !== 'paused') {
      this.props.cameraPlayer.stop();
      console.log('stopping ' + this.props.camera.id.toString());
    }
  }

  /**
   * scroll event handler, debounces by setting a timer and then calls
   * visibilityCheck
   * @private
   */
  handleScroll() {
    window.clearTimeout(this.isScrolling);
    this.isScrolling = window.setTimeout(this.visibilityCheck, 33);
  }

  /**
   * resize event handler. It debounces by setting a timer and then calls
   * visibilityCheck
   * @private
   */
  handleResize() {
    window.clearTimeout(this.isResizing);
    this.isResizing = window.setTimeout(this.visibilityCheck, 33);
  }

  /**
   * handle fullscreen event
   *
   */
  handleFullscreen() {
    screen.lockOrientationUniversal = screen.lockOrientation
      || screen.mozLockOrientation
      || screen.msLockOrientation;

    if (this._container === fscreen.fullscreenElement) {
      this.props.cameraPlayer.setResolution('hd');
      screen.lockOrientationUniversal('landscape-primary');
    } else {
      this.props.cameraPlayer.setResolution('sd');
    }
  }

  /*
   * fullscreen callback to pass to camera overlay
   * @private
   **/
  fullscreenCallback() {
    if (this.props.fullscreenHandler) {
      this.props.fullscreenHandler(this._container);
    }
  }

  /**
   * react mount handler.
   * adds event handlers and starts playback if visible
   * @private
   */
  componentDidMount() {
    window.addEventListener('scroll', this.handleScroll);
    window.addEventListener('resize', this.handleResize);
    fscreen.addEventListener('fullscreenchange', this.handleFullscreen);
    this.initialTimeout = setTimeout(() => {
      if (verge.inY(this._container)) {
        this.setState({status: 'loading'});
        this.props.cameraPlayer.playRealtime(this._img, () => {
          this.setState({status: 'playing'});
        });
      } else {
        this.setState({status: 'paused'});
      }
    }, 500);
  }

  /**
   * react umount handler
   * removes event handlers and stops player
   */
  componentWillUnmount() {
    clearTimeout(this.initialTimeout);
    window.removeEventListener('scroll', this.handleScroll);
    window.removeEventListener('resize', this.handleResize);
    fscreen.removeEventListener('fullscreenchange', this.handleFullscreen);
    this.props.cameraPlayer.stop();
  }

  /**
   * react render function
   * @return {Object} react entity
   */
  render() {
    const id = 'camera' + this.props.camera.id.toString();
    let status;

    if (this.state.status === 'paused') {
      status = (
        <StatusOverlay status="paused" cameraName={this.props.camera.name}/>
      );
    } else if (this.state.status === 'loading') {
      status = (
        <StatusOverlay status="loading" cameraName={this.props.camera.name}/>
      );
    }
    return (
      <div id={id}
           className="camera"
           ref={
             (el) => {
               this._container = el;
             }
        }>
        <CameraOverlay camera={this.props.cameraPlayer}
                       fullscreenCallback={this.fullscreenCallback}
                       />
        { status }
        <img ref={
               (el) => {
                 this._img = el;
               }
          }/>
      </div>
    );
  }
}

CameraView.propTypes = {
  camera: PropTypes.object.isRequired,
  cameraPlayer: PropTypes.object.isRequired,
  fullscreenHandler: PropTypes.func,
};

export default CameraView;
