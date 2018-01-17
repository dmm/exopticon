'use strict';

import React from 'react';
import OverlayButton from './overlay_button';

class StatusOverlay extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
        <div className="status-overlay">
          <span>{this.props.status}</span>
        </div>
    );
  }

}

export default StatusOverlay;
