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

    this.playing = false;

    this.props.cameraPlayer.statusCallback = (newStatus) => {
      this.setState({status: newStatus});
    };

    this.play = this.play.bind(this);
    this.pause = this.pause.bind(this);

    this.isScrolling = true; // junk value to be replaced by timer
    this.isResizing = true;
    this.initialTimeout = true; // junk value to be replaced by timer
  }

  /**
   * setVideoResolution
   *
   *
   */
  setResolution(resolution) {
    this.props.cameraPlayer.setResolution(resolution);
  }

  play() {
    if (this.playing === true) {
      return;
    }
    this.playing = true;
    this.setState({
      status: 'loading',
    });
    this.props.cameraPlayer.playRealtime(this._img, () => {
      this.setState({status: 'playing'});
    });
  }

  pause() {
    this.playing = false;
    this.props.cameraPlayer.stop();
    this.setState({
      status: 'hidden',
    });
  }

  /**
   * react mount handler.
   * adds event handlers and starts playback if visible
   * @private
   */
  componentDidMount() {

    this.initialTimeout = setTimeout(() => {
      if (verge.inY(this._container)) {
        this.play();
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

    this.pause();
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
                       fullscreenCallback={this.props.fullscreenHandler}
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

CameraView.defaultProps = {
  fullscreenHandler: () => {},
};

export default CameraView;
