'use strict';

import React from 'react';
import verge from 'verge';
import CameraOverlay from './camera_overlay';
import StatusOverlay from './status_overlay';

class CameraView extends React.Component {
  constructor(props) {
    super(props);

    this.state = { playing: true };
    this.handleScroll = this.handleScroll.bind(this);
    this.isScrolling = true;
  }

  handleScroll() {
    window.clearTimeout(this.isScrolling);

    this.isScrolling = setTimeout(() => {
      const visible = verge.inY(this._img);
      if (this._img && visible && !this.props.cameraPlayer.playing) {
        this.props.cameraPlayer.playRealtime(this._img);
        this.setState({playing: true});
      } else if (this._img && !visible && this.props.cameraPlayer.playing) {
        this.props.cameraPlayer.stop();
        this.setState({playing: false});
      }

    }, 33);
  }

  componentDidMount() {
    window.addEventListener('scroll', this.handleScroll);
    this.props.cameraPlayer.playRealtime(this._img);
  }

  componentWillUnmount() {
    window.removeEventListener('scroll', this.handleScroll);
    this.props.cameraPlayer.stop();
  }

  render() {
    const id = 'camera' + this.props.camera.id.toString();
    var status = undefined;

    if (!this.state.playing) {
      status = (
        <StatusOverlay status="paused" />
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
        <CameraOverlay camera={this.props.cameraPlayer} />
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

export default CameraView;
