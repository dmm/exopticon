'use strict';

import React from 'react';

import '../../css/components/overlay_button.css';

class OverlayButton extends React.Component {
  render() {
    return (
      <button className={'overlay-button ' + this.props.extraClass}
              onClick={this.props.onClick}>
        {this.props.label}
      </button>
    );
  }
}

OverlayButton.defaultProps = {
  onClick: () => {},
  extraClass: '',
  label: ''
};

export default OverlayButton;
