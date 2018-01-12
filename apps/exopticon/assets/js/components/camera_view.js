'use strict';

import React from 'react';
import CameraOverlay from './camera_overlay';

class CameraView extends React.Component {
  constructor(props) {
    super(props);

    this.handleScroll = this.handleScroll.bind(this);
  }

  isVisible(element) {
    let rec = element.getBoundingClientRect();
    let viewportHeight = window.innerHeight;
    let y = window.scrollY;
    let bottomEdge = y + viewportHeight;

    if ((rec.bottom > 0 && rec.bottom < viewportHeight)
        || (rec.top > 0 && rec.top < viewportHeight)) {
        return true;
    }

    return false;
  }

  isElementInViewport (el) {

    var rect = el.getBoundingClientRect();

    return (
        rect.top >= 0 &&
        rect.left >= 0 &&
        rect.bottom <= (window.innerHeight || document.documentElement.clientHeight) && /*or $(window).height() */
        rect.right <= (window.innerWidth || document.documentElement.clientWidth) /*or $(window).width() */
    );
  }

  handleScroll() {
    const visible = this.isVisible(this._img);
    if (this._img && visible && !this.props.cameraPlayer.playing) {
      //      this.props.cameraPlayer.unpause();
      this.props.cameraPlayer.playRealtime(this._img);
    } else if (this._img && !visible && this.props.cameraPlayer.playing) {
      this.props.cameraPlayer.stop();
    }
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
    return (
      <div id={id} className="camera">
        <CameraOverlay camera={this.props.cameraPlayer} />
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
