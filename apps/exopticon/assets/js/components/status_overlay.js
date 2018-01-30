'use strict';

import React from 'react';
import OverlayButton from './overlay_button';

import '../../css/components/status_overlay.css';

class StatusOverlay extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
        <div className="status-overlay">
          <div className="status-message">
            {this.props.status}: {this.props.cameraName}
          </div>
        </div>
    );
  }

}

export default StatusOverlay;
