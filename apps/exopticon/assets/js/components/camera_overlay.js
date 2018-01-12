'use strict';

import React from 'react';
import OverlayButton from './overlay_button';

class CameraOverlay extends React.Component {
  constructor(props) {
    super(props);
    this.state = {opacity: 0};

    this.mouseEnter = this.mouseEnter.bind(this);
    this.mouseLeave = this.mouseLeave.bind(this);
    this.touchStart = this.touchStart.bind(this);
  }

  mouseEnter() {
    this.setState({opacity: 1.0});
  }

  mouseLeave() {
    this.setState({opacity: 0.0});
  }

  touchStart(e) {
    if (e.target !== e.currentTarget && this.state.opacity > 0) {
      return;
    }
    if (this.state.opacity != 0.0) {
      this.setState({opacity: 0.0});
    } else {
      this.setState({opacity: 1.0});
    }
  }

  render() {
    if (this.props.camera.hasPtz()) {
    return (
      <div style={{opacity: this.state.opacity}}
           className="camera-overlay"
           onMouseEnter={this.mouseEnter}
           onMouseLeave={this.mouseLeave}
           onTouchStart={this.touchStart}>
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
    } else {
      return <div />;
    }
  }
}

export default CameraOverlay;

