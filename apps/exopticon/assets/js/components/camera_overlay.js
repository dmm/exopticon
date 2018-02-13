'use strict';

import PropTypes from 'prop-types';
import React from 'react';

import OverlayButton from './overlay_button';

import '../../css/components/camera_overlay.css';

/**
 * CameraOverlay - class implementing ptz and camera name overlay for
 *                 CameraViews
 * @class
 */
class CameraOverlay extends React.Component {
  /**
   * @param {Object} props
   */
  constructor(props) {
    super(props);
    this.state = {opacity: 0};
    this.touchEnabled = false;
    this.touchMoving = false;

    this.mouseEnter = this.mouseEnter.bind(this);
    this.mouseLeave = this.mouseLeave.bind(this);
    this.touchMove = this.touchMove.bind(this);
    this.touchEnd = this.touchEnd.bind(this);
  }

  /**
   * callback called when mouse enters overlay
   * @private
   */
  mouseEnter() {
    if (this.touchEnabled) {
      return;
    }

    if (this.state.opacity != 0.0) {
      this.setState({opacity: 0.0});
    } else {
      this.setState({opacity: 1.0});
    }
  }

  /**
   * callback called when mouse leave overlay
   * @private
   */
  mouseLeave() {
    this.setState({opacity: 0.0});
  }

  /**
   * callback called on touchmove event
   * @private
   */
  touchMove() {
    this.touchMoving = true;
  }

  /**
   * callback called on touchend event
   * @param {Object} e - event object
   * @private
   */
  touchEnd(e) {
    this.touchEnabled = true;

    if (this.touchMoving === true) {
      this.touchMoving = false;
      return;
    }

    if (e.target !== e.currentTarget && this.state.opacity < 1.0) {
      // When the ptz overlay is not shown prevent the button from
      // getting the click event.
      e.preventDefault();
      // Show the overlay
      this.setState({opacity: 1.0});
      return;
    }

    if (e.target !== e.currentTarget) {
      // Touch event was on a button, ignore it
      return;
    } else if (this.state.opacity != 0.0) {
      this.setState({opacity: 0.0});
    } else {
      this.setState({opacity: 1.0});
    }
  }

  /**
   * render CameraOverlay
   * @return {Object} returns react instance for camera overlay
   */
  render() {
    let ptz = (
      <div className="ptz-controls"
           onMouseEnter={this.mouseEnter}
           onMouseLeave={this.mouseLeave}
           onTouchStart={this.touchStart}
           onTouchMove={this.touchMove}
           onTouchEnd={this.touchEnd}
           />
    );

    if (this.props.camera.hasPtz()) {
      ptz = (
        <div className="ptz-controls"
             onMouseEnter={this.mouseEnter}
             onMouseLeave={this.mouseLeave}
             onTouchStart={this.touchStart}
             onTouchMove={this.touchMove}
             onTouchEnd={this.touchEnd}
             >
          <OverlayButton
            label="◀"
            extraClass="left-arrow"
            onClick={this.props.camera.left} />
          <OverlayButton
            label="▶"
            extraClass="right-arrow"
            onClick={this.props.camera.right} />
          <OverlayButton
            label="▲"
            extraClass="up-arrow"
            onClick={this.props.camera.up} />
          <OverlayButton
            label="▼"
            extraClass="down-arrow"
            onClick={this.props.camera.down} />
        </div>
      );
    }
    return (
        <div style={{opacity: this.state.opacity}}
             className="camera-overlay"
             >
        { ptz }
        <div className="camera-name">{this.props.camera.name}</div>
      </div>
    );
  }
}

CameraOverlay.propTypes = {
  camera: PropTypes.object,
};

export default CameraOverlay;

