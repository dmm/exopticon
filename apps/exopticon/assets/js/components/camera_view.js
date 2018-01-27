'use strict';

import React from 'react';
import verge from 'verge';
import CameraOverlay from './camera_overlay';
import StatusOverlay from './status_overlay';

import '../../css/components/camera_view.css';

class CameraView extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      status: 'loading'
    };
    this.props.cameraPlayer.statusCallback = (newStatus) => {
      this.setState({status: newStatus});
    };
    this.visibilityCheck = this.visibilityCheck.bind(this);
    this.handleScroll = this.handleScroll.bind(this);
    this.handleResize = this.handleResize.bind(this);

    this.isScrolling = true; // junk value to be replaced by timer
    this.isResizing = true;
  }

  visibilityCheck() {
    const visible = verge.inY(this._container);
    if (this._img && visible
        && this.state.status === 'paused') {
      this.props.cameraPlayer.playRealtime(this._img);
    } else if (this._img && !visible && this.state.status !== 'paused') {
      this.props.cameraPlayer.stop();
      console.log('stopping ' + this.props.camera.id.toString());
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
        this.setState({ status: 'loading' });
        this.props.cameraPlayer.playRealtime(this._img, () => {
          this.setState({status: 'playing'});
        });
      } else {
        this.setState({status: 'paused'});
      }
    }, 500);

  }

  componentWillUnmount() {
    window.removeEventListener('scroll', this.handleScroll);
    window.removeEventListener('resize', this.handleResize);
    this.props.cameraPlayer.stop();
  }

  render() {
    const id = 'camera' + this.props.camera.id.toString();
    var status = undefined;

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
