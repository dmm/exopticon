'use strict';

import React from 'react';
import verge from 'verge';
import CameraOverlay from './camera_overlay';
import StatusOverlay from './status_overlay';

class CameraView extends React.Component {
  constructor(props) {
    super(props);

    this.state = { playing: false };
    this.visibilityCheck = this.visibilityCheck.bind(this);
    this.handleScroll = this.handleScroll.bind(this);
    this.handleResize = this.handleResize.bind(this);

    this.isScrolling = true; // junk value to be replaced by timer
    this.isResizing = true;
  }

  visibilityCheck() {
    console.log('viz check!');
    const visible = verge.inY(this._container);
    if (this._img && visible && !this.props.cameraPlayer.playing) {
      this.props.cameraPlayer.playRealtime(this._img);
      this.setState({playing: true});
    } else if (this._img && !visible && this.props.cameraPlayer.playing) {
      this.props.cameraPlayer.stop();
      this.setState({playing: false});
    }
  }

  handleScroll() {
    window.clearTimeout(this.isScrolling);
    this.isScrolling = window.setTimeout(this.visibilityCheck, 33);
  }

  handleResize() {
    window.clearTimeout(this.isResizing);
    this.isResizing = window.setTimeout(this.visibilityCheck, 33);
  }

  componentDidMount() {
    window.addEventListener('scroll', this.handleScroll);
    window.addEventListener('resize', this.handleResize);
    setTimeout(() => {
      if (verge.inY(this._container)) {
        this.setState({ playing: true });
        this.props.cameraPlayer.playRealtime(this._img);
      } else {
        this.setState({ playing: false });
      }
    }, 500);

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
